//! Attempt output and caller-owned deadlines on Skein's real world.
//! The pipe observer retains bounded bytes and parsed Codex records, never
//! agent state. Binary owns process-tree discovery, signals and settlement.
//! Contract: benchmarks.md, sections 6, 8 and 10.

use skein_io::kernel::{Complete, Done, Fd, Op, Signal, Submit};
use skein_lib::{Duration, Queue, Time, Token, Wall};
use skein_shell::Clock;
use skein_world::end_to_end::{Binary, Command, Expectation, Mode, Streams};
use skein_world::{Host, Referee, real};

use crate::agent::codex;

/// Application output interpretation selected by the prepared invocation.
#[derive(Debug)]
pub enum Capture {
    /// Codex's JSON-lines stream, including the final turn's answer.
    Codex,
    /// Setup or grading output with no agent terminal vocabulary.
    Raw,
    /// The observer paired with an adapter's pinned preparation.
    Agent(Box<crate::agent::Observing>),
}

/// The caller's complete deadline and retained-log bounds.
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub hard: Duration,
    pub teardown: Duration,
    pub grace: Duration,
    pub settlement: Duration,
    pub stdout: usize,
    pub stderr: usize,
    pub force_walk: bool,
}

/// Settled process evidence, on one clock, with raw output bounded in flight.
#[derive(Debug)]
pub struct ProcessResult {
    pub exit: crate::Exit,
    pub forced: crate::Forced,
    pub start: Time,
    pub start_wall: Wall,
    pub end: Time,
    pub answered: Option<Time>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub observer: codex::Observer,
    pub observing: Option<Box<crate::agent::Observing>>,
    pub error: Option<String>,
    pub counts: skein_world::end_to_end::Counts,
    pub tree: skein_world::end_to_end::TreeStatus,
}

#[derive(Debug)]
struct Output {
    input: Option<Fd>,
    stream: Option<Fd>,
    writing: bool,
    reading: bool,
    closing: u32,
    prompt: Box<[u8]>,
    from: u32,
    next: u64,
    maximum: usize,
    bytes: Vec<u8>,
    line: Vec<u8>,
    capture: Capture,
    observer: codex::Observer,
    answered: Option<Time>,
    error: Option<String>,
    completions: Queue<Complete>,
    submissions: Queue<Submit>,
}

impl Output {
    fn new(input: Fd, output: Fd, prompt: Box<[u8]>, maximum: usize, capture: Capture) -> Self {
        Self {
            input: Some(input),
            stream: Some(output),
            writing: false,
            reading: false,
            closing: 0,
            prompt,
            from: 0,
            next: 1,
            maximum,
            bytes: Vec::new(),
            line: Vec::new(),
            capture,
            observer: codex::Observer::default(),
            answered: None,
            error: None,
            completions: Queue::with_capacity(8),
            submissions: Queue::with_capacity(8),
        }
    }

    fn submit(&mut self, kind: Op) {
        self.submissions.push(Submit { op: Token::new(self.next), kind });
        self.next = self.next.checked_add(1).expect("bounded attempt operation count");
    }

    fn close(&mut self, fd: Fd) {
        self.closing = self.closing.checked_add(1).expect("two stream descriptors");
        self.submit(Op::Close { fd });
    }

    fn line(&mut self, now: Time) {
        if self.error.is_none() {
            match &mut self.capture {
                Capture::Raw => {}
                Capture::Codex => match std::str::from_utf8(&self.line) {
                    Ok(line) => {
                        if let Err(error) = self.observer.observe_line(line) {
                            self.error = Some(error.to_string());
                        }
                        if self.answered.is_none() && self.observer.answer().is_some() {
                            self.answered = Some(now);
                        }
                    }
                    Err(error) => self.error = Some(format!("stdout UTF-8: {error}")),
                },
                Capture::Agent(observing) => match std::str::from_utf8(&self.line) {
                    Ok(line) => match observing.feed_line(line) {
                        Ok(seen) => {
                            if self.answered.is_none() && seen.contains(&crate::agent::Seen::Answer) {
                                self.answered = Some(now);
                            }
                        }
                        Err(error) => self.error = Some(error.what),
                    },
                    Err(error) => self.error = Some(format!("stdout UTF-8: {error}")),
                },
            }
        }
        self.line.clear();
    }

    fn receive(&mut self, bytes: &[u8], now: Time) {
        let available = self.maximum.saturating_sub(self.bytes.len());
        let retained = bytes.len().min(available);
        self.bytes.extend_from_slice(&bytes[..retained]);
        if retained < bytes.len() && self.error.is_none() {
            self.error = Some("stdout exceeds the attempt log bound".into());
        }
        for byte in &bytes[..retained] {
            if *byte == b'\n' {
                self.line(now);
            } else {
                self.line.push(*byte);
            }
        }
    }
}

impl Host for Output {
    fn iterate(&mut self, now: Time, _wall: Wall) {
        while let Some(complete) = self.completions.pop() {
            match complete.kind {
                Op::PipeWrite { .. } => {
                    self.writing = false;
                    match complete.result {
                        Ok(Done::Count(count)) if count > 0 => {
                            self.from = self.from.checked_add(count).expect("prompt length fits u32");
                        }
                        other => {
                            self.error.get_or_insert_with(|| format!("prompt write: {other:?}"));
                            self.from = u32::try_from(self.prompt.len()).expect("bounded prompt");
                        }
                    }
                }
                Op::PipeRead { buf, .. } => {
                    self.reading = false;
                    match complete.result {
                        Ok(Done::Count(0)) => {
                            if !self.line.is_empty() {
                                self.line(now);
                            }
                            let fd = self.stream.take().expect("one retained output pipe");
                            self.close(fd);
                        }
                        Ok(Done::Count(count)) => {
                            self.receive(&buf[..usize::try_from(count).expect("bounded read")], now);
                        }
                        other => {
                            self.error.get_or_insert_with(|| format!("stdout read: {other:?}"));
                            let fd = self.stream.take().expect("one retained output pipe");
                            self.close(fd);
                        }
                    }
                }
                Op::Close { .. } => {
                    self.closing = self.closing.checked_sub(1).expect("a submitted close");
                    if let Err(error) = complete.result {
                        self.error.get_or_insert_with(|| format!("stream close: {error:?}"));
                    }
                }
                Op::Socket { .. }
                | Op::Bind { .. }
                | Op::Listen { .. }
                | Op::Accept { .. }
                | Op::Connect { .. }
                | Op::Recv { .. }
                | Op::Send { .. }
                | Op::Shutdown { .. }
                | Op::Open { .. }
                | Op::Read { .. }
                | Op::Write { .. }
                | Op::Append { .. }
                | Op::Sync { .. }
                | Op::Stat { .. }
                | Op::Rename { .. }
                | Op::Remove { .. }
                | Op::MakeDirectory { .. }
                | Op::List { .. }
                | Op::Spawn { .. }
                | Op::Wait { .. }
                | Op::Signal { .. }
                | Op::Usage
                | Op::ReadSignal { .. }
                | Op::Cancel { .. } => unreachable!("output submits only pipe operations and closes"),
            }
        }
        if !self.writing
            && let Some(fd) = self.input
        {
            if usize::try_from(self.from).expect("bounded prompt") < self.prompt.len() {
                self.writing = true;
                self.submit(Op::PipeWrite { fd, bytes: self.prompt.clone(), from: self.from });
            } else {
                self.input = None;
                self.close(fd);
            }
        }
        if !self.reading
            && let Some(fd) = self.stream
        {
            self.reading = true;
            self.submit(Op::PipeRead { fd, buf: Box::new([0; 4096]) });
        }
    }

    fn completions(&mut self) -> &mut Queue<Complete> {
        &mut self.completions
    }
    fn submissions(&mut self) -> &mut Queue<Submit> {
        &mut self.submissions
    }
    fn work_pending(&self, _now: Time) -> bool {
        !self.completions.is_empty() || !self.writing && self.input.is_some() || !self.reading && self.stream.is_some()
    }
    fn next_deadline(&self) -> Option<Time> {
        None
    }
    fn is_empty(&self) -> bool {
        self.input.is_none()
            && self.stream.is_none()
            && !self.reading
            && !self.writing
            && self.closing == 0
            && self.completions.is_empty()
            && self.submissions.is_empty()
    }
    fn worst_case(&self) -> u64 {
        u64::MAX
    }
    fn operations(&self) -> u32 {
        4
    }
}

#[derive(Debug)]
enum Process {
    Binary(Box<Binary>),
    Output(Box<Output>),
}

impl Process {
    fn host(&self) -> &dyn Host {
        match self {
            Self::Binary(binary) => binary.as_ref(),
            Self::Output(output) => output.as_ref(),
        }
    }
    fn host_mut(&mut self) -> &mut dyn Host {
        match self {
            Self::Binary(binary) => binary.as_mut(),
            Self::Output(output) => output.as_mut(),
        }
    }
}

impl Host for Process {
    fn iterate(&mut self, now: Time, wall: Wall) {
        self.host_mut().iterate(now, wall);
    }
    fn completions(&mut self) -> &mut Queue<Complete> {
        self.host_mut().completions()
    }
    fn submissions(&mut self) -> &mut Queue<Submit> {
        self.host_mut().submissions()
    }
    fn work_pending(&self, now: Time) -> bool {
        self.host().work_pending(now)
    }
    fn next_deadline(&self) -> Option<Time> {
        self.host().next_deadline()
    }
    fn is_empty(&self) -> bool {
        self.host().is_empty()
    }
    fn worst_case(&self) -> u64 {
        self.host().worst_case()
    }
    fn operations(&self) -> u32 {
        self.host().operations()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ladder {
    Watching,
    Terminated,
    Killed,
}

#[derive(Debug)]
struct Policy {
    deadline: Option<Time>,
    teardown: Duration,
    grace: Duration,
    answer_seen: bool,
    ladder: Ladder,
    settled: bool,
}

impl Referee<Process> for Policy {
    fn act(&mut self, now: Time, processes: &mut [Process]) {
        let (binaries, outputs) = processes.split_at_mut(1);
        let Process::Binary(binary) = &mut binaries[0] else { unreachable!("first host is Binary") };
        let Process::Output(output) = &mut outputs[0] else { unreachable!("second host is output") };
        if binary.stderr_overflowed() {
            output.error.get_or_insert_with(|| "stderr exceeds the attempt log bound".into());
        }
        if binary.counts().is_some() {
            self.settled = true;
            self.deadline = None;
            return;
        }
        if !self.answer_seen
            && let Some(answered) = output.answered
        {
            self.answer_seen = true;
            let teardown = answered.checked_add(self.teardown).expect("validated teardown deadline");
            self.deadline = Some(teardown);
        }
        let due = self.deadline.is_some_and(|deadline| deadline <= now) || output.error.is_some();
        if due && self.ladder == Ladder::Watching {
            binary.signal_tree(Signal::Terminate);
            self.ladder = Ladder::Terminated;
            self.deadline = Some(now.checked_add(self.grace).expect("validated grace deadline"));
        } else if self.ladder == Ladder::Terminated && self.deadline.is_some_and(|deadline| deadline <= now) {
            binary.signal_tree(Signal::Kill);
            self.ladder = Ladder::Killed;
            self.deadline = None;
        }
    }
    fn observe(&mut self, _now: Time, processes: &[Process]) {
        let Process::Binary(binary) = &processes[0] else { unreachable!("first host is Binary") };
        if binary.counts().is_some() {
            self.settled = true;
            self.deadline = None;
        }
    }
    fn next_deadline(&self) -> Option<Time> {
        self.deadline
    }
    fn overdue(&self, _now: Time) -> Option<String> {
        None
    }
    fn passed(&self) -> bool {
        self.settled
    }
}

/// Run one prepared pipe invocation. Every tree signal and count comes from
/// Binary; the policy supplies only its deadlines and output observations.
pub fn execute(
    clock: &Clock,
    command: Command,
    prompt: Box<[u8]>,
    capture: Capture,
    bounds: Bounds,
) -> Result<ProcessResult, String> {
    if prompt.len() > 4088 || bounds.stdout == 0 || bounds.stderr == 0 || bounds.hard == Duration::ZERO {
        return Err("invalid attempt prompt, log bound or hard deadline".into());
    }
    let started = clock.now();
    let start = started.now;
    let deadline = start.checked_add(bounds.hard).ok_or("hard deadline overflow")?;
    deadline.checked_add(bounds.teardown).and_then(|at| at.checked_add(bounds.grace)).ok_or("teardown overflow")?;
    let patience = bounds
        .hard
        .checked_add(bounds.teardown)
        .and_then(|span| span.checked_add(bounds.grace))
        .and_then(|span| span.checked_add(bounds.settlement))
        .ok_or("settlement patience overflow")?;
    let mut binary =
        Binary::start_with_tree(command, Mode::Pipes, bounds.stderr, Expectation::Supervise, bounds.force_walk)
            .map_err(|error| format!("binary start: {error:?}"))?;
    let streams = binary.take_streams().expect("new binary retains its streams");
    let Streams::Pipes { input, output } = streams else { unreachable!("pipe invocation") };
    let policy = Policy {
        deadline: Some(deadline),
        teardown: bounds.teardown,
        grace: bounds.grace,
        answer_seen: false,
        ladder: Ladder::Watching,
        settled: false,
    };
    let mut world = real::World::new(policy);
    world.spawn_with_fds(binary.descriptors(), || Process::Binary(Box::new(binary)));
    world.spawn_with_fds(vec![input, output], || {
        Process::Output(Box::new(Output::new(input, output, prompt, bounds.stdout, capture)))
    });
    let outcome = world.run(clock, patience);
    let mut processes = outcome.procs.into_iter();
    let Process::Binary(binary) = processes.next().expect("retained Binary") else { unreachable!("first host") };
    let Process::Output(output) = processes.next().expect("retained output") else { unreachable!("second host") };
    let error = if binary.stderr_overflowed() {
        output.error.or_else(|| Some("stderr exceeds the attempt log bound".into()))
    } else {
        output.error
    };
    let forced = match binary.tree_status().cleanup() {
        skein_world::end_to_end::Cleanup::None => crate::Forced::No,
        skein_world::end_to_end::Cleanup::Terminate => crate::Forced::Terminated,
        skein_world::end_to_end::Cleanup::Kill => crate::Forced::Killed,
    };
    let exit = match binary.exit_status().expect("settled leader has an exit") {
        skein_io::kernel::Exit::Code(code) => crate::Exit::Code(i32::from(code)),
        skein_io::kernel::Exit::Signal(signal) => crate::Exit::Signal(i32::try_from(signal).expect("kernel signal")),
    };
    let (observer, observing) = match output.capture {
        Capture::Agent(observing) => {
            let observer = match observing.as_ref() {
                crate::agent::Observing::Codex(observing) => observing.observer.clone(),
            };
            (observer, Some(observing))
        }
        Capture::Codex | Capture::Raw => (output.observer, None),
    };
    Ok(ProcessResult {
        exit,
        forced,
        start,
        start_wall: started.wall,
        end: outcome.end,
        answered: output.answered,
        stdout: output.bytes,
        stderr: binary.stderr().to_vec(),
        observer,
        observing,
        error,
        counts: binary.counts().expect("genuine tree settlement"),
        tree: {
            let tree = binary.tree_status();
            skein_world::end_to_end::TreeStatus {
                live_pids: tree.live_pids.clone(),
                live_at_leader_exit: tree.live_at_leader_exit.clone(),
                terminated_pids: tree.terminated_pids.clone(),
                killed_pids: tree.killed_pids.clone(),
                forced_cleanup: tree.forced_cleanup,
            }
        },
    })
}
