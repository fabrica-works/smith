//! Seeded file changes, failed children and deadlines over actual simulated IO.

use skein_fake_machine::Item;
use skein_lib::{Rng, Time, Token};
use smith_domain::{run, tools};
use smith_machine_world::World;
use smith_protocol_machine::{FromDomain, ToDomain};

fn place(root: Token, path: &[u8]) -> tools::Place {
    tools::Place { root, path: Box::from(path) }
}

fn load(world: &mut World, owner: Token) -> tools::Version {
    let done = world.request(FromDomain::Op {
        owner,
        op: tools::Op::Load { at: place(world.root(), b"file"), max: 16 },
        deadline: Time::from_nanos(100_000_000),
    });
    match done {
        ToDomain::Done { done: tools::Done::Loaded { version, .. }, .. } => version,
        other @ (ToDomain::Done { .. }
        | ToDomain::Read { .. }
        | ToDomain::Probed { .. }
        | ToDomain::Checked { .. }
        | ToDomain::Aborted { .. }) => panic!("seeded load: {other:?}"),
    }
}

fn run(seed: u64) -> (usize, ToDomain, Vec<String>) {
    let mut rng = Rng::new(seed);
    let mode = usize::try_from(rng.below(7)).expect("mode fits");
    let mut world = if mode == 6 {
        World::read_only(seed, &[Item::file(b"file", b"old")])
    } else {
        World::new(seed, &[Item::file(b"file", b"old")])
    };
    let root = world.root();
    let owner = Token::new(100);
    let deadline = Time::from_nanos(10_000_000);
    let terminal = match mode {
        0 | 1 | 6 => {
            let version = load(&mut world, Token::new(99));
            if mode == 0 {
                world.change_behind_session(b"file", b"new");
            }
            let done = world.request(FromDomain::Op {
                owner,
                op: tools::Op::Store {
                    at: place(root, b"file"),
                    content: Box::from(&b"ours"[..]),
                    expect: tools::Expect::Is { version },
                },
                deadline,
            });
            match mode {
                0 => assert!(
                    matches!(done, ToDomain::Done { done: tools::Done::Conflict { .. }, .. }),
                    "seed {seed}: {done:?}"
                ),
                1 => assert!(
                    matches!(done, ToDomain::Done { done: tools::Done::Stored { .. }, .. }),
                    "seed {seed}: {done:?}"
                ),
                6 => assert_eq!(
                    done,
                    ToDomain::Done { owner, done: tools::Done::Failed { fault: tools::Fault::Denied } }
                ),
                2..=5 | 7.. => unreachable!("store modes"),
            }
            done
        }
        2 | 3 => {
            let command = if mode == 2 { b"fail".as_slice() } else { b"hang".as_slice() };
            let done = world.request(FromDomain::Op {
                owner,
                op: tools::Op::Spawn {
                    cwd: place(root, b""),
                    command: Box::from(command),
                    head: 8,
                    tail: 8,
                    after: None,
                },
                deadline,
            });
            let exit = if mode == 2 { tools::Exit::Code { code: 1 } } else { tools::Exit::TimedOut };
            assert_eq!(
                done,
                ToDomain::Done {
                    owner,
                    done: tools::Done::Exited { exit, head: Box::new([]), tail: Box::new([]), dropped: 0 }
                },
                "seed {seed}"
            );
            done
        }
        4 | 5 => {
            let program = if mode == 4 { b"check-fail".as_slice() } else { b"check-hang".as_slice() };
            let done = world.request(FromDomain::Check {
                owner,
                program: run::Place { root, path: Box::from(program) },
                deadline,
                tail: 8,
            });
            let exit = if mode == 4 { run::Exit::Code { code: 1 } } else { run::Exit::TimedOut };
            assert_eq!(
                done,
                ToDomain::Checked { owner, ran: run::Ran { exit, output: Box::new([]), cut: 0 } },
                "seed {seed}"
            );
            done
        }
        7.. => unreachable!("seven modes"),
    };
    world.settle();
    (mode, terminal, world.trace().to_vec())
}

#[test]
fn seeded_file_changes_child_failures_and_deadlines_replay_and_settle() {
    let mut covered = [0_u32; 7];
    for seed in 100_u64..228 {
        let first = run(seed);
        let second = run(seed);
        assert_eq!(first, second, "seed {seed} replays its terminal and complete kernel trace");
        let slot = covered.get_mut(first.0).expect("one of seven modes");
        *slot = slot.checked_add(1).expect("seed count fits");
    }
    assert!(covered.iter().all(|count| *count > 0), "every fault class appeared: {covered:?}");
}

#[test]
fn configured_environment_spawns_replay_and_searches_keep_none() {
    for seed in 0..64 {
        let mut rng = Rng::new(seed);
        let value: Vec<u8> =
            (0..rng.below(64)).map(|_| b'a' + u8::try_from(rng.below(26)).expect("alphabet")).collect();
        let variables = [smith_protocol_machine::Variable {
            name: b"BUILD_FLAG".as_slice().into(),
            value: value.into_boxed_slice(),
        }];
        let run = || {
            let mut world = World::configured(seed, &[], &variables);
            for owner in 0..3 {
                let root = world.root();
                let request = match owner {
                    0 => FromDomain::Op {
                        owner: Token::new(1),
                        op: tools::Op::Spawn {
                            cwd: place(root, b""),
                            command: b"true".as_slice().into(),
                            head: 8,
                            tail: 8,
                            after: None,
                        },
                        deadline: world.now().saturating_add(skein_lib::Duration::from_secs(1)),
                    },
                    1 => FromDomain::Check {
                        owner: Token::new(2),
                        program: run::Place { root, path: b"check-ok".as_slice().into() },
                        deadline: world.now().saturating_add(skein_lib::Duration::from_secs(1)),
                        tail: 8,
                    },
                    2 => FromDomain::Op {
                        owner: Token::new(3),
                        op: tools::Op::Search {
                            at: place(root, b""),
                            pattern: b"x".as_slice().into(),
                            glob: None,
                            hits: 1,
                            bytes: 8,
                        },
                        deadline: world.now().saturating_add(skein_lib::Duration::from_secs(1)),
                    },
                    _ => unreachable!("three process purposes"),
                };
                let _terminal = world.request(request);
            }
            let seen = world.spawns();
            assert_eq!(seen.len(), 3);
            assert!(!seen[0].environment.is_empty());
            assert_eq!(seen[0].environment, seen[1].environment);
            assert!(seen[2].environment.is_empty());
            world.settle();
            (seen, world.trace().to_vec())
        };
        assert_eq!(run(), run(), "environment and complete kernel trace replay for seed {seed}");
    }
}
