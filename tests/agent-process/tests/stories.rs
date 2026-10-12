use smith_agent_process_world::{World, charter};
use smith_channel::{RunFailure, RunResult};

#[test]
fn an_answered_agent_exits_0_with_no_signal_from_its_host() {
    let charter = charter();
    let mut world = World::new(7, &charter);
    assert!(world.settle(), "the admitted run answered");
    assert_eq!(world.observed().iter().filter(|frame| frame.kind == 0x0110).count(), 1);
    assert!(world.peer_replied(), "the fake LLM served a plaintext response: {:?}", world.peer_observations());
    assert_eq!(world.peer_queries().len(), 1, "the independent peer decoded one model request");
    let answer = world.answer().expect("host saw final answer");
    assert_eq!(answer.turns(), 1);
    assert!(matches!(answer.result(), RunResult::Failed(_)), "one-turn budget ended the run");
    assert_eq!(world.exit(), Some(skein_io::kernel::Exit::Code(0)));
    assert!(!world.signalled(), "answered process exits without a host signal");
    assert!(!String::from_utf8(world.errors()).expect("diagnostic UTF-8").contains("could not answer"));
    assert!(world.errors().starts_with(b"smith: commands run in process groups and are not contained\n"));
    world.assert_agent_clean();
}

#[test]
fn a_failed_answer_ends_standard_error_with_its_failure_in_words() {
    let mut world = World::new(7, &charter());
    assert!(world.settle());
    let errors = String::from_utf8(world.errors()).expect("diagnostic UTF-8");
    assert_eq!(errors.lines().last(), Some("smith: run failed: out of turns"));
    assert_eq!(errors.lines().filter(|line| line.starts_with("smith: run failed:")).count(), 1);
    assert_eq!(world.exit(), Some(skein_io::kernel::Exit::Code(0)));
    world.assert_agent_clean();
}

#[test]
fn a_termination_signal_in_the_middle_of_a_turn_answers_cancelled() {
    let charter = charter();
    let mut world = World::new(8, &charter);
    world.signal();
    assert!(world.settle(), "cancellation was answered");
    assert!(world.observed().iter().any(|frame| frame.kind == 0x0106), "run admitted before signal");
    let answer = world.answer().expect("host saw final answer");
    assert!(matches!(answer.result(), RunResult::Failed(failed) if failed.reason() == &RunFailure::Cancelled));
    world.assert_agent_clean();
}

#[test]
fn full_kernel_replay_leaves_the_same_answer() {
    for cancel in [false, true] {
        skein_world::domain::assert_replays(7, 8, |seed| {
            let mut world = World::new(seed, &charter());
            if cancel {
                world.signal();
            }
            assert!(world.settle());
            (world.trace(), world.answer())
        });
    }
}

#[test]
fn an_agent_whose_host_hangs_up_before_its_answer_says_so_and_exits_1() {
    let mut world = World::new(9, &charter());
    world.hang_up();
    assert!(world.settle(), "the unstarted channel settles without a signal");
    assert_eq!(world.exit(), Some(skein_io::kernel::Exit::Code(1)));
    assert!(world.answer().is_none());
    assert!(!world.signalled());
    let errors = String::from_utf8(world.errors()).expect("diagnostic UTF-8");
    assert!(
        errors.starts_with(
            "smith: commands run in process groups and are not contained\nsmith: agent started; worst case "
        )
    );
    assert_eq!(errors.lines().last(), Some("smith: the run could not answer: the channel ended before the answer"));
    assert_eq!(errors.lines().count(), 3, "containment, startup and failure each appear once");
    world.assert_agent_clean();
}

#[test]
fn a_refused_configuration_writes_one_line_and_exits_1_without_a_channel() {
    use smith_agent_shell::{Agent, Resources};
    let errors = smith_agent_process_world::AgentErrors::default();
    let startup = Agent::read(
        std::path::Path::new(""),
        Resources {
            input: skein_io::kernel::Fd::new(100),
            output: skein_io::kernel::Fd::new(101),
            signals: skein_io::kernel::Fd::new(102),
            error: None,
            seed: 1,
            pid: 101,
            trace: None,
            roots: Some(Box::new([])),
        },
        Box::new(errors.clone()),
    );
    let exit = match startup {
        Ok(agent) => {
            assert!(skein_world::Host::is_empty(&agent), "a refused startup never adopts these nonexistent streams");
            skein_world::Host::exit(&agent)
        }
        Err(_) => Some(skein_io::kernel::Exit::Code(1)),
    };
    assert_eq!(exit, Some(skein_io::kernel::Exit::Code(1)));
    let text = String::from_utf8(errors.bytes()).expect("diagnostic UTF-8");
    assert!(text.starts_with("smith: configuration metadata:"));
    assert_eq!(text.lines().count(), 1);
}

#[test]
fn the_configured_environment_reaches_the_actual_agents_command_spawn() {
    let mut world = World::new(42, &charter());
    world.configured_environment();
    assert!(world.settle());
    let observed = world.command_environments();
    assert_eq!(observed.len(), 1, "the granted model call started one actual command");
    let expected: &[Box<[u8]>] =
        &[Box::from(b"BUILD_FLAG=configured=value".as_slice()), Box::from(b"EMPTY=".as_slice())];
    assert_eq!(observed[0].as_ref(), expected);
    assert_eq!(world.exit(), Some(skein_io::kernel::Exit::Code(0)));
    world.assert_agent_clean();
}

#[test]
fn a_broken_declared_relationship_is_named_before_channel_adoption() {
    use smith_agent_shell::{Agent, Resources};
    let errors = smith_agent_process_world::AgentErrors::default();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/read-past-payload.json");
    let startup = Agent::read(
        &path,
        Resources {
            input: skein_io::kernel::Fd::new(100),
            output: skein_io::kernel::Fd::new(101),
            signals: skein_io::kernel::Fd::new(102),
            error: None,
            seed: 1,
            pid: 101,
            trace: None,
            roots: Some(Box::new([])),
        },
        Box::new(errors.clone()),
    );
    match startup {
        Ok(_) => panic!("broken read/payload relationship must refuse before adopting nonexistent channel handles"),
        Err(refusal) => {
            assert_eq!(refusal, "read-in-payload: declared.read_window=1025 bytes; declared.tool_payload=1024 bytes");
        }
    }
    let text = String::from_utf8(errors.bytes()).expect("diagnostic UTF-8");
    assert_eq!(text, "smith: read-in-payload: declared.read_window=1025 bytes; declared.tool_payload=1024 bytes\n");
}

#[test]
fn the_complete_shell_memory_refusal_precedes_channel_adoption() {
    use smith_agent_shell::{Agent, Resources, config};
    let errors = smith_agent_process_world::AgentErrors::default();
    let mut configuration = config::parse(
        br#"{"profile":{"name":"standard","declared":{"memory":1099511627776,"tool_payload":256,"read_window":256}},"endpoints":[{"name":"fake","number":0,"dialect":0,"account":0,"provider":"codex","address":"127.0.0.1:8080","transport":"plaintext","connect":1000,"handshake":1000,"models":[{"name":"fake","window":65536,"output":512,"reasoning_item":64,"head":1000,"idle":1000}]}],"environment":[]}"#,
    )
    .expect("valid startup declaration");
    let available =
        smith_agent_service::worst_case(&configuration.service.limits, &configuration.service.llm_endpoints)
            .expect("bounded service");
    configuration.memory = available;
    let startup = Agent::new(
        configuration,
        Resources {
            input: skein_io::kernel::Fd::new(100),
            output: skein_io::kernel::Fd::new(101),
            signals: skein_io::kernel::Fd::new(102),
            error: None,
            seed: 1,
            pid: 101,
            trace: None,
            roots: Some(Box::new([])),
        },
        Box::new(errors.clone()),
    );
    let Err(refusal) = startup else {
        panic!("outer shell memory must refuse before adopting nonexistent channel handles");
    };
    assert!(refusal.starts_with("memory: service.worst_case="));
    assert!(refusal.ends_with(&format!("; declared.memory={available} bytes")));
    assert_eq!(String::from_utf8(errors.bytes()).expect("UTF-8 refusal"), format!("smith: {refusal}\n"));
}
