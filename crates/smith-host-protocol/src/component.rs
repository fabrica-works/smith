//! Host-side channel (protocol/hosts.md, section 2; domain/host.md, section 6).
//! Keeps the opening, one application demand and at most one held closure reason,
//! never host policy or read-ahead records. `open` runs the handshake; `read`
//! demands one record, live piece or end; `send_*`, `from_below` and `fire`
//! translate output and stream events without spending that demand.
use alloc::boxed::Box;
use skein_channel::{Closed, Event, Lower, LowerEvent, Machine, Request, Role, Schema, StreamMode, frame_writer};
use skein_lib::{Duration, Map, Queue, Reader, Time, Token, Writer};
use smith_host_domain::channel;

use crate::answer::decode_answer;
use crate::limits::{Error, Limits};
use crate::translate::{Values, decode_ask, encode_reply, encode_start};

mod notices;

const RULES: u16 = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Opening,
    Opened,
    Started,
    Admitted,
    Answered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CallKey {
    activation: u64,
    completion: u32,
    position: u32,
}

impl CallKey {
    fn from_wire(name: &smith_channel::CallName) -> CallKey {
        CallKey { activation: name.activation(), completion: name.completion(), position: name.position() }
    }

    fn domain(self) -> channel::CallName {
        channel::CallName { activation: self.activation, completion: self.completion, position: self.position }
    }
}

/// An opening result for the host service.
#[derive(Debug, PartialEq, Eq)]
pub enum OpenEvent {
    /// Framing and terms have been agreed at this version.
    Opened { version: u16 },
    /// The channel ended before or after opening.
    Hangup { why: Closed },
    /// The framed Start entered the channel's bounded output queue.
    Sent { token: Token },
    /// The Start did not enter the channel's output queue.
    Unsent { token: Token, why: skein_channel::Unsent },
    /// The agent's last word in host-domain vocabulary, with its wire record for audit.
    Answer { answer: channel::Answer, record: smith_channel::Answer },
    /// The agent admitted the Start before its later final answer.
    Admitted,
    /// The run refused one named host message at ingress.
    MessageRefused { name: Token, reason: smith_host_domain::MessageRefusal },
    /// The run is waiting after reading the named message, if any.
    Waiting { read: Option<Token> },
    /// The run announced a long operation with a bounded progress extension.
    Long { operation: smith_domain::Operation, span: Duration },
    /// The run ended its previously announced long operation.
    LongDone,
    /// One concrete turn and its checked host envelope.
    Turn { turn: channel::Turn },
    /// One content-free, best-effort fact kept in its bounded wire body.
    Fact { body: Box<[u8]> },
    /// One live piece checked against the start and stream order, for the owner.
    Live {
        turn: u32,
        retry: u32,
        kind: smith_channel::LiveKind,
        offset: u64,
        bytes: Box<[u8]>,
        call: Option<smith_channel::LiveCall>,
    },
    /// A provider rejected this known account and credential generation.
    Unauthorized { account: u32, generation: u64 },
    /// A provider exhausted this known account for a relative cooldown.
    Limited { account: u32, reset: Option<Duration> },
    /// A provider refused an account request, with bounded code and reason.
    Refused { account: u32, code: Box<[u8]>, reason: Box<[u8]> },
    /// The independent write stream failed while reads may still settle.
    WriteFailed,
    /// A named operation whose one host terminal remains owed.
    Call { call: Token, name: channel::CallName, deadline: Time, ask: channel::Ask },
    /// The agent withdrew a call; its terminal still remains owed.
    Withdraw { call: Token },
}

/// The initiator's framed channel and checked sending terms.
#[derive(Debug)]
pub struct Component {
    machine: Machine,
    schema: Schema,
    bodies: smith_channel::Limits,
    events: Queue<Event>,
    phase: Phase,
    ended: bool,
    reading: bool,
    held: Option<Closed>,
    calls_by_name: Map<CallKey, Token>,
    calls_by_token: Map<Token, CallKey>,
    next_call: u64,
    now: Time,
    cancelled: bool,
    endpoints: smith_protocol_channel::Endpoints,
    charter: smith_charter::v4::Limits,
    transcript: smith_transcript::v6::Limits,
    sequence: u32,
    long_open: u32,
    live: crate::live::Streams,
}

impl Component {
    /// Build one channel with a pipe pair or one socket-like stream.
    pub fn new(
        limits: &Limits,
        mode: StreamMode,
        endpoints: smith_protocol_channel::Endpoints,
    ) -> Result<Component, Error> {
        if !endpoints.fits(limits.endpoints, limits.charter.llm_endpoint) {
            return Err(Error::Value);
        }
        let schema = match smith_protocol_channel::schema_for_start(&limits.bodies, limits.start_bytes) {
            Ok(schema) => schema,
            Err(error) => return Err(Error::Codec(error)),
        };
        let machine = match Machine::with_mode(schema.clone(), Role::Initiator, limits.channel, mode) {
            Ok(machine) => machine,
            Err(error) => return Err(Error::Channel(error)),
        };
        Ok(Component {
            machine,
            schema,
            bodies: limits.bodies,
            events: Queue::with_capacity(8),
            phase: Phase::Opening,
            ended: false,
            reading: false,
            held: None,
            calls_by_name: Map::with_capacity(limits.calls),
            calls_by_token: Map::with_capacity(limits.calls),
            next_call: 1,
            now: Time::ZERO,
            cancelled: false,
            endpoints,
            charter: limits.charter,
            transcript: limits.transcript,
            sequence: 0,
            long_open: 0,
            live: crate::live::Streams::new(None, 0),
        })
    }

    /// Start the pipe opening with its empty credential.
    pub fn open(&mut self, to_service: &mut Queue<OpenEvent>, below: &mut Queue<Lower>) {
        self.machine.down(Request::Open { credential: Box::default() }, &mut self.events, below);
        self.fire(to_service, below);
    }

    /// Demand one record, one live piece or the channel's end, answered once.
    pub fn read(&mut self, to_service: &mut Queue<OpenEvent>, below: &mut Queue<Lower>) {
        assert!(self.phase != Phase::Opening, "application reads follow opening");
        assert!(!self.reading, "one application read at a time");
        assert!(!self.ended, "no read after the delivered end");
        self.reading = true;
        match self.held.take() {
            Some(why) => self.end(why, to_service),
            None => {
                self.machine.down(Request::Read, &mut self.events, below);
                self.drain(to_service, below);
                self.fire(to_service, below);
            }
        }
    }

    /// Set the monotonic clock used to turn agent-relative deadlines into host deadlines.
    pub fn set_now(&mut self, now: Time) {
        self.now = now;
    }

    /// Submit the first application record after opening, with host service values.
    pub fn send_start(
        &mut self,
        start: channel::Start,
        window: channel::Window,
        values: Values,
        token: Token,
        to_service: &mut Queue<OpenEvent>,
        below: &mut Queue<Lower>,
    ) -> Result<(), Error> {
        let sequence = match &start.transcript {
            Some(history) => match history.value().turns.last() {
                Some(turn) => turn.sequence,
                None => 0,
            },
            None => 0,
        };
        let live = start.live;
        let frame =
            encode_start(start, window, values, &self.bodies, &self.charter, &self.transcript, &self.endpoints)?;
        if self.phase != Phase::Opened {
            return Err(Error::MissingValue);
        }
        self.live = crate::live::Streams::new(live, 0);
        self.sequence = sequence;
        self.phase = Phase::Started;
        self.machine.down(Request::Send { token, frame }, &mut self.events, below);
        self.drain(to_service, below);
        self.fire(to_service, below);
        Ok(())
    }

    /// Relay one named sender label and text to the admitted agent.
    pub fn send_message(
        &mut self,
        message: channel::Message,
        token: Token,
        to_service: &mut Queue<OpenEvent>,
        below: &mut Queue<Lower>,
    ) -> Result<(), Error> {
        if self.phase != Phase::Admitted {
            return Err(Error::Order);
        }
        let record = crate::translate::encode_message(message, &self.bodies)?;
        let Ok(length) = usize::try_from(record.measure()) else {
            return Err(Error::MissingValue);
        };
        let mut body = Writer::new(length);
        record.encode(&mut body)?;
        let mut frame = frame_writer(0x0101, record.measure())?;
        frame.put(&body.finish())?;
        self.machine.down(Request::Send { token, frame: frame.finish()? }, &mut self.events, below);
        self.drain(to_service, below);
        self.fire(to_service, below);
        Ok(())
    }

    /// Answer one admitted agent call, keeping its durable wire name.
    pub fn send_reply(
        &mut self,
        call: Token,
        reply: channel::Reply,
        token: Token,
        to_service: &mut Queue<OpenEvent>,
        below: &mut Queue<Lower>,
    ) -> Result<(), Error> {
        if self.phase != Phase::Admitted {
            return Err(Error::Order);
        }
        let key = *self.calls_by_token.get(&call).ok_or(Error::Calls)?;
        let frame = encode_reply(key.domain(), reply, &self.bodies)?;
        self.calls_by_token.remove(&call);
        self.calls_by_name.remove(&key);
        self.machine.down(Request::Send { token, frame }, &mut self.events, below);
        self.drain(to_service, below);
        self.fire(to_service, below);
        Ok(())
    }

    /// Acknowledge exact durable commitment of one forwarded turn.
    pub fn send_acknowledge(
        &mut self,
        turn: u32,
        token: Token,
        to_service: &mut Queue<OpenEvent>,
        below: &mut Queue<Lower>,
    ) -> Result<(), Error> {
        if self.phase != Phase::Admitted {
            return Err(Error::Order);
        }
        let record = smith_channel::Acknowledge::new(&self.bodies, smith_channel::AcknowledgeParts { number: turn })?;
        let Ok(length) = usize::try_from(record.measure()) else {
            return Err(Error::MissingValue);
        };
        let mut body = Writer::new(length);
        record.encode(&mut body)?;
        let mut frame = frame_writer(0x0103, record.measure())?;
        frame.put(&body.finish())?;
        self.machine.down(Request::Send { token, frame: frame.finish()? }, &mut self.events, below);
        self.drain(to_service, below);
        self.fire(to_service, below);
        Ok(())
    }

    /// Refresh one named grant with the service-owned credential value.
    pub fn send_grant(
        &mut self,
        grant: channel::Grant,
        credential: smith_protocol_channel::GrantValue,
        token: Token,
        to_service: &mut Queue<OpenEvent>,
        below: &mut Queue<Lower>,
    ) -> Result<(), Error> {
        if self.phase != Phase::Admitted {
            return Err(Error::Order);
        }
        if crate::grant_form(&credential)? != grant.form {
            return Err(Error::Grants);
        }
        let value = crate::translate::grant_value(credential, &self.bodies)?;
        let grant = smith_channel::Grant::new(
            &self.bodies,
            smith_channel::GrantParts {
                account: grant.account,
                generation: grant.generation,
                valid: grant.valid,
                value,
            },
        )?;
        let record = smith_channel::GrantRefresh::new(&self.bodies, smith_channel::GrantRefreshParts { grant })?;
        let Ok(length) = usize::try_from(record.measure()) else {
            return Err(Error::MissingValue);
        };
        let mut body = Writer::new(length);
        record.encode(&mut body)?;
        let mut frame = frame_writer(0x0104, record.measure())?;
        frame.put(&body.finish())?;
        self.machine.down(Request::Send { token, frame: frame.finish()? }, &mut self.events, below);
        self.drain(to_service, below);
        self.fire(to_service, below);
        Ok(())
    }

    /// Ask the agent to stop politely once, preserving answers owed to calls.
    pub fn send_cancel(
        &mut self,
        token: Token,
        to_service: &mut Queue<OpenEvent>,
        below: &mut Queue<Lower>,
    ) -> Result<(), Error> {
        if self.phase != Phase::Started && self.phase != Phase::Admitted || self.cancelled {
            return Err(Error::Order);
        }
        let record = smith_channel::Cancel::new(&self.bodies, smith_channel::CancelParts {})?;
        let Ok(length) = usize::try_from(record.measure()) else {
            return Err(Error::MissingValue);
        };
        let mut body = Writer::new(length);
        record.encode(&mut body)?;
        let mut frame = frame_writer(0x0105, record.measure())?;
        frame.put(&body.finish())?;
        self.cancelled = true;
        self.machine.down(Request::Send { token, frame: frame.finish()? }, &mut self.events, below);
        self.drain(to_service, below);
        self.fire(to_service, below);
        Ok(())
    }

    /// Consume one stream event and progress the opening.
    pub fn from_below(&mut self, event: LowerEvent, to_service: &mut Queue<OpenEvent>, below: &mut Queue<Lower>) {
        self.machine.up(event, &mut self.events, below);
        self.drain(to_service, below);
        self.fire(to_service, below);
    }

    /// Request the next read or write operation.
    pub fn fire(&mut self, to_service: &mut Queue<OpenEvent>, below: &mut Queue<Lower>) {
        self.machine.poll(&mut self.events, below);
        self.drain(to_service, below);
    }

    /// Whether the channel can take application records.
    #[must_use]
    pub fn opened(&self) -> bool {
        self.phase != Phase::Opening
    }

    #[expect(
        clippy::manual_map,
        clippy::too_many_lines,
        reason = "bounded channel event dispatch uses explicit matches"
    )]
    fn drain(&mut self, to_service: &mut Queue<OpenEvent>, below: &mut Queue<Lower>) {
        for _ in 0_u8..8_u8 {
            let event = self.events.pop();
            match event {
                Some(Event::Opening { .. }) => {
                    self.machine.down(
                        Request::Refuse { reason: RULES, text: Box::from(*b"unexpected opening") },
                        &mut self.events,
                        below,
                    );
                }
                Some(Event::Ready { version, terms }) => {
                    match smith_channel::peer_terms_gap(&self.schema, Role::Initiator, version, &terms) {
                        Some(kind) => {
                            self.machine.down(
                                Request::Refuse { reason: 2, text: kind_text(kind) },
                                &mut self.events,
                                below,
                            );
                        }
                        None => {
                            self.phase = Phase::Opened;
                            to_service.push(OpenEvent::Opened { version });
                        }
                    }
                }
                Some(Event::Body { kind, body }) => match kind {
                    0x0111 if self.phase == Phase::Admitted => {
                        match smith_channel::MessageRefused::decode(&self.bodies, &mut Reader::new(&body)) {
                            Ok(record) => {
                                let reason = match record.reason() {
                                    smith_channel::MessageRefusal::TooLarge => {
                                        smith_host_domain::MessageRefusal::TooLarge
                                    }
                                    smith_channel::MessageRefusal::Full => smith_host_domain::MessageRefusal::Full,
                                    smith_channel::MessageRefusal::NameInUse => {
                                        smith_host_domain::MessageRefusal::NameInUse
                                    }
                                    smith_channel::MessageRefusal::Ending => smith_host_domain::MessageRefusal::Ending,
                                };
                                self.deliver(
                                    OpenEvent::MessageRefused { name: Token::new(record.name()), reason },
                                    to_service,
                                );
                            }
                            Err(_) => self.refuse_rules(b"invalid answer", below),
                        }
                    }
                    0x0107 if self.phase == Phase::Admitted => {
                        match smith_channel::Call::decode(&self.bodies, &mut Reader::new(&body)) {
                            Ok(record) => {
                                let key = CallKey::from_wire(record.name());
                                if self.calls_by_name.contains_key(&key)
                                    || self.calls_by_name.len() >= self.calls_by_name.capacity()
                                {
                                    self.refuse_rules(b"invalid answer", below);
                                } else {
                                    match decode_ask(&record) {
                                        Ok(Some(ask)) => {
                                            let call = Token::new(self.next_call);
                                            match self.next_call.checked_add(1) {
                                                Some(next) => {
                                                    self.next_call = next;
                                                    if self.calls_by_name.insert(key, call).is_err()
                                                        || self.calls_by_token.insert(call, key).is_err()
                                                    {
                                                        self.refuse_rules(b"invalid answer", below);
                                                    } else {
                                                        self.deliver(
                                                            OpenEvent::Call {
                                                                call,
                                                                name: key.domain(),
                                                                deadline: self.now.saturating_add(record.deadline()),
                                                                ask,
                                                            },
                                                            to_service,
                                                        );
                                                    }
                                                }
                                                None => self.refuse_rules(b"invalid answer", below),
                                            }
                                        }
                                        Ok(None) | Err(_) => self.refuse_rules(b"invalid answer", below),
                                    }
                                }
                            }
                            Err(_) => self.refuse_rules(b"invalid answer", below),
                        }
                    }
                    0x0108 if self.phase == Phase::Admitted => {
                        match smith_channel::Withdraw::decode(&self.bodies, &mut Reader::new(&body)) {
                            Ok(record) => {
                                let key = CallKey::from_wire(record.name());
                                match self.calls_by_name.get(&key) {
                                    Some(call) => {
                                        self.deliver(OpenEvent::Withdraw { call: *call }, to_service);
                                    }
                                    None => self.refuse_rules(b"invalid answer", below),
                                }
                            }
                            Err(_) => self.refuse_rules(b"invalid answer", below),
                        }
                    }
                    0x010a if self.phase == Phase::Admitted => {
                        match smith_channel::Waiting::decode(&self.bodies, &mut Reader::new(&body)) {
                            Ok(waiting) => {
                                let read = match waiting.last_read() {
                                    Some(name) => Some(Token::new(*name)),
                                    None => None,
                                };
                                self.deliver(OpenEvent::Waiting { read }, to_service);
                            }
                            Err(_) => self.refuse_rules(b"invalid answer", below),
                        }
                    }
                    0x0109 if self.phase == Phase::Admitted => {
                        match smith_channel::Turn::decode(&self.bodies, &mut Reader::new(&body)) {
                            Ok(turn) => {
                                let decoded = match self.sequence.checked_add(1) {
                                    Some(sequence) => smith_protocol_channel::decode_turn(
                                        turn.body(),
                                        sequence,
                                        &self.transcript,
                                        &self.endpoints,
                                    )
                                    .ok(),
                                    None => None,
                                };
                                match decoded {
                                    Some(value) => {
                                        self.sequence = value.sequence;
                                        self.live.turn(turn.number());
                                        let body = smith_host_domain::TurnValue::new(value, u64::MAX)
                                            .expect("codec bounds ensure representable ownership");
                                        self.deliver(
                                            OpenEvent::Turn {
                                                turn: channel::Turn {
                                                    number: turn.number(),
                                                    opens_window: turn.opens_window(),
                                                    spent: turn.spent(),
                                                    read: match turn.last_read() {
                                                        Some(name) => Some(Token::new(*name)),
                                                        None => None,
                                                    },
                                                    body,
                                                },
                                            },
                                            to_service,
                                        );
                                    }
                                    None => self.refuse_rules(b"invalid answer", below),
                                }
                            }
                            Err(_) => self.refuse_rules(b"invalid answer", below),
                        }
                    }
                    0x0112 => {
                        if self.phase != Phase::Admitted {
                            self.refuse_rules(b"live admission", below);
                            continue;
                        }
                        match smith_channel::Live::decode(&self.bodies, &mut Reader::new(&body)) {
                            Ok(piece) => match self.live.check(&piece) {
                                Ok(()) => {
                                    self.deliver(
                                        OpenEvent::Live {
                                            turn: piece.turn(),
                                            retry: piece.retry(),
                                            kind: piece.kind().clone(),
                                            offset: piece.offset(),
                                            bytes: Box::from(piece.bytes()),
                                            call: piece.call().clone(),
                                        },
                                        to_service,
                                    );
                                }
                                Err(rule) => self.refuse_rules(rule.reason(), below),
                            },
                            Err(_) => self.refuse_rules(b"live body", below),
                        }
                    }
                    0x010f if self.phase == Phase::Admitted => {
                        match smith_channel::Fact::decode(&self.bodies, &mut Reader::new(&body)) {
                            Ok(_) => {
                                self.deliver(OpenEvent::Fact { body }, to_service);
                            }
                            Err(_) => self.refuse_rules(b"invalid answer", below),
                        }
                    }
                    0x010d | 0x010e | 0x0113 if self.phase == Phase::Admitted => {
                        match notices::decode(&self.bodies, kind, &body) {
                            Ok(notice) => {
                                self.deliver(notice, to_service);
                            }
                            Err(_) => self.refuse_rules(b"invalid answer", below),
                        }
                    }
                    0x010b if self.phase == Phase::Admitted && self.long_open < u32::MAX => {
                        match smith_channel::Long::decode(&self.bodies, &mut Reader::new(&body)) {
                            Ok(long) => {
                                let operation = match long.operation() {
                                    smith_channel::Operation::Check => smith_domain::Operation::Checks,
                                    smith_channel::Operation::Completion => smith_domain::Operation::Completion,
                                    smith_channel::Operation::Command => smith_domain::Operation::Command,
                                };
                                self.long_open = self.long_open.checked_add(1).expect("bounded active operations");
                                self.deliver(OpenEvent::Long { operation, span: long.span() }, to_service);
                            }
                            Err(_) => self.refuse_rules(b"invalid answer", below),
                        }
                    }
                    0x010c if self.phase == Phase::Admitted && self.long_open > 0 => {
                        match smith_channel::LongDone::decode(&self.bodies, &mut Reader::new(&body)) {
                            Ok(_) => {
                                self.long_open = self.long_open.checked_sub(1).expect("one begun operation ends");
                                self.deliver(OpenEvent::LongDone, to_service);
                            }
                            Err(_) => self.refuse_rules(b"invalid answer", below),
                        }
                    }
                    0x0106 if self.phase == Phase::Started => {
                        match smith_channel::Admitted::decode(&self.bodies, &mut Reader::new(&body)) {
                            Ok(_) => {
                                self.phase = Phase::Admitted;
                                self.deliver(OpenEvent::Admitted, to_service);
                            }
                            Err(_) => self.refuse_rules(b"invalid answer", below),
                        }
                    }
                    0x0110 => match smith_channel::Answer::decode(&self.bodies, &mut Reader::new(&body)) {
                        Ok(record) => {
                            let permitted = match record.result() {
                                smith_channel::RunResult::Refused(_) => self.phase == Phase::Started,
                                smith_channel::RunResult::Accepted(_)
                                | smith_channel::RunResult::Parked
                                | smith_channel::RunResult::Failed(_) => self.phase == Phase::Admitted,
                            };
                            if permitted {
                                match decode_answer(&record, &self.charter) {
                                    Ok(answer) => {
                                        self.phase = Phase::Answered;
                                        self.deliver(OpenEvent::Answer { answer, record }, to_service);
                                    }
                                    Err(_) => self.refuse_rules(b"invalid answer", below),
                                }
                            } else {
                                self.refuse_rules(b"invalid answer", below);
                            }
                        }
                        Err(_) => self.refuse_rules(b"invalid answer", below),
                    },
                    _ => self.refuse_rules(b"invalid answer", below),
                },
                Some(Event::Closed { why }) => self.end(why, to_service),
                Some(Event::Ended) => {
                    // Clean EOF between demands stays in the machine's ReadState::Ended.
                    if self.reading {
                        self.end(Closed::Stream, to_service);
                    }
                }
                Some(Event::Sent { token }) => {
                    to_service.push(OpenEvent::Sent { token });
                }
                Some(Event::Unsent { token, why }) => {
                    to_service.push(OpenEvent::Unsent { token, why });
                }
                Some(Event::Ping | Event::Unsupported { .. }) => {
                    if self.reading {
                        self.machine.down(Request::Read, &mut self.events, below);
                    }
                }
                Some(Event::Refused { .. } | Event::Drained) | None => {}
                Some(Event::OutputFailed) => to_service.push(OpenEvent::WriteFailed),
            }
        }
    }

    fn deliver(&mut self, event: OpenEvent, to_service: &mut Queue<OpenEvent>) {
        assert!(self.reading, "one record answers one application read");
        self.reading = false;
        to_service.push(event);
    }

    fn end(&mut self, why: Closed, to_service: &mut Queue<OpenEvent>) {
        if self.ended || self.held.is_some() {
            return;
        }
        if self.phase == Phase::Opening || self.reading {
            self.reading = false;
            self.ended = true;
            to_service.push(OpenEvent::Hangup { why });
        } else {
            self.held = Some(why);
        }
    }

    fn refuse_rules(&mut self, reason: &[u8], below: &mut Queue<Lower>) {
        self.machine.down(Request::Refuse { reason: RULES, text: Box::from(reason) }, &mut self.events, below);
    }
}

fn kind_text(kind: u16) -> Box<[u8]> {
    let bytes = kind.to_be_bytes();
    Box::from([
        b'k',
        b'i',
        b'n',
        b'd',
        b' ',
        b'0',
        b'x',
        hex(bytes[0] >> 4),
        hex(bytes[0] & 15),
        hex(bytes[1] >> 4),
        hex(bytes[1] & 15),
    ])
}

fn hex(nibble: u8) -> u8 {
    if nibble < 10 {
        b'0'.checked_add(nibble).expect("hex digit fits")
    } else {
        b'a'.checked_add(nibble.checked_sub(10).expect("letter digit")).expect("hex digit fits")
    }
}

#[cfg(test)]
mod tests {
    use super::{Component, Event, OpenEvent, Phase};
    use alloc::boxed::Box;
    use skein_channel::{Lower, LowerEvent, StreamMode};
    use skein_lib::{Queue, Writer, stream};

    fn component() -> Component {
        let mut names = skein_lib::List::with_capacity(1);
        names.push(smith_protocol_channel::Endpoint { name: Box::new([]), number: 0, dialect: 0 }).expect("endpoint");
        let limits = crate::Limits {
            bodies: smith_channel::CEILINGS,
            charter: smith_charter::CEILINGS,
            transcript: smith_transcript::CEILINGS,
            endpoints: 1,
            calls: 1,
            start_bytes: u32::try_from(
                smith_channel::v5::Start::worst_case_bytes(&smith_channel::CEILINGS).expect("Start ceiling"),
            )
            .expect("Start frame range"),
            channel: skein_channel::Limits {
                chunk: 8,
                credential: 0,
                skip: 8,
                output_bytes: u32::MAX,
                output_frames: 2,
                kinds: 20,
            },
        };
        let mut component =
            Component::new(&limits, StreamMode::Two, smith_protocol_channel::Endpoints::new(names)).expect("component");
        component.phase = Phase::Admitted;
        component.reading = true;
        component
    }

    fn turn_body(payload: Box<[u8]>) -> Box<[u8]> {
        let record = smith_channel::Turn::new(
            &smith_channel::CEILINGS,
            smith_channel::TurnParts { number: 1, spent: 2, last_read: None, opens_window: true, body: payload },
        )
        .expect("envelope");
        let mut writer = Writer::new(usize::try_from(record.measure()).expect("measure"));
        record.encode(&mut writer).expect("encode");
        writer.finish()
    }

    fn accepted_body(payload: Box<[u8]>) -> Box<[u8]> {
        let result =
            smith_channel::Accepted::new(&smith_channel::CEILINGS, smith_channel::AcceptedParts { result: payload })
                .expect("accepted");
        let result =
            smith_channel::RunResult::new(&smith_channel::CEILINGS, smith_channel::RunResult::Accepted(result))
                .expect("result");
        let record = smith_channel::Answer::new(
            &smith_channel::CEILINGS,
            smith_channel::AnswerParts { turns: 0, spent: 0, last_read: None, result },
        )
        .expect("answer");
        let mut writer = Writer::new(usize::try_from(record.measure()).expect("measure"));
        record.encode(&mut writer).expect("encode");
        writer.finish()
    }

    #[test]
    fn malformed_typed_bodies_send_rules_before_the_domain_hears_payloads() {
        for (kind, body) in [
            (0x0109, turn_body(Box::from(&b"malformed turn"[..]))),
            (0x0110, accepted_body(Box::from(&b"malformed result"[..]))),
        ] {
            let mut component = component();
            let mut above = Queue::with_capacity(8);
            let mut below = Queue::with_capacity(16);
            component.events.push(Event::Body { kind, body });
            component.drain(&mut above, &mut below);
            component.fire(&mut above, &mut below);
            assert!(above.is_empty(), "malformed value never reaches the host domain");
            assert!(component.reading, "refusal preserves demand until Hangup");
            let Some(Lower::Write(stream::OutputDown::Room { right, .. })) = below.pop() else {
                panic!("rules refusal requests write room")
            };
            component.from_below(
                LowerEvent::Write(stream::OutputUp::Settled { right, outcome: stream::OutputOutcome::Granted }),
                &mut above,
                &mut below,
            );
            let Some(Lower::Write(stream::OutputDown::Send { bytes, .. })) = below.pop() else {
                panic!("rules refusal is sent")
            };
            assert_eq!(skein_channel::parse_header(&bytes[..8]).expect("header").kind, 3);
            assert_eq!(u16::from_be_bytes([bytes[8], bytes[9]]), super::RULES);
        }
    }

    #[test]
    fn a_closed_reason_waits_for_one_read_without_being_overwritten() {
        let mut component = component();
        component.reading = false;
        let mut above = Queue::with_capacity(8);
        let mut below = Queue::with_capacity(16);
        component.events.push(Event::Closed { why: skein_channel::Closed::Truncated });
        component.events.push(Event::Closed { why: skein_channel::Closed::Stream });
        component.events.push(Event::Ended);
        component.drain(&mut above, &mut below);
        assert!(above.is_empty(), "closure between reads is held");
        component.read(&mut above, &mut below);
        assert_eq!(above.pop(), Some(OpenEvent::Hangup { why: skein_channel::Closed::Truncated }));
        assert!(!component.reading);
        assert!(below.is_empty(), "held closure answers synchronously");
        component.events.push(Event::Closed { why: skein_channel::Closed::Stream });
        component.events.push(Event::Ended);
        component.drain(&mut above, &mut below);
        assert!(above.is_empty(), "duplicate closure cannot repeat the end");
    }

    #[test]
    fn a_concrete_turn_and_result_reach_the_host_as_equal_values() {
        let mut component = component();
        let source = smith_host_world::turn_value(512).into_value();
        let bytes = smith_protocol_channel::encode_turn(&source, &component.transcript, &component.endpoints)
            .expect("turn codec");
        let mut above = Queue::with_capacity(8);
        let mut below = Queue::with_capacity(16);
        component.events.push(Event::Body { kind: 0x0109, body: turn_body(bytes) });
        component.drain(&mut above, &mut below);
        let Some(OpenEvent::Turn { turn }) = above.pop() else { panic!("typed turn") };
        assert_eq!(turn.body.into_value(), source);
        let source = smith_host_world::declared_value(32).into_value();
        let bytes = smith_protocol_channel::encode_result(&source, &component.charter).expect("result codec");
        component.reading = true;
        component.events.push(Event::Body { kind: 0x0110, body: accepted_body(bytes) });
        component.drain(&mut above, &mut below);
        let Some(OpenEvent::Answer { answer, .. }) = above.pop() else { panic!("typed result") };
        let smith_host_domain::RunResult::Accepted { outcome } = answer.result else { panic!("accepted result") };
        assert_eq!(outcome.into_value(), source);
    }
}

#[cfg(test)]
#[path = "component/tests.rs"]
mod grant_tests;
