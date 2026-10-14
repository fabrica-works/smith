//! A job's between-call execution holds no long-operation span.
use super::{Harness, bytes, charter, owned};
use crate::{Event, Operation, Request, tools};
use skein_lib::{Duration, Token};

fn running(harness: &mut Harness) -> (Token, Token, Token) {
    let mut requested = charter();
    requested.grants.tools.shell = true;
    let (run, owner, _) = harness.admit(7, requested);
    let emitted = harness.answer(
        owner,
        Box::new([owned(
            b"build",
            tools::Call::Shell {
                after: tools::Yield::Asked(Duration::from_secs(1)),
                command: bytes(b"cargo build"),
                timeout: tools::Timeout::Default,
                output: tools::Budget::Default,
            },
        )]),
    );
    let [Request::Io { owner, .. }] = emitted.as_ref() else { panic!("shell starts: {emitted:?}") };
    let command = *owner;
    let emitted = harness.step(Event::Done {
        owner: command,
        done: tools::Done::Held { head: Box::default(), tail: Box::default(), dropped: 0 },
    });
    let [Request::Complete { owner, .. }] = emitted.as_ref() else {
        panic!("session completes beside job: {emitted:?}")
    };
    (run, command, *owner)
}

fn read_to_end(harness: &mut Harness, command: Token, completing: Token) {
    let emitted = harness.answer(
        completing,
        Box::new([owned(
            b"poll",
            tools::Call::Job {
                job: tools::JobNumber(1),
                wait: tools::Wait::Asked(Duration::from_secs(7)),
                output: tools::Budget::Default,
                stop: false,
            },
        )]),
    );
    assert_eq!(
        emitted.as_ref(),
        &[Request::Job {
            owner: command,
            act: tools::JobAct::Read { head: 64, tail: 64, wait: Duration::from_secs(7) }
        }]
    );
    assert_eq!(harness.domain.commands.len(), 1, "the reading call owns its span");
    harness.step(Event::Done {
        owner: command,
        done: tools::Done::Job {
            head: Box::default(),
            tail: Box::default(),
            dropped: 0,
            end: tools::JobEnd::Ended(tools::Exit::Code { code: 0 }),
        },
    });
}

fn close(harness: &mut Harness, run: Token) {
    harness.step(Event::Cancel { run });
    let emitted = harness.next();
    let [Request::Cancel { owner }] = emitted.as_ref() else { panic!("provider cancellation: {emitted:?}") };
    harness.step(Event::Cancelled { owner: *owner });
}

#[test]
fn a_jobs_wait_announces_only_its_active_call_span() {
    let mut harness = Harness::new();
    let (run, command, completing) = running(&mut harness);
    assert_eq!(harness.domain.commands.len(), 0, "the background job holds no command span");
    assert_eq!(
        harness.long.as_slice(),
        &[Some(Operation::Completion), None, Some(Operation::Command), None, Some(Operation::Completion)]
    );
    read_to_end(&mut harness, command, completing);
    assert_eq!(harness.domain.commands.len(), 0);
    assert_eq!(
        harness.long.as_slice(),
        &[
            Some(Operation::Completion),
            None,
            Some(Operation::Command),
            None,
            Some(Operation::Completion),
            None,
            Some(Operation::Command),
            None,
            Some(Operation::Completion)
        ]
    );
    close(&mut harness, run);
}
