#![allow(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "programming-model.md section 10.2: deterministic test harnesses use ordinary Rust"
)]

use super::*;

const STAND_IN: &[u8] = b"smith: the program wrote no result for this call";

fn builtin(id: &[u8], effect: Effect) -> Block {
    Block::ToolCall {
        id: bytes(id),
        name: bytes(b"Read"),
        input: bytes(br#"{"file_path":"file"}"#),
        call: Decoded::Builtin { effect },
        replay: None,
    }
}

fn result_event(session: Token, id: &[u8], text: &[u8], error: bool) -> Event {
    Event::BuiltinResult { session, id: bytes(id), text: bytes(text), error }
}

fn begin(harness: &mut Harness, blocks: Box<[Block]>) -> Token {
    let (session, _) = harness.open(1);
    assert_eq!(harness.step(Event::Completed { owner: session, completion: completion(blocks, Stop::ToolUse) }), None);
    session
}

fn next_prompt(harness: &mut Harness) -> Prompt {
    harness.domain.reclaim();
    calling(harness.resume()).1
}

fn returned(messages: &[Message], id: &[u8]) -> Returned {
    messages
        .iter()
        .flat_map(|message| &message.content)
        .find_map(|block| match block {
            Block::ToolResult { id: found, result } if found.as_ref() == id => Some(result.clone()),
            Block::ToolResult { .. }
            | Block::ToolCall { .. }
            | Block::Text { .. }
            | Block::Refusal { .. }
            | Block::Opaque { .. } => None,
        })
        .expect("result for the call")
}

fn expected(text: &[u8], error: bool) -> Returned {
    Returned::Text { text: bytes(text), error, replay: None }
}

#[test]
fn a_builtin_call_is_done_at_once() {
    let mut harness = Harness::new(&LIMITS);
    begin(&mut harness, Box::new([builtin(b"read", Effect::Read)]));
    assert_eq!(harness.domain.flights(), 0);
    let prompt = next_prompt(&mut harness);
    assert_eq!(returned(&prompt.messages, b"read"), expected(STAND_IN, true));
    assert_eq!(harness.turn_records.len(), 0, "the turn still owes its result");
}

#[test]
fn a_builtin_result_patches_its_slot() {
    let mut harness = Harness::new(&LIMITS);
    let session = begin(&mut harness, Box::new([builtin(b"read", Effect::Read)]));
    assert_eq!(harness.step(result_event(session, b"read", b"contents", false)), None);
    assert_eq!(harness.turn_records.len(), 0, "slots are assembled before the turn is told");
    let prompt = next_prompt(&mut harness);
    assert_eq!(returned(&prompt.messages, b"read"), expected(b"contents", false));
    assert_eq!(harness.turn_records.len(), 1);
}

#[test]
fn a_builtin_result_patches_the_results_message() {
    let mut harness = Harness::new(&LIMITS);
    let session = begin(&mut harness, Box::new([builtin(b"read", Effect::Read)]));
    drop(next_prompt(&mut harness));
    assert_eq!(harness.step(result_event(session, b"read", b"permission denied", true)), None);
    let turn = harness.turn_records.last().expect("settled turn");
    assert_eq!(returned(&turn.messages, b"read"), expected(b"permission denied", true));
}

#[test]
fn a_builtin_result_is_cut_to_its_credit() {
    let limits = Limits { delegated_result_bytes: 64, ..LIMITS };
    let mut harness = Harness::new(&limits);
    let session = begin(&mut harness, Box::new([builtin(b"read", Effect::Read)]));
    let payload = vec![b'a'; 200];
    assert_eq!(harness.step(result_event(session, b"read", &payload, false)), None);
    let prompt = next_prompt(&mut harness);
    let Returned::Text { text, error, replay } = returned(&prompt.messages, b"read") else {
        panic!("text result");
    };
    assert!(text.len() <= 64);
    assert!(text.ends_with(b" bytes omitted]"));
    assert!(!error);
    assert_eq!(replay, None);
    let (reserved, _) = crate::session::provider_credit_for_test(&harness.domain, session);
    assert!(reserved <= limits.history_bytes);
}

#[test]
fn a_turn_owing_results_is_told_once_they_came() {
    let mut harness = Harness::new(&LIMITS);
    let session = begin(&mut harness, Box::new([builtin(b"one", Effect::Read), builtin(b"two", Effect::Write)]));
    drop(next_prompt(&mut harness));
    assert_eq!(harness.step(result_event(session, b"two", b"second", false)), None);
    assert_eq!(harness.turn_records.len(), 0);
    assert_eq!(harness.step(result_event(session, b"one", b"first", false)), None);
    assert_eq!(harness.turn_records.len(), 1);
    assert_eq!(harness.step(result_event(session, b"one", b"duplicate", true)), None);
    assert_eq!(harness.turn_records.len(), 1);
    let turn = harness.turn_records.last().unwrap();
    assert_eq!(returned(&turn.messages, b"one"), expected(b"first", false));
    assert_eq!(returned(&turn.messages, b"two"), expected(b"second", false));
}

#[test]
fn an_owed_result_keeps_its_stand_in_at_the_next_completion() {
    let mut harness = Harness::new(&LIMITS);
    let session = begin(&mut harness, Box::new([builtin(b"read", Effect::Read)]));
    drop(next_prompt(&mut harness));
    drop(yielded(harness.step(Event::Completed { owner: session, completion: done() })));
    assert_eq!(harness.turn_records.len(), 2);
    assert_eq!(returned(&harness.turn_records.get(0).unwrap().messages, b"read"), expected(STAND_IN, true));
    assert_eq!(harness.step(result_event(session, b"read", b"too late", false)), None);
    assert_eq!(harness.turn_records.len(), 2);
}

#[test]
fn an_owed_result_keeps_its_stand_in_at_close() {
    for in_history in [false, true] {
        let mut harness = Harness::new(&LIMITS);
        let session = begin(&mut harness, Box::new([builtin(b"read", Effect::Read)]));
        if in_history {
            drop(next_prompt(&mut harness));
        }
        let request = harness.step(Event::Close { session });
        if in_history {
            assert_eq!(request, Some(Request::Cancel { owner: session }));
            assert_eq!(harness.turn_records.len(), 0);
            drop(harness.step(Event::Cancelled { owner: session }));
        } else {
            let Some(Request::Ended { .. }) = request else {
                panic!("closed session");
            };
        }
        assert_eq!(harness.turn_records.len(), 1);
        assert_eq!(returned(&harness.turn_records.get(0).unwrap().messages, b"read"), expected(STAND_IN, true));
        assert_eq!(harness.step(result_event(session, b"read", b"late", false)), None);
    }
}

#[test]
fn a_builtin_call_is_never_skipped() {
    let mut harness = Harness::new(&LIMITS);
    let (session, _) = harness.open(1);
    drop(calling(harness.step(Event::Narrowed {
        owner: session,
        note: bytes(b"only finish"),
        choice: crate::llm::ToolChoice::Only(Box::new([FINISH.ticket])),
        names: Some(Box::new([bytes(b"finish")])),
    })));
    let invalid = Block::ToolCall {
        id: bytes(b"bad"),
        name: bytes(b"bad"),
        input: bytes(b"{}"),
        replay: None,
        call: Decoded::Invalid { descriptor: Some(FINISH.ticket), problem: Problem::UnknownTool },
    };
    assert_eq!(
        harness.step(Event::Completed {
            owner: session,
            completion: completion(Box::new([invalid, builtin(b"write", Effect::Write)]), Stop::ToolUse)
        }),
        None
    );
    assert_eq!(harness.step(result_event(session, b"write", b"written", false)), None);
    let prompt = next_prompt(&mut harness);
    assert_eq!(returned(&prompt.messages, b"write"), expected(b"written", false));
}

#[test]
fn an_unknown_builtin_result_is_inert() {
    let mut harness = Harness::new(&LIMITS);
    let session = begin(&mut harness, Box::new([builtin(b"read", Effect::Read)]));
    drop(next_prompt(&mut harness));
    harness.keep_facts();
    assert_eq!(harness.step(result_event(session, b"unknown", b"text", false)), None);
    assert_eq!(harness.domain.pop_fact(), None);
    assert_eq!(harness.turn_records.len(), 0);
    assert_eq!(harness.step(result_event(Token::new(991), b"read", b"text", false)), None);
    assert_eq!(harness.domain.pop_fact(), None);
    assert_eq!(harness.step(result_event(session, b"read", b"first", false)), None);
    harness.keep_facts();
    assert_eq!(harness.step(result_event(session, b"read", b"again", false)), None);
    assert_eq!(harness.domain.pop_fact(), None);
    assert_eq!(harness.turn_records.len(), 1);
}

#[test]
fn builtin_calls_tell_their_facts() {
    let mut harness = Harness::new(&LIMITS);
    let session = begin(&mut harness, Box::new([builtin(b"read", Effect::Read), builtin(b"write", Effect::Write)]));
    let facts: Vec<_> = core::iter::from_fn(|| harness.pop_fact()).collect();
    let starts: Vec<_> = facts
        .into_iter()
        .filter(|fact| {
            fact.kind == Fact::BuiltinStarted { opener: Token::new(1), block: 0 }
                || fact.kind == Fact::BuiltinStarted { opener: Token::new(1), block: 1 }
        })
        .collect();
    assert_eq!(starts.len(), 2);
    assert_ne!(starts[0].call.as_ref().unwrap().owner, starts[1].call.as_ref().unwrap().owner);
    let call = starts[0].call.as_ref().unwrap();
    assert_eq!((call.source, call.effect, call.input_bytes), (crate::ToolSource::Workspace, Effect::Read, 20));
    assert_eq!(call.deadline, Time::ZERO.saturating_add(BUDGET.time));
    assert_eq!(harness.step(result_event(session, b"read", b"four", false)), None);
    let fact = harness.pop_fact().expect("program terminal");
    assert_eq!(fact.kind, Fact::BuiltinAnswered { opener: Token::new(1), bytes: 4, error: false });
    assert_eq!(fact.call, starts[0].call);
    drop(harness.step(Event::Close { session }));
    let facts: Vec<_> = core::iter::from_fn(|| harness.pop_fact()).collect();
    assert!(facts.iter().any(|fact| fact.kind
        == Fact::BuiltinAnswered {
            opener: Token::new(1),
            bytes: u64::try_from(STAND_IN.len()).unwrap(),
            error: true,
        }));
}

#[test]
fn a_builtin_after_waiting_writers_is_done_immediately() {
    let mut harness = Harness::new(&LIMITS);
    let (session, _) = harness.open(1);
    let blocks = Box::new([
        delegated(b"writer-one", 7, Effect::Write),
        delegated(b"writer-two", 7, Effect::Write),
        builtin(b"read", Effect::Read),
    ]);
    let first = harness.batch(Event::Completed { owner: session, completion: completion(blocks, Stop::ToolUse) });
    assert_eq!(first.len(), 1);
    let facts: Vec<_> = core::iter::from_fn(|| harness.domain.pop_fact()).collect();
    assert!(facts.iter().any(|fact| fact.kind == Fact::BuiltinStarted { opener: Token::new(1), block: 2 }));
    assert!(facts.iter().any(|fact| fact.kind == Fact::DelegateStarted { opener: Token::new(1), block: 0 }));
    let owners: Vec<_> = facts.iter().filter_map(|fact| fact.call.as_ref().map(|call| call.owner)).collect();
    assert_eq!(owners.len(), 2);
    assert_ne!(owners[0], owners[1], "program and opener calls share the same issuer");
    assert_eq!(harness.step(result_event(session, b"read", b"done early", false)), None);
    let second = harness.batch(answered(*first.get(0).unwrap(), 7, 4, false));
    assert_eq!(second.len(), 1);
    let (_, prompt) = calling(harness.step(answered(*second.get(0).unwrap(), 7, 4, false)));
    assert_eq!(returned(&prompt.messages, b"read"), expected(b"done early", false));
}

#[test]
fn an_owed_result_settles_at_a_retryable_failure() {
    let mut harness = Harness::new(&LIMITS);
    let session = begin(&mut harness, Box::new([builtin(b"read", Effect::Read)]));
    drop(next_prompt(&mut harness));
    assert_eq!(
        harness.step(Event::Failed {
            retry_ok: true,
            owner: session,
            failure: Failure::Unavailable,
            evidence: crate::llm::Evidence::Unsent,
            detail: Box::default()
        }),
        None
    );
    assert_eq!(harness.turn_records.len(), 1);
    assert_eq!(returned(&harness.turn_records.get(0).unwrap().messages, b"read"), expected(STAND_IN, true));
}

#[test]
fn an_owed_result_is_forgotten_by_compaction() {
    let mut harness = Harness::new(&LIMITS);
    let (session, _) = harness.open_with(1, Spec { window: 256, output: 64, line: 192, ..spec() });
    assert_eq!(
        harness.step(Event::Completed {
            owner: session,
            completion: Completion {
                usage: Usage { input_tokens: Some(180), ..USAGE },
                ..completion(Box::new([builtin(b"read", Effect::Read)]), Stop::ToolUse)
            },
        }),
        None
    );
    let prompt = next_prompt(&mut harness);
    let crate::llm::Purpose::Compaction { .. } = prompt.purpose else {
        panic!("compaction requested");
    };
    assert_eq!(harness.turn_records.len(), 1);
    let turn = harness.turn_records.last().unwrap();
    assert_eq!(returned(&turn.messages, b"read"), expected(STAND_IN, true));
    assert_eq!(turn.messages.last().unwrap().content.len(), 1, "the summary instruction follows the old turn");
    assert_eq!(harness.step(result_event(session, b"read", b"late", false)), None);
    assert_eq!(harness.turn_records.len(), 1);
}

#[test]
fn program_results_leave_the_turn_cell_bounded() {
    assert_eq!(size_of::<crate::record::Kind>(), 16);
    assert_eq!(size_of::<crate::record::Turn>(), 176);
}

#[test]
fn replacing_a_full_builtin_batch_stays_within_the_fact_reservation() {
    let mut limits = LIMITS;
    limits.parallel_tools = 1;
    limits.tools.jobs = 0;
    limits.delegated_result_bytes = 64;
    let mut harness = Harness::new(&limits);
    let blocks = || {
        (0..limits.completion_blocks)
            .map(|index| builtin(&[u8::try_from(index).unwrap()], Effect::Read))
            .collect::<Vec<_>>()
            .into_boxed_slice()
    };
    let session = begin(&mut harness, blocks());
    drop(next_prompt(&mut harness));
    // A live replacement leaves its new results owed, rather than settling them.
    let bound = limits.completion_blocks.checked_mul(3).unwrap().checked_add(2).unwrap();
    reserve_facts(&mut harness, bound);
    let mut answer = completion(blocks(), Stop::ToolUse);
    answer.reasoning_dropped = vec![1; usize::try_from(limits.completion_blocks).unwrap()].into_boxed_slice();
    crate::session::completed(&mut harness.domain, &harness.env, session, answer, &mut harness.out);
    assert_full_facts(&mut harness, bound);
}

fn reserve_facts(harness: &mut Harness, bound: u32) {
    harness.keep_facts();
    assert!(bound <= crate::max_facts(&harness.env.limits));
    harness.domain.facts = crate::facts::Facts::with_capacity(bound);
    harness.domain.facts.begin(harness.env.now);
}

fn assert_full_facts(harness: &mut Harness, bound: u32) {
    assert_eq!(harness.domain.facts_room(), 0, "the entrance has no spare fact slots");
    let facts: Vec<_> = core::iter::from_fn(|| harness.domain.pop_fact()).collect();
    assert_eq!(facts.len(), usize::try_from(bound).unwrap());
}

fn full_batch(limits: &Limits) -> Box<[Block]> {
    (0..limits.completion_blocks)
        .map(|index| builtin(&[u8::try_from(index).unwrap()], Effect::Read))
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

#[test]
fn closing_a_full_builtin_batch_fills_its_fact_bound() {
    let mut limits = LIMITS;
    limits.tools.jobs = 0;
    limits.delegated_result_bytes = 64;
    let mut harness = Harness::new(&limits);
    let session = begin(&mut harness, full_batch(&limits));
    let bound = crate::domain::builtin_close_facts(&limits);
    reserve_facts(&mut harness, bound);
    crate::session::close(&mut harness.domain, &harness.env, session, &mut harness.out);
    assert_full_facts(&mut harness, bound);
}

#[test]
fn failing_after_a_full_builtin_batch_fills_its_fact_bound() {
    let mut limits = LIMITS;
    limits.tools.jobs = 0;
    limits.delegated_result_bytes = 64;
    let mut harness = Harness::new(&limits);
    let session = begin(&mut harness, full_batch(&limits));
    drop(next_prompt(&mut harness));
    let bound = crate::domain::builtin_failure_facts(&limits);
    reserve_facts(&mut harness, bound);
    crate::session::failed(
        &mut harness.domain,
        &harness.env,
        session,
        Failure::Invalid,
        crate::llm::Evidence::Unsent,
        Box::default(),
        true,
        &mut harness.out,
    );
    assert_full_facts(&mut harness, bound);
}

#[test]
fn an_expired_completion_fills_every_fact_inventory() {
    let mut limits = LIMITS;
    limits.completion_blocks = 32;
    limits.completion_bytes = 16_384;
    limits.answer_bytes = 16_384;
    limits.history_bytes = 262_144;
    limits.parallel_tools = 1;
    limits.tools.jobs = 0;
    limits.delegated_result_bytes = 64;
    limits.facts = crate::max_facts(&limits);
    let mut harness = Harness::new(&limits);
    let session = begin(&mut harness, full_batch(&limits));
    drop(next_prompt(&mut harness));
    harness.env.now = Time::ZERO.saturating_add(BUDGET.time);
    // Old stand-ins, new starts, new stand-ins and independent reasoning;
    // CompletionAnswered, Used, the kit's Closed and the session's Ended.
    let bound = limits.completion_blocks.checked_mul(4).unwrap().checked_add(4).unwrap();
    reserve_facts(&mut harness, bound);
    let mut answer = completion(full_batch(&limits), Stop::ToolUse);
    answer.reasoning_dropped = vec![1; usize::try_from(limits.completion_blocks).unwrap()].into_boxed_slice();
    crate::session::completed(&mut harness.domain, &harness.env, session, answer, &mut harness.out);
    crate::session::pass_on_facts(&mut harness.domain, &harness.env);
    assert_full_facts(&mut harness, bound);
}

#[test]
fn a_builtin_in_a_late_completion_keeps_its_stand_in() {
    let mut harness = Harness::new(&LIMITS);
    let (session, _) = harness.open(1);
    assert_eq!(harness.step(Event::Close { session }), Some(Request::Cancel { owner: session }));
    drop(harness.step(Event::Completed {
        owner: session,
        completion: completion(Box::new([builtin(b"late", Effect::Read)]), Stop::ToolUse),
    }));
    let turn = harness.turn_records.last().expect("late completion is recorded");
    assert_eq!(returned(&turn.messages, b"late"), expected(STAND_IN, true));
    let facts: Vec<_> = core::iter::from_fn(|| harness.pop_fact()).collect();
    assert_eq!(
        facts.iter().filter(|fact| fact.kind == Fact::BuiltinStarted { opener: Token::new(1), block: 0 }).count(),
        1
    );
    assert_eq!(
        facts
            .iter()
            .filter(|fact| fact.kind
                == Fact::BuiltinAnswered {
                    opener: Token::new(1),
                    bytes: u64::try_from(STAND_IN.len()).unwrap(),
                    error: true
                })
            .count(),
        1
    );
}
