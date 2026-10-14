//! Command, search and listing argument admission before effects.

use super::{component, env, place};
use crate::{FromDomain, ToDomain};
use skein_lib::{Queue, Time, Token};
use smith_domain::tools;

fn admission(op: tools::Op) -> (Option<ToDomain>, u32) {
    let mut component = component();
    let mut env = env();
    env.limits.tool_payload = 4;
    let mut above = Queue::with_capacity(1);
    let mut below = Queue::with_capacity(1);
    component.from_domain(
        &env,
        FromDomain::Op { owner: Token::new(1), op, deadline: Time::from_nanos(100) },
        &mut above,
        &mut below,
    );
    (above.pop(), below.len())
}

fn shell(command: &[u8]) -> tools::Op {
    tools::Op::Spawn { cwd: place(b""), command: command.into(), head: 0, tail: 0, after: None }
}

fn search(pattern: &[u8], glob: &[u8]) -> tools::Op {
    tools::Op::Search { at: place(b""), pattern: pattern.into(), glob: Some(glob.into()), hits: 0, bytes: 0 }
}

fn listing(glob: &[u8]) -> tools::Op {
    tools::Op::List { at: place(b""), depth: tools::Depth::Whole, glob: Some(glob.into()), max: 0, max_bytes: 0 }
}

#[test]
fn a_command_at_the_tool_payload_starts_its_process() {
    assert_eq!(admission(shell(b"true")), (None, 1));
}

#[test]
fn a_command_past_the_tool_payload_refuses_before_io() {
    assert_eq!(
        admission(shell(b"false")),
        (Some(ToDomain::Done { owner: Token::new(1), done: tools::Done::Failed { fault: tools::Fault::Other } }), 0)
    );
}

#[test]
fn search_pattern_and_filter_share_the_tool_payload_bound() {
    assert_eq!(admission(search(b"xx", b"yy")), (None, 1));
}

#[test]
fn combined_search_arguments_past_the_payload_refuse_before_io() {
    assert_eq!(
        admission(search(b"xx", b"yyy")),
        (Some(ToDomain::Done { owner: Token::new(1), done: tools::Done::Failed { fault: tools::Fault::Other } }), 0)
    );
}

#[test]
fn a_listing_filter_at_the_payload_opens_its_directory() {
    assert_eq!(admission(listing(b"xxxx")), (None, 1));
}

#[test]
fn a_listing_filter_past_the_payload_refuses_before_io() {
    assert_eq!(
        admission(listing(b"xxxxx")),
        (Some(ToDomain::Done { owner: Token::new(1), done: tools::Done::Failed { fault: tools::Fault::Other } }), 0)
    );
}
