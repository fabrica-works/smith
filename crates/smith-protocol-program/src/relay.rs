//! The agent binary's stdio relay (protocol/programs.md, sections 5.3 and 9).
//! Keeps three stream owners, one chunk per direction and the handshake;
//! knows no MCP messages or policy. `parse` admits loopback startup and
//! `from_io`/`terminate` settle every stream before `ended` becomes visible.

use crate::mcp::{ACK, RELAY_FLAG, SECRET_BYTES, SECRET_ENV};
use alloc::boxed::Box;
use core::net::SocketAddr;
use skein_io::{self as io, Addr};
use skein_lib::{
    Queue, Secret, Token, Writer,
    stream::{Down, OutputDown, OutputOutcome, OutputUp, Read, Up},
};

#[cfg(test)]
mod tests;

const CHUNK: u32 = 4096;

/// Validated relay startup, sent by the shell (protocol/programs.md, section 9).
#[derive(Debug)]
pub struct Target {
    pub addr: SocketAddr,
    pub secret: Secret,
}

/// A startup refusal returned to the shell (protocol/programs.md, section 9).
#[derive(Debug, PartialEq, Eq)]
pub enum Usage {
    /// The flag or argument count is wrong.
    Arguments,
    /// The target is not loopback with a nonzero port.
    Address,
    /// The private environment secret is absent or malformed.
    Secret,
}

/// The relay's settled outcome, returned to the shell (programs.md, section 9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// Both directions reached EOF.
    Done,
    /// The listener could not be reached.
    Unreachable,
    /// The agent did not acknowledge the secret.
    Refused,
    /// A connected stream failed.
    Failed,
    /// SIGTERM ended the relay.
    Terminated,
}

impl Exit {
    /// The binary exit status required by protocol/programs.md, section 9.
    #[must_use]
    pub fn code(self) -> u8 {
        match self {
            Self::Done => 0,
            Self::Unreachable | Self::Refused | Self::Failed => 1,
            Self::Terminated => 143,
        }
    }
}

/// Admit arguments without argv[0], and `NAME=VALUE` environment entries (9).
pub fn parse(arguments: &[Box<[u8]>], environment: &[Box<[u8]>]) -> Result<Target, Usage> {
    if arguments.len() != 2 || arguments.first().expect("two arguments").as_ref() != RELAY_FLAG.as_bytes() {
        return Err(Usage::Arguments);
    }
    #[expect(clippy::disallowed_methods, reason = "this protocol boundary validates the command-line address text")]
    let address = core::str::from_utf8(arguments.last().expect("address argument")).or(Err(Usage::Address))?;
    #[expect(clippy::disallowed_methods, reason = "this protocol boundary parses the required SocketAddr syntax")]
    let addr = address.parse::<SocketAddr>().or(Err(Usage::Address))?;
    if !addr.ip().is_loopback() || addr.port() == 0 {
        return Err(Usage::Address);
    }
    let mut secret = None;
    for entry in environment {
        if let Some(rest) = entry.strip_prefix(SECRET_ENV.as_bytes())
            && let Some(value) = rest.strip_prefix(b"=")
        {
            if secret.is_some() || value.len() != SECRET_BYTES {
                return Err(Usage::Secret);
            }
            for byte in value {
                if !byte.is_ascii_digit() && !(b'a'..=b'f').contains(byte) {
                    return Err(Usage::Secret);
                }
            }
            secret = Some(Secret::new(Box::from(value)));
        }
    }
    Ok(Target { addr, secret: secret.ok_or(Usage::Secret)? })
}

/// Byte-copying step machine hosted by the shell (protocol/programs.md, 5.3, 9).
#[expect(
    clippy::struct_excessive_bools,
    reason = "three independent stream lifecycles, their read/output demands and authentication are tracked separately"
)]
pub struct Relay {
    input: Token,
    output: Token,
    socket_owner: Token,
    socket: Option<Token>,
    open_input: bool,
    open_output: bool,
    open_socket: bool,
    connected: bool,
    handshake: bool,
    outgoing: Option<Secret>,
    incoming: Option<Box<[u8]>>,
    input_end: bool,
    socket_end: bool,
    input_read: bool,
    socket_read: bool,
    socket_write: bool,
    output_write: bool,
    sequence: u64,
    exit: Option<Exit>,
}

impl core::fmt::Debug for Relay {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Relay")
            .field("connected", &self.connected)
            .field("handshake", &self.handshake)
            .field("exit", &self.exit)
            .finish_non_exhaustive()
    }
}

impl Relay {
    /// Connect before reading stdin; the secret is always the first socket send (9).
    #[must_use]
    pub fn new(target: Target, input: Token, output: Token, socket: Token, below: &mut Queue<io::Request>) -> Self {
        below.push(io::Request::Connect { owner: socket, addr: Addr::Ip(target.addr) });
        let mut writer = Writer::new(SECRET_BYTES.checked_add(1).expect("secret line length"));
        writer.put(target.secret.expose()).expect("validated secret length");
        writer.put(b"\n").expect("secret delimiter");
        Self {
            input,
            output,
            socket_owner: socket,
            socket: None,
            open_input: true,
            open_output: true,
            open_socket: true,
            connected: false,
            handshake: true,
            outgoing: Some(Secret::new(writer.finish())),
            incoming: None,
            input_end: false,
            socket_end: false,
            input_read: false,
            socket_read: false,
            socket_write: false,
            output_write: false,
            sequence: 0,
            exit: None,
        }
    }

    /// Whether this owner belongs to a relay stream (protocol/programs.md, 9).
    #[must_use]
    pub fn owns(&self, owner: Token) -> bool {
        owner == self.input || owner == self.output || owner == self.socket_owner
    }

    /// Consume a connect, stream or closure event (protocol/programs.md, 9).
    pub fn from_io(&mut self, event: io::Event, below: &mut Queue<io::Request>) {
        match event {
            io::Event::Connecting { socket, .. } => {
                self.socket = Some(socket);
                if self.exit.is_some() {
                    below.push(io::Request::Abort { entity: socket });
                }
            }
            io::Event::Connected { .. } => self.connected = true,
            io::Event::Failed { .. } => {
                let exit = if !self.connected {
                    Exit::Unreachable
                } else if self.handshake {
                    Exit::Refused
                } else {
                    Exit::Failed
                };
                self.stop(exit, below);
            }
            io::Event::Closed { owner } => {
                if owner == self.input {
                    self.open_input = false;
                } else if owner == self.output {
                    self.open_output = false;
                } else if owner == self.socket_owner {
                    self.open_socket = false;
                    self.socket = None;
                }
            }
            io::Event::Stream { owner, up } => {
                if self.exit.is_none() {
                    self.stream(owner, up, below);
                }
            }
            io::Event::Output { owner, up: OutputUp::Settled { right, outcome } } => {
                if owner == self.output {
                    self.output_write = false;
                } else {
                    self.socket_write = false;
                }
                if self.exit.is_none() {
                    match outcome {
                        OutputOutcome::Granted => self.send(owner, right, below),
                        OutputOutcome::Cancelled | OutputOutcome::Failed(_) => {
                            let exit =
                                if owner == self.socket_owner && self.handshake { Exit::Refused } else { Exit::Failed };
                            self.stop(exit, below);
                        }
                    }
                }
            }
            io::Event::Listening { .. }
            | io::Event::Accepted { .. }
            | io::Event::Spawned { .. }
            | io::Event::Exited { .. }
            | io::Event::Resized
            | io::Event::Window { .. }
            | io::Event::Shutdown { .. }
            | io::Event::Usage { .. } => unreachable!("only relay stream events route here"),
        }
        self.progress(below);
    }

    fn stream(&mut self, owner: Token, up: Up, below: &mut Queue<io::Request>) {
        match up {
            Up::Bytes(bytes) => {
                if owner == self.input {
                    self.input_read = false;
                    self.outgoing = Some(Secret::new(bytes));
                } else {
                    self.socket_read = false;
                    if self.handshake {
                        if bytes.as_ref() == ACK {
                            self.handshake = false;
                        } else {
                            self.stop(Exit::Refused, below);
                        }
                    } else {
                        self.incoming = Some(bytes);
                    }
                }
            }
            Up::End => {
                if owner == self.socket_owner && self.handshake {
                    self.stop(Exit::Refused, below);
                    return;
                }
                if owner == self.input {
                    self.input_end = true;
                    self.input_read = false;
                    below.push(io::Request::Stream {
                        stream: self.input,
                        down: Down::Demand { read: Read::Nothing, room: 0 },
                    });
                    below.push(io::Request::Close { entity: self.input });
                    below.push(io::Request::Stream {
                        stream: self.socket.expect("connected input read"),
                        down: Down::Finish,
                    });
                } else {
                    self.socket_end = true;
                    self.socket_read = false;
                    below.push(io::Request::Stream {
                        stream: self.socket.expect("connected socket read"),
                        down: Down::Demand { read: Read::Nothing, room: 0 },
                    });
                    below.push(io::Request::Stream { stream: self.output, down: Down::Finish });
                    below.push(io::Request::Close { entity: self.output });
                }
                if self.input_end && self.socket_end {
                    self.stop(Exit::Done, below);
                }
            }
            Up::Failed(_) => {
                let exit = if owner == self.socket_owner && self.handshake { Exit::Refused } else { Exit::Failed };
                self.stop(exit, below);
            }
            Up::Room => unreachable!("relay output uses independent grants"),
        }
    }

    fn send(&mut self, owner: Token, right: Token, below: &mut Queue<io::Request>) {
        if owner == self.output {
            let bytes = self.incoming.take().expect("stdout chunk");
            below.push(io::Request::Output { stream: self.output, down: OutputDown::Send { right, bytes } });
        } else {
            let bytes = self.outgoing.take().expect("socket chunk");
            below.push(io::Request::Output {
                stream: self.socket.expect("socket grant"),
                down: OutputDown::Send { right, bytes: Box::from(bytes.expose()) },
            });
        }
    }

    fn progress(&mut self, below: &mut Queue<io::Request>) {
        if self.exit.is_some() || !self.connected {
            return;
        }
        if !self.input_read && !self.input_end && !self.handshake && self.outgoing.is_none() && !self.socket_write {
            self.input_read = true;
            below.push(io::Request::Stream {
                stream: self.input,
                down: Down::Demand { read: Read::Arrived { max: CHUNK }, room: 0 },
            });
        }
        if !self.socket_read
            && !self.socket_end
            && self.incoming.is_none()
            && !self.output_write
            && (!self.handshake || self.outgoing.is_none())
        {
            self.socket_read = true;
            below.push(io::Request::Stream {
                stream: self.socket.expect("connected socket"),
                down: Down::Demand {
                    read: if self.handshake {
                        Read::Line { max: u32::try_from(ACK.len()).expect("short acknowledgement") }
                    } else {
                        Read::Arrived { max: CHUNK }
                    },
                    room: 0,
                },
            });
        }
        if !self.socket_write
            && let Some(bytes) = &self.outgoing
        {
            self.sequence = self.sequence.checked_add(1).expect("relay rights fit process lifetime");
            self.socket_write = true;
            below.push(io::Request::Output {
                stream: self.socket.expect("connected socket"),
                down: OutputDown::Room {
                    right: Token::new(self.sequence),
                    bytes: u32::try_from(bytes.len()).expect("bounded chunk"),
                },
            });
        }
        if !self.output_write
            && let Some(bytes) = &self.incoming
        {
            self.sequence = self.sequence.checked_add(1).expect("relay rights fit process lifetime");
            self.output_write = true;
            below.push(io::Request::Output {
                stream: self.output,
                down: OutputDown::Room {
                    right: Token::new(self.sequence),
                    bytes: u32::try_from(bytes.len()).expect("bounded chunk"),
                },
            });
        }
    }

    fn stop(&mut self, exit: Exit, below: &mut Queue<io::Request>) {
        if self.exit.is_some() {
            return;
        }
        self.exit = Some(exit);
        self.incoming = None;
        self.outgoing = None;
        if self.open_input {
            below.push(io::Request::Abort { entity: self.input });
        }
        if self.open_output {
            if exit == Exit::Done {
                below.push(io::Request::Close { entity: self.output });
            } else {
                below.push(io::Request::Abort { entity: self.output });
            }
        }
        if let Some(socket) = self.socket {
            if exit == Exit::Done {
                below.push(io::Request::Close { entity: socket });
            } else {
                below.push(io::Request::Abort { entity: socket });
            }
        }
    }

    /// Abort on SIGTERM, settling before exit 143 (protocol/programs.md, 9).
    pub fn terminate(&mut self, below: &mut Queue<io::Request>) {
        self.stop(Exit::Terminated, below);
    }

    /// The outcome once stdin, stdout and the socket are closed (programs.md, 9).
    #[must_use]
    pub fn ended(&self) -> Option<Exit> {
        if self.open_input || self.open_output || self.open_socket { None } else { self.exit }
    }

    /// A fixed failure diagnostic for stderr; never contains the secret (9).
    #[must_use]
    pub fn detail(&self) -> Option<&'static [u8]> {
        match self.exit {
            Some(Exit::Unreachable) => Some(b"smith-agent: the relay's agent is unreachable\n"),
            Some(Exit::Refused) => Some(b"smith-agent: the relay's agent refused it\n"),
            Some(Exit::Failed) => Some(b"smith-agent: the relay's connection failed\n"),
            Some(Exit::Done | Exit::Terminated) | None => None,
        }
    }
}
