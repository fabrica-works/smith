//! Bounded event destinations, sharing records and retaining independent delivery state.
//! Each stream keeps its queued lines, byte credit and abandonment notification.
//! No provider syntax or credential is known here. `record` accepts typed records;
//! `up` and `down` meet Skein IO, and `pop_given_up` supplies the shell diagnostic.
//! Contracts: protocol/events.md, sections 5.1–5.3; protocol/agent.md, sections 5–6.

#![expect(clippy::manual_let_else, reason = "bounded production entrances use explicit matches")]

use alloc::boxed::Box;

use skein_io as io;
use skein_lib::{Duration, List, Queue, Time, Token, stream};
use smith_events::{Event, Record};

/// An event destination, named by the shell if IO abandons it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Destination {
    /// The shell's append-only trace file.
    Trace,
    /// The product's standard output stream.
    Output,
}

/// One destination's delivery policy, supplied by its owner at adoption.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Delivery {
    /// A behind destination holds counted records until they can be delivered.
    Complete,
    /// A behind destination counts records it cannot retain as lost.
    BestEffort,
}

impl Delivery {
    /// The codec's total representation of this configured policy.
    #[must_use]
    pub const fn encoded(self) -> smith_events::Delivery {
        match self {
            Delivery::Complete => smith_events::Delivery::Complete,
            Delivery::BestEffort => smith_events::Delivery::BestEffort,
        }
    }
}

/// The owner's content policy; it grants observation, never execution authority.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Capture {
    /// Content-free names, classifications and measurements.
    None,
    /// Also call inputs and failure details.
    Calls,
    /// Also request deltas, completions, results and live text.
    Everything,
}

impl Capture {
    /// The codec's total representation of this configured policy.
    #[must_use]
    pub const fn encoded(self) -> smith_events::Capture {
        match self {
            Capture::None => smith_events::Capture::None,
            Capture::Calls => smith_events::Capture::Calls,
            Capture::Everything => smith_events::Capture::Everything,
        }
    }
}

/// Immutable sink ownership bounds, supplied before the first destination is adopted.
#[derive(Clone, Copy, Debug)]
pub struct SinkLimits {
    pub destinations: u32,
    /// Bytes retained per destination, including three authoritative records' reserved room.
    pub output: u32,
    pub records: smith_events::Limits,
    pub conversations: u32,
}

/// The settled process terminal supplied by the service after its last child closes.
/// Unread IO usage stays unknown; it never changes the answer or the exit.
#[derive(Clone, Copy, Debug)]
pub struct Ended {
    pub exit: u8,
    pub answered: bool,
    pub teardown: Option<Duration>,
    pub usage: io::Measured,
    pub channel: Option<u64>,
}

#[derive(Debug)]
struct Line {
    bytes: Box<[u8]>,
}

#[derive(Clone, Copy, Debug)]
struct Pending {
    bytes: u32,
}

/// The stream's physical lifecycle; Closed is its IO terminal.
#[derive(Clone, Copy, Debug)]
enum Stream {
    /// A queued line needs room, or its prior line is still draining.
    Waiting,
    /// IO has granted its full output credit for one Send.
    Ready,
    /// A full-credit demand is outstanding; its answer confirms the prior Send drained.
    Demanding,
    /// The owner ended the stream and its last bytes are draining.
    Closing,
    /// IO abandoned the destination and owes its Closed terminal.
    GivenUp,
    /// IO's descriptor terminal was received.
    Closed,
}

#[derive(Debug)]
struct Writer {
    destination: Destination,
    token: Token,
    delivery: Delivery,
    stream: Stream,
    lines: Queue<Line>,
    pending: Option<Pending>,
    held: u32,
    loss: u64,
    abandoned: bool,
    notify: bool,
    ended: bool,
}

/// One producer's bounded destinations; each closes after its last queued record.
#[derive(Debug)]
pub struct Sinks {
    limits: SinkLimits,
    capture: Capture,
    writers: List<Writer>,
    line: u32,
    reserve: u32,
    /// IO's actual output cap: a full-credit grant proves the previous line drained.
    io_output: u32,
    counts: [u64; TYPES],
    ended: bool,
}

const TYPES: usize = 18;
const AUTHORITATIVE: u32 = 3;
// Every codec line contains v, type and t_ms, already longer than sixteen bytes.
const MINIMUM_LINE: u32 = 16;

impl Sinks {
    /// Construct the destinations with a checked bound and the actual adopted IO output cap.
    #[must_use]
    pub fn new(limits: &SinkLimits, capture: Capture, io_output: u32) -> Sinks {
        assert!(sinks_worst_case(limits, capture).is_some(), "startup checked the sink's ownership bound");
        let line = largest_line(&limits.records, capture).expect("startup checked the produced record bound");
        assert!(io_output >= line, "every line fits the actual adopted IO output cap");
        Sinks {
            limits: *limits,
            capture,
            writers: List::with_capacity(limits.destinations),
            line,
            reserve: line.checked_mul(AUTHORITATIVE).expect("startup checked authoritative credit"),
            io_output,
            counts: [0; TYPES],
            ended: false,
        }
    }

    /// Adopt an IO stream exactly once, or return its destination to the caller.
    pub fn add(&mut self, destination: Destination, token: Token, delivery: Delivery) -> Result<(), Destination> {
        for writer in &self.writers {
            if writer.destination == destination || writer.token == token {
                return Err(destination);
            }
        }
        if self.writers.room() == 0 {
            return Err(destination);
        }
        self.writers
            .push(Writer {
                destination,
                token,
                delivery,
                stream: Stream::Waiting,
                lines: Queue::with_capacity(slots(&self.limits).expect("startup checked queue cells")),
                pending: None,
                held: 0,
                loss: 0,
                abandoned: false,
                notify: false,
                ended: false,
            })
            .expect("destination room checked");
        Ok(())
    }

    /// Whether every writing complete destination can retain the next counted record.
    #[must_use]
    pub fn takes(&self) -> bool {
        for writer in &self.writers {
            match writer.delivery {
                Delivery::Complete if !writer.abandoned && !writer.ended => {
                    if !room(writer, self.limits.output, self.reserve, self.line) {
                        return false;
                    }
                }
                Delivery::Complete | Delivery::BestEffort => {}
            }
        }
        true
    }

    /// Encode one record to every destination, counting it once per type.
    /// Only an accepted result's text and fields are captured until the content increment.
    /// The owner calls this only while takes holds, except for the three authoritative records.
    pub fn record(&mut self, record: &Record) {
        assert!(!self.ended, "no record follows the process terminal");
        let capture = match &record.event {
            Event::RunCompleted(_) => self.capture.encoded(),
            Event::SessionStarted(_)
            | Event::SessionEnded(_)
            | Event::RunStarted(_)
            | Event::ConversationOpened(_)
            | Event::ConversationClosed(_)
            | Event::ResponseStarted(_)
            | Event::ResponseCompleted(_)
            | Event::TextDelta(_)
            | Event::ToolStarted(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Notice(_)
            | Event::Budget(_) => smith_events::Capture::None,
        };
        let bytes = match smith_events::write(record, &capture, &self.limits.records)
            .expect("typed sources fit startup's receiving bounds")
        {
            Some(bytes) => bytes,
            None => return,
        };
        let index = kind(&record.event);
        let count = self.counts.get_mut(index).expect("the event has a fixed vocabulary index");
        *count = count.checked_add(1).expect("one process cannot exhaust a record count");
        let authoritative = authoritative(&record.event);
        for index in 0..self.writers.len() {
            let writer = self.writers.get_mut(index).expect("bounded destinations");
            offer(writer, bytes.clone(), authoritative, self.limits.output, self.reserve, self.line);
        }
    }

    /// Emit one authoritative end, with each destination's own loss, then close its stream.
    /// The owner supplies the process clock origin, independently of drain time.
    pub fn ended(&mut self, now: Time, started: Time, ended: &Ended) {
        assert!(!self.ended, "one authoritative end per process");
        self.ended = true;
        let count = self.counts.get_mut(1).expect("session.ended vocabulary index");
        *count = count.checked_add(1).expect("one process cannot exhaust a record count");
        for index in 0..self.writers.len() {
            let loss = self.writers.get(index).expect("bounded destinations").loss;
            let record = ended_record(now, started, ended, self.emitted(), loss);
            let bytes = smith_events::write(&record, &smith_events::Capture::None, &self.limits.records)
                .expect("the settled end fits its checked record bound")
                .expect("the process terminal survives every capture");
            let writer = self.writers.get_mut(index).expect("bounded destinations");
            offer(writer, bytes, true, self.limits.output, self.reserve, self.line);
        }
        self.finish();
    }

    /// The emitted table, including authoritative types and the end record itself once made.
    #[must_use]
    pub fn emitted(&self) -> List<smith_events::Count> {
        let mut result = List::with_capacity(u32::try_from(TYPES).expect("fixed vocabulary fits u32"));
        for index in 0..TYPES {
            let count = *self.counts.get(index).expect("bounded vocabulary index");
            if count > 0 {
                result
                    .push(smith_events::Count { record: names(index).into(), count })
                    .expect("one entry per vocabulary type");
            }
        }
        result
    }

    /// Stop accepting records and close each destination after its queued last line drains.
    pub fn finish(&mut self) {
        for index in 0..self.writers.len() {
            self.writers.get_mut(index).expect("bounded destinations").ended = true;
        }
    }

    /// Whether an IO token belongs to a destination retained until Closed.
    #[must_use]
    pub fn owns(&self, token: Token) -> bool {
        for writer in &self.writers {
            if writer.token == token {
                return true;
            }
        }
        false
    }

    /// The destination's IO event, including the exact terminal after abandonment.
    pub fn up(&mut self, event: io::Event) {
        let token = match &event {
            io::Event::Stream { owner, .. } | io::Event::Failed { owner, .. } | io::Event::Closed { owner } => *owner,
            io::Event::Listening { .. }
            | io::Event::Accepted { .. }
            | io::Event::Connecting { .. }
            | io::Event::Connected { .. }
            | io::Event::Output { .. }
            | io::Event::Spawned { .. }
            | io::Event::Exited { .. }
            | io::Event::Usage { .. }
            | io::Event::Shutdown { .. } => return,
        };
        for index in 0..self.writers.len() {
            let writer = self.writers.get_mut(index).expect("bounded destinations");
            if writer.token == token {
                received(writer, event);
                return;
            }
        }
    }

    /// Bounded demands, owning sends and closes, within the caller's output room.
    pub fn down(&mut self, out: &mut Queue<io::Request>) {
        for index in 0..self.writers.len() {
            if out.room() == 0 {
                return;
            }
            let writer = self.writers.get_mut(index).expect("bounded destinations");
            match writer.stream {
                Stream::Waiting => {
                    if !writer.lines.is_empty() || writer.pending.is_some() {
                        out.push(io::Request::Stream {
                            stream: writer.token,
                            down: stream::Down::Demand { read: stream::Read::Nothing, room: self.io_output },
                        });
                        writer.stream = Stream::Demanding;
                    } else if writer.ended {
                        out.push(io::Request::Close { entity: writer.token });
                        writer.stream = Stream::Closing;
                    }
                }
                Stream::Ready => match writer.lines.pop() {
                    Some(line) => {
                        writer.pending = Some(Pending {
                            bytes: u32::try_from(line.bytes.len()).expect("a line fits its checked byte bound"),
                        });
                        out.push(io::Request::Stream { stream: writer.token, down: stream::Down::Send(line.bytes) });
                        writer.stream = Stream::Waiting;
                    }
                    None if writer.ended => {
                        out.push(io::Request::Close { entity: writer.token });
                        writer.stream = Stream::Closing;
                    }
                    None => {}
                },
                Stream::Demanding | Stream::Closing | Stream::GivenUp | Stream::Closed => {}
            }
        }
    }

    /// One newly abandoned destination for the shell, exactly once.
    pub fn pop_given_up(&mut self) -> Option<Destination> {
        for index in 0..self.writers.len() {
            let writer = self.writers.get_mut(index).expect("bounded destinations");
            if writer.notify {
                writer.notify = false;
                return Some(writer.destination);
            }
        }
        None
    }

    /// Every retained destination reached its physical IO terminal.
    #[must_use]
    pub fn closed(&self) -> bool {
        for writer in &self.writers {
            match writer.stream {
                Stream::Closed => {}
                Stream::Waiting | Stream::Ready | Stream::Demanding | Stream::Closing | Stream::GivenUp => {
                    return false;
                }
            }
        }
        true
    }
}

#[expect(clippy::manual_map, reason = "the strict subset requires an explicit match instead of a closure")]
fn ended_record(now: Time, started: Time, ended: &Ended, emitted: List<smith_events::Count>, loss: u64) -> Record {
    let (cpu_ms, peak_rss_bytes) = match ended.usage {
        io::Measured::Read(usage) => (
            Some(smith_events::Cpu {
                user: usage.own.user.as_nanos() / 1_000_000,
                system: usage.own.system.as_nanos() / 1_000_000,
                children_user: usage.children.user.as_nanos() / 1_000_000,
                children_system: usage.children.system.as_nanos() / 1_000_000,
            }),
            Some(smith_events::PeakRss {
                self_bytes: usage.own.peak_rss_bytes,
                children: usage.children.peak_rss_bytes,
            }),
        ),
        io::Measured::Unread => (None, None),
    };
    Record {
        t_ms: now.saturating_since(started).as_nanos() / 1_000_000,
        event: Event::SessionEnded(smith_events::SessionEnded {
            exit: i64::from(ended.exit),
            answered: ended.answered,
            teardown_ms: match ended.teardown {
                Some(duration) => Some(duration.as_nanos() / 1_000_000),
                None => None,
            },
            emitted,
            loss: smith_events::Loss { events: loss, channel: ended.channel },
            cpu_ms,
            peak_rss_bytes,
        }),
    }
}

fn room(writer: &Writer, output: u32, reserve: u32, line: u32) -> bool {
    let available = output.checked_sub(writer.held).expect("writer stays within byte credit");
    available >= reserve.checked_add(line).expect("startup checked ordinary plus authoritative credit")
        && writer.lines.room() > AUTHORITATIVE
}

fn offer(writer: &mut Writer, bytes: Box<[u8]>, authoritative: bool, output: u32, reserve: u32, line: u32) {
    if writer.abandoned {
        lost(writer);
        return;
    }
    assert!(!writer.ended, "no record follows session.ended");
    if !authoritative && !room(writer, output, reserve, line) {
        match writer.delivery {
            Delivery::Complete => unreachable!("the owner checks takes before draining a fact"),
            Delivery::BestEffort => {
                lost(writer);
                return;
            }
        }
    }
    let length = u32::try_from(bytes.len()).expect("a record fits its checked bound");
    writer.held = writer.held.checked_add(length).expect("startup checked retained byte credit");
    assert!(writer.held <= output, "authoritative room was reserved before the run");
    writer.lines.push(Line { bytes });
}

fn received(writer: &mut Writer, event: io::Event) {
    match event {
        io::Event::Stream { up, .. } => match up {
            stream::Up::Room => {
                match writer.stream {
                    Stream::Demanding => {}
                    Stream::Waiting | Stream::Ready | Stream::Closing | Stream::GivenUp | Stream::Closed => return,
                }
                if let Some(pending) = writer.pending.take() {
                    writer.held = writer.held.checked_sub(pending.bytes).expect("pending bytes retain credit");
                }
                writer.stream = Stream::Ready;
            }
            stream::Up::Failed(_) => abandon(writer),
            stream::Up::End => {}
            stream::Up::Bytes(_) => unreachable!("an append destination never requests input"),
        },
        io::Event::Failed { .. } => abandon(writer),
        io::Event::Closed { .. } => {
            match writer.stream {
                Stream::Closing | Stream::GivenUp | Stream::Closed => {}
                Stream::Waiting | Stream::Ready | Stream::Demanding => abandon(writer),
            }
            writer.stream = Stream::Closed;
        }
        io::Event::Listening { .. }
        | io::Event::Accepted { .. }
        | io::Event::Connecting { .. }
        | io::Event::Connected { .. }
        | io::Event::Output { .. }
        | io::Event::Spawned { .. }
        | io::Event::Exited { .. }
        | io::Event::Usage { .. }
        | io::Event::Shutdown { .. } => {}
    }
}

fn abandon(writer: &mut Writer) {
    if writer.abandoned {
        return;
    }
    writer.abandoned = true;
    writer.notify = true;
    if writer.pending.take().is_some() {
        lost(writer);
    }
    for _ in 0..writer.lines.capacity() {
        match writer.lines.pop() {
            Some(_) => lost(writer),
            None => break,
        }
    }
    writer.held = 0;
    writer.stream = Stream::GivenUp;
}

fn lost(writer: &mut Writer) {
    writer.loss = writer.loss.checked_add(1).expect("one process cannot exhaust a record count");
}

fn authoritative(event: &Event) -> bool {
    match event {
        Event::SessionStarted(_) | Event::RunCompleted(_) | Event::SessionEnded(_) => true,
        Event::Budget(_)
        | Event::RunStarted(_)
        | Event::ConversationOpened(_)
        | Event::ConversationClosed(_)
        | Event::ResponseStarted(_)
        | Event::ResponseCompleted(_)
        | Event::TextDelta(_)
        | Event::ToolStarted(_)
        | Event::ToolCompleted(_)
        | Event::CheckStarted(_)
        | Event::CheckCompleted(_)
        | Event::MessageReceived(_)
        | Event::MessageRead(_)
        | Event::MessageUnread(_)
        | Event::Notice(_) => false,
    }
}

fn kind(event: &Event) -> usize {
    match event {
        Event::SessionStarted(_) => 0,
        Event::SessionEnded(_) => 1,
        Event::RunStarted(_) => 2,
        Event::RunCompleted(_) => 3,
        Event::ConversationOpened(_) => 4,
        Event::ConversationClosed(_) => 5,
        Event::ResponseStarted(_) => 6,
        Event::ResponseCompleted(_) => 7,
        Event::TextDelta(_) => 8,
        Event::ToolStarted(_) => 9,
        Event::ToolCompleted(_) => 10,
        Event::CheckStarted(_) => 11,
        Event::CheckCompleted(_) => 12,
        Event::Notice(_) => 13,
        Event::Budget(_) => 14,
        Event::MessageReceived(_) => 15,
        Event::MessageRead(_) => 16,
        Event::MessageUnread(_) => 17,
    }
}

fn names(index: usize) -> &'static [u8] {
    [
        b"session.started".as_slice(),
        b"session.ended",
        b"run.started",
        b"run.completed",
        b"conversation.opened",
        b"conversation.closed",
        b"response.started",
        b"response.completed",
        b"text.delta",
        b"tool.started",
        b"tool.completed",
        b"check.started",
        b"check.completed",
        b"notice",
        b"budget",
        b"message.received",
        b"message.read",
        b"message.unread",
    ]
    .get(index)
    .copied()
    .expect("bounded vocabulary index")
}

fn slots(limits: &SinkLimits) -> Option<u32> {
    limits.output.checked_div(MINIMUM_LINE)?.checked_add(AUTHORITATIVE)
}

fn largest_line(limits: &smith_events::Limits, capture: Capture) -> Option<u32> {
    let scalar = smith_events::largest_record_for(&smith_events::Capture::None, limits)?;
    match capture {
        Capture::None | Capture::Calls => Some(scalar),
        Capture::Everything => {
            // The record kind controls the size formula; no answer content is allocated here.
            let answer = crate::answer_record(
                crate::AnswerMeta { run: 0, wind_down: false, read: None, unread: 0, elapsed: Duration::ZERO },
                &smith_domain::run::Answer::Parked { spent: smith_domain::run::Spend::ZERO, turns: 0 },
                Capture::None,
            );
            Some(scalar.max(smith_events::largest_size(
                &Event::RunCompleted(answer),
                &smith_events::Capture::Everything,
                limits,
            )?))
        }
    }
}

/// Peak heap of the destination queues and their bounded output, including codec construction.
#[must_use]
pub fn sinks_worst_case(limits: &SinkLimits, capture: Capture) -> Option<u64> {
    if limits.records.items < u32::try_from(TYPES).ok()? {
        return None;
    }
    for index in 0..TYPES {
        if u32::try_from(names(index).len()).ok()? > limits.records.string {
            return None;
        }
    }
    let line = largest_line(&limits.records, capture)?;
    if limits.output < line.checked_mul(AUTHORITATIVE.checked_add(1)?)? {
        return None;
    }
    let writer = Queue::<Line>::worst_case(slots(limits)?)?.checked_add(u64::from(limits.output))?;
    // The measured writer can retain its encoded body and final newline-owning line together.
    List::<Writer>::worst_case(limits.destinations)?
        .checked_add(u64::from(limits.destinations).checked_mul(writer)?)?
        .checked_add(u64::from(line).checked_mul(2)?)?
        .checked_add(List::<smith_events::Count>::worst_case(u32::try_from(TYPES).ok()?)?)?
        .checked_add(u64::try_from(TYPES).ok()?.checked_mul(u64::from(limits.records.string))?)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::disallowed_macros,
        clippy::disallowed_types,
        clippy::disallowed_methods,
        reason = "programming-model.md section 10.2: deterministic harnesses use ordinary Rust"
    )]

    use super::*;

    fn limits(destinations: u32) -> SinkLimits {
        let records = smith_events::Limits { string: 64, content: 1024, items: 18 };
        let line = largest_line(&records, Capture::None).unwrap();
        SinkLimits { destinations, output: line.checked_mul(5).unwrap(), records, conversations: 1 }
    }

    #[test]
    fn only_everything_captures_the_exact_accepted_report_text_and_fields() {
        let actual = smith_domain::run::Answer::Accepted {
            outcome: smith_domain::run::outcome::Declared::Report(smith_domain::run::outcome::Report {
                text: b"Frozen task token: ALPHA.\n\"quoted\"".to_vec().into(),
                fields: Box::new([smith_domain::run::outcome::Field {
                    name: b"source".to_vec().into(),
                    value: b"actual\nreport".to_vec().into(),
                }]),
            }),
            spent: smith_domain::run::Spend {
                units: 17,
                input: Some(19),
                output: Some(23),
                ..smith_domain::run::Spend::ZERO
            },
            turns: 2,
        };
        for capture in [Capture::None, Capture::Calls, Capture::Everything] {
            let mut limits = limits(1);
            limits.output = largest_line(&limits.records, capture).unwrap() * 5;
            let token = Token::new(11);
            let mut sinks = Sinks::new(&limits, capture, limits.output);
            sinks.add(Destination::Trace, token, Delivery::Complete).unwrap();
            let answer = crate::answer_record(
                crate::AnswerMeta {
                    run: 7,
                    wind_down: true,
                    read: Some(31),
                    unread: 1,
                    elapsed: Duration::from_millis(29),
                },
                &actual,
                capture,
            );
            sinks.record(&Record { t_ms: 37, event: Event::RunCompleted(answer) });
            // Everything grants only the newly moved accepted-result content in 02.5.
            let request =
                golden(include_bytes!("../../smith-events/golden/v1/response_started_everything.jsonl"), &limits);
            sinks.record(&request);
            sinks.finish();
            let records = drain(&mut sinks, token);
            let Event::RunCompleted(answer) = &records[0].event else { panic!("typed answer") };
            assert_eq!(answer.status, smith_events::Status::Accepted);
            assert_eq!(
                (answer.run, answer.turns, answer.spent, answer.read, answer.unread, answer.duration_ms),
                (7, 2, 17, Some(31), 1, 29)
            );
            assert!(answer.wind_down);
            assert_eq!(answer.usage.input_tokens, Some(19));
            assert_eq!(answer.usage.output_tokens, Some(23));
            assert_eq!(answer.usage.reasoning_tokens, None);
            let result = answer.result.as_ref().unwrap();
            assert_eq!(result.form, smith_events::Form::Report);
            if capture == Capture::Everything {
                assert_eq!(result.text.as_deref(), Some(b"Frozen task token: ALPHA.\n\"quoted\"".as_slice()));
                assert_eq!(
                    result.fields.as_ref().unwrap().as_slice(),
                    &[smith_events::Field { name: b"source".to_vec().into(), text: b"actual\nreport".to_vec().into() }]
                );
            } else {
                assert_eq!(result.text, None);
                assert_eq!(result.fields, None);
            }
            let Event::ResponseStarted(request) = &records[1].event else { panic!("typed request") };
            assert_eq!(request.prompt, None, "prompt content belongs to 02.6");
        }
    }

    #[test]
    fn the_complete_emitted_vocabulary_must_fit_the_receiving_bounds() {
        let mut bounds = limits(1);
        bounds.records.string = 18;
        assert!(sinks_worst_case(&bounds, Capture::None).is_none());
        bounds.records.string = 19;
        assert!(sinks_worst_case(&bounds, Capture::None).is_some());
        bounds.records.items = 17;
        assert!(sinks_worst_case(&bounds, Capture::None).is_none());
    }

    #[test]
    fn settled_end_preserves_read_or_unread_resources_and_the_actual_exit() {
        let measured = io::kernel::Usage {
            own: io::kernel::Resources {
                user: Duration::from_nanos(1_000_000),
                system: Duration::from_nanos(2_000_000),
                peak_rss_bytes: 5,
            },
            children: io::kernel::Resources {
                user: Duration::from_nanos(3_000_000),
                system: Duration::from_nanos(4_000_000),
                peak_rss_bytes: 6,
            },
        };
        for usage in [io::Measured::Read(measured), io::Measured::Unread] {
            for exit in [0, 1] {
                let limits = limits(1);
                let token = Token::new(11);
                let mut sinks = Sinks::new(&limits, Capture::None, limits.output);
                sinks.add(Destination::Trace, token, Delivery::Complete).unwrap();
                sinks.record(&counted(1));
                let ended = Ended {
                    exit,
                    answered: exit == 0,
                    teardown: if exit == 0 { Some(Duration::from_nanos(3_000_000)) } else { None },
                    usage,
                    channel: Some(7),
                };
                sinks.ended(Time::from_nanos(12_000_000), Time::from_nanos(2_000_000), &ended);
                let records = drain(&mut sinks, token);
                assert!(sinks.closed());
                assert_eq!(records.len(), 2);
                let last = records.last().unwrap();
                assert_eq!(last.t_ms, 10);
                let Event::SessionEnded(end) = &last.event else { panic!("one final process end") };
                assert_eq!(end.exit, i64::from(exit));
                assert_eq!(end.answered, exit == 0);
                assert_eq!(end.teardown_ms, if exit == 0 { Some(3) } else { None });
                assert_eq!(end.loss, smith_events::Loss { events: 0, channel: Some(7) });
                assert_eq!(end.emitted.iter().map(|entry| entry.count).sum::<u64>(), 2);
                assert_eq!(
                    end.emitted.iter().find(|entry| entry.record.as_ref() == b"session.ended").unwrap().count,
                    1
                );
                match usage {
                    io::Measured::Read(_) => {
                        assert_eq!(
                            end.cpu_ms,
                            Some(smith_events::Cpu { user: 1, system: 2, children_user: 3, children_system: 4 })
                        );
                        assert_eq!(end.peak_rss_bytes, Some(smith_events::PeakRss { self_bytes: 5, children: 6 }));
                    }
                    io::Measured::Unread => {
                        assert_eq!(end.cpu_ms, None);
                        assert_eq!(end.peak_rss_bytes, None);
                    }
                }
            }
        }
    }

    #[test]
    fn each_settled_end_reports_its_destination_loss_and_counts_itself_once() {
        let limits = limits(2);
        let token = Token::new(11);
        let mut sinks = Sinks::new(&limits, Capture::None, limits.output);
        sinks.add(Destination::Trace, token, Delivery::BestEffort).unwrap();
        sinks.add(Destination::Output, Token::new(12), Delivery::BestEffort).unwrap();
        sinks.up(io::Event::Failed { owner: Token::new(12), error: io::Error::Other });
        sinks.up(io::Event::Closed { owner: Token::new(12) });
        for name in 0..10_000 {
            sinks.record(&counted(name));
        }
        let ended = Ended { exit: 0, answered: true, teardown: None, usage: io::Measured::Unread, channel: None };
        sinks.ended(Time::ZERO, Time::ZERO, &ended);
        let records = drain(&mut sinks, token);
        let Event::SessionEnded(end) = &records.last().unwrap().event else { panic!("end is last") };
        assert_eq!(end.emitted.iter().map(|entry| entry.count).sum::<u64>(), 10_001);
        assert_eq!(u64::try_from(records.len()).unwrap().checked_add(end.loss.events).unwrap(), 10_001);
        assert_eq!(sinks.writers.get(1).unwrap().loss, 10_001);
        assert_eq!(sinks.pop_given_up(), Some(Destination::Output));
        assert_eq!(sinks.pop_given_up(), None);
        assert!(sinks.closed());
    }

    fn counted(name: u64) -> Record {
        Record { t_ms: name, event: Event::MessageUnread(smith_events::MessageUnread { run: 7, name }) }
    }

    fn golden(bytes: &[u8], limits: &SinkLimits) -> Record {
        smith_events::read(bytes, &limits.records).unwrap().unwrap()
    }

    fn grant(sinks: &mut Sinks, token: Token) {
        sinks.up(io::Event::Stream { owner: token, up: stream::Up::Room });
    }

    fn drain(sinks: &mut Sinks, token: Token) -> Vec<Record> {
        let mut out = Queue::with_capacity(2);
        let mut read = Vec::new();
        for _ in 0_u32..10_000 {
            sinks.down(&mut out);
            match out.pop() {
                Some(io::Request::Stream { down: stream::Down::Demand { room, .. }, .. }) => {
                    assert_eq!(room, sinks.io_output);
                    grant(sinks, token);
                }
                Some(io::Request::Stream { down: stream::Down::Send(bytes), .. }) => {
                    read.push(smith_events::read(&bytes, &sinks.limits.records).unwrap().unwrap());
                }
                Some(io::Request::Close { .. }) => {
                    sinks.up(io::Event::Closed { owner: token });
                    return read;
                }
                None => panic!("writer failed to advance"),
                Some(other) => panic!("unexpected append request: {other:?}"),
            }
        }
        panic!("bounded writer did not close");
    }

    #[test]
    fn a_slow_complete_destination_preserves_order_and_reserves_authoritative_room() {
        let limits = limits(1);
        let token = Token::new(11);
        let mut sinks = Sinks::new(&limits, Capture::None, limits.output);
        sinks.add(Destination::Trace, token, Delivery::Complete).unwrap();
        let mut expected = Vec::new();
        let start = golden(include_bytes!("../../smith-events/golden/v1/session_started_none.jsonl"), &limits);
        sinks.record(&start);
        expected.push(start);
        for name in 0..10_000 {
            if !sinks.takes() {
                break;
            }
            let record = counted(name);
            sinks.record(&record);
            expected.push(record);
        }
        assert!(!sinks.takes(), "the actual output cap holds the next fact");
        let completed = golden(include_bytes!("../../smith-events/golden/v1/run_completed_none.jsonl"), &limits);
        sinks.record(&completed);
        expected.push(completed);
        let end = golden(include_bytes!("../../smith-events/golden/v1/session_ended_none.jsonl"), &limits);
        sinks.record(&end);
        expected.push(end);
        sinks.finish();
        assert_eq!(drain(&mut sinks, token), expected);
        assert_eq!(sinks.writers.get(0).unwrap().loss, 0);
        assert!(sinks.closed());
        let emitted = sinks.emitted();
        assert_eq!(emitted.iter().map(|entry| entry.count).sum::<u64>(), u64::try_from(expected.len()).unwrap());
    }

    #[test]
    fn best_effort_loses_only_the_counted_records_that_find_it_behind() {
        let limits = limits(1);
        let token = Token::new(11);
        let mut sinks = Sinks::new(&limits, Capture::None, limits.output);
        sinks.add(Destination::Trace, token, Delivery::BestEffort).unwrap();
        for name in 0..10_000 {
            sinks.record(&counted(name));
        }
        assert!(sinks.takes(), "best effort never holds its owner");
        let lost = sinks.writers.get(0).unwrap().loss;
        assert!(lost > 0);
        sinks.finish();
        let records = drain(&mut sinks, token);
        assert_eq!(u64::try_from(records.len()).unwrap().checked_add(lost).unwrap(), 10_000);
        assert_eq!(sinks.emitted().get(0).unwrap().count, 10_000);
    }

    #[test]
    fn a_failed_stream_counts_queued_inflight_and_later_records_and_notifies_once() {
        let limits = limits(1);
        let token = Token::new(11);
        let mut sinks = Sinks::new(&limits, Capture::None, limits.output);
        sinks.add(Destination::Trace, token, Delivery::Complete).unwrap();
        sinks.record(&counted(1));
        let mut out = Queue::with_capacity(2);
        sinks.down(&mut out);
        let Some(io::Request::Stream { down: stream::Down::Demand { .. }, .. }) = out.pop() else {
            panic!("actual full-credit demand")
        };
        grant(&mut sinks, token);
        sinks.down(&mut out);
        let Some(io::Request::Stream { down: stream::Down::Send(_), .. }) = out.pop() else {
            panic!("one owning send")
        };
        sinks.record(&counted(2));
        sinks.up(io::Event::Stream { owner: token, up: stream::Up::Failed(stream::Fault::Other) });
        assert_eq!(sinks.pop_given_up(), Some(Destination::Trace));
        assert_eq!(sinks.pop_given_up(), None);
        sinks.record(&counted(3));
        assert!(sinks.takes());
        assert_eq!(sinks.writers.get(0).unwrap().loss, 3);
        sinks.up(io::Event::Failed { owner: token, error: io::Error::TimedOut });
        assert_eq!(sinks.pop_given_up(), None);
        assert!(!sinks.closed(), "the append descriptor still owes Closed");
        sinks.up(io::Event::Closed { owner: token });
        assert!(sinks.closed());
        sinks.record(&counted(4));
        assert_eq!(sinks.writers.get(0).unwrap().loss, 4);
        sinks.down(&mut out);
        assert!(out.is_empty(), "an abandoned destination is never written again");
    }

    #[test]
    fn one_destination_failing_does_not_abandon_the_other() {
        let limits = limits(2);
        let token = Token::new(11);
        let mut sinks = Sinks::new(&limits, Capture::None, limits.output);
        sinks.add(Destination::Trace, token, Delivery::Complete).unwrap();
        sinks.add(Destination::Output, Token::new(12), Delivery::Complete).unwrap();
        sinks.record(&counted(1));
        sinks.up(io::Event::Failed { owner: Token::new(12), error: io::Error::Other });
        sinks.up(io::Event::Closed { owner: Token::new(12) });
        sinks.record(&counted(2));
        assert_eq!(sinks.pop_given_up(), Some(Destination::Output));
        sinks.finish();
        assert_eq!(drain(&mut sinks, token), [counted(1), counted(2)]);
        assert_eq!(sinks.writers.get(0).unwrap().loss, 0);
        assert_eq!(sinks.writers.get(1).unwrap().loss, 2);
        assert!(sinks.closed());
    }
}
