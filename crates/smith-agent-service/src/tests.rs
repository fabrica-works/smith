//! Profile construction stays with the service whose limits it selects.

#![allow(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "programming-model.md section10.2: deterministic test harnesses use ordinary Rust"
)]

use alloc::boxed::Box;
use skein_lib::List;

fn declarations(environment_bytes: u32) -> crate::profile::Configuration {
    use crate::profile::{Configuration, Endpoint, Model, OversizedReasoning};
    Configuration {
        environment_bytes,
        endpoints: Box::new([Endpoint {
            number: 0,
            provider: skein_llm::Provider::OpenAiCodex,
            option_bytes: 0,
            connect: skein_lib::Duration::from_secs(1),
            handshake: skein_lib::Duration::from_secs(1),
            models: Box::new([Model {
                name: Box::from(b"fake".as_slice()),
                window: 2_097_152,
                output: 65_536,
                reasoning_item: 2048,
                oversized_reasoning: OversizedReasoning::Fail,
                head: skein_lib::Duration::from_secs(1),
                idle: skein_lib::Duration::from_secs(1),
            }]),
        }]),
    }
}

fn priced_charter() -> smith_domain::run::Charter {
    use skein_lib::Duration;
    use smith_domain::run;
    run::Charter {
        reserve: run::charter::Reserve { turns: 0, time: Duration::ZERO },
        instructions: Box::default(),
        brief: run::Brief {
            sections: Box::new([run::Section { title: Box::default(), text: b"Review the change.".as_slice().into() }]),
        },
        conventions: None,
        resume: false,
        waiting: Duration::from_secs(300),
        grants: run::charter::Grants {
            wait: true,
            deliver: None,
            agents: false,
            host_tools: Box::default(),
            tools: run::charter::Tools { inspect: false, modify: false, shell: false },
        },
        outcome: run::outcome::OutcomeSpec {
            change: None,
            verdicts: Box::default(),
            failure: None,
            report: Some(run::outcome::TextSpec { max: 4096, fields: Box::default() }),
        },
        budget: run::Budget { turns: 512, spend: 25_000_000, time: Duration::from_secs(3600) },
        llm: run::charter::Llm {
            prices: run::Prices {
                input: 1_250_000,
                cached: 125_000,
                cache_write: None,
                output: 10_000_000,
                unit: 1_000_000,
            },
            account: 0,
            endpoint: run::charter::Endpoint(1),
            model: b"gpt-5".as_slice().into(),
            window: 2_097_152,
            output: 65_536,
            dialect: 1,
        },
        models: Box::default(),
    }
}

#[test]
fn standard_profile_admits_the_priced_shell_run_defaults() {
    use skein_lib::{Duration, Env, Queue, ReplyTo, Time, Token, Wall};
    use smith_domain::run;
    let mut profile = crate::profile::standard();
    profile.declared.memory = Some(u64::MAX);
    let limits = crate::profile::derive(&profile, &declarations(0)).expect("standard profile").domain;
    let charter = priced_charter();
    let mut domain = run::Domain::new(&limits.run, 0);
    let mut out = Queue::with_capacity(run::MAX_OUT);
    run::step(
        &mut domain,
        &Env { now: Time::ZERO, wall: Wall::EPOCH, limits: limits.run },
        run::Event::Start {
            messages: Box::default(),
            reply_to: ReplyTo::new(Token::new(1)),
            host_run: Token::new(1),
            identity: None,
            activation: 1,
            window: run::Window { turns: u32::MAX, bytes: u64::MAX, largest_turn: 1 },
            charter,
            workspace: None,
            transcript: None,
            resumed: false,
        },
        &mut out,
    );
    let Some(run::Request::Admitted { .. }) = out.pop() else { panic!("priced defaults are admitted") };
    let Some(run::Request::Open { .. }) = out.pop() else { panic!("priced main opens") };
    assert!(out.is_empty());
    assert!(limits.session.spend >= 25_000_000);
    assert!(limits.session.budget.turns >= 512);
    assert!(limits.session.budget.time >= Duration::from_secs(3600));
}

#[test]
fn standard_profile_builds_a_complete_bounded_service() {
    let limits = crate::profile::derive(
        &{
            let mut profile = crate::profile::standard();
            profile.declared.memory = Some(u64::MAX);
            profile
        },
        &declarations(0),
    )
    .expect("standard profile");
    let configuration = crate::Config {
        limits,
        domain: smith_domain::Config { endpoints: Box::new([]), models: Box::new([]) },
        channel_endpoints: smith_protocol_channel::Endpoints::new(List::with_capacity(0)),
        llm_endpoints: smith_protocol_llm::Endpoints::new(
            Box::new([]),
            crate::profile::ENDPOINTS,
            crate::profile::ACCOUNTS,
        )
        .expect("empty endpoints"),
        environment: smith_protocol_machine::Environment::encode(&[], &limits.machine).expect("empty environment fits"),
        stream_mode: skein_channel::StreamMode::Two,
        capture_prompts: false,
    };
    let bound = crate::worst_case(&configuration.limits, &configuration.llm_endpoints).expect("checked bound");
    assert!(bound > 0);
    let service = crate::Service::new(configuration, 1).expect("service");
    assert!(crate::done(&service).is_none());
}

#[test]
fn random_tool_render_caps_derive_a_machine_that_holds_them() {
    let mut rng = skein_lib::Rng::new(941);
    for _ in 0_u32..256 {
        let mut profile = crate::profile::standard();
        profile.declared.memory = Some(u64::MAX);
        profile.declared.tool_payload = u32::try_from(rng.next_u64() % 65536 + 1).expect("bounded payload");
        profile.declared.read_window =
            u32::try_from(rng.next_u64() % u64::from(profile.declared.tool_payload) + 1).expect("bounded read");
        profile.declared.shell_head = u32::try_from(rng.next_u64() % 4096 + 1).expect("bounded head");
        profile.declared.shell_tail = u32::try_from(rng.next_u64() % 4096 + 1).expect("bounded tail");
        profile.declared.search_hits = u32::try_from(rng.next_u64() % 32 + 1).expect("bounded hits");
        profile.declared.search_bytes = u32::try_from(rng.next_u64() % 4096 + 1).expect("bounded search");
        profile.declared.list_entries = u32::try_from(rng.next_u64() % 64 + 1).expect("bounded listing");
        let configuration = declarations(37);
        let limits = crate::profile::derive(&profile, &configuration).expect("bounded declarations derive");
        let tools = limits.domain.session.tools;
        let machine = limits.machine;
        assert!(machine.output_bytes >= tools.shell_head);
        assert!(machine.output_bytes >= tools.shell_tail);
        assert!(machine.file_bytes >= tools.file_bytes);
        assert!(machine.entries >= tools.list_entries);
        assert!(machine.entry_bytes >= tools.list_bytes);
        assert!(machine.search_hits >= tools.search_hits);
        assert!(machine.search_bytes >= tools.search_bytes);
        assert!(machine.path_bytes >= tools.path_bytes);
        assert_eq!(machine.env_bytes, configuration.environment_bytes);
        let mut stated = smith_protocol_llm::Stated::from_tools(&tools, limits.llm.adapter.client.input)
            .expect("derived quantities fit");
        stated.host_part = limits.domain.run.host_part;
        assert_eq!(limits.llm.adapter.stated, stated);
    }
    let mut profile = crate::profile::standard();
    profile.declared.memory = Some(u64::MAX);
    profile.declared.search_bytes = u32::MAX;
    assert_eq!(
        crate::profile::derive(&profile, &declarations(0)).err(),
        Some(crate::profile::Refusal::Derived {
            limit: crate::profile::LimitName::deployment(crate::profile::Layer::Render, "result"),
        })
    );
}

#[test]
fn every_declared_conversation_can_use_the_same_endpoint() {
    for conversations in [1, 2, 3, 5, 8] {
        let mut profile = crate::profile::standard();
        profile.declared.memory = Some(u64::MAX);
        profile.declared.conversations = conversations;
        let configuration = declarations(0);
        let limits = crate::profile::derive(&profile, &configuration).expect("declared conversations derive");
        assert_eq!(limits.llm.connection.calls, conversations);
        assert_eq!(limits.llm.connection.connections, conversations);
        assert_eq!(limits.llm.connection.per_endpoint, conversations);
    }
}

#[test]
fn scalar_event_projection_preserves_subsecond_child_time_and_absent_requests() {
    use skein_lib::{Duration, Token};
    use smith_domain::run;
    use smith_events::{ChildStop, ShareBound, SharePart, Usage};
    for millis in [750, 1999] {
        let kind = run::facts::Kind::Child {
            parent: Token::new(1),
            call: Token::new(2),
            asked: run::Share::UNASKED,
            granted: run::Granted {
                turns: 3,
                time: Duration::from_millis(millis),
                spend: 100,
                turns_set: run::Set::Pool,
                time_set: run::Set::Deadline,
            },
        };
        let fields = crate::child_share(kind).expect("actual child role");
        assert_eq!(fields.requested, smith_events::Requested { turns: None, seconds: None });
        assert_eq!(fields.share.time_ms, millis);
        assert_eq!(
            fields.clamped.as_slice(),
            &[smith_events::Clamp { part: SharePart::Time, by: ShareBound::Deadline }]
        );
        let usage = Usage {
            input_tokens: None,
            cache_read_tokens: Some(7),
            cache_write_tokens: None,
            output_tokens: Some(8),
            reasoning_tokens: None,
        };
        let terminal = crate::child_completed(
            2,
            ChildStop::Budget,
            run::facts::Used { turns: 1, time: Duration::from_millis(millis), spend: 3 },
            usage.clone(),
        );
        assert_eq!(terminal.time_ms, millis);
        assert_eq!(terminal.usage, usage, "unknown native counts remain unknown");
    }
    assert!(crate::child_share(run::facts::Kind::Main).is_none());
}

#[test]
fn actual_root_budget_facts_encode_without_content_under_every_capture() {
    use smith_domain::{Fact, run};
    use smith_events::{Capture, Event};
    let mut world = smith_agent_world::World::new(smith_agent_world::Settings::calm(34));
    world.run(1_000_000);
    let mut budget_count = 0_u32;
    let limits = smith_events::Limits { string: 64, content: 1024, items: 8 };
    for fact in world.facts() {
        let Fact::Run { fact } = fact else { continue };
        let Some(record) = crate::budget_record(fact, skein_lib::Time::ZERO) else { continue };
        budget_count += 1;
        for capture in [Capture::None, Capture::Calls, Capture::Everything] {
            let line = smith_events::write(&record, &capture, &limits)
                .expect("valid bounded event fixture")
                .expect("valid bounded event fixture");
            assert_eq!(smith_events::read(&line, &limits).expect("valid bounded event fixture"), Some(record.clone()));
            let Event::Budget(value) = &record.event else { panic!("budget projection") };
            let run::facts::FactKind::Budget { standing, .. } = fact.kind else { panic!("actual budget fact") };
            assert_eq!(
                value.time_ms.used,
                standing.time.as_nanos() / 1_000_000,
                "run time retains its admission origin"
            );
        }
    }
    assert!(budget_count > 0, "real root/session completions produce charges");
}

#[test]
fn the_measured_large_contract_is_admitted_at_the_host_edge_and_refused_one_byte_later() {
    use skein_lib::{Env, Queue, ReplyTo, Time, Token, Wall};
    use smith_domain::run;
    let mut profile = crate::profile::standard();
    profile.declared.memory = Some(u64::MAX);
    let receiving = crate::profile::derive(&profile, &declarations(0)).expect("declared model").domain.run;
    let enter = |bound, instructions: &[u8]| {
        let limits = run::Limits { host_part: bound, ..receiving };
        let mut charter = priced_charter();
        charter.instructions = instructions.into();
        charter.outcome.report = Some(run::outcome::TextSpec {
            max: 32,
            fields: (0_u32..20)
                .map(|index| run::outcome::FieldRule {
                    name: format!("long_contract_field_{index:02}").into_bytes().into(),
                    max: 32,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        });
        let mut domain = run::Domain::new(&limits, 0);
        let mut out = Queue::with_capacity(run::MAX_OUT);
        run::step(
            &mut domain,
            &Env { now: Time::ZERO, wall: Wall::EPOCH, limits },
            run::Event::Start {
                messages: Box::default(),
                reply_to: ReplyTo::new(Token::new(1)),
                host_run: Token::new(1),
                identity: None,
                activation: 1,
                window: run::Window { turns: u32::MAX, bytes: u64::MAX, largest_turn: 1 },
                charter,
                workspace: None,
                transcript: None,
                resumed: false,
            },
            &mut out,
        );
        out.pop().expect("one entrance terminal")
    };
    let mut lower = 0;
    let mut upper = receiving.host_part;
    let run::Request::Admitted { .. } = enter(upper, b"base") else { panic!("upper edge admits") };
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        match enter(middle, b"base") {
            run::Request::Admitted { .. } => upper = middle,
            run::Request::Answer {
                answer: run::Answer::Refused(run::Refusal::Invalid(run::Invalid::HostPart { .. })),
                ..
            } => lower = middle + 1,
            other @ (run::Request::MessageRefused { .. }
            | run::Request::CompletionReady { .. }
            | run::Request::Waiting { .. }
            | run::Request::Turn { .. }
            | run::Request::HostCall { .. }
            | run::Request::WithdrawHost { .. }
            | run::Request::Answer { .. }
            | run::Request::Open { .. }
            | run::Request::Say { .. }
            | run::Request::Close { .. }
            | run::Request::Read { .. }
            | run::Request::Probe { .. }
            | run::Request::Check { .. }
            | run::Request::Abort { .. }
            | run::Request::Checking { .. }
            | run::Request::ChecksEnded { .. }
            | run::Request::Deliver { .. }
            | run::Request::Return { .. }) => panic!("unexpected entrance: {other:?}"),
        }
    }
    assert!(lower > 2000, "the contract carries substantial rendered schemas and text");
    let run::Request::Admitted { .. } = enter(lower, b"base") else { panic!("exact edge admits") };
    let run::Request::Answer {
        answer: run::Answer::Refused(run::Refusal::Invalid(run::Invalid::HostPart { bound })),
        ..
    } = enter(lower, b"basex")
    else {
        panic!("one byte over refuses host part")
    };
    assert_eq!(bound, lower);
}

#[test]
fn an_admitted_opening_fits_the_derived_request_with_escaped_host_text() {
    use skein_lib::{Env, Queue, ReplyTo, Time, Token, Wall};
    use smith_domain::{llm, run};
    let mut profile = crate::profile::standard();
    profile.declared.memory = Some(u64::MAX);
    let limits = crate::profile::derive(&profile, &declarations(0)).expect("real model");
    let mut selected = priced_charter();
    selected.outcome.report = Some(run::outcome::TextSpec { max: 32, fields: Box::default() });
    let mut domain = run::Domain::new(&limits.domain.run, 0);
    let mut out = Queue::with_capacity(run::MAX_OUT);
    selected.instructions = vec![1; 3500].into();
    run::step(
        &mut domain,
        &Env { now: Time::ZERO, wall: Wall::EPOCH, limits: limits.domain.run },
        run::Event::Start {
            messages: Box::default(),
            reply_to: ReplyTo::new(Token::new(1)),
            host_run: Token::new(1),
            identity: None,
            activation: 1,
            window: run::Window { turns: u32::MAX, bytes: u64::MAX, largest_turn: 1 },
            charter: selected,
            workspace: None,
            transcript: None,
            resumed: false,
        },
        &mut out,
    );
    let Some(run::Request::Admitted { .. }) = out.pop() else { panic!("opening admitted") };
    let Some(run::Request::Open { opening, .. }) = out.pop() else { panic!("admitted opening") };
    let prompt = llm::Prompt {
        choice: llm::ToolChoice::Auto,
        namespace: smith_domain::tools::Namespace::One,
        affinity: llm::Affinity { key: [0; 16], thread: 0 },
        asker: llm::Asker::Main,
        endpoint: llm::Endpoint(opening.llm.endpoint.0),
        model: opening.llm.model,
        system: opening.system,
        tools: smith_domain::tools::Grants { inspect: false, modify: false, shell: false },
        served: Box::new([llm::Served::Wait, llm::Served::Finish]),
        messages: Box::new([llm::Message {
            role: llm::Role::User,
            content: Box::new([llm::Block::Text { text: opening.prompt, replay: None }]),
        }]),
        max_tokens: opening.llm.output,
    };
    let application = smith_protocol_llm::schemas_for_contract(
        &prompt,
        &limits.llm.adapter.stated,
        &run::outcome::OutcomeSpec {
            change: None,
            verdicts: Box::default(),
            failure: None,
            report: Some(run::outcome::TextSpec { max: 32, fields: Box::default() }),
        },
        None,
        limits.llm.adapter.client.request,
    )
    .expect("schemas");
    let mut translated =
        smith_protocol_llm::prompt(prompt, &application, &limits.llm.adapter).expect("translated opening");
    translated.output_ceiling(skein_llm::Provider::OpenAiCodex, 4096).expect("configured model output");
    let prepared = skein_llm::client::Client::prepare(
        skein_llm::Call {
            owner: Token::new(1),
            prompt: translated,
            endpoint: skein_llm::Endpoint::codex(),
            credential: skein_llm::Credential {
                access_token: b"fake".as_slice().into(),
                account_id: b"fake".as_slice().into(),
            },
        },
        &limits.llm.adapter.client,
    );
    assert!(prepared.is_ok(), "admitted host bytes must fit the derived request: {:?}", prepared.err());
}
