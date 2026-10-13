//! Exec's durable result file (shell.md, sections 4.2–4.3).
//! The file owner holds teardown until its whole-file write and closes end.
//! It knows the output path and kernel terminals, not the run's outcome.
use super::{Opened, Refused, root};
use crate::command::Usage;
use skein_io::{file, file_layer, kernel};
use skein_lib::{Duration, Queue, Time, Token, Wall};
use skein_shell::{Clock, Host, Kernel};
use std::path::{Path, PathBuf};

const PATH_BYTES: u64 = 4096;

pub(super) struct Files {
    pub(super) output: Option<Box<ResultFile>>,
    target: Option<PathBuf>,
    maximum: u32,
    timeout: Duration,
    reservation: u64,
}

impl Files {
    pub(super) fn prepare(opened: &Opened) -> Result<Self, Refused> {
        let target = opened.invocation.output.clone();
        if let Some(path) = &target {
            check_path(path)?;
            if path.file_name().is_none() {
                return Err(Refused::Usage(Usage("-o requires a file name".into())));
            }
        }
        let maximum = u32::try_from(opened.resolved.limits.smith.run.outcome_bytes)
            .map_err(|_| Refused::Usage(Usage("the result file bound exceeds u32".into())))?
            .max(opened.resolved.limits.show_bytes)
            .max(1);
        let paths = u64::from(target.is_some()).checked_mul(2).expect("result paths");
        let reservation = PATH_BYTES
            .checked_mul(paths)
            .and_then(|bytes| bytes.checked_add(u64::try_from(size_of::<Self>()).ok()?))
            .expect("bounded paths and file choices");
        Ok(Self {
            target,
            output: None,
            maximum,
            timeout: opened.resolved.agent.service.limits.file_timeout,
            reservation,
        })
    }

    pub(super) fn reservation(&self) -> u64 {
        self.reservation
    }

    pub(super) fn live_memory(&self) -> Option<u64> {
        if self.target.is_some() { ResultFile::footprint(self.maximum) } else { Some(0) }
    }

    pub(super) fn prepare_output(&mut self, seed: u64) -> Result<(), Refused> {
        if let Some(path) = &self.target {
            self.output = Some(Box::new(ResultFile::open(path, self.maximum, self.timeout, seed)?));
        }
        Ok(())
    }

    pub(super) fn close_startup(&mut self, kernel: &mut Kernel, clock: &Clock) {
        if let Some(mut output) = self.output.take() {
            output.close();
            let _exit = skein_shell::drive(kernel, clock, output.as_mut());
        }
    }
}

fn check_path(path: &Path) -> Result<(), Refused> {
    if path.as_os_str().len() >= usize::try_from(PATH_BYTES).expect("path bound fits") {
        return Err(Refused::Usage(Usage("a product file path exceeds 4095 bytes".into())));
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Waiting,
    Saving,
    Closing,
    Closed,
}

/// The write or its later directory close failed, reported to product teardown.
pub(crate) enum Failure {
    /// The whole-file write or sync could not complete.
    Write(kernel::Error),
    /// The result has settled, but the directory's close failed.
    Close(kernel::Error),
}

/// The product's whole result write and the directory close it owes.
pub(crate) struct ResultFile {
    io: file_layer::FileIo,
    directory: Token,
    name: Box<[u8]>,
    path: PathBuf,
    request: Option<file::Request>,
    state: State,
    failure: Option<Failure>,
    bound: u64,
    events: Queue<file::Event>,
    completions: Queue<kernel::Complete>,
    submissions: Queue<kernel::Submit>,
}

impl ResultFile {
    fn new(io: file_layer::FileIo, directory: Token, path: &Path, maximum: u32) -> Self {
        Self {
            io,
            directory,
            name: path.file_name().expect("checked file name").as_encoded_bytes().into(),
            path: path.into(),
            request: None,
            state: State::Waiting,
            failure: None,
            bound: Self::footprint(maximum).expect("file footprint fits"),
            events: Queue::with_capacity(4),
            completions: Queue::with_capacity(4),
            submissions: Queue::with_capacity(4),
        }
    }

    fn open(path: &Path, maximum: u32, timeout: Duration, seed: u64) -> Result<Self, Refused> {
        let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let mut io = file_layer::FileIo::with_whole_limit(2, 4096, 1, maximum, timeout);
        let directory = io.adopt_root(root(parent)?).expect("one result directory");
        io.seed_randomness(seed);
        Ok(Self::new(io, directory, path, maximum))
    }

    pub(crate) fn begin(&mut self, text: Box<[u8]>) {
        assert!(self.state == State::Waiting, "one result is offered to its file");
        self.state = State::Saving;
        self.request = Some(file::Request::Store {
            owner: Token::new(1),
            root: self.directory,
            path: self.name.clone(),
            bytes: text,
            expected: file::Expect::Any,
            no_follow: true,
        });
    }

    pub(crate) fn close(&mut self) {
        if self.state == State::Waiting {
            self.state = State::Closing;
        }
    }

    pub(crate) fn abort(&mut self) {
        match self.state {
            State::Saving => {
                if self.request.take().is_some() {
                    self.failure = Some(Failure::Write(kernel::Error::Cancelled));
                    self.state = State::Closing;
                } else {
                    file_layer::cancel(&mut self.io, Token::new(1), &mut self.submissions);
                }
            }
            State::Waiting | State::Closing | State::Closed => {}
        }
    }

    pub(crate) fn waiting(&self) -> bool {
        self.state == State::Waiting
    }
    pub(crate) fn failure(&mut self) -> Option<Failure> {
        self.failure.take()
    }
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    fn failed(&mut self, error: kernel::Error) {
        match self.state {
            State::Saving => {
                self.failure = Some(Failure::Write(error));
                self.state = State::Closing;
            }
            State::Closing => {
                self.failure = Some(Failure::Close(error));
                self.state = State::Closed;
            }
            State::Waiting | State::Closed => unreachable!("only active file requests fail"),
        }
    }

    fn footprint(maximum: u32) -> Option<u64> {
        file_layer::FileIo::worst_case(2, 4096, 1, maximum)?
            // Retained path, its lossy rendering (up to three bytes per input
            // byte), and the formatted failure beside it, including growth.
            .checked_add(PATH_BYTES.checked_mul(10)?.checked_add(511)?)?
            .checked_add(Queue::<file::Event>::worst_case(4)?)?
            .checked_add(Queue::<kernel::Complete>::worst_case(4)?)?
            .checked_add(Queue::<kernel::Submit>::worst_case(4)?)?
            .checked_add(u64::try_from(size_of::<Self>()).ok()?)
    }
}

impl Host for ResultFile {
    fn iterate(&mut self, now: Time, _wall: Wall) {
        while let Some(complete) = self.completions.pop() {
            file_layer::up(&mut self.io, complete, &mut self.events, &mut self.submissions);
        }
        if self.io.is_due(now) {
            file_layer::expire(&mut self.io, now, &mut self.submissions);
        }
        while let Some(event) = self.events.pop() {
            match event {
                file::Event::Stored { .. } => self.state = State::Closing,
                file::Event::Closed { .. } => self.state = State::Closed,
                file::Event::Failed { error, .. } => self.failed(error),
                file::Event::Cancelled { .. } => self.failed(kernel::Error::Cancelled),
                file::Event::Removed { .. }
                | file::Event::Made { .. }
                | file::Event::Refused { .. }
                | file::Event::Opened { .. }
                | file::Event::Loaded { .. }
                | file::Event::Scanned { .. }
                | file::Event::Conflict { .. }
                | file::Event::TooLarge { .. }
                | file::Event::Stated { .. }
                | file::Event::Written { .. }
                | file::Event::Read { .. }
                | file::Event::Cut { .. }
                | file::Event::Synced { .. }
                | file::Event::Renamed { .. }
                | file::Event::Listed { .. } => unreachable!("product file requests have whole-file terminals"),
            }
        }
        if self.io.takes() {
            let request = self.request.take().or_else(|| {
                (self.state == State::Closing)
                    .then_some(file::Request::Close { owner: Token::new(2), file: self.directory })
            });
            if let Some(request) = request {
                file_layer::down(&mut self.io, now, request, &mut self.events, &mut self.submissions);
            }
        }
    }
    fn completions(&mut self) -> &mut Queue<kernel::Complete> {
        &mut self.completions
    }
    fn submissions(&mut self) -> &mut Queue<kernel::Submit> {
        &mut self.submissions
    }
    fn work_pending(&self, now: Time) -> bool {
        self.request.is_some()
            || !self.events.is_empty()
            || !self.completions.is_empty()
            || self.io.is_due(now)
            || (self.state == State::Closing && self.io.takes())
    }
    fn next_deadline(&self) -> Option<Time> {
        self.io.next_deadline()
    }
    fn next_policy_deadline(&self) -> Option<Time> {
        None
    }
    fn is_empty(&self) -> bool {
        self.state == State::Closed
            && self.io.takes()
            && self.io.open_files() == 0
            && self.events.is_empty()
            && self.completions.is_empty()
            && self.submissions.is_empty()
    }
    fn operations(&self) -> u32 {
        4
    }
    fn worst_case(&self) -> u64 {
        self.bound
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> ResultFile {
        let mut io = file_layer::FileIo::with_whole_limit(2, 4096, 1, 4096, Duration::from_secs(1));
        let directory = io.adopt_root(kernel::Fd::new(41)).unwrap();
        io.seed_randomness(1);
        ResultFile::new(io, directory, Path::new("answer.txt"), 4096)
    }

    fn complete(file: &mut ResultFile, submit: kernel::Submit, result: Result<kernel::Done, kernel::Error>) {
        file.completions.push(kernel::Complete { op: submit.op, kind: submit.kind, result });
        file.iterate(Time::ZERO, Wall::from_nanos(0));
    }

    #[test]
    fn a_failed_result_write_closes_its_directory_before_it_is_empty() {
        let mut file = file();
        file.begin(b"answer".as_slice().into());
        file.iterate(Time::ZERO, Wall::from_nanos(0));
        let opening = file.submissions.pop().unwrap();
        complete(&mut file, opening, Err(kernel::Error::Permission));
        assert!(matches!(file.failure(), Some(Failure::Write(kernel::Error::Permission))));
        assert!(!file.is_empty());
        let closing = file.submissions.pop().unwrap();
        assert!(matches!(closing.kind, kernel::Op::Close { fd } if fd == kernel::Fd::new(41)));
        complete(&mut file, closing, Ok(kernel::Done::Nothing));
        assert!(file.is_empty());
    }

    #[test]
    fn an_interrupted_result_write_waits_for_the_cancel_and_completion() {
        let mut file = file();
        file.begin(b"answer".as_slice().into());
        file.iterate(Time::ZERO, Wall::from_nanos(0));
        let opening = file.submissions.pop().unwrap();
        file.abort();
        let cancelling = file.submissions.pop().unwrap();
        assert!(matches!(cancelling.kind, kernel::Op::Cancel { target } if target == opening.op));
        complete(&mut file, opening, Err(kernel::Error::Cancelled));
        assert!(!file.is_empty());
        complete(&mut file, cancelling, Ok(kernel::Done::Nothing));
        assert!(matches!(file.failure(), Some(Failure::Write(kernel::Error::Cancelled))));
        let closing = file.submissions.pop().unwrap();
        complete(&mut file, closing, Ok(kernel::Done::Nothing));
        assert!(file.is_empty());
    }

    #[test]
    fn an_unused_output_directory_is_closed_after_a_startup_refusal() {
        let mut file = file();
        file.close();
        file.iterate(Time::ZERO, Wall::from_nanos(0));
        let closing = file.submissions.pop().unwrap();
        assert!(matches!(closing.kind, kernel::Op::Close { fd } if fd == kernel::Fd::new(41)));
        complete(&mut file, closing, Ok(kernel::Done::Nothing));
        assert!(file.is_empty());
    }

    #[test]
    fn the_result_owner_prices_its_retained_text_paths_and_queues() {
        let span = skein_heap::Span::start();
        let mut file = file();
        file.begin(vec![b'x'; 4096].into_boxed_slice());
        file.iterate(Time::ZERO, Wall::from_nanos(0));
        let measured = span.end();
        assert!(u64::try_from(measured.peak).unwrap() <= file.worst_case());
        assert_eq!(file.operations(), 4);
    }
}
