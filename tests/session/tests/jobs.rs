//! Jobs retain execution beside turns and end before their session's terminal.
use skein_lib::{Duration, Time, Token};
use smith_domain_session::{self as session, llm, record};
use smith_domain_tools::{self as tools, Budget, Call, Done, JobEnd, JobNumber, Wait};
use smith_session_world::recorded::{self, World};

fn opening(transcript: Option<record::Transcript>) -> record::Opening {
    let mut opening = recorded::opening(transcript, 100);
    opening.spec.authority.grants = tools::Grants { inspect: true, modify: true, shell: true };
    opening.spec.authority.repos =
        Box::new([tools::Repo { host: Box::default(), mount: Box::default(), root: Token::new(7), writable: true }]);
    opening
}

fn world() -> World {
    let mut world = World::new(1, 256);
    world.open(opening(None));
    world
}

fn called(id: &[u8], call: Call) -> llm::Block {
    llm::Block::ToolCall {
        id: id.into(),
        name: b"workspace".as_slice().into(),
        input: Box::default(),
        call: llm::Decoded::Owned { call },
        replay: None,
    }
}

fn shell() -> llm::Block {
    called(
        b"build",
        Call::Shell {
            command: b"cargo build".as_slice().into(),
            timeout: tools::Timeout::Default,
            output: Budget::Default,
            after: tools::Yield::Asked(Duration::from_secs(1)),
        },
    )
}

fn job(stop: bool) -> llm::Block {
    called(b"poll", Call::Job { job: JobNumber(1), wait: Wait::Default, output: Budget::Default, stop })
}

fn read(id: &[u8]) -> llm::Block {
    called(
        id,
        Call::Read {
            path: tools::Path {
                absolute: false,
                parts: Box::new([tools::Part::Name {
                    name: tools::Name::new(b"README.md".as_slice().into()).expect("fixture name"),
                }]),
            },
            offset: 1,
            window: Budget::Default,
        },
    )
}

fn complete(world: &mut World, blocks: Vec<llm::Block>) {
    world.complete(blocks.into_boxed_slice(), llm::Stop::ToolUse, llm::Usage::ZERO);
}

fn start_job(world: &mut World) -> Token {
    complete(world, vec![shell()]);
    let owner = world.operations[0].0;
    world.env.now = Time::from_nanos(1_000_000_000);
    world.step(session::Event::Done {
        owner,
        done: Done::Held { head: b"Compiling\n".as_slice().into(), tail: Box::default(), dropped: 0 },
    });
    owner
}

fn end_job(world: &mut World, owner: Token, end: JobEnd) {
    world.step(session::Event::Done {
        owner,
        done: Done::Job { head: b"Done\n".as_slice().into(), tail: Box::default(), dropped: 0, end },
    });
}

fn finish(world: &mut World) {
    world.complete(
        Box::new([llm::Block::Text { text: b"finished".as_slice().into(), replay: None }]),
        llm::Stop::EndTurn,
        llm::Usage::ZERO,
    );
    world.close();
}

fn outcomes(world: &World) -> Vec<tools::Outcome> {
    world
        .turns
        .iter()
        .flat_map(|turn| &turn.messages)
        .flat_map(|message| &message.content)
        .filter_map(|block| {
            if let llm::Block::ToolResult { result: llm::Returned::Owned { outcome }, .. } = block {
                Some(outcome.clone())
            } else {
                None
            }
        })
        .collect()
}

fn read_and_edit(world: &mut World) {
    complete(world, vec![read(b"read"), edit(), job(false)]);
    let read_owner = world.operations[0].0;
    world.step(session::Event::Done {
        owner: read_owner,
        done: Done::Loaded { content: b"old\n".as_slice().into(), version: tools::Version::new([1; 4]) },
    });
    let edit_owner = world.operations[0].0;
    world.step(session::Event::Done {
        owner: edit_owner,
        done: Done::Loaded { content: b"old\n".as_slice().into(), version: tools::Version::new([1; 4]) },
    });
    world.step(session::Event::Done { owner: edit_owner, done: Done::Stored { version: tools::Version::new([2; 4]) } });
}

#[test]
fn a_command_answered_running_while_the_session_reads_edits_and_completes() {
    let mut world = world();
    let owner = start_job(&mut world);
    assert_eq!(world.prompts.len(), 2, "the command still runs during the next completion");
    read_and_edit(&mut world);
    assert_eq!(
        world.job_acts,
        vec![(owner, tools::JobAct::Read { head: 256, tail: 256, wait: Duration::from_secs(30) })]
    );
    end_job(&mut world, owner, JobEnd::Ended(tools::Exit::Code { code: 0 }));
    finish(&mut world);
    assert_eq!(world.terminal_usage, Some((3, llm::Usage::ZERO)));
    assert_eq!(world.turns.iter().map(|turn| turn.jobs).collect::<Vec<_>>(), vec![1, 1, 1]);
    assert_eq!(
        outcomes(&world).last(),
        Some(&tools::Outcome::Job {
            job: JobNumber(1),
            head: b"Done\n".as_slice().into(),
            tail: Box::default(),
            dropped: 0,
            output: tools::BytesClamp::Within,
            wait: tools::TimeoutClamp::Within,
            end: JobEnd::Ended(tools::Exit::Code { code: 0 }),
        })
    );
}

#[test]
fn a_job_with_stop_runs_alone_in_its_batch() {
    let mut world = world();
    let owner = start_job(&mut world);
    complete(&mut world, vec![read(b"before"), job(true), read(b"after")]);
    assert_eq!(world.operations.len(), 1);
    assert_eq!(world.job_acts.len(), 0);
    let read_owner = world.operations[0].0;
    world.step(session::Event::Done { owner: read_owner, done: Done::Missing });
    assert_eq!(world.operations.len(), 0);
    assert_eq!(world.job_acts, vec![(owner, tools::JobAct::Stop { head: 256, tail: 256 })]);
    end_job(&mut world, owner, JobEnd::Stopped);
    assert_eq!(world.job_acts.len(), 0);
    assert_eq!(world.operations.len(), 1);
    let read_owner = world.operations[0].0;
    world.step(session::Event::Done { owner: read_owner, done: Done::Missing });
    finish(&mut world);
}

#[test]
fn jobs_cancelled_by_the_opener_before_another_writer_answer_cancelled() {
    let mut world = world();
    let owner = start_job(&mut world);
    world.step(session::Event::CancelJobs { session: world.session.expect("opened") });
    assert_eq!(world.jobs_cancelled, 0, "the opener still waits for the command's terminal");
    assert_eq!(world.cancelled_operations, vec![owner]);
    end_job(&mut world, owner, JobEnd::Cancelled);
    assert_eq!(world.jobs_cancelled, 1, "another writer may now take its lease");
    complete(&mut world, vec![job(false)]);
    world.resume();
    finish(&mut world);
    assert_eq!(
        outcomes(&world).last(),
        Some(&tools::Outcome::Job {
            job: JobNumber(1),
            head: b"Done\n".as_slice().into(),
            tail: Box::default(),
            dropped: 0,
            output: tools::BytesClamp::Within,
            wait: tools::TimeoutClamp::Within,
            end: JobEnd::Cancelled,
        })
    );
}

#[test]
fn a_session_closing_with_a_job_running_tells_its_job_ended() {
    let mut world = world();
    let owner = start_job(&mut world);
    world.step(session::Event::Close { session: world.session.expect("opened") });
    let completing = world.completing.expect("pending completion");
    world.step(session::Event::Cancelled { owner: completing });
    assert_eq!(world.end, None);
    world.env.now = Time::from_nanos(3_000_000_000);
    end_job(&mut world, owner, JobEnd::Cancelled);
    let ended =
        world.facts.iter().filter(|fact| matches!(fact.kind, session::FactKind::JobEnded { .. })).collect::<Vec<_>>();
    assert_eq!(ended.len(), 1);
    assert_eq!(
        ended[0].kind,
        session::FactKind::JobEnded {
            opener: Token::new(31),
            job: JobNumber(1),
            end: JobEnd::Cancelled,
            unread_bytes: 5,
            started: Time::ZERO,
        }
    );
    assert_eq!(ended[0].at, Time::from_nanos(3_000_000_000));
    assert_eq!(ended[0].call.as_ref().expect("provider identity").id.as_ref(), b"build");
    let order = world
        .facts
        .iter()
        .filter_map(|fact| {
            if matches!(fact.kind, session::FactKind::JobEnded { .. }) {
                Some("job")
            } else if matches!(fact.kind, session::FactKind::Ended { .. }) {
                Some("session")
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(order, vec!["job", "session"]);
    world.close();
}

#[test]
fn a_session_closing_while_a_job_read_waits_answers_the_read_before_it_ends() {
    const READ_WITHOUT_STOPPING: bool = false;
    let mut world = world();
    let owner = start_job(&mut world);
    complete(&mut world, vec![job(READ_WITHOUT_STOPPING)]);
    world.facts.clear();
    world.step(session::Event::Close { session: world.session.expect("opened") });
    end_job(&mut world, owner, JobEnd::Cancelled);
    let order = world
        .facts
        .iter()
        .filter_map(|fact| {
            if matches!(fact.kind, session::FactKind::Tools { fact: tools::FactKind::Answered { .. }, .. }) {
                Some("answer")
            } else if matches!(fact.kind, session::FactKind::Ended { .. }) {
                Some("end")
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(order, vec!["answer", "end"]);
    world.close();
}

#[test]
fn a_resumed_session_numbers_its_next_job_after_its_last_turns() {
    let old = recorded::scenario(1, 256);
    let mut transcript = recorded::transcript(&old);
    transcript.turns.last_mut().expect("last turn").jobs = 3;
    let mut world = World::new(1, 256);
    world.open(opening(Some(transcript)));
    let owner = start_job(&mut world);
    assert_eq!(world.turns[0].jobs, 4);
    assert_eq!(
        outcomes(&world)[0],
        tools::Outcome::Running {
            job: JobNumber(4),
            head: b"Compiling\n".as_slice().into(),
            tail: Box::default(),
            dropped: 0,
            output: tools::BytesClamp::Within,
            timeout: tools::TimeoutClamp::Within,
            after: tools::TimeoutClamp::Within,
        }
    );
    world.step(session::Event::CancelJobs { session: world.session.expect("opened") });
    end_job(&mut world, owner, JobEnd::Cancelled);
    world.close();
}

fn polling(blocks: Vec<llm::Block>) -> (World, Token) {
    let mut world = world();
    let owner = start_job(&mut world);
    complete(&mut world, blocks);
    (world, owner)
}

#[test]
fn a_job_call_beside_reads_answers_once_they_have_answered() {
    let (mut world, owner) = polling(vec![job(false), read(b"first"), read(b"last")]);
    assert_eq!(world.hastened, vec![]);
    let first = world.operations[0].0;
    world.step(session::Event::Done { owner: first, done: Done::Missing });
    assert_eq!(world.hastened, vec![]);
    let last = world.operations[0].0;
    world.step(session::Event::Done { owner: last, done: Done::Missing });
    assert_eq!(world.hastened, vec![owner]);
    end_job(&mut world, owner, JobEnd::Ended(tools::Exit::Code { code: 0 }));
    finish(&mut world);
}

fn edit() -> llm::Block {
    called(
        b"edit",
        Call::Edit {
            path: tools::Path {
                absolute: false,
                parts: Box::new([tools::Part::Name {
                    name: tools::Name::new(b"README.md".as_slice().into()).expect("fixture name"),
                }]),
            },
            edits: Box::new([tools::Replacement {
                old: b"old".as_slice().into(),
                new: b"new".as_slice().into(),
                occurrences: tools::Occurrences::Once,
            }]),
        },
    )
}

fn answer_operation(world: &mut World, done: Option<Done>) {
    if let Some(done) = done {
        let owner = world.operations[0].0;
        world.step(session::Event::Done { owner, done });
    }
}

#[test]
fn a_job_call_in_a_batch_of_its_own_beside_other_calls_answers_at_once() {
    let exited =
        Done::Exited { head: Box::default(), tail: Box::default(), dropped: 0, exit: tools::Exit::Code { code: 0 } };
    for (case, blocks, before, after) in [
        ("before edit", vec![job(false), edit()], None, Some(Done::Missing)),
        ("after shell", vec![shell(), job(false)], Some(exited), None),
    ] {
        let (mut world, owner) = polling(blocks);
        answer_operation(&mut world, before);
        assert_eq!(world.hastened, vec![owner], "{case}");
        end_job(&mut world, owner, JobEnd::Ended(tools::Exit::Code { code: 0 }));
        answer_operation(&mut world, after);
        finish(&mut world);
    }
}

#[test]
fn job_calls_alone_wait_as_each_asks() {
    let (mut world, owner) = polling(vec![job(false), job(false)]);
    assert_eq!(world.hastened, vec![]);
    end_job(&mut world, owner, JobEnd::Running);
    assert_eq!(world.hastened, vec![]);
    end_job(&mut world, owner, JobEnd::Ended(tools::Exit::Code { code: 0 }));
    finish(&mut world);
}
