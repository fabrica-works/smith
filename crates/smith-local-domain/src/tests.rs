#![expect(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    reason = "unit harnesses are ordinary Rust (programming-model.md, section 10.2)"
)]

use alloc::boxed::Box;
use skein_lib::{Duration, Env, Queue, Time, Token, Wall};
use smith_domain::run::{self, charter::Endpoint, outcome};

use crate::{Config, Contract, Domain, Event, Invalid, Limits, Request, charter};

pub(super) fn limits() -> Limits {
    Limits {
        smith: smith_agent_world::LIMITS,
        agent: crate::AgentLimits::inline(
            smith_agent_world::LIMITS,
            2,
            Duration::from_secs(60),
            Duration::from_secs(5),
        )
        .expect("bounded inline child"),
        endpoints: Box::new([Endpoint(4)]),
        chat_bytes: 64,
        text_bytes: 1024,
        models: 2,
        line_bytes: 256,
        show_bytes: 1024,
        lines: 8,
        unsaved: 2,
        facts: 16,
    }
}

pub(super) fn config() -> Config {
    Config {
        reserve: run::charter::Reserve { turns: 0, time: Duration::ZERO },
        chat: Box::from(&b"main"[..]),
        instructions: Box::from(&b"Assist the person"[..]),
        brief: run::charter::Brief {
            sections: Box::new([run::charter::Section {
                title: Box::from(&b"Context"[..]),
                text: Box::from(&b"Work carefully"[..]),
            }]),
        },
        protocol_allowances: Box::new([0]),
        progress: Box::new([smith_domain::session::Progress {
            head: Duration::from_secs(60),
            idle: Duration::from_secs(30),
        }]),
        models: Box::new([run::charter::Llm {
            prices: run::Prices { input: 1, cached: 1, cache_write: None, output: 1, unit: 1_000_000 },
            dialect: 1,
            account: 7,
            endpoint: Endpoint(4),
            model: Box::from(&b"small"[..]),
            window: 8192,
            output: 128,
        }]),
        sub_agents: false,
        budget: run::Budget { turns: 8, spend: 1, time: Duration::from_secs(60) },
        conventions: Some(run::Conventions { guide: Box::from(&b"GUIDE.md"[..]), checks: Box::from(&b"check"[..]) }),
        contract: Contract::Report(outcome::TextSpec { max: 256, fields: Box::new([]) }),
        deliver: None,
        title_field: Box::from(&b"title"[..]),
        waiting: Duration::from_secs(30),
        resume: true,
        accounts: Box::new([7]),
        workspace: None,
        push: None,
    }
}

#[test]
fn the_local_inline_child_must_have_one_slot_and_matching_native_receiving_bounds() {
    for extra_slot in [false, true] {
        let mut limits = limits();
        let crate::AgentLimits::Inline(child) = &mut limits.agent else { panic!("inline fixture") };
        if extra_slot {
            child.slots = 2;
        } else {
            child.smith.accounts = child.smith.accounts.checked_add(1).expect("small receiving profile");
        }
        let Err(Invalid::Limits) = Domain::new(config(), &limits, 97) else {
            panic!("incompatible inline child refuses startup")
        };
    }
}

#[test]
fn report_charter_copies_every_local_choice_without_workspace_tools() {
    let config = config();
    let limits = limits();
    assert_eq!(config.validate(&limits), Ok(()));
    let charter = charter(&config);
    assert!(charter.resume, "configured resume is retained");
    assert_eq!(charter.waiting, config.waiting);
    assert_eq!(charter.instructions, config.instructions);
    assert_eq!(charter.brief.sections, config.brief.sections);
    assert_eq!(charter.conventions, config.conventions);
    assert_eq!(charter.budget, config.budget);
    assert_eq!(charter.llm, config.models[0]);
    assert!(charter.models.is_empty(), "only the first model is the main model");
    assert!(charter.outcome.report.is_some(), "report is selected");
    assert!(charter.outcome.change.is_none(), "change was not selected");
    assert!(charter.grants.wait, "local chat may wait for the person");
    assert!(!charter.grants.tools.inspect && !charter.grants.tools.modify && !charter.grants.tools.shell);
    assert!(!charter.grants.agents, "sub-agent authority is not configured");
    assert!(charter.grants.host_tools.is_empty(), "local host has no tools");
    assert!(Domain::new(config, &limits, 11).is_ok(), "validated local domain builds its agent");
}

#[test]
fn change_and_second_model_map_to_distinct_charter_choices() {
    let mut config = config();
    let mut second = config.models[0].clone();
    second.model = Box::from(&b"large"[..]);
    config.models = Box::new([config.models[0].clone(), second.clone()]);
    config.contract = Contract::Change(outcome::ChangeSpec { checks_must_pass: true, fields: Box::new([]) });
    config.sub_agents = true;
    config.resume = false;
    config.waiting = Duration::ZERO;
    let charter = charter(&config);
    assert_eq!(charter.models.as_ref(), &[second]);
    assert!(charter.grants.agents, "the configured grant permits native sub-agent calls");
    assert!(!charter.resume, "fresh run was configured");
    assert_eq!(charter.waiting, Duration::ZERO);
    assert!(charter.outcome.report.is_none(), "report was not selected");
    assert!(charter.outcome.change.is_some(), "change was selected");
}

#[test]
fn invalid_local_configuration_is_refused_before_agent_construction() {
    let limits = limits();
    let mut invalid = config();
    invalid.chat = Box::new([]);
    assert_eq!(invalid.validate(&limits), Err(Invalid::Chat));
    assert_eq!(Domain::new(invalid, &limits, 1).err(), Some(Invalid::Chat));

    let mut invalid = config();
    invalid.instructions = Box::from([b'x'; 1025]);
    assert_eq!(invalid.validate(&limits), Err(Invalid::Text));

    let mut invalid = config();
    invalid.models = Box::new([]);
    assert_eq!(invalid.validate(&limits), Err(Invalid::Models));

    let mut invalid = config();
    invalid.accounts = Box::new([7, 7]);
    assert_eq!(invalid.validate(&limits), Err(Invalid::Accounts));

    let mut invalid = config();
    invalid.models[0].endpoint = Endpoint(99);
    assert_eq!(invalid.validate(&limits), Err(Invalid::Endpoint));
}

#[test]
fn budget_and_rate_refusals_precede_agent_construction() {
    let limits = limits();
    let ceiling = limits.smith.run.budget;
    for (budget, reason) in [
        (run::Budget { turns: ceiling.turns + 1, ..config().budget }, run::Ceiling::Turns(ceiling.turns)),
        (run::Budget { spend: ceiling.spend + 1, ..config().budget }, run::Ceiling::Spend(ceiling.spend)),
        (
            run::Budget {
                time: ceiling.time.checked_add(Duration::from_secs(1)).expect("bounded time"),
                ..config().budget
            },
            run::Ceiling::Time(ceiling.time),
        ),
    ] {
        let mut invalid = config();
        invalid.budget = budget;
        assert_eq!(Domain::new(invalid, &limits, 1).err(), Some(Invalid::Budget(run::BudgetRefusal::Ceiling(reason))));
    }
    for rate in [run::Rate::Input, run::Rate::Cached, run::Rate::CacheWrite, run::Rate::Output] {
        let mut invalid = config();
        match rate {
            run::Rate::Input => invalid.models[0].prices.input = 0,
            run::Rate::Cached => invalid.models[0].prices.cached = 0,
            run::Rate::Output => invalid.models[0].prices.output = 0,
            run::Rate::CacheWrite => invalid.models[0].prices.cache_write = Some(0),
        }
        assert_eq!(Domain::new(invalid, &limits, 1).err(), Some(Invalid::Price(rate)));
    }
}

#[test]
#[expect(
    clippy::wildcard_enum_match_arm,
    clippy::match_like_matches_macro,
    reason = "unit follows one actual process lifecycle and durable ACK through Gone"
)]
fn spawned_agent_receives_saved_start_message_and_durable_turn_ack() {
    use smith_host_domain::{self as host, process};
    let (mut domain, env, mut out, owner) = spawned_chat_with_saved_hello();
    let turn = smith_domain::Turn {
        kind: smith_domain::session::record::Kind::First {
            affinity: smith_domain::llm::Affinity { key: [0; 16], thread: 0 },
        },
        version: smith_domain::session::record::VERSION,
        endpoint: smith_domain::session::llm::Endpoint(4),
        dialect: 1,
        sequence: 1,
        usage: smith_domain::session::llm::Usage::ZERO,
        spent: 0,
        messages: Box::default(),
    };
    crate::step(
        &mut domain,
        &env,
        Event::Process(process::Event::Received {
            owner,
            message: host::Up::Turn {
                turn: host::Turn {
                    opens_window: true,
                    number: 1,
                    read: Some(Token::new(1)),
                    spent: 0,
                    body: host::TurnValue::new(turn, host_limits().turn_bytes).expect("sealed concrete turn"),
                },
            },
        }),
        &mut out,
    );
    let outputs = drain_requests(&mut out);
    assert!(outputs.iter().any(|request| match request {
        Request::SaveTurn { number: 1, read: Some(name), .. } if *name == Token::new(1) => true,
        _ => false,
    }));
    assert!(!outputs.iter().any(|request| match request {
        Request::Process(process::Request::Send { message: host::Down::Acknowledge { .. }, .. }) => true,
        _ => false,
    }));
    crate::step(&mut domain, &env, Event::TurnSaved { number: 1 }, &mut out);
    let outputs = drain_requests(&mut out);
    assert!(outputs.iter().any(|request| match request {
        Request::Process(process::Request::Send { message: host::Down::Acknowledge { turn: 1 }, .. }) => true,
        _ => false,
    }));
    crate::step(&mut domain, &env, Event::Process(process::Event::Sent { owner }), &mut out);
    drain_requests(&mut out);
    crate::step(
        &mut domain,
        &env,
        Event::Process(process::Event::Received {
            owner,
            message: host::Up::Answer {
                answer: host::Answer { read: Some(Token::new(1)), turns: 1, spent: 0, result: host::RunResult::Parked },
            },
        }),
        &mut out,
    );
    let outputs = drain_requests(&mut out);
    assert!(
        !outputs.iter().any(|request| match request {
            Request::Show { text } if text.as_ref() == b"Chat parked" => true,
            _ => false,
        }),
        "answer waits for process exit and empty tree"
    );
    for request in outputs {
        if let Request::Process(process::Request::Signal { owner, .. }) = request {
            crate::step(&mut domain, &env, Event::Process(process::Event::Signalled { owner }), &mut out);
        }
    }
    drain_requests(&mut out);
    crate::step(&mut domain, &env, Event::Process(process::Event::Hangup { owner }), &mut out);
    crate::step(
        &mut domain,
        &env,
        Event::Process(process::Event::Exited { owner, exit: Some(smith_host_domain::Exit::Code(0)) }),
        &mut out,
    );
    crate::step(&mut domain, &env, Event::Process(process::Event::Reaped { owner, detail: Box::default() }), &mut out);
    let outputs = drain_requests(&mut out);
    assert!(outputs.iter().any(|request| match request {
        Request::Show { text } if text.as_ref() == b"Chat parked" => true,
        _ => false,
    }));
    assert!(outputs.iter().any(|request| match request {
        Request::Load => true,
        _ => false,
    }));
}

#[expect(
    clippy::wildcard_enum_match_arm,
    clippy::match_like_matches_macro,
    reason = "unit fixture validates selected process effects"
)]
pub(super) fn spawned_chat_with_saved_hello() -> (Domain, Env<Limits>, Queue<Request>, Token) {
    use smith_host_domain::{self as host, process};
    let mut limits = limits();
    let mut child_limits = host_limits();
    child_limits.turns = 9;
    limits.agent = crate::AgentLimits::Spawned(child_limits);
    let env = Env { now: Time::ZERO, wall: Wall::from_nanos(0), limits: limits.clone() };
    let mut domain = Domain::new(config(), &limits, 11).expect("spawned local domain");
    let mut out = Queue::with_capacity(crate::max_out(&limits));
    crate::resume(&mut domain, &env, &mut out);
    assert!(match out.pop() {
        Some(Request::Load) => true,
        _ => false,
    });
    crate::step(
        &mut domain,
        &env,
        Event::Loaded { state: None, transcript: None, deliveries: Box::default() },
        &mut out,
    );
    crate::step(&mut domain, &env, Event::Line { text: Box::from(&b"hello"[..]) }, &mut out);
    assert!(match out.pop() {
        Some(Request::Credential { account: 7 }) => true,
        _ => false,
    });
    let Some(Request::SaveState { state, fresh: false }) = out.pop() else { panic!("activation saved") };
    assert_eq!(state.activation, 1);
    crate::step(
        &mut domain,
        &env,
        Event::Credential {
            grant: smith_domain::Grant {
                name: smith_domain::GrantName { account: 7, generation: 1 },
                valid: Duration::from_secs(60),
            },
        },
        &mut out,
    );
    crate::step(&mut domain, &env, Event::StateSaved, &mut out);
    let Some(Request::Process(process::Request::Spawn { owner, .. })) = out.pop() else { panic!("owned child spawn") };
    crate::step(&mut domain, &env, Event::Process(process::Event::Spawned { owner, process: Token::new(3) }), &mut out);
    let Some(Request::Process(process::Request::Send { message: host::Down::Start { start, window }, .. })) = out.pop()
    else {
        panic!("typed start first")
    };
    assert_eq!(window.turns, limits.unsaved, "advertised credit fits the durable save queue");
    assert_eq!(start.activation, 1);
    assert_eq!(start.grants.len(), 1);
    assert_eq!(start.charter.into_value().instructions.as_ref(), b"Assist the person");
    assert!(match out.pop() {
        Some(Request::Process(process::Request::Wait { .. })) => true,
        _ => false,
    });
    assert!(match out.pop() {
        Some(Request::Process(process::Request::Reap { .. })) => true,
        _ => false,
    });
    crate::step(&mut domain, &env, Event::Process(process::Event::Sent { owner }), &mut out);
    assert!(match out.pop() {
        Some(Request::Process(process::Request::Read { .. })) => true,
        _ => false,
    });
    crate::step(
        &mut domain,
        &env,
        Event::Process(process::Event::Received { owner, message: host::Up::Admitted }),
        &mut out,
    );
    let outputs = drain_requests(&mut out);
    let message = outputs
        .iter()
        .find_map(|request| match request {
            Request::Process(process::Request::Send { message: host::Down::Message { name, label, text }, .. }) => {
                Some((name, label, text))
            }
            _ => None,
        })
        .expect("first named person line");
    assert_eq!(*message.0, Token::new(1));
    assert_eq!(message.1.as_ref(), b"person");
    assert_eq!(message.2.as_ref(), b"hello");
    crate::step(&mut domain, &env, Event::Process(process::Event::Sent { owner }), &mut out);
    drain_requests(&mut out);
    (domain, env, out, owner)
}

fn drain_requests(out: &mut Queue<Request>) -> Vec<Request> {
    let mut requests = Vec::new();
    while let Some(request) = out.pop() {
        requests.push(request);
    }
    requests
}

pub(super) fn host_limits() -> smith_host_domain::Limits {
    smith_host_domain::Limits {
        agents: 1,
        directories: 4,
        conflicts: 16,
        path_bytes: 1024,
        name_bytes: 256,
        accounts: 8,
        charter_bytes: 65_536,
        transcript_bytes: 1_048_576,
        answered_bytes: 1_048_576,
        message_bytes: 4096,
        messages: 8,
        calls: 8,
        call_bytes: 4096,
        answer_bytes: smith_host_domain::Delivered::worst_case(),
        turns: 2,
        turn_bytes: 1_048_576,
        unacknowledged_bytes: 2_097_152,
        fact_bytes: 512,
        outcome_bytes: 4096,
        detail_bytes: 512,
        spawn_timeout: Duration::from_secs(5),
        no_progress: Duration::from_secs(60),
        silence: Duration::ZERO,
        long_span: Duration::from_secs(3600),
        long_open: 64,
        wall_time: Duration::from_secs(3600),
        grace: Duration::from_secs(5),
        kill_after: Duration::from_secs(1),
        facts: 64,
    }
}

#[test]
fn invalid_or_unowned_model_declarations_refuse_local_startup() {
    let limits = limits();
    for output in [false, true] {
        let mut invalid = config();
        if output {
            invalid.models[0].output = 0;
        } else {
            invalid.models[0].window = 0;
        }
        assert_eq!(invalid.validate(&limits), Err(Invalid::Models));
    }
    let config = config();
    let limits = Limits { smith: smith_domain::Limits { configured_model_bytes: 0, ..limits.smith }, ..limits };
    assert_eq!(config.validate(&limits), Err(Invalid::Limits));
}

#[test]
fn impossible_completion_or_reserve_spend_refuses_before_local_agent_construction() {
    let limits = limits();
    for (spend, reason) in [
        (4_127, run::BudgetRefusal::Completion { most: 4_128 }),
        (8_256, run::BudgetRefusal::Reserve(run::Part::Spend)),
    ] {
        let mut impossible = config();
        impossible.models[0].prices.unit = 1;
        impossible.models[0].window = 4000;
        impossible.budget.spend = spend;
        impossible.reserve = run::charter::Reserve { turns: 2, time: Duration::from_secs(1) };
        assert_eq!(Domain::new(impossible, &limits, 71).err(), Some(Invalid::Budget(reason)));
    }
}

#[test]
fn a_one_turn_configuration_keeps_scalar_and_completion_startup_checks() {
    let limits = limits();
    let mut configured = config();
    configured.models[0].prices.unit = 1;
    configured.models[0].window = 4000;
    configured.budget.turns = 1;
    configured.reserve = run::charter::Reserve { turns: 2, time: Duration::from_secs(1) };
    configured.budget.spend = 4_127;
    assert_eq!(configured.validate(&limits), Err(Invalid::Budget(run::BudgetRefusal::Completion { most: 4_128 })));
    configured.budget.spend = 4_128;
    assert_eq!(configured.validate(&limits), Ok(()), "the real run still refuses the finishing turns");
    configured.budget.turns = 0;
    assert_eq!(configured.validate(&limits), Err(Invalid::Budget(run::BudgetRefusal::Empty(run::Part::Turns))));
    configured.budget.turns = limits.smith.run.budget.turns + 1;
    assert_eq!(
        configured.validate(&limits),
        Err(Invalid::Budget(run::BudgetRefusal::Ceiling(run::Ceiling::Turns(limits.smith.run.budget.turns))))
    );
}

fn saved_window_turn() -> smith_domain::Turn {
    smith_domain::Turn {
        kind: smith_domain::session::record::Kind::First {
            affinity: smith_domain::llm::Affinity { key: [0; 16], thread: 0 },
        },
        version: smith_domain::session::record::VERSION,
        endpoint: smith_domain::session::llm::Endpoint(4),
        dialect: 1,
        sequence: 1,
        usage: smith_domain::session::llm::Usage::ZERO,
        spent: 0,
        messages: Box::default(),
    }
}

#[test]
fn a_told_turns_window_flag_reaches_its_save() {
    use smith_host_domain::{self as host, process};
    for opens_window in [false, true] {
        let (mut domain, env, mut out, owner) = spawned_chat_with_saved_hello();
        let turn = saved_window_turn();
        crate::step(
            &mut domain,
            &env,
            Event::Process(process::Event::Received {
                owner,
                message: host::Up::Turn {
                    turn: host::Turn {
                        opens_window,
                        number: 1,
                        read: None,
                        spent: 0,
                        body: host::TurnValue::new(turn, host_limits().turn_bytes).expect("sealed turn"),
                    },
                },
            }),
            &mut out,
        );
        let outputs = drain_requests(&mut out);
        assert!(outputs.iter().any(|request| {
            if let Request::SaveTurn { opens_window: saved, .. } = request { *saved == opens_window } else { false }
        }));
    }
}
