//! A short program child (protocol/programs.md, sections 3 and 4.1).
//! Keeps the child, two output pipes and bounded captures until closure;
//! knows neither source credentials nor host vocabulary. `from_io` and
//! `fire` settle the child before `ended` exposes its borrowed record.

use alloc::boxed::Box;
use core::mem::size_of;
use skein_io::{self as io, kernel};
use skein_lib::{List, Queue, Time, Token, stream};

use crate::STATUS_BYTES;

const READ_BYTES: u32 = 4096;

/// A caller's executable, arguments, whole environment and working directory.
pub struct Launch {
    pub program: Box<[u8]>,
    pub arguments: Box<[Box<[u8]>]>,
    pub environment: Box<[Box<[u8]>]>,
    pub root: kernel::Fd,
    pub directory: Box<[u8]>,
}

impl core::fmt::Debug for Launch {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.debug_struct("Launch").field("environment_entries", &self.environment.len()).finish_non_exhaustive()
    }
}

/// A settled program command returned to its host or agent reader.
#[derive(Debug, PartialEq, Eq)]
pub enum Ended<'a> {
    /// The program could not start; all spawn resources have settled.
    Unspawned { detail: &'a [u8] },
    /// The host's deadline killed the child; its resources have settled.
    Deadline { detail: &'a [u8] },
    /// The child exited or was signalled; stdout is absent past `STATUS_BYTES`.
    Exited { exit: kernel::Exit, stdout: Option<&'a [u8]>, detail: &'a [u8] },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Active,
    ChildClosed,
    Done,
}

/// One program command driven by its caller through I/O events and a deadline.
#[derive(Debug)]
pub struct Command {
    io_owner: Token,
    child: Option<Token>,
    pipes: [Option<Token>; 2],
    deadline: Time,
    phase: Phase,
    timed_out: bool,
    exit: Option<kernel::Exit>,
    stdout: List<u8>,
    stderr: List<u8>,
    tail_next: u32,
}

/// Command captures, one maximal delivery per output and its status-reading peak.
#[must_use]
pub fn command_worst_case(detail_bytes: u32) -> Option<u64> {
    u64::try_from(size_of::<Command>())
        .ok()?
        .checked_add(List::<u8>::worst_case(STATUS_BYTES.checked_add(1)?)?)?
        .checked_add(List::<u8>::worst_case(detail_bytes)?.checked_mul(2)?)?
        .checked_add(u64::from(READ_BYTES).checked_mul(2)?)?
        .checked_add(crate::status::worst_case()?)
}

impl Command {
    /// Spawn with no stdin pipe, bounded stdout and a stderr tail (programs.md, 4.2).
    pub fn new(
        io_owner: Token,
        launch: Launch,
        deadline: Time,
        detail_bytes: u32,
        below: &mut Queue<io::Request>,
    ) -> Self {
        below.push(io::Request::Spawn {
            owner: io_owner,
            spawn: kernel::Spawn {
                program: launch.program,
                args: launch.arguments,
                env: launch.environment,
                root: launch.root,
                dir: launch.directory,
                pipes: Box::new([
                    kernel::Pipe { child: 1, way: kernel::Way::Out, parent: None },
                    kernel::Pipe { child: 2, way: kernel::Way::Out, parent: None },
                ]),
            },
        });
        Self {
            io_owner,
            child: None,
            pipes: [None, None],
            deadline,
            phase: Phase::Active,
            timed_out: false,
            exit: None,
            stdout: List::with_capacity(STATUS_BYTES.checked_add(1).expect("status sentinel")),
            stderr: List::with_capacity(detail_bytes),
            tail_next: 0,
        }
    }

    /// Whether the caller should route this owner to the command.
    #[must_use]
    pub fn owns(&self, owner: Token) -> bool {
        self.io_owner == owner || self.pipes.contains(&Some(owner))
    }

    /// Consume one child or output-pipe event, retaining closure evidence.
    pub fn from_io(&mut self, event: io::Event, below: &mut Queue<io::Request>) {
        match event {
            io::Event::Spawned { child, pipes, .. } => {
                assert_eq!(pipes.len(), 2, "command asks for two output pipes");
                self.child = Some(child);
                for (index, pipe) in pipes.iter().enumerate() {
                    *self.pipes.get_mut(index).expect("two output slots") = Some(*pipe);
                    Self::demand(*pipe, below);
                }
                if self.timed_out {
                    self.stop(below);
                }
            }
            io::Event::Stream { owner, up } => match up {
                stream::Up::Bytes(bytes) => {
                    if self.pipes.first().copied().expect("stdout slot") == Some(owner) {
                        for byte in bytes {
                            if self.stdout.room() > 0 {
                                self.stdout.push(byte).expect("stdout room");
                            }
                        }
                    } else {
                        self.keep_tail(&bytes);
                    }
                    if !self.timed_out {
                        Self::demand(owner, below);
                    }
                }
                stream::Up::End => below.push(io::Request::Close { entity: owner }),
                stream::Up::Failed(_) => {
                    // A broken capture cannot be read as a complete status.
                    for _ in 0..self.stdout.room() {
                        self.stdout.push(0).expect("stdout sentinel room");
                    }
                    below.push(io::Request::Abort { entity: owner });
                }
                stream::Up::Room => unreachable!("output pipes ask for no write room"),
            },
            io::Event::Exited { exit, .. } => {
                self.exit = Some(exit);
                if let Some(child) = self.child {
                    below.push(io::Request::Close { entity: child });
                }
            }
            io::Event::Failed { .. } => {
                self.stop(below);
                if let Some(child) = self.child {
                    below.push(io::Request::Close { entity: child });
                }
            }
            io::Event::Closed { owner } => {
                if owner == self.io_owner {
                    self.phase = Phase::ChildClosed;
                } else {
                    for pipe in &mut self.pipes {
                        if *pipe == Some(owner) {
                            *pipe = None;
                        }
                    }
                }
            }
            io::Event::Listening { .. }
            | io::Event::Accepted { .. }
            | io::Event::Connecting { .. }
            | io::Event::Connected { .. }
            | io::Event::Output { .. }
            | io::Event::Resized
            | io::Event::Window { .. }
            | io::Event::Shutdown { .. }
            | io::Event::Usage { .. } => unreachable!("only child and output events route to a command"),
        }
        self.settle();
    }

    /// Kill at the deadline, waiting for child and pipe closures before ending.
    pub fn fire(&mut self, now: Time, below: &mut Queue<io::Request>) {
        if self.phase != Phase::Done && !self.timed_out && now >= self.deadline {
            self.cancel(below);
        }
    }

    /// Cancel once, retaining the child and pipes until their close reports.
    pub fn cancel(&mut self, below: &mut Queue<io::Request>) {
        if self.phase != Phase::Done && !self.timed_out {
            self.timed_out = true;
            self.stop(below);
        }
    }

    /// The host deadline while the command is active and has not timed out.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Time> {
        if self.phase == Phase::Done || self.timed_out { None } else { Some(self.deadline) }
    }

    /// The ending once the child and both output pipes have settled.
    #[must_use]
    pub fn ended(&self) -> Option<Ended<'_>> {
        if self.phase != Phase::Done {
            return None;
        }
        let detail = self.stderr.as_slice();
        if self.timed_out {
            return Some(Ended::Deadline { detail });
        }
        match self.exit {
            Some(exit) => Some(Ended::Exited {
                exit,
                stdout: if self.stdout.len() > STATUS_BYTES { None } else { Some(self.stdout.as_slice()) },
                detail,
            }),
            None => Some(Ended::Unspawned { detail }),
        }
    }

    fn demand(pipe: Token, below: &mut Queue<io::Request>) {
        below.push(io::Request::Stream {
            stream: pipe,
            down: stream::Down::Demand { read: stream::Read::Arrived { max: READ_BYTES }, room: 0 },
        });
    }

    fn stop(&self, below: &mut Queue<io::Request>) {
        if let Some(child) = self.child {
            below.push(io::Request::Signal { child, to: kernel::Target::Child, signal: kernel::Signal::Kill });
        }
        for pipe in self.pipes.into_iter().flatten() {
            below.push(io::Request::Abort { entity: pipe });
        }
    }

    fn keep_tail(&mut self, bytes: &[u8]) {
        for byte in bytes {
            if self.stderr.room() > 0 {
                self.stderr.push(*byte).expect("stderr room");
            } else if self.stderr.capacity() > 0 {
                *self.stderr.get_mut(self.tail_next).expect("tail slot") = *byte;
                self.tail_next = self
                    .tail_next
                    .checked_add(1)
                    .expect("tail index")
                    .checked_rem(self.stderr.capacity())
                    .expect("nonempty tail");
            }
        }
    }

    fn settle(&mut self) {
        if self.phase != Phase::ChildClosed || self.pipes != [None, None] {
            return;
        }
        if self.tail_next > 0 {
            let mut tail = List::with_capacity(self.stderr.len());
            for index in 0..self.stderr.len() {
                let at = self
                    .tail_next
                    .checked_add(index)
                    .expect("tail offset")
                    .checked_rem(self.stderr.len())
                    .expect("nonempty tail");
                tail.push(*self.stderr.get(at).expect("tail slot")).expect("tail room");
            }
            self.stderr = tail;
            self.tail_next = 0;
        }
        self.phase = Phase::Done;
    }
}
