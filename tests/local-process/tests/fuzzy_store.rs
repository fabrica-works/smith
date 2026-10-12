//! The store/io composition uses the shared world's seeded power-loss model.
use skein_fake_machine::{Machine as Files, Opened};
use skein_io::{
    file,
    file_layer::{self, FileIo},
    kernel::{self, Complete, Fd, Op, Submit},
};
use skein_lib::{Duration, Queue, Time, Token, Wall};
use skein_shell::Host;
use skein_sim::{Answer, Call, Handle};
use skein_world::{Cut, Inherited, Machine, Memory, Referee, StartupRoot, World};
use smith_local_domain as local;
use smith_local_protocol::{Action, History, Store, StoreLimits, StoreRequest};

#[global_allocator]
static HEAP: skein_world::domain::heap::Counting = skein_world::domain::heap::Counting;

const SIGNAL: Token = Token::new(u64::MAX);
fn limits() -> StoreLimits {
    StoreLimits { file_bytes: 4096, turns: 2, deliveries: 2, scan_bytes: 4096 }
}
struct Owner {
    store: Store,
    io: FileIo,
    signal: Fd,
    completions: Queue<Complete>,
    submissions: Queue<Submit>,
    events: Queue<file::Event>,
    actions: Queue<Action>,
    reserved: Option<u64>,
    next: Option<local::ChatState>,
    settled: bool,
    submitted: u32,
    fresh: bool,
}
fn make(inherited: &Inherited) -> Owner {
    let fd = inherited.roots[0].1;
    let mut io = FileIo::with_whole_limit(2, 4096, 6, 4096, Duration::from_secs(30));
    io.seed_randomness(19);
    let root = io.adopt_root(fd).unwrap();
    let mut store = Store::new(
        Token::new(1),
        root,
        fd,
        limits(),
        smith_local_process_world::process::lower_configuration().channel_endpoints,
    )
    .unwrap();
    let mut actions = Queue::with_capacity(3);
    store.request(StoreRequest::Load, Time::ZERO, &mut actions);
    Owner {
        store,
        io,
        signal: inherited.signal,
        completions: Queue::with_capacity(4),
        submissions: Queue::with_capacity(4),
        events: Queue::with_capacity(1),
        actions,
        reserved: None,
        next: None,
        settled: false,
        submitted: 0,
        fresh: inherited.roots[0].0.as_ref() == b"fresh",
    }
}
impl Owner {
    fn actions(&mut self, now: Time) {
        for _ in 0..3 {
            if !self.io.takes() || self.submissions.room() < 2 {
                break;
            }
            let Some(action) = self.actions.pop() else { break };
            match action {
                Action::Io { request, deadline } => {
                    file_layer::down_until(&mut self.io, deadline, request, &mut self.events, &mut self.submissions)
                }
                Action::Domain(local::Event::Loaded { state, .. }) => {
                    let mut state = state.unwrap_or(local::ChatState {
                        identity: smith_domain::run::Identity::new([42; 16]),
                        activation: 0,
                        next_message: 1,
                        read: None,
                    });
                    state.activation += 1;
                    self.next = Some(state);
                    self.store.request(
                        StoreRequest::SaveState {
                            state,
                            history: if self.fresh { History::Fresh } else { History::Kept },
                        },
                        now,
                        &mut self.actions,
                    );
                }
                Action::Domain(local::Event::StateSaved) => {
                    self.reserved = Some(self.next.unwrap().activation);
                    self.store.close(now, &mut self.actions);
                }
                Action::Closed => self.submissions.push(Submit { op: SIGNAL, kind: Op::Close { fd: self.signal } }),
                other => panic!("store reservation terminal: {other:?}"),
            }
        }
    }
}
impl Host for Owner {
    fn iterate(&mut self, now: Time, _: Wall) {
        while let Some(complete) = self.completions.pop() {
            if complete.op == SIGNAL {
                assert!(complete.result.is_ok());
                self.settled = true;
            } else {
                file_layer::up(&mut self.io, complete, &mut self.events, &mut self.submissions);
            }
        }
        if self.io.is_due(now) {
            file_layer::expire(&mut self.io, now, &mut self.submissions);
        }
        for _ in 0..4 {
            self.actions(now);
            let Some(event) = self.events.pop() else { break };
            self.store.from_io(event, now, &mut self.actions);
        }
        self.submitted += self.submissions.len();
    }
    fn completions(&mut self) -> &mut Queue<Complete> {
        &mut self.completions
    }
    fn submissions(&mut self) -> &mut Queue<Submit> {
        &mut self.submissions
    }
    fn work_pending(&self, _: Time) -> bool {
        !self.completions.is_empty() || !self.events.is_empty() || (!self.actions.is_empty() && self.io.takes())
    }
    fn next_deadline(&self) -> Option<Time> {
        self.io.next_deadline()
    }
    fn next_policy_deadline(&self) -> Option<Time> {
        None
    }
    fn is_empty(&self) -> bool {
        self.settled
            && self.io.open_files() == 0
            && self.completions.is_empty()
            && self.submissions.is_empty()
            && self.actions.is_empty()
            && self.events.is_empty()
    }
    fn worst_case(&self) -> u64 {
        Store::worst_case(limits()).unwrap() + FileIo::worst_case(2, 4096, 6, 4096).unwrap() + 65536
    }
    fn operations(&self) -> u32 {
        4
    }
}
struct Disk(Files);
impl Machine for Disk {
    fn open_root(&mut self, _: &[u8]) -> Result<Handle, kernel::Error> {
        Ok(Handle::new(self.0.reopen_root(0).raw()))
    }
    fn close_root(&mut self, root: Handle) {
        self.0.close(Opened::new(root.raw()));
    }
    fn step(&mut self, call: Call, answers: &mut Queue<Answer>) {
        skein_fake_machine::step(&mut self.0, call, answers);
    }
    fn cut(&mut self, cut: Cut, held: &[Handle], seed: u64) {
        for handle in held {
            self.0.close(Opened::new(handle.raw()));
        }
        assert_eq!(cut, Cut::PowerLoss);
        self.0.crash(seed);
    }
}
#[derive(Default)]
struct Judge {
    activations: Vec<u64>,
    reserved: bool,
}
impl Referee<Owner> for Judge {
    fn act(&mut self, _: Time, _: &mut [Owner]) {}
    fn observe(&mut self, _: Time, owners: &[Owner]) {
        if let Some(activation) = owners[0].reserved {
            if !self.reserved {
                assert!(self.activations.last().is_none_or(|prior| *prior < activation));
                self.activations.push(activation);
                self.reserved = true;
            }
        } else {
            self.reserved = false;
        }
    }
    fn next_deadline(&self) -> Option<Time> {
        None
    }
    fn overdue(&self, _: Time) -> Option<String> {
        None
    }
    fn passed(&self) -> bool {
        !self.activations.is_empty()
    }
}
fn world(seed: u64) -> World<Owner, Judge, Disk> {
    configured(seed, false)
}

fn configured(seed: u64, fresh: bool) -> World<Owner, Judge, Disk> {
    let mut files = Files::new();
    let state = local::ChatState {
        identity: smith_domain::run::Identity::new([42; 16]),
        activation: 1,
        next_message: 1,
        read: None,
    };
    let delivery = smith_local_protocol::save_delivery(
        &local::DeliveryRecord {
            name: smith_domain::run::CallName { activation: 1, completion: 1, position: 0 },
            state: local::DeliveryState::Intent(local::DeliveryIntent { directories: Box::new([]) }),
            landed: Box::new([]),
        },
        4096,
    )
    .unwrap();
    let root = files.lay(&[
        skein_fake_machine::Item::file(b"state", &smith_local_protocol::save_state(state).bytes),
        skein_fake_machine::Item::file(&delivery.name, &delivery.bytes),
    ]);
    files.close(root);
    let mut world =
        World::new(seed, skein_sim::Config::calm(), Judge::default(), Memory::Checked).with_machine(Disk(files));
    world.spawn_restartable(
        vec![StartupRoot {
            name: if fresh { b"fresh".as_slice().into() } else { b"chat".as_slice().into() },
            path: b"chat".as_slice().into(),
        }],
        make,
    );
    world
}
#[test]
fn a_power_loss_at_any_store_operation_resumes_without_reusing_an_activation() {
    let baseline = world(70).run();
    let operations = baseline.procs[0].submitted;
    assert!(operations >= 12);
    for before in 0..operations {
        let mut world = world(70 + u64::from(before));
        world.cut(0, before, Cut::PowerLoss);
        let outcome = world.run();
        assert!(outcome.procs[0].reserved.is_some(), "cut {before}");
        assert_eq!(outcome.machine.0.open_handles(), 0);
    }
}

#[test]
fn a_fresh_journal_resumes_its_reset_after_a_power_loss() {
    let baseline = configured(80, true).run();
    for before in 0..baseline.procs[0].submitted {
        let mut world = configured(80 + u64::from(before), true);
        world.cut(0, before, Cut::PowerLoss);
        let mut outcome = world.run();
        let root = outcome.machine.0.reopen_root(0);
        let names = outcome.machine.0.list(root, 6, 4096).unwrap();
        assert_eq!(
            names.iter().filter(|(_, name)| name.len() == 42 || name.as_ref() == b"fresh").count(),
            0,
            "cut {before}"
        );
        outcome.machine.0.close(root);
    }
}
