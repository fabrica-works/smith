//! Entry-point tests for the local and host domain composition.

use alloc::boxed::Box;
use skein_lib::{Duration, List, Time, Wall};
use skein_world::Host;
use smith_domain::run::{self, charter::Endpoint, outcome};
use smith_local_domain as local;

use crate::{Config, Launch, Limits, ProcessLimits, Service, iterate};

fn windows(service: &mut Service) -> skein_lib::Queue<skein_io::kernel::Submit> {
    iterate(service, Time::ZERO, Wall::EPOCH);
    let mut windows = skein_lib::Queue::with_capacity(2);
    for _ in 0..service.submissions().capacity() {
        let Some(submit) = service.submissions().pop() else { break };
        if let skein_io::kernel::Op::Window { .. } = submit.kind {
            windows.push(submit);
        }
    }
    windows
}

#[test]
fn a_resize_during_a_window_read_is_read_after_its_terminal() {
    use skein_io::kernel::{Complete, Done, Fd, Size};
    let mut service = Service::new(config(), 7).unwrap();
    service.adopt_terminal(Some(Fd::new(10)), Fd::new(11)).unwrap();
    service.read_window();
    service.read_window();
    let mut first = windows(&mut service);
    assert_eq!(first.len(), 1, "startup has one window read");
    service.read_window();
    service.read_window();
    assert!(windows(&mut service).is_empty(), "only one window read is in flight");
    let submit = first.pop().unwrap();
    service.completions().push(Complete {
        op: submit.op,
        kind: submit.kind,
        result: Ok(Done::Window(Size { rows: 24, columns: 80 })),
    });
    assert_eq!(windows(&mut service).len(), 1, "the later resize is read next");
    let Some(skein_io::Event::Window { window: skein_io::Window::Size(size), .. }) = service.writer_events().pop()
    else {
        panic!("the shell hears the first window's size")
    };
    assert_eq!(size.columns, 80);
}

pub(super) fn config() -> Config {
    let mut local_limits = local::Limits {
        smith: smith_agent_world::LIMITS,
        agent: local::AgentLimits::inline(
            smith_agent_world::LIMITS,
            2,
            Duration::from_secs(60),
            Duration::from_secs(5),
        )
        .expect("bounded inline child"),
        endpoints: Box::new([Endpoint(4)]),
        chat_bytes: 64,
        text_bytes: 1024,
        models: 1,
        line_bytes: 256,
        show_bytes: 1024,
        unsaved: 2,
        facts: 16,
    };
    let mut host_limits = smith_host_world::limits();
    host_limits.agents = 1;
    host_limits.accounts = 1;
    local_limits.agent = local::AgentLimits::Spawned(host_limits);
    let queue = local::max_out(&local_limits).max(smith_host_domain::max_out(&host_limits)).max(64);
    let agent_limits = smith_agent_process_world::limits();
    let mut process = ProcessLimits {
        io: skein_io::Limits {
            sockets: 8,
            names: 0,
            refusals: 1,
            intake: 4096,
            receive: 4096,
            output: 1_000_000,
            sends: 8,
            accepts: 1,
            backlog: 2,
            close_timeout: Duration::from_secs(1),
            retry: Duration::from_millis(1),
        },
        channel: smith_host_protocol::Limits {
            bodies: agent_limits.channel.bodies,
            charter: agent_limits.channel.charter,
            transcript: agent_limits.channel.transcript,
            endpoints: agent_limits.channel.endpoints,
            channel: agent_limits.channel.channel,
            start_bytes: agent_limits.channel.start_bytes,
            calls: host_limits.calls,
        },
        detail_bytes: host_limits.detail_bytes,
        queue,
    };
    process.channel.channel.output_bytes =
        smith_host_protocol::output_bytes(&process.channel).expect("host directional frame");
    Config {
        agent: crate::AgentConfig::Spawned,
        local: local::Config {
            reserve: run::charter::Reserve { turns: 0, time: Duration::ZERO },
            protocol_allowances: Box::new([0]),
            progress: Box::new([smith_domain::session::Progress {
                head: Duration::from_secs(60),
                idle: Duration::from_secs(30),
            }]),
            chat: Box::from(&b"main"[..]),
            instructions: Box::from(&b"Assist"[..]),
            brief: run::charter::Brief { sections: Box::new([]) },
            efforts: Box::new([Box::new([])]),
            models: Box::new([run::charter::Llm {
                effort: None,
                prices: run::Prices { input: 1, cached: 1, cache_write: None, output: 1, unit: 1_000_000 },
                dialect: 1,
                account: Some(7),
                endpoint: Endpoint(4),
                model: Box::from(&b"small"[..]),
                window: 65_536,
                output: 128,
            }]),
            sub_agents: false,
            budget: run::Budget { turns: 8, spend: 1, time: Duration::from_secs(60) },
            conventions: None,
            contract: local::Contract::Report(outcome::TextSpec { max: 256, fields: Box::new([]) }),
            deliver: None,
            title_field: Box::from(&b"title"[..]),
            mode: smith_local_domain::Mode::Chat { waiting: Duration::from_secs(30) },
            resume: true,
            accounts: Box::new([7]),
            workspace: None,
            push: None,
        },
        limits: Limits { local: local_limits, inline: None, envelope_bytes: host_limits.answer_bytes, process, queue },
        endpoints: smith_protocol_channel::Endpoints::new(List::with_capacity(0)),
        paths: Box::new([]),
        launch: Launch {
            program: Box::from(&b"smith"[..]),
            arguments: Box::from([Box::from(&b"agent"[..]), Box::from(&b"agent.json"[..])]),
            environment: Box::new([]),
            root: skein_io::kernel::Fd::new(0),
            directory: Box::from(&b"."[..]),
        },
    }
}

#[test]
fn the_local_service_loads_then_routes_durable_start_requests_to_the_shell() {
    let mut service = Service::new(config(), 7).expect("bounded local service");
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    let Some(smith_local_protocol::StoreRequest::Load) = service.store_requests().pop() else { panic!("load first") };
    service.local_event(local::Event::Loaded { state: None, transcript: None, deliveries: Box::new([]) });
    for byte in b"hello\n" {
        service.feed(*byte);
    }
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    let Some(local::Request::Credential { account: 7 }) = service.shell_requests().pop() else {
        panic!("credential before start")
    };
    let Some(smith_local_protocol::StoreRequest::SaveState { state, history: smith_local_protocol::History::Kept }) =
        service.store_requests().pop()
    else {
        panic!("durable state before start")
    };
    assert_eq!(state.activation, 1);
}

#[test]
fn idle_chat_selection_routes_to_the_store_and_back_before_loading() {
    let mut service = Service::new(config(), 7).unwrap();
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    let Some(smith_local_protocol::StoreRequest::Load) = service.store_requests().pop() else {
        panic!("load selected chat")
    };
    service.local_event(local::Event::Loaded { state: None, transcript: None, deliveries: Box::new([]) });
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    service.local_event(local::Event::SelectChat { name: b"next".as_slice().into(), create: true });
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    let Some(smith_local_protocol::StoreRequest::SelectChat { name, create: true }) = service.store_requests().pop()
    else {
        panic!("typed selection")
    };
    assert_eq!(name.as_ref(), b"next");
    service.local_event(local::Event::ChatSelected { name, result: Ok(()) });
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    let Some(smith_local_protocol::StoreRequest::Load) = service.store_requests().pop() else {
        panic!("load selected chat")
    };
}

#[test]
fn a_local_service_reserves_the_selected_childs_complete_transition() {
    for mut config in [config(), colocated_config()] {
        config.limits.queue = local::max_out(&config.limits.local).checked_sub(1).expect("nonzero child output");
        let Err(crate::Error::Queue) = Service::new(config, 93) else {
            panic!("one missing output cell refuses startup")
        };
    }
}

#[test]
fn line_mode_selects_the_initial_chat_before_loading_and_shows_its_read_fence() {
    let mut service = Service::new(config(), 7).unwrap();
    service.line_mode(false);
    service.initial_chat();
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    let Some(smith_local_protocol::StoreRequest::SelectChat { name, create: false }) = service.store_requests().pop()
    else {
        panic!("select before loading")
    };
    assert_eq!(name.as_ref(), b"main");
    assert!(service.store_requests().is_empty());
    service.local_event(local::Event::ChatSelected { name, result: Ok(()) });
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    let Some(smith_local_protocol::StoreRequest::Load) = service.store_requests().pop() else {
        panic!("load selection")
    };
    service.local_event(local::Event::Loaded { state: None, transcript: None, deliveries: Box::new([]) });
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    assert_eq!(service.output().pop().unwrap().as_ref(), b"> ");
}

#[test]
fn slash_new_and_resume_route_only_to_the_idle_store() {
    let mut service = Service::new(config(), 7).unwrap();
    service.line_mode(true);
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    service.store_requests().pop().unwrap();
    service.local_event(local::Event::Loaded { state: None, transcript: None, deliveries: Box::new([]) });
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    for (line, expected, create) in [(b"/new\n".as_slice(), b"7".as_slice(), true), (b"/resume main\n", b"main", false)]
    {
        for byte in line {
            service.feed(*byte);
        }
        iterate(&mut service, Time::ZERO, Wall::EPOCH);
        let Some(smith_local_protocol::StoreRequest::SelectChat { name, create: selected }) =
            service.store_requests().pop()
        else {
            panic!("a local command selects, without a run")
        };
        assert_eq!(name.as_ref(), expected);
        assert_eq!(selected, create);
        assert!(service.shell_requests().is_empty());
        service.local_event(local::Event::ChatSelected { name, result: Ok(()) });
        iterate(&mut service, Time::ZERO, Wall::EPOCH);
        let Some(smith_local_protocol::StoreRequest::Load) = service.store_requests().pop() else {
            panic!("load selection")
        };
        service.local_event(local::Event::Loaded { state: None, transcript: None, deliveries: Box::new([]) });
        iterate(&mut service, Time::ZERO, Wall::EPOCH);
    }
}

#[test]
fn an_unknown_slash_line_never_requests_a_credential_or_starts_a_run() {
    let mut service = Service::new(config(), 7).unwrap();
    service.line_mode(true);
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    service.store_requests().pop().unwrap();
    service.local_event(local::Event::Loaded { state: None, transcript: None, deliveries: Box::new([]) });
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    while service.output().pop().is_some() {}
    for byte in b"/unknown\n" {
        service.feed(*byte);
    }
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    assert!(service.shell_requests().is_empty());
    assert!(service.store_requests().is_empty());
    assert_eq!(service.output().pop().unwrap().as_ref(), b"Unknown or incomplete command");
}

#[test]
fn model_and_effort_are_refused_in_words_without_changing_the_chat() {
    let mut service = Service::new(config(), 7).unwrap();
    service.line_mode(true);
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    service.store_requests().pop().unwrap();
    service.local_event(local::Event::Loaded { state: None, transcript: None, deliveries: Box::new([]) });
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    while service.output().pop().is_some() {}
    for (line, words) in [
        (
            b"/model other/small\n".as_slice(),
            Some(b"Model other/small needs another invocation: smith --chat main -m other/small".as_slice()),
        ),
        (b"/effort high\n", None),
    ] {
        for byte in line {
            service.feed(*byte);
        }
        iterate(&mut service, Time::ZERO, Wall::EPOCH);
        assert_eq!(service.output().pop().as_deref(), words);
        assert_eq!(service.chat_name(), b"main");
        assert!(service.shell_requests().is_empty());
        assert!(service.store_requests().is_empty());
    }
}

#[test]
fn a_local_service_refuses_lower_configuration_for_the_other_agent_kind() {
    let mut inline = colocated_config();
    inline.agent = crate::AgentConfig::Spawned;
    let Err(crate::Error::Process) = Service::new(inline, 94) else { panic!("inline limits require native effects") };

    let mut spawned = config();
    spawned.agent =
        crate::AgentConfig::Inline { configuration: smith_agent_process_world::configuration(), roots: Box::new([]) };
    let Err(crate::Error::Process) = Service::new(spawned, 95) else {
        panic!("spawned limits require the process adapter")
    };
}

#[test]
fn an_inline_service_refuses_an_unpriced_part_of_its_lower_inventory() {
    let mut config = colocated_config();
    let priced = config.limits.inline.as_mut().expect("inline ownership priced");
    priced.memory = priced.memory.checked_sub(1).expect("nonzero lower ownership");
    let Err(crate::Error::Memory) = Service::new(config, 96) else {
        panic!("the actual inventory exceeds its advertised ownership")
    };
}

#[test]
fn original_conflict_files_are_read_through_the_services_kernel_routes() {
    use skein_lib::{Map, Queue, Token};
    let mut machine = skein_fake_machine::Machine::new();
    let handle = machine.lay(&[
        skein_fake_machine::Item::file(b"resolved.rs", b"clean\n"),
        skein_fake_machine::Item::file(b"conflict.rs", b"<<<<<<< ours\n"),
    ]);
    let mut sim = skein_sim::Sim::new(8, skein_sim::Config::calm());
    let pid = sim.spawn_process();
    let root = sim.root(pid, skein_sim::Handle::new(handle.raw()));
    let config = config();
    let mut process =
        crate::process::ProcessAdapter::new(&config.limits.process, config.launch, config.endpoints, 1).unwrap();
    process.adopt_delivery_roots(Box::new([root]), Box::new([]));
    let mut local_events = Queue::with_capacity(256);
    let mut host_events = Queue::with_capacity(256);
    let mut host_requests = Queue::with_capacity(256);
    let mut terminal_events = Queue::with_capacity(256);
    let mut terminal =
        smith_local_protocol::Terminal::new(smith_local_protocol::TerminalLimits { line_bytes: 256, show_bytes: 1024 })
            .unwrap();
    process.start_markers(
        Token::new(7),
        0,
        Box::new([Box::from(b"resolved.rs".as_slice()), Box::from(b"conflict.rs".as_slice())]),
        Time::from_nanos(1_000_000_000),
        &mut local_events,
    );
    for _ in 0_u32..100 {
        sim.reap(pid, process.completions());
        process.io_up(&process.at(sim.now(), sim.wall()));
        process.up(
            &process.at(sim.now(), sim.wall()),
            &mut host_events,
            &mut terminal,
            &mut terminal_events,
            &mut local_events,
            &mut None,
        );
        process.down(
            &process.at(sim.now(), sim.wall()),
            &mut host_requests,
            &mut None,
            &Map::with_capacity(1),
            &mut host_events,
            &mut None,
        );
        sim.submit(pid, process.submissions());
        let mut calls = Queue::with_capacity(256);
        let mut answers = Queue::with_capacity(256);
        sim.calls(&mut calls);
        while let Some(call) = calls.pop() {
            skein_fake_machine::step(&mut machine, call, &mut answers);
        }
        sim.answer(&mut answers);
        if !local_events.is_empty() {
            break;
        }
        if let Some(at) = sim.next_due() {
            sim.advance_to(at);
        }
    }
    let Some(local::Event::Git { owner, result: local::GitResult::Markers { first: Some(path) } }) = local_events.pop()
    else {
        panic!("marker terminal after file settlement")
    };
    assert_eq!(owner, Token::new(7));
    assert_eq!(path.as_ref(), b"conflict.rs");
    assert!(local_events.is_empty());
    assert!(!process.work_pending(sim.now()));
}

fn settle_file_io(
    process: &mut crate::process::ProcessAdapter,
    sim: &mut skein_sim::Sim,
    machine: &mut skein_fake_machine::Machine,
    pid: skein_sim::Pid,
    local_events: &mut skein_lib::Queue<local::Event>,
) {
    let mut host_events = skein_lib::Queue::with_capacity(256);
    let mut host_requests = skein_lib::Queue::with_capacity(256);
    let mut terminal_events = skein_lib::Queue::with_capacity(256);
    let mut terminal =
        smith_local_protocol::Terminal::new(smith_local_protocol::TerminalLimits { line_bytes: 256, show_bytes: 1024 })
            .unwrap();
    for _ in 0_u32..400 {
        sim.reap(pid, process.completions());
        process.io_up(&process.at(sim.now(), sim.wall()));
        process.up(
            &process.at(sim.now(), sim.wall()),
            &mut host_events,
            &mut terminal,
            &mut terminal_events,
            local_events,
            &mut None,
        );
        process.down(
            &process.at(sim.now(), sim.wall()),
            &mut host_requests,
            &mut None,
            &skein_lib::Map::with_capacity(1),
            &mut host_events,
            &mut None,
        );
        sim.submit(pid, process.submissions());
        let mut calls = skein_lib::Queue::with_capacity(256);
        let mut answers = skein_lib::Queue::with_capacity(256);
        sim.calls(&mut calls);
        while let Some(call) = calls.pop() {
            skein_fake_machine::step(machine, call, &mut answers);
        }
        sim.answer(&mut answers);
        if !process.work_pending(sim.now()) && sim.ready(pid) == 0 && !sim.deferred(pid) {
            match sim.next_due() {
                Some(at) => sim.advance_to(at),
                None => return,
            }
        }
    }
    panic!("file IO settles within bounded iterations")
}

#[test]
fn plain_status_uses_the_runs_snapshot_and_refreshes_it_before_the_next_run() {
    use skein_lib::{Queue, Token};
    let mut machine = skein_fake_machine::Machine::new();
    let handle = machine
        .lay(&[skein_fake_machine::Item::directory(b"src"), skein_fake_machine::Item::file(b"src/file", b"before")]);
    let mut sim = skein_sim::Sim::new(8, skein_sim::Config::calm());
    let pid = sim.spawn_process();
    let root = sim.root(pid, skein_sim::Handle::new(handle.raw()));
    let config = config();
    let mut process =
        crate::process::ProcessAdapter::new(&config.limits.process, config.launch, config.endpoints, 1).unwrap();
    process.adopt_delivery_roots(Box::new([root]), Box::new([]));
    let mut events = Queue::with_capacity(256);
    let mut directories = List::with_capacity(1);
    directories.push(0).unwrap();
    process.capture_plain(directories.clone(), Time::from_nanos(10_000_000_000));
    settle_file_io(&mut process, &mut sim, &mut machine, pid, &mut events);
    assert_eq!(process.take_capture_done(), Some(true));
    process.plain_status(Token::new(7), 0, Time::from_nanos(10_000_000_000), &mut events);
    settle_file_io(&mut process, &mut sim, &mut machine, pid, &mut events);
    let Some(local::Event::PlainStatus { changed: false, .. }) = events.pop() else { panic!("unchanged snapshot") };
    let file = machine.open(handle, b"src/file", skein_fake_machine::How::Read).unwrap();
    machine.write(file, 0, b"edited").unwrap();
    machine.close(file);
    process.plain_status(Token::new(8), 0, Time::from_nanos(10_000_000_000), &mut events);
    settle_file_io(&mut process, &mut sim, &mut machine, pid, &mut events);
    let Some(local::Event::PlainStatus { owner, changed: true }) = events.pop() else { panic!("content changed") };
    assert_eq!(owner, Token::new(8));
    process.capture_plain(directories, Time::from_nanos(10_000_000_000));
    settle_file_io(&mut process, &mut sim, &mut machine, pid, &mut events);
    assert_eq!(process.take_capture_done(), Some(true));
    process.plain_status(Token::new(9), 0, Time::from_nanos(10_000_000_000), &mut events);
    settle_file_io(&mut process, &mut sim, &mut machine, pid, &mut events);
    let Some(local::Event::PlainStatus { changed: false, .. }) = events.pop() else { panic!("fresh run baseline") };
    assert!(events.is_empty());
}

#[test]
#[expect(clippy::wildcard_enum_match_arm, reason = "the story names unexpected shell requests")]
fn a_colocated_chat_routes_a_turn_through_the_shared_plaintext_effects_without_spawning() {
    use skein_lib::Queue;
    let config = colocated_config();
    let mut sim_config = skein_sim::Config::calm();
    sim_config.wall = skein_tls_world::pki::VALID;
    let mut sim = skein_sim::Sim::new(8, sim_config);
    let pid = sim.spawn_process();
    let peer_pid = sim.spawn_process();
    let mut peer = smith_agent_process_world::fake::peer();
    let mut service = Service::new(config, 8).expect("colocated service");
    let mut machine = skein_fake_machine::Machine::new();
    let mut saved_turn = false;
    let mut shown = false;
    let mut sent = false;
    for _ in 0_u32..2000 {
        sim.reap(peer_pid, peer.completions());
        peer.iterate(sim.now(), sim.wall());
        sim.submit(peer_pid, peer.submissions());
        sim.reap(pid, service.completions());
        iterate(&mut service, sim.now(), sim.wall());
        while let Some(request) = service.shell_requests().pop() {
            match request {
                local::Request::Credential { account: 0 } => service.credential(
                    smith_domain::Grant {
                        name: smith_domain::GrantName { account: 0, generation: 1 },
                        valid: Duration::from_secs(7200),
                    },
                    smith_protocol_channel::GrantValue::Withdrawn {
                        account_id: Box::from(&b"acc"[..]),
                        bearer: Box::from(&b"token"[..]),
                    },
                ),
                local::Request::Notice {
                    notice:
                        local::Notice::Failure(local::Failure::Run {
                            failure:
                                smith_host_domain::RunFailure::Budget {
                                    exhausted: smith_host_domain::Exhausted::Turns, ..
                                },
                            ..
                        }),
                } => {}
                other => panic!("unexpected shell request {other:?}"),
            }
        }
        while let Some(request) = service.store_requests().pop() {
            match request {
                smith_local_protocol::StoreRequest::Load => {
                    service.local_event(local::Event::Loaded {
                        state: None,
                        transcript: None,
                        deliveries: Box::new([]),
                    });
                    if !sent {
                        for byte in b"hello\n" {
                            service.feed(*byte);
                        }
                        sent = true;
                    }
                }
                smith_local_protocol::StoreRequest::SaveState { .. } => service.local_event(local::Event::StateSaved),
                smith_local_protocol::StoreRequest::SaveTurn { number, .. } => {
                    saved_turn = true;
                    service.local_event(local::Event::TurnSaved { number });
                }
                other @ (smith_local_protocol::StoreRequest::SaveDelivery { .. }
                | smith_local_protocol::StoreRequest::SelectChat { .. }) => {
                    panic!("unexpected store request {other:?}");
                }
            }
        }
        while let Some(text) = service.output().pop() {
            shown |= !text.is_empty();
        }
        sim.submit(pid, service.submissions());
        let mut calls = Queue::with_capacity(256);
        let mut answers = Queue::with_capacity(256);
        sim.calls(&mut calls);
        while let Some(call) = calls.pop() {
            skein_fake_machine::step(&mut machine, call, &mut answers);
        }
        sim.answer(&mut answers);
        if saved_turn && !service.work_pending(sim.now()) && sim.ready(pid) == 0 && !sim.deferred(pid) {
            break;
        }
        if sim.ready(pid) == 0
            && sim.ready(peer_pid) == 0
            && !sim.deferred(pid)
            && !sim.deferred(peer_pid)
            && !service.work_pending(sim.now())
            && !peer.work_pending(sim.now())
            && let Some(at) =
                [sim.next_due(), service.next_deadline(), peer.next_deadline()].into_iter().flatten().min()
        {
            sim.advance_to(at);
        }
    }
    assert!(saved_turn, "local policy acknowledged the shared effect's turn");
    assert!(smith_agent_process_world::fake::replied(&peer), "the fake plaintext LLM received the completion");
    assert!(shown, "the terminal observed the run");
    settle_colocated_exit(&mut service, &mut sim, pid, &mut peer, peer_pid);
}

#[expect(clippy::wildcard_enum_match_arm, reason = "the story names unexpected ending requests")]
fn settle_colocated_exit(
    service: &mut Service,
    sim: &mut skein_sim::Sim,
    pid: skein_sim::Pid,
    peer: &mut skein_fake_peers::llm::Peer,
    peer_pid: skein_sim::Pid,
) {
    service.closed();
    let mut exited = false;
    for _ in 0_u32..1000 {
        sim.reap(peer_pid, peer.completions());
        peer.iterate(sim.now(), sim.wall());
        sim.submit(peer_pid, peer.submissions());
        sim.reap(pid, service.completions());
        iterate(service, sim.now(), sim.wall());
        while let Some(request) = service.shell_requests().pop() {
            match request {
                local::Request::Exit { .. } => exited = true,
                other => panic!("unexpected ending {other:?}"),
            }
        }
        sim.submit(pid, service.submissions());
        if exited {
            break;
        }
        if sim.ready(pid) == 0
            && sim.ready(peer_pid) == 0
            && !sim.deferred(pid)
            && !sim.deferred(peer_pid)
            && !service.work_pending(sim.now())
            && !peer.work_pending(sim.now())
            && let Some(at) =
                [sim.next_due(), service.next_deadline(), peer.next_deadline()].into_iter().flatten().min()
        {
            sim.advance_to(at);
        }
    }
    assert!(exited, "local exit waits for lower cleanup");
    sim.assert_quiescent(pid);
    sim.assert_no_open_fds(pid);
    assert!(!service.work_pending(sim.now()));
}

fn colocated_config() -> Config {
    let lower = smith_agent_process_world::configuration();
    let mut config = config();
    config.limits.local.smith = lower.limits.domain;
    config.limits.process.io = lower.limits.io;
    config.limits.local.line_bytes =
        local::max_line_bytes(lower.limits.domain.run.message_bytes).expect("line allowance");
    config.limits.local.endpoints = lower.domain.endpoints.clone();
    config.local.accounts = Box::new([0]);
    config.local.models[0].account = Some(0);
    config.local.models[0].endpoint = Endpoint(0);
    config.local.models[0].prices = run::Prices { input: 1, cached: 1, cache_write: None, output: 1, unit: 1_000_000 };
    config.local.models[0].window = lower.domain.models[0].window;
    config.local.models[0].output = lower.domain.models[0].output;
    config.local.protocol_allowances[0] = lower.domain.models[0].protocol_allowance;
    config.local.budget.turns = 1;
    config.local.budget.spend = 1000;
    config.local.models[0].model = Box::from(b"fake".as_slice());
    config.limits.local.agent = local::AgentLimits::inline(
        config.limits.local.smith,
        config.limits.local.unsaved,
        config.local.budget.time,
        Duration::from_secs(5),
    )
    .expect("bounded inline child");
    config.limits.inline = Some(crate::InlineLimits::new(&lower).expect("bounded lower configuration"));
    config.limits.queue = config.limits.queue.max(local::max_out(&config.limits.local));
    config.agent = crate::AgentConfig::Inline { configuration: lower, roots: Box::new([]) };
    config
}

#[test]
fn the_local_start_window_admits_the_standard_agents_largest_turn() {
    let agent = smith_agent_service::profile::derive(
        &{
            let mut profile = smith_agent_service::profile::standard();
            profile.declared.memory = u64::MAX;
            profile.declared.tool_payload = 256;
            profile.declared.read_window = 256;
            profile
        },
        &smith_agent_process_world::declarations(0),
    )
    .expect("standard agent");
    let largest = smith_domain::max_turn_bytes(&agent.domain).expect("bounded turn");
    let host = crate::profile::host_limits(&smith_agent_service::profile::standard(), &agent).expect("host limits");
    assert!(host.unacknowledged_bytes >= largest, "agent accepts only a window holding its largest turn");
    assert!(host.turn_bytes >= largest, "host can receive the turn it grants");
    assert_eq!(host.message_bytes, u64::from(agent.domain.run.message_bytes));
}

#[test]
fn local_memory_refusal_names_the_actual_placement_footprint() {
    use smith_agent_service::profile::{Check, Layer, LimitName, Refusal, Value};
    let limits = config().limits;
    let required = crate::worst_case(&limits).expect("bounded placement");
    let available = required.checked_sub(1).expect("nonempty placement");
    let refusal = crate::profile::memory(&limits, available).expect_err("local memory target");
    assert_eq!(
        refusal,
        Refusal::Broken {
            check: Check::Memory,
            left: LimitName::deployment(Layer::Service, "local_worst_case").value(Value::Bytes(required)),
            right: LimitName::deployment(Layer::Declared, "memory").value(Value::Bytes(available)),
        }
    );
    crate::profile::memory(&limits, required).expect("exact local fit");
}

#[test]
fn the_standard_no_progress_is_silence_and_one_close() {
    let mut agent = smith_agent_process_world::limits();
    agent.file_timeout = Duration::from_secs(30);
    agent.io.close_timeout = Duration::from_secs(5);
    let host =
        crate::profile::host_limits(&smith_agent_service::profile::standard(), &agent).expect("derived watchdog");
    assert_eq!(host.silence, Duration::from_secs(30));
    assert_eq!(host.no_progress, Duration::from_secs(35));
}

#[test]
fn an_inline_service_refuses_a_different_common_io_capacity() {
    let mut config = colocated_config();
    config.limits.process.io.intake = config.limits.process.io.intake.checked_sub(1).unwrap();
    let Err(crate::Error::Process) = Service::new(config, 1) else { panic!("one common IO configuration") };
}

fn session_started() -> smith_events::SessionStarted {
    smith_events::SessionStarted {
        chat: None,
        wall_ms: 0,
        agent: smith_events::Agent { name: b"test".as_slice().into(), version: Box::default(), build: Box::default() },
        mode: smith_events::Mode::Exec,
        containment: smith_events::Containment::Groups,
        pid: 1,
        profile: b"test".as_slice().into(),
        capture: smith_events::Capture::Calls,
        delivery: smith_events::Delivery::Complete,
        versions: smith_events::Versions { events: 4, channel: 4, charter: 3, transcript: 6, chat_store: None },
        limits: List::with_capacity(0),
    }
}

/// The trace's first `Append`, within a few passes; the inline agent's
/// requests reach the kernel one pass after it makes them.
fn trace_append(service: &mut Service, trace: skein_io::kernel::Fd) -> skein_io::kernel::Submit {
    for _ in 0_u32..4 {
        iterate(service, Time::ZERO, Wall::EPOCH);
        while let Some(submit) = service.submissions().pop() {
            if let skein_io::kernel::Op::Append { fd, .. } = submit.kind
                && fd == trace
            {
                return submit;
            }
        }
    }
    panic!("the trace's session.started is appended")
}

#[test]
fn an_inline_trace_whose_append_fails_is_given_up_once_with_why() {
    use smith_agent_service::{Destination, GivenUp, Why, sinks};
    let trace = skein_io::kernel::Fd::new(20);
    let mut config = colocated_config();
    let crate::AgentConfig::Inline { configuration: lower, .. } = &mut config.agent else { panic!("colocated agent") };
    lower.limits.events = Some(
        smith_agent_service::EventLimits::derive(
            &lower.limits,
            sinks::Capture::Calls,
            1,
            0,
            &[],
            Duration::from_secs(30),
        )
        .expect("bounded event stream"),
    );
    config.limits.inline = Some(crate::InlineLimits::new(lower).expect("bounded lower configuration"));
    let mut service = Service::new(config, 8).expect("colocated service");
    service.adopt_trace(trace, Time::ZERO, session_started(), sinks::Delivery::Complete).expect("inline trace");
    let append = trace_append(&mut service, trace);
    assert_eq!(service.pop_given_up(), None, "nothing is given up while the write is in flight");
    service.completions().push(skein_io::kernel::Complete {
        op: append.op,
        kind: append.kind,
        result: Err(skein_io::kernel::Error::NoSpace),
    });
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    assert_eq!(service.pop_given_up(), Some(GivenUp { destination: Destination::Trace, why: Why::WriteFailed }));
    assert_eq!(service.pop_given_up(), None, "the destination is named once");
}

#[test]
fn a_spawned_trace_is_held_to_the_configured_write_deadline() {
    use smith_agent_service::{Destination, GivenUp, Why, sinks};
    let trace = skein_io::kernel::Fd::new(20);
    let deadline = Duration::from_secs(1);
    let mut product = smith_agent_process_world::configuration();
    product.limits.events = Some(
        smith_agent_service::EventLimits::derive(&product.limits, sinks::Capture::Calls, 1, 0, &[], deadline)
            .expect("bounded event stream"),
    );
    let mut local = config();
    local.limits.process.io = product.limits.io;
    let mut service = Service::new(local, 8).expect("spawned service");
    service.prepare_trace(&product);
    service.adopt_trace(trace, Time::ZERO, session_started(), sinks::Delivery::Complete).expect("spawned trace");
    drop(trace_append(&mut service, trace));
    let before = Time::ZERO.checked_add(Duration::from_millis(999)).unwrap();
    iterate(&mut service, before, Wall::EPOCH);
    assert_eq!(service.pop_given_up(), None, "the write is still within its deadline");
    iterate(&mut service, Time::ZERO.checked_add(deadline).unwrap(), Wall::EPOCH);
    assert_eq!(service.pop_given_up(), Some(GivenUp { destination: Destination::Trace, why: Why::Behind }));
    assert_eq!(service.pop_given_up(), None, "the destination is named once");
}
