//! Both agent kinds share the parent referee (domain/host.md, section 10).
use skein_lib::Token;
use smith_agent_world::Job;
use smith_domain::{run, session::record};
use smith_host_domain::{self as host, End, Input, RunFailure, RunResult, parent};
use smith_host_world::{
    World as Spawned,
    inline::{self, World},
};

fn spawn(world: &mut World, client: u64, job: Job) {
    world.event(parent::Event::Spawn { client: Token::new(client), start: inline::start(client, job) });
}

#[test]
fn an_inline_run_answers_and_its_slot_is_released_after_its_last_completion() {
    let mut world = World::new(311, inline::limits());
    world.auto_ack = false;
    spawn(&mut world, 1, Job::HostTools);
    for _ in 0..100 {
        if world.seen[&Token::new(1)].answer.is_some() {
            break;
        }
        let next = world.next_deadline().expect("actual provider or root alarm");
        world.at(next);
    }
    let seen = &world.seen[&Token::new(1)];
    assert!(
        matches!(seen.answer.as_ref().map(|answer| &answer.result), Some(RunResult::Accepted { .. })),
        "{:?}",
        seen.answer
    );
    assert!(seen.gone.is_none(), "answered root retains exact told-turn ACK rights");
    assert_eq!(world.agent.hosted(), 1);
    world.lower.assert_settled();
    let agent = world.handle(Token::new(1));
    let turns: Vec<_> = seen.turns.keys().copied().collect();
    for turn in turns {
        world.event(parent::Event::Acknowledge { agent, turn });
    }
    world.settled();
}

#[test]
fn a_drop_after_an_answer_releases_held_turns_without_replacing_the_answer() {
    let mut inline = World::new(319, inline::limits());
    inline.auto_ack = false;
    spawn(&mut inline, 1, Job::HostTools);
    for _ in 0..100 {
        if inline.seen[&Token::new(1)].answer.is_some() {
            break;
        }
        inline.at(inline.next_deadline().expect("actual provider or root alarm"));
    }
    assert!(inline.seen[&Token::new(1)].answer.is_some(), "actual native answer");
    let answer = format!("{:?}", inline.seen[&Token::new(1)].answer);
    assert!(!inline.seen[&Token::new(1)].turns.is_empty());
    inline.event(parent::Event::Drop { agent: inline.handle(Token::new(1)) });
    assert_eq!(format!("{:?}", inline.seen[&Token::new(1)].answer), answer);
    assert_eq!(inline.seen[&Token::new(1)].fault, None);
    inline.settled();

    let mut spawned = Spawned::new(319, smith_host_world::limits());
    spawned.live();
    spawned.up(host::Up::Turn {
        turn: host::Turn {
            opens_window: true,
            number: 1,
            spent: 0,
            read: None,
            body: smith_host_world::turn_value(64),
        },
    });
    let answer = host::Answer { read: None, turns: 1, spent: 0, result: RunResult::Parked };
    spawned.up(host::Up::Answer { answer });
    spawned.event(host::Input::Parent(parent::Event::Drop { agent: spawned.agent() }));
    spawned.cleanup(Some(smith_host_domain::Exit::Code(0)));
    assert_eq!(spawned.seen.answer, Some(host::Answer { read: None, turns: 1, spent: 0, result: RunResult::Parked }));
    assert_eq!(spawned.seen.fault, None);
    assert!(!spawned.seen.down.iter().any(|message| matches!(message, host::Down::Acknowledge { .. })));
    spawned.settled();
}

#[test]
fn a_late_drop_is_ignored_after_the_ordinary_answer_and_gone() {
    let mut world = World::new(333, inline::limits());
    spawn(&mut world, 1, Job::HostTools);
    let agent = world.handle(Token::new(1));
    world
        .schedule
        .send(skein_lib::Time::ZERO.saturating_add(skein_lib::Duration::from_secs(10)), parent::Event::Drop { agent });
    world.run();
    assert!(world.seen[&Token::new(1)].answer.is_some());
    assert!(world.seen[&Token::new(1)].fault.is_none());
    world.settled();
}

#[test]
fn a_drop_before_the_answer_waits_for_actual_lower_settlement() {
    let mut world = World::new(320, inline::limits());
    spawn(&mut world, 1, Job::HostTools);
    assert!(!world.lower.is_empty());
    world.event(parent::Event::Drop { agent: world.handle(Token::new(1)) });
    assert_eq!(world.seen[&Token::new(1)].fault, Some(host::Fault::Exited));
    assert!(world.seen[&Token::new(1)].gone.is_none());
    world.run();
    assert!(world.seen[&Token::new(1)].answer.is_none());
    world.settled();
}

#[test]
fn a_drop_asked_at_once_settles_completions_file_operations_and_command_trees() {
    let mut provider = World::new(322, inline::limits());
    provider.provider_latency(skein_lib::Duration::from_secs(20));
    spawn(&mut provider, 1, Job::HostTools);
    provider.event(parent::Event::Drop { agent: provider.handle(Token::new(1)) });
    assert!(provider.seen[&Token::new(1)].gone.is_none());
    provider.run();
    provider.settled();

    let mut guide = World::new(323, inline::limits());
    guide.auto_machine = false;
    guide.event(parent::Event::Spawn { client: Token::new(1), start: workspace_start(323) });
    assert!(
        guide.pending_machine.iter().any(|(_, _, terminal)| matches!(terminal, smith_inline_agent::Below::Read { .. }))
    );
    guide.event(parent::Event::Drop { agent: guide.handle(Token::new(1)) });
    assert!(guide.seen[&Token::new(1)].gone.is_none());
    assert_eq!(guide.machine_trace.iter().filter(|request| request.starts_with("cancel")).count(), 1);
    guide.machine_replies();
    guide.run();
    guide.settled();
    assert!(guide.seen[&Token::new(1)].answer.is_none());

    let mut command = child_command_world(324);
    command.event(parent::Event::Drop { agent: command.handle(Token::new(1)) });
    assert_eq!(command.seen[&Token::new(1)].fault, Some(host::Fault::Exited));
    assert!(command.seen[&Token::new(1)].gone.is_none());
    command.tool_terminals = inline::ToolTerminals::Forward;
    command.machine_replies();
    command.run();
    command.settled();
    assert!(command.seen[&Token::new(1)].answer.is_none());
}

#[test]
fn an_inline_run_that_ignores_its_cancel_is_dropped_at_its_grace_with_everything_below_settled() {
    use skein_lib::{Duration, Time};
    let mut bounds = inline::limits();
    bounds.cancel_grace = Duration::from_secs(5);
    let mut world = World::new(321, bounds);
    world.provider_latency(Duration::from_secs(20));
    spawn(&mut world, 1, Job::HostTools);
    world.event(parent::Event::Stop { agent: world.handle(Token::new(1)) });
    world.at(Time::ZERO.saturating_add(Duration::from_secs(4)));
    assert_eq!(world.seen[&Token::new(1)].fault, None);
    world.at(Time::ZERO.saturating_add(Duration::from_secs(5)));
    assert_eq!(world.seen[&Token::new(1)].fault, Some(host::Fault::Exited));
    assert!(world.seen[&Token::new(1)].answer.is_none());
    assert!(world.seen[&Token::new(1)].gone.is_none(), "actual provider terminal remains owed");
    assert!(!world.lower.is_empty());
    world.run();
    world.settled();
    assert!(world.seen[&Token::new(1)].answer.is_none(), "dropped root hears no terminal");
    assert_eq!(
        world
            .observations
            .iter()
            .filter(|fact| matches!(fact.kind, smith_inline_agent::FactKind::Dropped { .. }))
            .count(),
        1
    );
}

#[test]
fn an_inline_run_waits_parks_and_resumes_from_its_transcript() {
    let mut world = World::new(312, inline::limits());
    spawn(&mut world, 1, Job::Waiting);
    world.run();
    world.settled();
    assert!(
        matches!(world.seen[&Token::new(1)].answer.as_ref().expect("answer").result, RunResult::Parked),
        "{:?}",
        world.seen[&Token::new(1)].answer
    );
    let turns = world.history[&Token::new(1)].clone();
    assert!(!turns.is_empty());
    let endpoint = turns[0].endpoint;
    let dialect = turns[0].dialect;
    let history = record::Transcript { version: record::VERSION, endpoint, dialect, turns: turns.into() };
    let mut next = inline::start(313, Job::Waiting);
    let mut charter = next.charter.into_value();
    charter.resume = true;
    next.charter =
        host::Charter::new(charter, world.stage.env.limits.smith.run.run_bytes).expect("bounded resumed policy");
    next.activation = 2;
    next.logical_run = Token::new(1);
    next.transcript = Some(host::Transcript::new(history, u64::MAX).expect("saved concrete history"));
    next.messages = Box::from([host::Message {
        finish_requested: false,
        name: Token::new(92),
        label: b"person".as_slice().into(),
        text: b"Continue".as_slice().into(),
    }]);
    world.event(parent::Event::Spawn { client: Token::new(2), start: next });
    world.run();
    world.settled();
    let restored = &world.history[&Token::new(2)];
    assert!(restored[0].sequence > world.history[&Token::new(1)].last().expect("saved turn").sequence);
    assert_eq!(world.seen[&Token::new(2)].answer.as_ref().expect("resumed answer").read, Some(Token::new(92)));
}

#[test]
fn an_inline_run_cancelled_mid_turn_answers_cancelled() {
    let mut world = World::new(314, inline::limits());
    spawn(&mut world, 1, Job::HostTools);
    assert!(!world.lower.is_empty());
    let agent = world.handle(Token::new(1));
    world.event(parent::Event::Stop { agent });
    assert!(world.seen[&Token::new(1)].gone.is_none(), "cancel still owes actual provider terminal");
    world.run();
    world.settled();
    assert!(matches!(
        world.seen[&Token::new(1)].answer.as_ref().expect("answer").result,
        RunResult::Failed { failure: RunFailure::Cancelled }
    ));
}

#[test]
fn a_message_past_the_runs_bound_is_refused_typed_by_either_kind() {
    let mut inline_bounds = inline::limits();
    inline_bounds.smith.run.message_bytes = 64;
    let mut world = World::new(315, inline_bounds);
    spawn(&mut world, 1, Job::HostTools);
    let agent = world.handle(Token::new(1));
    world.event(parent::Event::Message {
        finish_requested: false,
        agent,
        name: Token::new(90),
        label: Box::default(),
        text: vec![b'x'; 63].into(),
    });
    assert_eq!(world.seen[&Token::new(1)].bounces, [host::MessageRefusal::TooLarge]);
    world.run();
    world.settled();
    let mut spawned = Spawned::new(315, smith_host_world::limits());
    spawned.live();
    let agent = spawned.agent();
    spawned.event(Input::Parent(parent::Event::Message {
        finish_requested: false,
        agent,
        name: Token::new(90),
        label: Box::default(),
        text: vec![b'x'; 63].into(),
    }));
    assert_eq!(spawned.seen.bounces, [host::MessageRefusal::TooLarge]);
}

#[test]
fn an_inline_runs_facts_are_drained_every_pass() {
    let mut world = World::new(316, inline::limits());
    spawn(&mut world, 1, Job::HostTools);
    assert!(!world.facts.is_empty(), "admission observations available before provider finishes");
    assert!(world.agent.pop_fact(Token::new(1)).is_none(), "drained at this actual iteration");
    world.run();
    world.settled();
    assert!(!world.content.is_empty());
    assert!(
        world
            .observations
            .iter()
            .any(|fact| matches!(fact.kind, smith_inline_agent::FactKind::Gone { client: _, end: End::Settled }))
    );
}

#[test]
fn two_inline_slots_keep_provider_owners_independent() {
    let mut world = World::new(317, inline::limits());
    spawn(&mut world, 1, Job::HostTools);
    spawn(&mut world, 2, Job::HostTools);
    assert_ne!(world.handle(Token::new(1)), world.handle(Token::new(2)));
    world.run();
    world.settled();
    for client in [Token::new(1), Token::new(2)] {
        assert!(matches!(world.seen[&client].answer.as_ref().expect("answer").result, RunResult::Accepted { .. }));
    }
}

#[test]
fn inline_scripts_use_actual_root_types_without_a_codec() {
    let start = inline::start(318, Job::HostTools);
    assert!(start.workspace.is_none());
    assert!(!start.charter.value().grants.host_tools.is_empty());
    assert_eq!(start.charter.value().grants.tools, run::charter::Tools { inspect: false, modify: false, shell: false });
}

fn workspace_start(seed: u64) -> smith_host_domain::Start {
    let settings = smith_agent_world::Settings::calm(seed);
    let mut charter = smith_agent_world::scripted_charter(&settings);
    charter.models = Box::default();
    charter.grants.agents = false;
    charter.brief.sections[0].text = b"inline-workspace".as_slice().into();
    charter.conventions = Some(smith_domain::run::Conventions {
        guide: b"AGENTS.md".as_slice().into(),
        checks: b"check-ok".as_slice().into(),
    });
    let mut source = inline::start(seed, smith_agent_world::Job::Coding);
    source.charter =
        smith_host_domain::Charter::new(charter, settings.limits.run.run_bytes).expect("workspace charter");
    source.workspace = Some(Token::new(9));
    source.directories = Box::from([smith_host_domain::Directory {
        host: Box::from(b"/host".as_slice()),
        name: b"work".as_slice().into(),
        root: Token::new(1),
        writable: true,
        git: true,
        conflicts: Box::default(),
    }]);
    source
}

fn workspace_script(commands: &[(&[u8], &[u8])]) -> Box<[skein_fake_llm_domain::api::Script]> {
    Box::from([skein_fake_llm_domain::api::Script {
        cue: b"inline-workspace".as_slice().into(),
        turns: commands
            .iter()
            .map(|(name, arguments)| skein_fake_llm_domain::api::Turn {
                lines: Box::from([skein_fake_llm_domain::api::Line::Call {
                    name: (*name).into(),
                    arguments: (*arguments).into(),
                }]),
                finish: skein_fake_llm_domain::api::Finish::ToolCalls,
                tokens: 40,
            })
            .collect(),
    }])
}

#[test]
fn an_inline_run_checks_and_delivers_a_change_through_its_host() {
    let mut world = World::new(81, inline::limits());
    world.scripts(workspace_script(&[(b"finish", br#"{"title":"checked","body":"change"}"#)]));
    world.event(parent::Event::Spawn { client: Token::new(1), start: workspace_start(81) });
    world.run();
    assert!(
        matches!(
            world.seen[&Token::new(1)].answer,
            Some(smith_host_domain::Answer { result: smith_host_domain::RunResult::Accepted { .. }, .. })
        ),
        "{}",
        world.trace.lines().join("\n")
    );
    assert!(world.machine_trace.iter().any(|request| request.starts_with("Check")));
    assert!(world.trace.lines().iter().any(|request| request.contains("LongDone")));
    assert!(world.trace.lines().iter().any(|request| request.contains("ask: Deliver")));
    world.settled();
}

#[test]
fn checks_are_long_operations_for_either_kind() {
    let mut world = World::new(82, inline::limits());
    world.scripts(workspace_script(&[(b"finish", br#"{"title":"checked","body":"change"}"#)]));
    world.event(parent::Event::Spawn { client: Token::new(1), start: workspace_start(82) });
    world.run();
    assert_eq!(world.trace.lines().iter().filter(|request| request.contains("operation: Checks")).count(), 1);
    assert_eq!(
        world.long_notices.iter().filter(|(_, span)| span.is_some()).count(),
        world.long_notices.iter().filter(|(_, span)| span.is_none()).count()
    );
    world.settled();
    let mut spawned = Spawned::new(82, smith_host_world::limits());
    spawned.live();
    spawned.up(host::Up::Long {
        operation: smith_host_domain::Operation::Checks,
        span: skein_lib::Duration::from_secs(1),
    });
    spawned.up(host::Up::LongDone);
    assert!(spawned.trace.lines().iter().any(|line| line.contains("Long")));
}

#[test]
fn an_inline_cancel_waits_for_its_pending_machine_terminal() {
    let mut world = World::new(83, inline::limits());
    world.auto_machine = false;
    world.event(parent::Event::Spawn { client: Token::new(1), start: workspace_start(83) });
    assert_eq!(world.pending_machine.len(), 1);
    let handle = world.handle(Token::new(1));
    world.event(parent::Event::Stop { agent: handle });
    assert!(world.seen[&Token::new(1)].gone.is_none());
    assert_eq!(world.agent.hosted(), 1);
    world.auto_machine = true;
    world.machine_replies();
    world.run();
    world.settled();
}

fn child_command_world(seed: u64) -> World {
    use skein_fake_llm_domain::api::{Finish, Line, Script, Turn};
    let mut world = World::new(seed, inline::limits());
    world.tool_terminals = inline::ToolTerminals::Hold;
    let mut source = inline::workspace_start(seed, Job::Reporting);
    let mut charter = source.charter.into_value();
    charter.grants.agents = true;
    charter.brief.sections[0].text = b"inline-parent-command".as_slice().into();
    source.charter = host::Charter::new(charter, u64::MAX).expect("bounded parent charter");
    world.scripts(Box::from([
        Script {
            cue: b"inline-parent-command".as_slice().into(),
            turns: Box::from([Turn {
                lines: Box::from([Line::Call {
                    name: b"sub_agent".as_slice().into(),
                    arguments: br#"{"brief":"inline-child-command","tools":["shell"]}"#.as_slice().into(),
                }]),
                finish: Finish::ToolCalls,
                tokens: 40,
            }]),
        },
        Script {
            cue: b"inline-child-command".as_slice().into(),
            turns: Box::from([Turn {
                lines: Box::from([Line::Call {
                    name: b"shell".as_slice().into(),
                    arguments: br#"{"command":"true"}"#.as_slice().into(),
                }]),
                finish: Finish::ToolCalls,
                tokens: 40,
            }]),
        },
    ]));
    world.event(parent::Event::Spawn { client: Token::new(1), start: source });
    for _ in 0..100 {
        if world
            .pending_machine
            .iter()
            .any(|(_, _, terminal)| matches!(terminal, smith_inline_agent::Below::Done { .. }))
        {
            break;
        }
        world.at(world.next_deadline().expect("native provider alarm"));
    }
    assert!(world.machine_trace.iter().any(|request| request.contains("Spawn")), "actual child command requested");
    world
}

#[test]
fn an_inline_run_cancels_its_child_command_and_is_gone_once_it_settles() {
    let mut world = child_command_world(85);
    let handle = world.handle(Token::new(1));
    world.event(parent::Event::Stop { agent: handle });
    assert!(world.seen[&Token::new(1)].answer.is_none(), "the native root also waits for its command terminal");
    assert!(
        world.seen[&Token::new(1)].gone.is_none(),
        "the command's actual native terminal is held across cancellation"
    );
    assert!(world.machine_trace.iter().any(|request| request.starts_with("cancel")));
    world.tool_terminals = inline::ToolTerminals::Forward;
    world.machine_replies();
    world.run();
    world.settled();
}

#[test]
fn a_real_completion_stretches_the_spawned_watchdog_past_no_progress() {
    use skein_lib::{Duration, Time};
    let mut native_limits = inline::limits();
    // This watchdog control waits for the held completion, before the separate grace control may drop it.
    native_limits.cancel_grace = Duration::from_secs(30);
    let mut native = World::new(95, native_limits);
    native.provider_latency(Duration::from_secs(20));
    spawn(&mut native, 1, Job::HostTools);
    let [(_, Some((operation, span)))] = native.long_notices.as_slice() else {
        panic!("actual provider issuance reports one span: {:?}", native.long_notices);
    };
    assert_eq!(*operation, host::Operation::Completion);
    let mut bounds = smith_host_world::limits();
    bounds.long_span = *span;
    let mut spawned = Spawned::new(95, bounds);
    spawned.live();
    spawned.up(host::Up::Long { operation: *operation, span: *span });
    native.at(Time::ZERO.saturating_add(Duration::from_secs(11)));
    spawned.at(11);
    assert_eq!(spawned.seen.fault, None, "actual completion outlasts no_progress");
    assert_eq!(native.long_notices.len(), 1, "actual provider has not ended");
    native.event(parent::Event::Stop { agent: native.handle(Token::new(1)) });
    assert_eq!(native.long_notices.len(), 1, "cancel alone does not end the completion");
    native.run();
    native.settled();
    assert_eq!(native.long_notices.len(), 2);
    assert_eq!(native.long_notices[1].1, None);
    spawned.at(20);
    spawned.up(host::Up::LongDone);
    assert_eq!(spawned.seen.fault, None);
    spawned.up(host::Up::Answer { answer: host::Answer { read: None, turns: 0, spent: 0, result: RunResult::Parked } });
    spawned.cleanup(Some(smith_host_domain::Exit::Code(0)));
    spawned.settled();
}

#[test]
fn an_inline_answer_keeps_its_typed_completion_failure() {
    let mut bounds = inline::limits();
    bounds.smith.session.retries = 2;
    let start = inline::start(6808, Job::HostTools);
    let mut world = World::new(6808, bounds);
    world.fail_with(
        smith_domain::llm::Failure::TimedOut { phase: smith_domain::llm::Phase::Head },
        smith_domain::llm::Evidence::Response { status: Some(503) },
    );
    world.event(parent::Event::Spawn { client: Token::new(1), start });
    world.run();
    world.settled();
    let RunResult::Failed { failure: RunFailure::Model(host::ModelFault::Completion { failure, evidence, .. }) } =
        world.seen[&Token::new(1)].answer.as_ref().expect("answer").result
    else {
        panic!("typed model failure")
    };
    assert_eq!(failure, host::CompletionFailure::TimedOut { phase: host::Phase::Head });
    assert_eq!(evidence, host::CompletionEvidence::Response { status: Some(503) });
}

#[test]
fn an_inline_answer_keeps_its_account_failure() {
    use smith_domain::llm;
    for (failure, expected) in [
        (
            llm::Failure::Exhausted { retry_after: Some(skein_lib::Duration::from_nanos(80_000_000)) },
            host::ModelFault::Limited { account: 0, reset: Some(skein_lib::Duration::from_nanos(80_000_000)) },
        ),
        (llm::Failure::Exhausted { retry_after: None }, host::ModelFault::Limited { account: 0, reset: None }),
        (llm::Failure::Refused, host::ModelFault::Refused { account: 0, code: Box::default() }),
    ] {
        let mut world = World::new(6809, inline::limits());
        world.fail_with(failure, llm::Evidence::Response { status: Some(403) });
        world.event(parent::Event::Spawn { client: Token::new(1), start: inline::start(6809, Job::HostTools) });
        world.run();
        world.settled();
        assert_eq!(
            world.seen[&Token::new(1)].answer.as_ref().expect("answer").result,
            RunResult::Failed { failure: RunFailure::Model(expected) },
            "{failure:?}"
        );
    }
}
