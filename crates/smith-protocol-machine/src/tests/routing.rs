//! Scripted concurrent listings with colliding domain and io token values.

use super::{component, env, listed_entry, place};
use crate::{Below, BelowEvent, Component, FromDomain, ToDomain};
use skein_io::{Event, file, kernel};
use skein_lib::{Queue, Time, Token, stream::Up};
use smith_domain::tools;

const FIRST: Token = Token::new(8 << 32);
const SECOND: Token = Token::new((3 << 32) | 1);
const FIRST_CHILD: Token = Token::new(5 << 32);
const SECOND_CHILD: Token = Token::new(10 << 32);
const FIRST_PIPES: [Token; 2] = [SECOND, Token::new((4 << 32) | 1)];
const SECOND_PIPES: [Token; 2] = [FIRST, Token::new(9 << 32)];

struct Listings {
    component: Component,
    above: Queue<ToDomain>,
    below: Queue<Below>,
    next_directory_file: u64,
}

impl Listings {
    fn new() -> Self {
        let emissions = crate::max_out(&env().limits);
        Self {
            component: component(),
            above: Queue::with_capacity(emissions.to_domain),
            below: Queue::with_capacity(emissions.below),
            next_directory_file: FIRST.raw(),
        }
    }

    fn event(&mut self, event: BelowEvent) {
        self.component.from_below(&env(), event, &mut self.above, &mut self.below);
        while self.below.pop().is_some() {}
    }

    fn start(&mut self, owner: Token, path: &[u8], child: Token, pipes: [Token; 2]) {
        self.open(owner, path);
        self.event(BelowEvent::Process(Event::Spawned { owner, child, pipes: Box::new(pipes) }));
    }

    fn open(&mut self, owner: Token, path: &[u8]) {
        self.component.from_domain(
            &env(),
            FromDomain::Op {
                owner,
                op: tools::Op::List {
                    at: place(path),
                    depth: tools::Depth::Levels(2),
                    glob: None,
                    max: env().limits.entries,
                    max_bytes: env().limits.entry_bytes,
                },
                deadline: Time::from_nanos(u64::MAX),
            },
            &mut self.above,
            &mut self.below,
        );
        while self.below.pop().is_some() {}
        // File events echo the domain owner, even when the descriptor's
        // token equals another process's owner or pipe.
        let file = Token::new(self.next_directory_file);
        self.next_directory_file = self.next_directory_file.checked_add(1).expect("unique directory file tokens");
        self.event(BelowEvent::File(file::Event::Opened { owner, file, len: 0 }));
        self.event(BelowEvent::File(file::Event::Closed { owner }));
    }

    fn bytes(&mut self, pipe: Token, bytes: &[u8]) {
        self.event(BelowEvent::Pipe(Event::Stream { owner: pipe, up: Up::Bytes(Box::from(bytes)) }));
    }

    fn exited(&mut self, owner: Token) {
        self.event(BelowEvent::Process(Event::Exited { owner, exit: kernel::Exit::Code(0) }));
    }

    fn finish(&mut self, owner: Token, pipes: [Token; 2]) {
        self.exited(owner);
        for pipe in pipes {
            self.event(BelowEvent::Pipe(Event::Closed { owner: pipe }));
        }
        self.event(BelowEvent::Process(Event::Closed { owner }));
    }

    fn assert_listing(&mut self, owner: Token, directory: &[u8], file: &[u8]) {
        assert_eq!(
            self.above.pop(),
            Some(ToDomain::Done {
                owner,
                done: tools::Done::Listed {
                    entries: Box::new([
                        listed_entry(directory, tools::Kind::Directory),
                        listed_entry(file, tools::Kind::File),
                    ]),
                    more: 0,
                },
            })
        );
    }

    fn assert_closed(mut self) {
        assert!(self.above.is_empty());
        self.component.close(&mut self.below);
        assert!(self.component.closed());
    }
}

#[test]
fn concurrent_listings_keep_child_and_pipe_namespaces_separate() {
    let mut listings = Listings::new();
    listings.start(FIRST, b"kit", FIRST_CHILD, FIRST_PIPES);
    listings.start(SECOND, b"smith", SECOND_CHILD, SECOND_PIPES);
    listings.bytes(FIRST_PIPES[0], b"a/file\0");
    listings.bytes(SECOND_PIPES[0], b"b/other\0");
    listings.exited(SECOND);
    listings.exited(FIRST);
    // Both domain owners equal a live pipe of the other child. Close
    // the second child before any pipe, then interleave pipe terminals.
    listings.event(BelowEvent::Process(Event::Closed { owner: SECOND }));
    for pipe in [FIRST_PIPES[0], SECOND_PIPES[1], SECOND_PIPES[0], FIRST_PIPES[1]] {
        listings.event(BelowEvent::Pipe(Event::Closed { owner: pipe }));
    }
    listings.event(BelowEvent::Process(Event::Closed { owner: FIRST }));
    for (owner, directory, file) in [(SECOND, b"b".as_slice(), b"b/other".as_slice()), (FIRST, b"a", b"a/file")] {
        listings.assert_listing(owner, directory, file);
    }
    listings.assert_closed();
}

#[test]
fn a_failed_child_cannot_fail_another_childs_pipe() {
    let mut listings = Listings::new();
    listings.start(FIRST, b"kit", FIRST_CHILD, FIRST_PIPES);
    listings.open(SECOND, b"smith");
    listings.event(BelowEvent::Process(Event::Failed { owner: SECOND, error: skein_io::Error::Busy }));
    listings.event(BelowEvent::Process(Event::Closed { owner: SECOND }));
    assert_eq!(
        listings.above.pop(),
        Some(ToDomain::Done { owner: SECOND, done: tools::Done::Failed { fault: tools::Fault::Other } })
    );
    listings.bytes(FIRST_PIPES[0], b"a/file\0");
    listings.finish(FIRST, FIRST_PIPES);
    listings.assert_listing(FIRST, b"a", b"a/file");
    listings.assert_closed();
}

// IO reuses slab slots with a new generation, never the whole token.
const REUSED_CHILD: Token = Token::new((5 << 32) | 1);
const REUSED_PIPES: [Token; 2] = [Token::new((3 << 32) | 2), Token::new((4 << 32) | 2)];

fn reused_slots() -> Listings {
    let mut listings = Listings::new();
    listings.start(FIRST, b"kit", FIRST_CHILD, FIRST_PIPES);
    listings.finish(FIRST, FIRST_PIPES);
    drop(listings.above.pop().expect("first listing ended"));
    listings.start(SECOND, b"smith", REUSED_CHILD, REUSED_PIPES);
    listings
}

#[test]
fn reused_io_slots_bind_their_new_generation_to_the_new_child() {
    let mut listings = reused_slots();
    listings.bytes(REUSED_PIPES[0], b"b/other\0");
    listings.finish(SECOND, REUSED_PIPES);
    listings.assert_listing(SECOND, b"b", b"b/other");
    listings.assert_closed();
}

#[test]
#[should_panic(expected = "pipe event has a live route")]
fn bytes_after_closed_cannot_use_a_colliding_process_owner() {
    let mut listings = reused_slots();
    // Closed is IO's last event for a pipe. A violation must fail at the
    // missing binding, even when its value now names an admitted process.
    listings.bytes(FIRST_PIPES[0], b"late\0");
}
