//! Real run/session/tools jobs on the shared agent world and fake checkout.
use skein_fake_checkout::Exit;
use skein_fake_llm_domain::api::{Finish, Line, Script, Turn};
use skein_lib::{Duration, Token};
use skein_world::domain::Verdict;
use smith_agent_world::{Job, Settings, World, scripted_charter};
use smith_domain::{Fact, run, session, tools};
use smith_tools_world::fixture::{Fixture, program};

const QUICK_BUILD_MS: u64 = 2_000;
const BUILD_OUTLASTING_THE_RUN_MS: u64 = 60_000;

#[derive(Clone, Copy)]
struct Checkout {
    checks: bool,
    build_ms: u64,
    wind_down: bool,
}

fn call(name: &[u8], arguments: &[u8]) -> Turn {
    Turn {
        lines: Box::new([Line::Call { name: name.into(), arguments: arguments.into() }]),
        finish: Finish::ToolCalls,
        tokens: 1,
    }
}

fn shell() -> Turn {
    call(b"shell", br#"{"command":"cargo build","yield_seconds":0}"#)
}

fn poll() -> Turn {
    call(b"job", br#"{"job":1,"wait_seconds":5}"#)
}

fn report() -> Turn {
    call(b"finish", br#"{"report":"Jobs settled.","source":"workspace"}"#)
}

fn finish_change() -> Turn {
    call(b"finish", br#"{"title":"Jobs settled","body":"The checked change."}"#)
}

fn script(cue: &[u8], turns: Vec<Turn>) -> Script {
    Script { cue: cue.into(), turns: turns.into() }
}

fn setup(settings: &Settings, turns: Vec<Turn>, children: Vec<Script>, checkout: Checkout) -> World {
    let mut fixture = Fixture::new();
    fixture.checkout.write(b"work/temper/src/lib.rs", b"pub fn answer() -> u32 { 43 }\n");
    fixture.checkout.program(b"cargo build", program(checkout.build_ms, b"Finished\n", Exit::Code(0), &[]));
    if checkout.checks {
        fixture.checkout.write(b"work/temper/.temper/pre-pr", b"#!checks\n");
    }
    let workspace = run::Workspace {
        directories: Box::new([run::Directory {
            host: b"/work/temper".as_slice().into(),
            name: b"work".as_slice().into(),
            root: fixture.repos[0].root,
            writable: true,
            git: true,
            conflicts: Box::default(),
        }]),
    };
    let mut charter = scripted_charter(settings);
    if checkout.wind_down {
        charter.reserve.turns = 1;
    }
    let cue = charter.brief.sections[0].text.clone();
    let scripts = std::iter::once(script(&cue, turns)).chain(children).collect::<Vec<_>>();
    World::with_workspace_scripts_charter(*settings, None, Some(workspace), fixture.checkout, scripts.into(), charter)
}

fn run(settings: &Settings, turns: Vec<Turn>, children: Vec<Script>, checkout: Checkout) -> World {
    let mut world = setup(settings, turns, children, checkout);
    world.run(20_000);
    assert_eq!(world.judged().1, 1);
    let seen = world
        .facts()
        .iter()
        .filter_map(|fact| {
            let Fact::Session { fact } = fact else { return None };
            let observation = match fact.kind {
                session::FactKind::Tools {
                    opener,
                    fact: tools::FactKind::Answered { job: Some(job), tool, verdict, .. },
                } if tool == tools::Tool::Shell && verdict == tools::Verdict::Running => {
                    smith_run_world::lease_referee::Seen::JobStarted { conversation: opener, job: job.0 }
                }
                session::FactKind::Tools {
                    opener,
                    fact: tools::FactKind::Answered { job: Some(job), verdict, .. },
                } if verdict != tools::Verdict::Running => {
                    smith_run_world::lease_referee::Seen::JobTerminal { conversation: opener, job: job.0 }
                }
                session::FactKind::JobEnded { opener, job, .. } => {
                    smith_run_world::lease_referee::Seen::JobTerminal { conversation: opener, job: job.0 }
                }
                session::FactKind::Ended { opener, .. } => {
                    smith_run_world::lease_referee::Seen::ConversationClosed { conversation: opener }
                }
                session::FactKind::Tools { .. }
                | session::FactKind::Opened { .. }
                | session::FactKind::CompactionStarted { .. }
                | session::FactKind::CompactionEnded { .. }
                | session::FactKind::CompletionStarted { .. }
                | session::FactKind::CompletionAnswered { .. }
                | session::FactKind::ReasoningDropped { .. }
                | session::FactKind::CompletionFailed { .. }
                | session::FactKind::CompletionCancelled { .. }
                | session::FactKind::CompletionRetried { .. }
                | session::FactKind::BuiltinStarted { .. }
                | session::FactKind::BuiltinAnswered { .. }
                | session::FactKind::DelegateStarted { .. }
                | session::FactKind::DelegateAnswered { .. }
                | session::FactKind::DelegateCancelled { .. }
                | session::FactKind::DelegateWithheld { .. }
                | session::FactKind::Yielded { .. }
                | session::FactKind::Used { .. } => return None,
            };
            Some((fact.at, Token::new(1), observation))
        })
        .collect::<Vec<_>>();
    assert_eq!(smith_run_world::lease_referee::judge(&seen), vec![Verdict::Passed]);
    world
}

fn ends(world: &World) -> Vec<tools::JobEnd> {
    world
        .turns()
        .iter()
        .flat_map(|turn| &turn.messages)
        .flat_map(|message| &message.content)
        .filter_map(|block| match block {
            session::llm::Block::ToolResult {
                result: session::llm::Returned::Owned { outcome: tools::Outcome::Job { end, .. } },
                ..
            } => Some(*end),
            session::llm::Block::Refusal { .. }
            | session::llm::Block::Text { .. }
            | session::llm::Block::Opaque { .. }
            | session::llm::Block::ToolCall { .. }
            | session::llm::Block::ToolResult { .. } => None,
        })
        .chain(world.facts().iter().filter_map(|fact| match fact {
            Fact::Session { fact: session::Fact { kind: session::FactKind::JobEnded { end, .. }, .. } } => Some(*end),
            Fact::Session { .. } | Fact::Run { .. } => None,
        }))
        .collect()
}

#[test]
fn a_build_run_as_a_job_while_main_reads_and_edits_is_read_to_its_end() {
    let world = run(
        &Settings { job: Job::Reporting, ..Settings::calm(1001) },
        vec![
            shell(),
            call(b"read", br#"{"path":"src/lib.rs"}"#),
            call(b"edit", br#"{"path":"src/lib.rs","edits":[{"old":"43","new":"44"}]}"#),
            poll(),
            report(),
        ],
        vec![],
        Checkout { checks: false, build_ms: QUICK_BUILD_MS, wind_down: false },
    );
    assert_eq!(ends(&world), vec![tools::JobEnd::Ended(tools::Exit::Code { code: 0 })]);
    assert_eq!(world.code(), b"pub fn answer() -> u32 { 44 }\n");
}

#[test]
fn mains_jobs_are_cancelled_before_a_writing_child_opens() {
    let child = script(
        b"@job-child",
        vec![Turn {
            lines: Box::new([Line::Text { text: b"child done".as_slice().into() }]),
            finish: Finish::Stop,
            tokens: 1,
        }],
    );
    let world = run(
        &Settings { job: Job::Reporting, ..Settings::calm(1002) },
        vec![shell(), call(b"sub_agent", br#"{"brief":"@job-child","tools":["inspect","modify"]}"#), poll(), report()],
        vec![child],
        Checkout { checks: false, build_ms: BUILD_OUTLASTING_THE_RUN_MS, wind_down: false },
    );
    assert_eq!(ends(&world), vec![tools::JobEnd::Cancelled]);
}

#[test]
fn a_finishs_checks_and_a_deliver_run_after_mains_jobs_are_cancelled() {
    let world = run(
        &Settings::calm(1003),
        vec![shell(), finish_change()],
        vec![],
        Checkout { checks: true, build_ms: BUILD_OUTLASTING_THE_RUN_MS, wind_down: false },
    );
    assert_eq!(ends(&world), vec![tools::JobEnd::Cancelled]);
    assert_eq!(world.checked(), [true]);
    assert_eq!(world.pushes(), [smith_agent_world::delivered()]);
}

#[test]
fn a_delivers_checks_run_after_mains_jobs_are_cancelled() {
    let world = run(
        &Settings { job: Job::MidReport, ..Settings::calm(1004) },
        vec![shell(), call(b"deliver", br#"{"ticket":"jobs"}"#), poll(), report()],
        vec![],
        Checkout { checks: true, build_ms: BUILD_OUTLASTING_THE_RUN_MS, wind_down: false },
    );
    assert_eq!(ends(&world), vec![tools::JobEnd::Cancelled]);
    assert_eq!(world.checked(), [true]);
}

#[test]
fn a_writing_childs_job_ends_with_the_child() {
    let child = script(
        b"@job-child",
        vec![
            shell(),
            Turn {
                lines: Box::new([Line::Text { text: b"child done".as_slice().into() }]),
                finish: Finish::Stop,
                tokens: 1,
            },
        ],
    );
    let world = run(
        &Settings { job: Job::Reporting, ..Settings::calm(1005) },
        vec![call(b"sub_agent", br#"{"brief":"@job-child","tools":["shell"]}"#), report()],
        vec![child],
        Checkout { checks: false, build_ms: BUILD_OUTLASTING_THE_RUN_MS, wind_down: false },
    );
    assert_eq!(ends(&world), vec![tools::JobEnd::Cancelled]);
    assert_eq!(
        world
            .facts()
            .iter()
            .filter(|fact| matches!(
                fact,
                Fact::Session { fact: session::Fact { kind: session::FactKind::JobEnded { .. }, .. } }
            ))
            .count(),
        1
    );
}

#[test]
fn a_wind_down_with_a_job_running_is_cancelled_by_the_finish_that_delivers() {
    let settings = Settings { budget: run::Budget { turns: 2, ..smith_agent_world::BUDGET }, ..Settings::calm(1006) };
    let world = run(
        &settings,
        vec![shell(), finish_change()],
        vec![],
        Checkout { checks: true, build_ms: BUILD_OUTLASTING_THE_RUN_MS, wind_down: true },
    );
    assert_eq!(ends(&world), vec![tools::JobEnd::Cancelled]);
    assert_eq!(world.checked(), [true]);
    assert_eq!(
        world
            .facts()
            .iter()
            .filter(|fact| matches!(
                fact,
                Fact::Run {
                    fact: run::facts::Fact {
                        kind: run::facts::FactKind::Budget {
                            standing: run::facts::Standing { stage: run::Stage::WindDown, .. },
                            ..
                        },
                        ..
                    },
                    ..
                }
            ))
            .count(),
        2
    );
}

fn resume(first: &World, settings: &Settings) -> World {
    let turn = first.turns().first().expect("main recorded its job");
    let history = smith_domain::Transcript {
        version: turn.version,
        endpoint: turn.endpoint,
        dialect: turn.dialect,
        turns: first.turns().into(),
    };
    let mut fixture = Fixture::new();
    fixture.checkout.write(b"work/temper/src/lib.rs", b"pub fn answer() -> u32 { 43 }\n");
    let workspace = run::Workspace {
        directories: Box::new([run::Directory {
            host: b"/work/temper".as_slice().into(),
            name: b"work".as_slice().into(),
            root: fixture.repos[0].root,
            writable: true,
            git: true,
            conflicts: Box::default(),
        }]),
    };
    let mut next = World::with_workspace_scripts(
        Settings { resume: true, ..*settings },
        Some(history),
        Some(workspace),
        fixture.checkout,
        Box::new([script(b"@waiting", vec![shell(), call(b"wait", b"{}"), poll(), call(b"wait", b"{}")])]),
    );
    next.run(20_000);
    next
}

#[test]
fn a_run_parked_with_a_job_running_tells_it_ended_and_a_later_job_call_is_invalid() {
    let settings = Settings { job: Job::Waiting, waiting: Duration::ZERO, ..Settings::calm(1007) };
    let first = run(
        &settings,
        vec![shell(), call(b"wait", b"{}")],
        vec![],
        Checkout { checks: false, build_ms: BUILD_OUTLASTING_THE_RUN_MS, wind_down: false },
    );
    assert_eq!(ends(&first), vec![tools::JobEnd::Cancelled]);
    let next = resume(&first, &settings);
    let problems = next
        .turns()
        .iter()
        .flat_map(|turn| &turn.messages)
        .flat_map(|message| &message.content)
        .filter_map(|block| match block {
            session::llm::Block::ToolResult { result: session::llm::Returned::Invalid { problem }, .. } => {
                Some(problem.clone())
            }
            session::llm::Block::Refusal { .. }
            | session::llm::Block::Text { .. }
            | session::llm::Block::Opaque { .. }
            | session::llm::Block::ToolCall { .. }
            | session::llm::Block::ToolResult { .. } => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(problems, vec![session::llm::Problem::UnknownJob { job: 1 }]);
}
