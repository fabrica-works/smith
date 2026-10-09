//! Profile construction stays with the service whose limits it selects.

use alloc::boxed::Box;
use skein_lib::List;

#[test]
fn standard_profile_admits_the_priced_shell_run_defaults() {
    use skein_lib::{Duration, Env, Queue, ReplyTo, Time, Token, Wall};
    use smith_domain::run;
    let mut profile = crate::profile::standard();
    profile.declared.memory = Some(u64::MAX);
    let limits = crate::profile::derive(
        &profile,
        &crate::profile::Configuration { environment_bytes: 0, endpoints: Box::new([]) },
    )
    .expect("standard profile")
    .domain;
    let charter = run::Charter {
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
            window: 131_072,
            output: 8192,
            dialect: 1,
        },
        models: Box::default(),
    };
    let mut domain = run::Domain::new(&limits.run);
    let mut out = Queue::with_capacity(run::MAX_OUT);
    run::step(
        &mut domain,
        &Env { now: Time::ZERO, wall: Wall::EPOCH, limits: limits.run },
        run::Event::Start {
            messages: Box::default(),
            reply_to: ReplyTo::new(Token::new(1)),
            host_run: Token::new(1),
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
        &crate::profile::Configuration { environment_bytes: 0, endpoints: Box::new([]) },
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
        let configuration = crate::profile::Configuration { environment_bytes: 37, endpoints: Box::new([]) };
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
    }
    let mut profile = crate::profile::standard();
    profile.declared.memory = Some(u64::MAX);
    profile.declared.search_bytes = u32::MAX;
    assert_eq!(
        crate::profile::derive(
            &profile,
            &crate::profile::Configuration { environment_bytes: 0, endpoints: Box::new([]) }
        )
        .err(),
        Some(crate::profile::ProfileError::Receiving)
    );
}

#[test]
fn every_declared_conversation_can_use_the_same_endpoint() {
    for conversations in [1, 2, 3, 5, 8] {
        let mut profile = crate::profile::standard();
        profile.declared.memory = Some(u64::MAX);
        profile.declared.conversations = conversations;
        let configuration = crate::profile::Configuration { environment_bytes: 0, endpoints: Box::new([]) };
        let limits = crate::profile::derive(&profile, &configuration).expect("declared conversations derive");
        assert_eq!(limits.llm.connection.calls, conversations);
        assert_eq!(limits.llm.connection.connections, conversations);
        assert_eq!(limits.llm.connection.per_endpoint, conversations);
    }
}
