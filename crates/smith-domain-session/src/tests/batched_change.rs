use super::*;
use crate::llm::NotRun;

fn edit(new: &[u8]) -> Call {
    Call::Edit {
        path: path(b"first"),
        edits: Box::new([tools::Replacement {
            old: bytes(b"old"),
            new: bytes(new),
            occurrences: tools::Occurrences::Once,
        }]),
    }
}

fn command() -> Call {
    Call::Shell {
        command: bytes(b"check"),
        timeout: tools::Timeout::Default,
        output: tools::Budget::Default,
        after: tools::Yield::Never,
    }
}

fn after_edit(new: &[u8], loaded: &[u8], next: Block) -> (Harness, Option<Request>) {
    let mut harness = Harness::new(&LIMITS);
    let (owner, _) = harness.open(1);
    let content = Box::new([tool_call(b"first", edit(new)), next]);
    let (load, _) = running(harness.step(Event::Completed { owner, completion: completion(content, Stop::ToolUse) }));
    let mut request = harness.step(ran(load, loaded));
    if request.is_none() {
        harness.domain.reclaim();
        request = harness.resume();
    }
    (harness, request)
}

fn settled(mut harness: Harness, mut request: Option<Request>) -> Prompt {
    if request.is_none() {
        harness.domain.reclaim();
        request = harness.resume();
    }
    calling(request).1
}

fn not_run(id: &[u8], call: u16) -> Block {
    Block::ToolResult { id: bytes(id), result: Returned::NotRun { why: NotRun::AfterFailedWrite { call } } }
}

#[test]
fn a_failed_edit_leaves_the_writes_after_it_not_run() {
    let (harness, request) = after_edit(b"new", b"absent", tool_call(b"next", write(b"next")));
    let prompt = settled(harness, request);
    assert_eq!(prompt.messages[2].content[1], not_run(b"next", 1));
}

#[test]
fn a_failed_edit_leaves_the_command_after_it_not_run() {
    let (harness, request) = after_edit(b"new", b"absent", tool_call(b"next", command()));
    let prompt = settled(harness, request);
    assert_eq!(prompt.messages[2].content[1], not_run(b"next", 1));
}

#[test]
fn a_read_after_a_failed_edit_still_runs() {
    let (mut harness, request) = after_edit(b"new", b"absent", tool_call(b"next", cat(b"next")));
    let (load, _) = running(request);
    let request = harness.step(ran(load, b"after"));
    let prompt = settled(harness, request);
    assert_eq!(prompt.messages[2].content[1], result(b"next", read(b"after")));
}

fn after_invalid(first: Block) -> Prompt {
    let mut harness = Harness::new(&LIMITS);
    let (owner, _) = harness.open(1);
    let content = Box::new([first, tool_call(b"next", write(b"next"))]);
    let request = harness.step(Event::Completed { owner, completion: completion(content, Stop::ToolUse) });
    settled(harness, request)
}

#[test]
fn an_invalid_call_stops_the_writes_after_it() {
    let prompt = after_invalid(invalid(b"first", Problem::UnknownTool));
    assert_eq!(prompt.messages[2].content[1], not_run(b"next", 1));
}

#[test]
fn an_unchanged_edit_stops_the_writes_after_it() {
    let (harness, request) = after_edit(b"old", b"old", tool_call(b"next", write(b"next")));
    let prompt = settled(harness, request);
    assert_eq!(prompt.messages[2].content[1], not_run(b"next", 1));
}

#[test]
fn an_invalid_read_stops_the_writes_after_it() {
    let mut first = invalid(b"first", Problem::BadValue { field: bytes(b"offset") });
    let Block::ToolCall { name, .. } = &mut first else { unreachable!() };
    *name = bytes(b"read");
    let prompt = after_invalid(first);
    assert_eq!(prompt.messages[2].content[1], not_run(b"next", 1));
}

#[test]
fn a_finish_after_a_failed_write_is_not_run() {
    let (harness, request) = after_edit(b"new", b"absent", delegated(b"finish", FINISH.ticket.raw(), Effect::Write));
    let prompt = settled(harness, request);
    assert_eq!(prompt.messages[2].content[1], not_run(b"finish", 1));
}

#[test]
fn a_call_that_is_no_failed_write_stops_nothing() {
    for (label, first, terminal) in [
        (
            "nonzero command",
            tool_call(b"first", command()),
            Some(Done::Exited { exit: tools::Exit::Code { code: 1 }, head: bytes(b""), tail: bytes(b""), dropped: 0 }),
        ),
        ("timed out command", tool_call(b"first", command()), Some(Done::TimedOut)),
        (
            "running job",
            tool_call(
                b"first",
                Call::Job {
                    job: tools::JobNumber(1),
                    wait: tools::Wait::Default,
                    output: tools::Budget::Default,
                    stop: false,
                },
            ),
            Some(Done::Job { head: bytes(b""), tail: bytes(b""), dropped: 0, end: tools::JobEnd::Running }),
        ),
        ("host error", delegated(b"first", 9, Effect::Write), None),
        ("sub-agent answer", delegated(b"first", 10, Effect::Read), None),
        ("withdrawn", delegated(b"first", 11, Effect::Write), None),
    ] {
        let mut harness = Harness::new(&LIMITS);
        let mut selected = spec();
        selected.authority.grants.shell = true;
        let (owner, _) = harness.open_with(1, selected);
        if label == "running job" {
            let mut shell = command();
            let Call::Shell { after, .. } = &mut shell else { unreachable!() };
            *after = tools::Yield::Asked(Duration::from_secs(1));
            let content = Box::new([tool_call(b"shell", shell)]);
            let (spawn, _) =
                running(harness.step(Event::Completed { owner, completion: completion(content, Stop::ToolUse) }));
            drop(calling(harness.step(Event::Done {
                owner: spawn,
                done: Done::Held { head: bytes(b""), tail: bytes(b""), dropped: 0 },
            })));
        }
        let content = Box::new([first, tool_call(b"next", edit(b"new"))]);
        let runs = harness.batch(Event::Completed { owner, completion: completion(content, Stop::ToolUse) });
        let run = runs.as_slice()[0];
        let event = match terminal {
            Some(done) => Event::Done { owner: run, done },
            None if label == "withdrawn" => answer_cancelled(run),
            None => answered(run, 1, 3, label == "host error"),
        };
        let (_, operation) = running(harness.step(event));
        assert_eq!(loading(&operation), place(b"first"), "{label}: later edit starts");
    }
}
