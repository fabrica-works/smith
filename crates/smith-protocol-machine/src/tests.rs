//! Entry-point tests for file admission and translation.

use alloc::boxed::Box;
use skein_io::{digest, file, kernel};
use skein_lib::{Duration, Env, Queue, Time, Token, Wall};
use smith_domain::{run, tools};

use crate::{Below, BelowEvent, Component, FromDomain, Limits, ToDomain};

fn limits() -> Limits {
    Limits {
        operations: 2,
        roots: 1,
        path_bytes: 64,
        file_bytes: 16,
        entries: 2,
        entry_bytes: 512,
        processes: 2,
        output_bytes: 64,
        search_hits: 8,
        search_bytes: 256,
        search_line_bytes: 512,
        env_bytes: 512,
        group_stop: Duration::from_millis(10),
    }
}

fn env() -> Env<Limits> {
    Env { now: Time::ZERO, wall: Wall::EPOCH, limits: limits() }
}

fn component() -> Component {
    configured_component(&[])
}

fn configured_component(variables: &[crate::Variable]) -> Component {
    let mut component = Component::new(
        &limits(),
        crate::Environment::encode(variables, &limits()).expect("configured environment fits"),
    );
    component.workspace(&run::Workspace {
        directories: Box::new([run::Directory {
            host: Box::from(b"/host".as_slice()),
            name: Box::from(&b"code"[..]),
            root: Token::new(10),
            writable: true,
            git: true,
            conflicts: Box::new([]),
        }]),
    });
    component
}

fn place(path: &[u8]) -> tools::Place {
    tools::Place { root: Token::new(10), path: Box::from(path) }
}

#[test]
fn git_part_and_unknown_root_are_refused_before_io() {
    let mut component = component();
    let mut to_domain = Queue::with_capacity(2);
    let mut below = Queue::with_capacity(2);
    component.from_domain(
        &env(),
        FromDomain::Op {
            owner: Token::new(1),
            op: tools::Op::Load { at: place(b"src/.GIT/config"), max: 16 },
            deadline: Time::from_nanos(u64::MAX),
        },
        &mut to_domain,
        &mut below,
    );
    assert!(below.is_empty());
    assert_eq!(
        to_domain.pop(),
        Some(ToDomain::Done { owner: Token::new(1), done: tools::Done::Failed { fault: tools::Fault::Denied } })
    );
    component.from_domain(
        &env(),
        FromDomain::Op {
            owner: Token::new(2),
            op: tools::Op::Load { at: tools::Place { root: Token::new(99), path: Box::from(&b"a"[..]) }, max: 16 },
            deadline: Time::from_nanos(u64::MAX),
        },
        &mut to_domain,
        &mut below,
    );
    assert!(below.is_empty());
    assert_eq!(to_domain.pop(), Some(ToDomain::Done { owner: Token::new(2), done: tools::Done::Escapes }));
}

#[test]
fn store_uses_no_follow_and_returns_digest() {
    let mut component = component();
    let mut to_domain = Queue::with_capacity(2);
    let mut below = Queue::with_capacity(2);
    component.from_domain(
        &env(),
        FromDomain::Op {
            owner: Token::new(1),
            op: tools::Op::Store {
                at: place(b"src/a"),
                content: Box::from(&b"new"[..]),
                expect: tools::Expect::Absent,
            },
            deadline: Time::from_nanos(u64::MAX),
        },
        &mut to_domain,
        &mut below,
    );
    assert_eq!(
        below.pop(),
        Some(Below::File {
            request: file::Request::Store {
                owner: Token::new(1),
                root: Token::new(10),
                path: Box::from(&b"src/a"[..]),
                bytes: Box::from(&b"new"[..]),
                expected: file::Expect::Absent,
                no_follow: true,
            },
            deadline: Time::from_nanos(u64::MAX)
        })
    );
    let version = digest::digest(b"new");
    component.from_below(
        &env(),
        BelowEvent::File(file::Event::Stored { owner: Token::new(1), digest: version }),
        &mut to_domain,
        &mut below,
    );
    assert_eq!(
        to_domain.pop(),
        Some(ToDomain::Done {
            owner: Token::new(1),
            done: tools::Done::Stored { version: tools::Version::new(version.0) }
        })
    );
}

#[test]
fn a_symbolic_link_refusal_becomes_linked() {
    let mut component = component();
    let mut to_domain = Queue::with_capacity(2);
    let mut below = Queue::with_capacity(2);
    component.from_domain(
        &env(),
        FromDomain::Op {
            owner: Token::new(1),
            op: tools::Op::Store { at: place(b"link/a"), content: Box::from(&b"x"[..]), expect: tools::Expect::Absent },
            deadline: Time::from_nanos(u64::MAX),
        },
        &mut to_domain,
        &mut below,
    );
    assert!(below.pop().is_some());
    component.from_below(
        &env(),
        BelowEvent::File(file::Event::Failed {
            owner: Token::new(1),
            error: kernel::Error::TooManyLinks,
            committed: false,
            residue: None,
        }),
        &mut to_domain,
        &mut below,
    );
    assert_eq!(to_domain.pop(), Some(ToDomain::Done { owner: Token::new(1), done: tools::Done::Linked }));
}

#[test]
fn cancellation_is_sent_below_and_the_actual_terminal_decides_the_race() {
    let mut component = component();
    let mut to_domain = Queue::with_capacity(2);
    let mut below = Queue::with_capacity(2);
    component.from_domain(
        &env(),
        FromDomain::Op {
            owner: Token::new(1),
            op: tools::Op::Load { at: place(b"a"), max: 16 },
            deadline: Time::from_nanos(100),
        },
        &mut to_domain,
        &mut below,
    );
    assert_eq!(
        below.pop(),
        Some(Below::File {
            request: file::Request::Load {
                owner: Token::new(1),
                root: Token::new(10),
                path: Box::from(&b"a"[..]),
                max: 16,
                no_follow: false,
            },
            deadline: Time::from_nanos(100),
        })
    );
    component.from_domain(&env(), FromDomain::Cancel { owner: Token::new(1) }, &mut to_domain, &mut below);
    assert_eq!(below.pop(), Some(Below::CancelFile { owner: Token::new(1) }));
    component.from_below(
        &env(),
        BelowEvent::File(file::Event::Loaded { owner: Token::new(1), bytes: Box::from(&b"old"[..]) }),
        &mut to_domain,
        &mut below,
    );
    assert_eq!(
        to_domain.pop(),
        Some(ToDomain::Done {
            owner: Token::new(1),
            done: tools::Done::Loaded {
                content: Box::from(&b"old"[..]),
                version: tools::Version::new(digest::digest(b"old").0),
            }
        })
    );

    component.from_domain(
        &env(),
        FromDomain::Op {
            owner: Token::new(2),
            op: tools::Op::Load { at: place(b"b"), max: 16 },
            deadline: Time::from_nanos(100),
        },
        &mut to_domain,
        &mut below,
    );
    assert!(below.pop().is_some());
    component.from_domain(&env(), FromDomain::Cancel { owner: Token::new(2) }, &mut to_domain, &mut below);
    assert_eq!(below.pop(), Some(Below::CancelFile { owner: Token::new(2) }));
    component.from_below(
        &env(),
        BelowEvent::File(file::Event::Cancelled { owner: Token::new(2) }),
        &mut to_domain,
        &mut below,
    );
    assert_eq!(to_domain.pop(), Some(ToDomain::Done { owner: Token::new(2), done: tools::Done::Cancelled }));
}

#[test]
fn guide_read_cuts_utf8_and_closes_before_answer() {
    let mut component = component();
    let mut to_domain = Queue::with_capacity(1);
    let mut below = Queue::with_capacity(3);
    let owner = Token::new(31);
    let deadline = Time::from_nanos(100);
    component.from_domain(
        &env(),
        FromDomain::Read {
            owner,
            at: run::Place { root: Token::new(10), path: Box::from(&b"AGENTS.md"[..]) },
            max: 3,
            deadline,
        },
        &mut to_domain,
        &mut below,
    );
    match below.pop() {
        Some(Below::OpenRead { .. }) => {}
        other => panic!("expected open read: {other:?}"),
    }
    let file = Token::new(41);
    component.from_below(
        &env(),
        BelowEvent::File(file::Event::Opened { owner, file, len: 4 }),
        &mut to_domain,
        &mut below,
    );
    assert_eq!(
        below.pop(),
        Some(Below::File { request: file::Request::ReadAt { owner, file, offset: 0, max: 3 }, deadline })
    );
    component.from_below(
        &env(),
        BelowEvent::File(file::Event::Read { owner, bytes: Box::from(&b"a\xc3\xa9"[..]) }),
        &mut to_domain,
        &mut below,
    );
    match below.pop() {
        Some(Below::File { request: file::Request::Close { .. }, .. }) => {}
        other => panic!("expected close: {other:?}"),
    }
    assert!(to_domain.is_empty());
    component.from_below(&env(), BelowEvent::File(file::Event::Closed { owner }), &mut to_domain, &mut below);
    assert_eq!(
        to_domain.pop(),
        Some(ToDomain::Read { owner, read: run::Read::Text { text: Box::from(&b"a\xc3\xa9"[..]), whole: false } })
    );
}

#[test]
fn cancelling_a_guide_open_waits_for_its_actual_file_terminal() {
    for probe in [false, true] {
        let mut component = component();
        let mut to_domain = Queue::with_capacity(1);
        let mut below = Queue::with_capacity(3);
        let owner = Token::new(61);
        let at = run::Place { root: Token::new(10), path: Box::from(&b"AGENTS.md"[..]) };
        let deadline = Time::from_nanos(100);
        let request = if probe {
            FromDomain::Probe { owner, at, deadline }
        } else {
            FromDomain::Read { owner, at, max: 3, deadline }
        };
        component.from_domain(&env(), request, &mut to_domain, &mut below);
        let Some(Below::OpenRead { .. }) = below.pop() else { panic!("expected an actual guide open") };
        component.from_domain(&env(), FromDomain::Cancel { owner }, &mut to_domain, &mut below);
        assert_eq!(below.pop(), Some(Below::CancelFile { owner }));
        assert!(to_domain.is_empty(), "cancel request does not fabricate the guide's terminal");
        component.from_below(&env(), BelowEvent::File(file::Event::Cancelled { owner }), &mut to_domain, &mut below);
        let expected = if probe {
            ToDomain::Probed { owner, executable: false }
        } else {
            ToDomain::Read { owner, read: run::Read::Failed }
        };
        assert_eq!(to_domain.pop(), Some(expected));
        // Installing a new workspace asserts that every retained operation has ended.
        component.workspace(&run::Workspace { directories: Box::new([]) });
    }
}

#[test]
fn a_cancelled_guide_closes_its_open_descriptor_before_its_terminal() {
    let mut component = component();
    let mut to_domain = Queue::with_capacity(1);
    let mut below = Queue::with_capacity(3);
    let owner = Token::new(62);
    let descriptor = Token::new(72);
    component.from_domain(
        &env(),
        FromDomain::Read {
            owner,
            at: run::Place { root: Token::new(10), path: Box::from(&b"AGENTS.md"[..]) },
            max: 3,
            deadline: Time::from_nanos(100),
        },
        &mut to_domain,
        &mut below,
    );
    below.pop().expect("actual open request");
    component.from_below(
        &env(),
        BelowEvent::File(file::Event::Opened { owner, file: descriptor, len: 4 }),
        &mut to_domain,
        &mut below,
    );
    let Some(Below::File { request: file::Request::ReadAt { .. }, .. }) = below.pop() else {
        panic!("expected an actual guide read");
    };
    component.from_domain(&env(), FromDomain::Cancel { owner }, &mut to_domain, &mut below);
    assert_eq!(below.pop(), Some(Below::CancelFile { owner }));
    component.from_below(&env(), BelowEvent::File(file::Event::Cancelled { owner }), &mut to_domain, &mut below);
    let Some(Below::File { request: file::Request::Close { file, .. }, .. }) = below.pop() else {
        panic!("expected an actual descriptor close");
    };
    assert_eq!(file, descriptor);
    component.from_domain(&env(), FromDomain::Cancel { owner }, &mut to_domain, &mut below);
    assert!(below.is_empty(), "a repeated cancel leaves descriptor close to finish");
    assert!(to_domain.is_empty());
    component.from_below(&env(), BelowEvent::File(file::Event::Closed { owner }), &mut to_domain, &mut below);
    assert_eq!(to_domain.pop(), Some(ToDomain::Read { owner, read: run::Read::Failed }));
    // Installing a new workspace asserts that every retained operation has ended.
    component.workspace(&run::Workspace { directories: Box::new([]) });
}

#[test]
fn command_output_waits_for_both_pipes_and_child_close() {
    use skein_io::Event;
    use skein_lib::stream::Up;
    let mut component = component();
    let mut to_domain = Queue::with_capacity(1);
    let mut below = Queue::with_capacity(3);
    let owner = Token::new(32);
    component.from_domain(
        &env(),
        FromDomain::Op {
            owner,
            op: tools::Op::Spawn {
                cwd: place(b"."),
                command: Box::from(&b"printf abcdef"[..]),
                head: 2,
                tail: 2,
                after: None,
            },
            deadline: Time::from_nanos(100),
        },
        &mut to_domain,
        &mut below,
    );
    match below.pop() {
        Some(Below::Spawn { owner: spawned, .. }) => assert_eq!(spawned, owner),
        other => panic!("expected spawn: {other:?}"),
    }
    component.from_below(
        &env(),
        BelowEvent::Process(Event::Spawned {
            owner,
            child: Token::new(42),
            pipes: Box::new([Token::new(43), Token::new(44)]),
        }),
        &mut to_domain,
        &mut below,
    );
    assert_eq!(below.len(), 2);
    assert!(below.pop().is_some());
    assert!(below.pop().is_some());
    component.from_below(
        &env(),
        BelowEvent::Process(Event::Stream { owner: Token::new(43), up: Up::Bytes(Box::from(&b"abcdef"[..])) }),
        &mut to_domain,
        &mut below,
    );
    assert!(below.pop().is_some());
    component.from_below(
        &env(),
        BelowEvent::Process(Event::Exited { owner, exit: kernel::Exit::Code(0) }),
        &mut to_domain,
        &mut below,
    );
    assert_eq!(below.pop(), Some(Below::Process(skein_io::Request::Close { entity: Token::new(42) })));
    assert!(to_domain.is_empty(), "natural exit still awaits pipe closure and reap");
    for pipe in [Token::new(43), Token::new(44)] {
        component.from_below(&env(), BelowEvent::Process(Event::Closed { owner: pipe }), &mut to_domain, &mut below);
    }
    assert!(to_domain.is_empty());
    component.from_below(&env(), BelowEvent::Process(Event::Closed { owner }), &mut to_domain, &mut below);
    assert_eq!(
        to_domain.pop(),
        Some(ToDomain::Done {
            owner,
            done: tools::Done::Exited {
                exit: tools::Exit::Code { code: 0 },
                head: Box::from(&b"ab"[..]),
                tail: Box::from(&b"ef"[..]),
                dropped: 2,
            }
        })
    );
}

#[test]
fn deadline_sends_group_term_kill_close_and_answers_after_reap() {
    use skein_io::Event;
    let mut component = component();
    let mut to_domain = Queue::with_capacity(1);
    let mut below = Queue::with_capacity(3);
    let owner = Token::new(33);
    component.from_domain(
        &env(),
        FromDomain::Check {
            owner,
            program: run::Place { root: Token::new(10), path: Box::from(&b"check"[..]) },
            deadline: Time::from_nanos(100),
            tail: 4,
        },
        &mut to_domain,
        &mut below,
    );
    match below.pop() {
        Some(Below::Spawn { .. }) => {}
        other => panic!("expected spawn: {other:?}"),
    }
    component.from_below(
        &env(),
        BelowEvent::Process(Event::Spawned {
            owner,
            child: Token::new(45),
            pipes: Box::new([Token::new(46), Token::new(47)]),
        }),
        &mut to_domain,
        &mut below,
    );
    assert_eq!(below.len(), 2);
    assert!(below.pop().is_some());
    assert!(below.pop().is_some());
    let mut later = env();
    later.now = Time::from_nanos(101);
    component.fire(&later, &mut to_domain, &mut below);
    assert_eq!(
        below.pop(),
        Some(Below::Process(skein_io::Request::Signal {
            child: Token::new(45),
            to: kernel::Target::Group,
            signal: kernel::Signal::Terminate
        }))
    );
    later.now = Time::from_nanos(10_000_102);
    component.fire(&later, &mut to_domain, &mut below);
    assert_eq!(
        below.pop(),
        Some(Below::Process(skein_io::Request::Signal {
            child: Token::new(45),
            to: kernel::Target::Group,
            signal: kernel::Signal::Kill
        }))
    );
    assert_eq!(component.next_deadline(), Some(Time::from_nanos(20_000_102)));
    later.now = Time::from_nanos(20_000_101);
    component.fire(&later, &mut to_domain, &mut below);
    assert!(below.is_empty(), "each stage receives its complete group grace");
    later.now = Time::from_nanos(20_000_102);
    component.fire(&later, &mut to_domain, &mut below);
    assert_eq!(below.pop(), Some(Below::Process(skein_io::Request::Close { entity: Token::new(45) })));
    assert_eq!(component.next_deadline(), None);
    assert!(to_domain.is_empty(), "Close issuance is not reaping evidence");
    component.from_below(
        &later,
        BelowEvent::Process(Event::Exited { owner, exit: kernel::Exit::Signal(9) }),
        &mut to_domain,
        &mut below,
    );
    for pipe in [Token::new(46), Token::new(47)] {
        component.from_below(&later, BelowEvent::Process(Event::Closed { owner: pipe }), &mut to_domain, &mut below);
    }
    assert!(to_domain.is_empty());
    component.from_below(&later, BelowEvent::Process(Event::Closed { owner }), &mut to_domain, &mut below);
    assert_eq!(
        to_domain.pop(),
        Some(ToDomain::Checked { owner, ran: run::Ran { exit: run::Exit::TimedOut, output: Box::new([]), cut: 0 } })
    );
}

fn directory_request(operation: u32, path: &[u8]) -> FromDomain {
    let owner = Token::new(1);
    let deadline = Time::from_nanos(u64::MAX);
    let at = run::Place { root: Token::new(10), path: Box::from(path) };
    match operation {
        0 => FromDomain::Op { owner, op: tools::Op::Load { at: place(path), max: 16 }, deadline },
        1 => FromDomain::Op {
            owner,
            op: tools::Op::Store { at: place(path), content: Box::new([]), expect: tools::Expect::Absent },
            deadline,
        },
        2 => FromDomain::Op {
            owner,
            op: tools::Op::List { at: place(path), depth: tools::Depth::Whole, glob: None, max: 2, max_bytes: 512 },
            deadline,
        },
        3 => FromDomain::Op {
            owner,
            op: tools::Op::Spawn { cwd: place(path), command: Box::from(&b"true"[..]), head: 4, tail: 4, after: None },
            deadline,
        },
        4 => FromDomain::Op {
            owner,
            op: tools::Op::Search { at: place(path), pattern: Box::from(&b"x"[..]), glob: None, hits: 2, bytes: 16 },
            deadline,
        },
        5 => FromDomain::Read { owner, at, max: 16, deadline },
        6 => FromDomain::Probe { owner, at, deadline },
        7 => FromDomain::Check {
            owner,
            program: run::Place { root: Token::new(10), path: Box::from(&b"check"[..]) },
            deadline,
            tail: 4,
        },
        _ => unreachable!(),
    }
}

#[test]
fn every_operation_translates_the_directory_itself_and_nested_paths_once() {
    for path in [&b""[..], &b"src/file"[..]] {
        let expected = if path.is_empty() { &b"."[..] } else { path };
        for operation in 0_u32..8 {
            let request = directory_request(operation, path);
            let mut component = component();
            let mut to_domain = Queue::with_capacity(2);
            let mut below = Queue::with_capacity(2);
            component.from_domain(&env(), request, &mut to_domain, &mut below);
            assert!(to_domain.is_empty(), "operation {operation}, path {path:?}");
            match below.pop().expect("one io request") {
                Below::File { request, .. } => match request {
                    file::Request::Load { path, .. }
                    | file::Request::Store { path, .. }
                    | file::Request::Scan { path, .. } => assert_eq!(path.as_ref(), expected),
                    other @ (file::Request::Create { .. }
                    | file::Request::CreateNoFollow { .. }
                    | file::Request::OpenWrite { .. }
                    | file::Request::OpenWriteNoFollow { .. }
                    | file::Request::OpenRead { .. }
                    | file::Request::OpenReadNoFollow { .. }
                    | file::Request::OpenDirectory { .. }
                    | file::Request::Stat { .. }
                    | file::Request::WriteAt { .. }
                    | file::Request::ReadAt { .. }
                    | file::Request::Cut { .. }
                    | file::Request::Sync { .. }
                    | file::Request::Close { .. }
                    | file::Request::SyncDirectory { .. }
                    | file::Request::Rename { .. }
                    | file::Request::Remove { .. }
                    | file::Request::List { .. }
                    | file::Request::MakeDirectory { .. }
                    | file::Request::OpenPrivate { .. }) => panic!("unexpected file request: {other:?}"),
                },
                Below::OpenRead { path, .. } | Below::OpenDirectory { path, .. } => assert_eq!(path.as_ref(), expected),
                Below::Spawn { spawn, .. } => {
                    if operation == 2 || operation == 3 {
                        assert_eq!(spawn.dir.as_ref(), expected);
                    } else {
                        assert_eq!(spawn.dir.as_ref(), b".");
                        if operation == 4 {
                            assert_eq!(spawn.args.last().expect("search path").as_ref(), expected);
                        }
                    }
                }
                other @ (Below::CancelFile { .. } | Below::Process(_)) => panic!("unexpected request: {other:?}"),
            }
            assert!(below.is_empty());
        }
    }
}

#[test]
fn configured_variables_are_encoded_once_for_commands_and_checks_and_absent_for_searches() {
    let variables = [
        crate::Variable { name: Box::from(&b"PATH"[..]), value: Box::from(&b"/configured"[..]) },
        crate::Variable { name: Box::from(&b"HOME"[..]), value: Box::from(&b"/readonly"[..]) },
    ];
    let expected = Box::new([Box::from(&b"PATH=/configured"[..]), Box::from(&b"HOME=/readonly"[..])]);
    for operation in 0_u32..3 {
        for _ in 0_u32..2 {
            let mut component = configured_component(&variables);
            let owner = Token::new(1);
            let deadline = Time::from_nanos(u64::MAX);
            let request = match operation {
                0 => FromDomain::Op {
                    owner,
                    deadline,
                    op: tools::Op::Spawn {
                        cwd: place(b""),
                        command: Box::from(&b"true"[..]),
                        head: 4,
                        tail: 4,
                        after: None,
                    },
                },
                1 => FromDomain::Check {
                    owner,
                    deadline,
                    program: run::Place { root: Token::new(10), path: Box::from(&b"check"[..]) },
                    tail: 4,
                },
                2 => FromDomain::Op {
                    owner,
                    deadline,
                    op: tools::Op::Search {
                        at: place(b""),
                        pattern: Box::from(&b"x"[..]),
                        glob: None,
                        hits: 1,
                        bytes: 16,
                    },
                },
                _ => unreachable!(),
            };
            let mut up = Queue::with_capacity(2);
            let mut down = Queue::with_capacity(2);
            component.from_domain(&env(), request, &mut up, &mut down);
            assert!(up.is_empty());
            match down.pop().expect("spawn") {
                Below::Spawn { spawn, .. } => {
                    if operation == 2 {
                        assert!(spawn.env.is_empty());
                    } else {
                        assert_eq!(spawn.env.as_ref(), expected.as_ref());
                    }
                }
                other @ (Below::File { .. }
                | Below::OpenRead { .. }
                | Below::OpenDirectory { .. }
                | Below::CancelFile { .. }
                | Below::Process(_)) => {
                    panic!("expected process spawn: {other:?}");
                }
            }
        }
    }
}

#[test]
fn environment_encoding_refuses_invalid_names_duplicates_and_excess_bytes() {
    for (name, value) in [(&b""[..], &b"x"[..]), (b"A=B", b"x"), (b"A\0", b"x"), (b"A", b"x\0")] {
        let variables = [crate::Variable { name: name.into(), value: value.into() }];
        assert_eq!(
            crate::Environment::encode(&variables, &limits()).unwrap_err(),
            crate::Unencodable::Variable { index: 0 }
        );
    }
    let variables = [
        crate::Variable { name: Box::from(&b"A"[..]), value: Box::from(&b"x"[..]) },
        crate::Variable { name: Box::from(&b"A"[..]), value: Box::from(&b"y"[..]) },
    ];
    assert_eq!(
        crate::Environment::encode(&variables, &limits()).unwrap_err(),
        crate::Unencodable::Repeated { index: 1 }
    );
    let limits = Limits { env_bytes: 3, ..limits() };
    assert!(crate::Environment::encode(&variables[..1], &limits).is_ok(), "name, equals and value exactly fit");
    assert_eq!(
        crate::Environment::encode(&variables, &limits).unwrap_err(),
        crate::Unencodable::TooLarge { bytes: 6, bound: 3 }
    );
}

#[test]
fn a_cancel_before_spawn_reaches_group_close_and_waits_for_every_terminal() {
    use skein_io::Event;
    let mut component = component();
    let mut to_domain = Queue::with_capacity(1);
    let mut below = Queue::with_capacity(3);
    let owner = Token::new(94);
    component.from_domain(
        &env(),
        FromDomain::Op {
            owner,
            op: tools::Op::Spawn { cwd: place(b""), command: b"hang".as_slice().into(), head: 4, tail: 4, after: None },
            deadline: Time::from_nanos(u64::MAX),
        },
        &mut to_domain,
        &mut below,
    );
    assert!(below.pop().is_some());
    component.from_domain(&env(), FromDomain::Cancel { owner }, &mut to_domain, &mut below);
    let mut later = env();
    later.now = Time::from_nanos(10_000_000);
    component.fire(&later, &mut to_domain, &mut below);
    later.now = Time::from_nanos(20_000_000);
    component.fire(&later, &mut to_domain, &mut below);
    assert!(below.is_empty(), "spawn is still pending");
    component.from_below(
        &later,
        BelowEvent::Process(Event::Spawned {
            owner,
            child: Token::new(95),
            pipes: Box::new([Token::new(96), Token::new(97)]),
        }),
        &mut to_domain,
        &mut below,
    );
    assert!(below.pop().is_some());
    assert!(below.pop().is_some());
    assert_eq!(below.pop(), Some(Below::Process(skein_io::Request::Close { entity: Token::new(95) })));
    component.from_below(
        &later,
        BelowEvent::Process(Event::Exited { owner, exit: kernel::Exit::Code(0) }),
        &mut to_domain,
        &mut below,
    );
    assert!(to_domain.is_empty(), "a polite successful exit still owes closure");
    component.from_below(
        &later,
        BelowEvent::Process(Event::Closed { owner: Token::new(96) }),
        &mut to_domain,
        &mut below,
    );
    assert!(to_domain.is_empty());
    component.from_below(&later, BelowEvent::Process(Event::Closed { owner }), &mut to_domain, &mut below);
    assert!(to_domain.is_empty(), "second pipe remains outstanding");
    component.from_below(
        &later,
        BelowEvent::Process(Event::Closed { owner: Token::new(97) }),
        &mut to_domain,
        &mut below,
    );
    assert_eq!(to_domain.pop(), Some(ToDomain::Done { owner, done: tools::Done::Cancelled }));
    assert!(to_domain.is_empty());
}

#[expect(
    clippy::manual_map,
    clippy::too_many_lines,
    reason = "entry-point fixture orders directory, spawn, pipes and child terminals"
)]
fn listed(output: &[u8], depth: tools::Depth, maximum: u32, bytes: u64, glob: Option<&[u8]>) -> tools::Done {
    let mut component = component();
    let mut terminals = Queue::with_capacity(4);
    let mut below = Queue::with_capacity(4);
    let owner = Token::new(50);
    component.from_domain(
        &env(),
        FromDomain::Op {
            owner,
            op: tools::Op::List {
                at: place(b"src"),
                depth,
                glob: match glob {
                    Some(glob) => Some(Box::from(glob)),
                    None => None,
                },
                max: maximum,
                max_bytes: bytes,
            },
            deadline: Time::from_nanos(u64::MAX),
        },
        &mut terminals,
        &mut below,
    );
    assert_eq!(
        below.pop(),
        Some(Below::OpenDirectory {
            owner,
            root: Token::new(10),
            path: Box::from(b"src".as_slice()),
            deadline: Time::from_nanos(u64::MAX)
        })
    );
    component.from_below(
        &env(),
        BelowEvent::File(file::Event::Opened { owner, file: Token::new(49), len: 0 }),
        &mut terminals,
        &mut below,
    );
    assert_eq!(
        below.pop(),
        Some(Below::File {
            request: file::Request::Close { owner, file: Token::new(49) },
            deadline: Time::from_nanos(u64::MAX)
        })
    );
    assert!(terminals.is_empty(), "directory closure is owed before spawning");
    component.from_below(&env(), BelowEvent::File(file::Event::Closed { owner }), &mut terminals, &mut below);
    match below.pop().expect("listing starts rg") {
        Below::Spawn { spawn, root, .. } => {
            assert_eq!(root, Token::new(10));
            assert_eq!(spawn.dir.as_ref(), b"src");
            assert_eq!(spawn.program.as_ref(), b"/usr/bin/rg");
            assert!(spawn.env.is_empty());
            assert!(has_argument(&spawn.args, b"--files"));
            assert!(has_argument(&spawn.args, b"--null"));
            assert!(has_argument(&spawn.args, b"--sort=path"));
            if let Some(glob) = glob {
                let mut found = false;
                for word in &spawn.args {
                    if word.strip_prefix(b"--glob=") == Some(glob) {
                        found = true;
                    }
                }
                assert!(found);
            }
        }
        Below::File { .. }
        | Below::CancelFile { .. }
        | Below::OpenRead { .. }
        | Below::OpenDirectory { .. }
        | Below::Process(_) => {
            panic!("listing is a process")
        }
    }
    let child = Token::new(51);
    let stdout = Token::new(52);
    let stderr = Token::new(53);
    component.from_below(
        &env(),
        BelowEvent::Process(skein_io::Event::Spawned { owner, child, pipes: Box::new([stdout, stderr]) }),
        &mut terminals,
        &mut below,
    );
    while below.pop().is_some() {}
    for byte in output {
        let chunk = Box::new([*byte]);
        component.from_below(
            &env(),
            BelowEvent::Process(skein_io::Event::Stream { owner: stdout, up: skein_lib::stream::Up::Bytes(chunk) }),
            &mut terminals,
            &mut below,
        );
        while below.pop().is_some() {}
    }
    component.from_below(
        &env(),
        BelowEvent::Process(skein_io::Event::Exited { owner, exit: kernel::Exit::Code(0) }),
        &mut terminals,
        &mut below,
    );
    while below.pop().is_some() {}
    for entity in [owner, stdout, stderr] {
        component.from_below(
            &env(),
            BelowEvent::Process(skein_io::Event::Closed { owner: entity }),
            &mut terminals,
            &mut below,
        );
    }
    match terminals.pop().expect("one listing terminal") {
        ToDomain::Done { done, .. } => done,
        ToDomain::Read { .. } | ToDomain::Probed { .. } | ToDomain::Checked { .. } | ToDomain::Aborted { .. } => {
            panic!("listing terminal")
        }
    }
}

fn listed_entry(path: &[u8], kind: tools::Kind) -> tools::Entry {
    tools::Entry { name: Box::from(path), kind }
}

#[test]
fn selected_files_derive_a_sorted_tree_and_count_distinct_entries_at_the_cap() {
    // a.b sorts before a/x among files, but directory a must sort before a.b.
    assert_eq!(
        listed(b"./a.b\0./a/x\0./a/y\0./b\0", tools::Depth::Whole, 2, 512, None),
        tools::Done::Listed {
            entries: Box::new([listed_entry(b"a", tools::Kind::Directory), listed_entry(b"a.b", tools::Kind::File)]),
            more: 3
        }
    );
    assert_eq!(
        listed(b"./a.b\0./a/x\0./a/y\0./b\0", tools::Depth::Whole, 1, 512, None),
        tools::Done::Listed { entries: Box::new([listed_entry(b"a", tools::Kind::Directory)]), more: 4 }
    );
}

#[test]
fn a_matching_file_below_depth_keeps_its_directory_and_git_is_always_omitted() {
    assert_eq!(
        listed(b"./.Git/config\0./a/deep/only.rs\0", tools::Depth::Levels(1), 2, 512, Some(b"*.rs")),
        tools::Done::Listed { entries: Box::new([listed_entry(b"a", tools::Kind::Directory)]), more: 0 }
    );
    // rg selected no files from empty, ignored-only or unmatched directories.
    assert_eq!(listed(b"", tools::Depth::Whole, 2, 512, None), tools::Done::Listed { entries: Box::new([]), more: 0 });
}

#[test]
fn listing_byte_cap_keeps_an_exact_prefix_and_malformed_records_fail() {
    assert_eq!(
        listed(b"./a/x\0./b\0", tools::Depth::Whole, 2, 0, None),
        tools::Done::Listed { entries: Box::new([]), more: 3 }
    );
    let long = [b'a'; 513];
    for output in [b"./b\0./a\0".as_slice(), b"../a\0", b"./a", b"./a\0./a\0", &long] {
        assert_eq!(
            listed(output, tools::Depth::Whole, 2, 512, None),
            tools::Done::Failed { fault: tools::Fault::Other }
        );
    }
}

fn has_argument(arguments: &[Box<[u8]>], expected: &[u8]) -> bool {
    for argument in arguments {
        if argument.as_ref() == expected {
            return true;
        }
    }
    false
}

#[test]
#[expect(clippy::wildcard_enum_match_arm, reason = "fixture restricts foreign kernel errors to three cases")]
fn a_listing_directory_failure_or_cancel_never_launches_rg() {
    for error in [kernel::Error::NotFound, kernel::Error::NotADirectory, kernel::Error::Permission] {
        let mut component = component();
        let mut up = Queue::with_capacity(2);
        let mut down = Queue::with_capacity(2);
        let owner = Token::new(71);
        component.from_domain(
            &env(),
            FromDomain::Op {
                owner,
                op: tools::Op::List {
                    at: place(b"missing"),
                    depth: tools::Depth::Whole,
                    glob: None,
                    max: 2,
                    max_bytes: 512,
                },
                deadline: Time::from_nanos(100),
            },
            &mut up,
            &mut down,
        );
        assert_eq!(
            down.pop(),
            Some(Below::OpenDirectory {
                owner,
                root: Token::new(10),
                path: Box::from(b"missing".as_slice()),
                deadline: Time::from_nanos(100)
            })
        );
        component.from_below(
            &env(),
            BelowEvent::File(file::Event::Failed { owner, error, committed: false, residue: None }),
            &mut up,
            &mut down,
        );
        let expected = match error {
            kernel::Error::NotFound => tools::Done::Missing,
            kernel::Error::NotADirectory => tools::Done::NotDirectory,
            kernel::Error::Permission => tools::Done::Failed { fault: tools::Fault::Denied },
            _ => unreachable!("three path errors"),
        };
        assert_eq!(up.pop(), Some(ToDomain::Done { owner, done: expected }));
        assert!(down.is_empty());
    }
    let mut component = component();
    let mut up = Queue::with_capacity(2);
    let mut down = Queue::with_capacity(2);
    let owner = Token::new(72);
    component.from_domain(
        &env(),
        FromDomain::Op {
            owner,
            op: tools::Op::List { at: place(b"src"), depth: tools::Depth::Whole, glob: None, max: 2, max_bytes: 512 },
            deadline: Time::from_nanos(100),
        },
        &mut up,
        &mut down,
    );
    drop(down.pop());
    component.from_domain(&env(), FromDomain::Cancel { owner }, &mut up, &mut down);
    assert_eq!(down.pop(), Some(Below::CancelFile { owner }));
    component.from_below(
        &env(),
        BelowEvent::File(file::Event::Opened { owner, file: Token::new(73), len: 0 }),
        &mut up,
        &mut down,
    );
    assert!(down.pop().is_some());
    assert!(up.is_empty());
    component.from_below(&env(), BelowEvent::File(file::Event::Closed { owner }), &mut up, &mut down);
    assert_eq!(up.pop(), Some(ToDomain::Done { owner, done: tools::Done::Cancelled }));
    assert!(down.is_empty());
}

#[test]
fn a_tools_output_is_the_same_whatever_its_deliveries() {
    use crate::process::{Process, Purpose, stream};
    use skein_lib::stream::Up;
    let bytes = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+-";
    let mut answers = skein_lib::List::with_capacity(2);
    for cut in [64, 1] {
        let mut process = Process::new(Purpose::Shell, Time::ZERO, 8, 8, None);
        process.pipes = [Some(Token::new(1)), Some(Token::new(2))];
        let mut below = Queue::with_capacity(1);
        for delivery in bytes.chunks(cut) {
            stream(&mut process, Token::new(1), Up::Bytes(Box::from(delivery)), &mut below);
            assert!(below.pop().is_some());
        }
        answers.push(process.answer(Token::new(3))).expect("two delivery cuts");
    }
    assert_eq!(answers.get(0), answers.get(1));
}
