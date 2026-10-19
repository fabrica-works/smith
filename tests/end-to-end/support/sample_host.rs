//! The sample host's existing parent, peer and referee on the shipped agent binary.
use super::Process;
use skein_lib::{Duration, Time};
use skein_shell::{Clock, Host};
use skein_world::{Referee, real};
use smith_hosts_world::{HostService, referee};
use smith_local_process_world::{Placement, process::Proc};
use smith_real_world::{Scratch, World};

struct Judge;

impl Referee<Process> for Judge {
    fn act(&mut self, _now: Time, procs: &mut [Process]) {
        let gone = procs.iter().any(|process| {
            matches!(process,
            Process::Sample(host) if host.seen().gone.is_some())
        });
        if gone {
            for process in procs {
                if let Process::Script(Proc::Peer(peer)) = process {
                    peer.shutdown();
                }
            }
        }
    }

    fn observe(&mut self, _now: Time, _procs: &[Process]) {}
    fn next_deadline(&self) -> Option<Time> {
        None
    }
    fn overdue(&self, _now: Time) -> Option<String> {
        None
    }
    fn passed(&self) -> bool {
        true
    }
}

pub fn run() {
    let scratch = Scratch::new();
    let mut scenario = World::new(91, &scratch, Placement::Spawned, b"First answer").scenario;
    scenario.launch.tls = true;
    scenario.launch.trust_der = Some(scratch.path().join("root.der"));
    let document = scratch.path().join("agent.json");
    let bytes = serde_json::to_vec(&smith_local_process_world::process::agent_document(&scenario.launch))
        .expect("sample host fixture");
    std::fs::write(&document, bytes).expect("sample host fixture");
    let kind = smith_local_shell::compose::AgentKind::Spawned { program: env!("CARGO_BIN_EXE_smith-agent").into() };
    let root = skein_shell::open_root(scratch.path()).expect("sample host fixture");
    let clock = Clock::new();
    let at = clock.now();
    let launch = kind.launch(root, &document);
    let mut world = real::World::new(Judge);
    world.spawn_with_fds(vec![root], || {
        Process::Sample(Box::new(HostService::new(launch, at.now, at.wall, 4096, None)))
    });
    world.spawn(|| {
        Process::Script(Proc::Peer(Box::new(smith_agent_process_world::fake::configured_transport(
            Box::new([peer_script()]),
            skein_llm::Credential { access_token: b"token".as_slice().into(), account_id: b"acc".as_slice().into() },
            Duration::ZERO,
            skein_fake_peers::Transport::Tls,
        ))))
    });
    let outcome = world.run(&clock, Duration::from_secs(2));
    assert!(outcome.procs.iter().all(Host::is_empty), "sample host and peer settle");
    let host = outcome
        .procs
        .iter()
        .find_map(|process| match process {
            Process::Sample(host) => Some(host),
            Process::Binary(_) | Process::Script(_) => None,
        })
        .expect("sample host fixture");
    assert!(host.seen().started && host.seen().admitted, "the generated configuration admitted the run");
    assert!(host.seen().answered.is_some(), "the agent answered");
    assert_eq!(host.seen().exits, 1, "one actual agent exit");
    assert_eq!(host.seen().reaps, 1, "the child is reaped");
    referee::review(host.observations()).assert_passed(91);
    smith_real_world::settled::assert_no_children();
}

fn peer_script() -> skein_fake_llm_domain::api::Script {
    use skein_fake_llm_domain::api::{Finish, Line, Script, Turn};
    Script {
        cue: Box::new([]),
        turns: Box::new([Turn {
            lines: Box::new([Line::Text { text: b"Hello".as_slice().into() }]),
            finish: Finish::Stop,
            tokens: 1,
        }]),
    }
}
