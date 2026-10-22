use skein_lib::Token;
use smith_domain_session::{self as session, llm, record};
use smith_session_world::recorded::{self, World, opening, scenario, transcript};

#[test]
fn concrete_turns_resume_verbatim_without_local_tickets() {
    let old = scenario(1, 256);
    let mut resumed = World::new(1, 256);
    resumed.open(opening(Some(transcript(&old)), 100));
    assert!(resumed.end.is_none());
    let prompt = &resumed.prompts[0];
    assert_eq!(prompt.endpoint, llm::Endpoint(7));
    assert_eq!(prompt.messages[1].content[0], llm::Block::Opaque { bytes: recorded::OPAQUE.into() });
    assert!(matches!(prompt.messages[1].content[1], llm::Block::ToolCall { call: llm::Decoded::Historical, .. }));
    assert_eq!(prompt.messages[2].content[0], old.turns[0].messages[2].content[0]);
    assert_eq!(
        prompt.messages.last().expect("the scenario supplied a value").content[0],
        llm::Block::Text { text: b"wake".as_slice().into(), replay: None }
    );
    resumed.complete(
        Box::new([llm::Block::Text { text: b"resumed".as_slice().into(), replay: None }]),
        llm::Stop::EndTurn,
        llm::Usage::ZERO,
    );
    assert_eq!(resumed.turns[0].sequence, 3);
    assert_eq!(resumed.turns[0].spent, 0, "old activation spend is not charged again");
    resumed.close();
}

#[test]
fn transcript_refusals_precede_tools_and_completions() {
    for row in refusal_cases() {
        let mut world = World::new(2, 256);
        world.open(opening(Some(row.history), 100));
        world.domain.reclaim();
        let result = Refused { end: world.end, prompts: world.prompts.len(), kits: world.domain.kits() };
        assert_eq!(result, row.expected, "{}", row.name);
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Refused {
    end: Option<session::End>,
    prompts: usize,
    kits: u32,
}

struct RefusalCase {
    name: &'static str,
    history: record::Transcript,
    expected: Refused,
}

fn refusal_case(name: &'static str, history: record::Transcript, reason: record::Refusal) -> RefusalCase {
    RefusalCase {
        name,
        history,
        expected: Refused { end: Some(session::End::TranscriptRefused { reason }), prompts: 0, kits: 0 },
    }
}

fn refusal_cases() -> Vec<RefusalCase> {
    let base = transcript(&scenario(2, 256));
    let mut header = base.clone();
    header.version = 99;
    let mut version = base.clone();
    version.turns[0].version = 99;
    let mut sequence = base.clone();
    sequence.turns[1].sequence = 9;
    let mut result = base.clone();
    result.turns[0].messages[2].content[0] = llm::Block::ToolResult {
        id: b"unmatched".as_slice().into(),
        result: llm::Returned::NotRun { why: llm::NotRun::Stopped },
    };
    let mut delegated = base.clone();
    delegated.turns[0].messages[1].content = recorded::called();
    let mut invalid = base.clone();
    historical_call(&mut invalid, llm::Decoded::Invalid { descriptor: None, problem: llm::Problem::UnknownTool });
    let mut owned = base;
    historical_call(
        &mut owned,
        llm::Decoded::Owned {
            call: smith_domain_tools::Call::List {
                path: smith_domain_tools::Path { absolute: true, parts: Box::default() },
                depth: smith_domain_tools::Depth::Whole,
                glob: None,
            },
        },
    );
    vec![
        refusal_case("header version", header, record::Refusal::Version),
        refusal_case("turn version", version, record::Refusal::Version),
        refusal_case("gap", sequence, record::Refusal::Malformed),
        refusal_case("unmatched", result, record::Refusal::Malformed),
        refusal_case("delegated", delegated, record::Refusal::Unresolved),
        refusal_case("invalid", invalid, record::Refusal::Unresolved),
        refusal_case("owned", owned, record::Refusal::Unresolved),
    ]
}

fn historical_call(history: &mut record::Transcript, decoded: llm::Decoded) {
    let llm::Block::ToolCall { call, .. } = &mut history.turns[0].messages[1].content[1] else {
        panic!("the saved scenario contains the provider call");
    };
    *call = decoded;
}

#[test]
fn oversized_history_and_fresh_specs_are_refused_at_the_entrance() {
    let old = scenario(3, 256);
    let mut world = World::new(3, 256);
    world.env.limits.history_messages = 5;
    world.open(opening(Some(transcript(&old)), 100));
    assert_eq!(world.end, Some(session::End::TranscriptRefused { reason: record::Refusal::TooLarge }));
    let mut world = World::new(3, 256);
    world.env.limits.history_bytes = 512;
    world.open(opening(Some(transcript(&old)), 100));
    assert_eq!(world.end, Some(session::End::TranscriptRefused { reason: record::Refusal::TooLarge }));
    let mut world = World::new(3, 256);
    world.env.limits.spend = 99;
    world.open(opening(None, 100));
    assert_eq!(world.end, Some(session::End::Invalid));
    let mut world = World::new(3, 256);
    let mut spec = opening(None, 100);
    spec.prices.unit = 0;
    world.open(spec);
    assert_eq!(world.end, Some(session::End::Invalid));
}

#[test]
fn unit_budget_stops_after_the_crossing_turn_settles_and_child_counts_once() {
    let mut world = World::new(4, 256);
    world.open(opening(None, 20));
    world.complete(recorded::called(), llm::Stop::ToolUse, recorded::USAGE);
    let owner = world.delegated[0];
    world.step(session::Event::Answered { owner, text: b"answer".as_slice().into(), error: false, spent: 9 });
    assert_eq!(world.end, Some(session::End::Budget { spent: session::Dimension::Unit }));
    assert_eq!(world.prompts.len(), 1);
    assert_eq!(world.turns[0].spent, 25);
    let trace = world.trace.clone();
    // Redelivery before reclaim and after reclaim is harmless, with no charge.
    for reclaim in [false, true] {
        if reclaim {
            world.domain.reclaim();
        }
        session::step(
            &mut world.domain,
            &world.env,
            session::Event::Answered { owner, text: b"duplicate".as_slice().into(), error: false, spent: 9 },
            &mut world.out,
        );
        assert!(world.out.is_empty());
    }
    assert_eq!(trace, world.trace);
    world.close();
}

#[test]
fn pricing_rounds_the_combined_completion_and_rejects_overflow() {
    assert_eq!(recorded::PRICES.price(recorded::USAGE), Some(16));
    assert_eq!(
        record::Prices { input: 1, cached: 1, cache_write: None, output: 1, unit: 3 }.price(llm::Usage {
            input_tokens: Some(1),
            output_tokens: Some(1),
            ..llm::Usage::ZERO
        }),
        Some(1)
    );
    assert_eq!(
        record::Prices { input: u64::MAX, cached: 0, cache_write: None, output: 0, unit: 1 }
            .price(llm::Usage { input_tokens: Some(2), ..llm::Usage::ZERO }),
        None
    );
    let mut world = World::new(5, 256);
    let mut spec = opening(None, u64::MAX);
    spec.prices = record::Prices { input: u64::MAX, cached: 0, cache_write: None, output: 0, unit: 1 };
    world.open(spec);
    world.complete(
        Box::new([llm::Block::Text { text: b"done".as_slice().into(), replay: None }]),
        llm::Stop::EndTurn,
        llm::Usage { input_tokens: Some(2), ..llm::Usage::ZERO },
    );
    assert_eq!(world.spend, []);
    assert_eq!(world.own_spend, []);
    assert!(world.turns.is_empty());
    assert_eq!(world.end, Some(session::End::PriceOverflow));
    world.close();
}

#[test]
fn closing_preserves_withdrawn_and_late_answers_and_provider_completions() {
    for wins in [false, true] {
        let mut world = World::new(6, 256);
        world.open(opening(None, 100));
        world.complete(recorded::called(), llm::Stop::ToolUse, recorded::USAGE);
        let owner = world.delegated[0];
        world.step(session::Event::Close { session: world.session.expect("the scenario supplied a value") });
        if wins {
            world.step(session::Event::Answered {
                owner,
                text: b"late child".as_slice().into(),
                error: false,
                spent: 9,
            });
            assert_eq!(world.turns[0].spent, 25);
        } else {
            world.step(session::Event::AnswerCancelled { owner, spent: 9 });
            assert_eq!(world.turns[0].spent, 25, "withdrawn child spend is included");
            assert_eq!(
                world.turns[0].messages[2].content[0],
                llm::Block::ToolResult { id: b"provider-call".as_slice().into(), result: llm::Returned::Withdrawn }
            );
        }
        assert_eq!(world.end, Some(session::End::Closed));
        let mut resumed = World::new(6, session::max_facts(&smith_session_world::Settings::calm(0).agent));
        resumed.open(opening(Some(transcript(&world)), 100));
        assert!(resumed.end.is_none());
        resumed.close();
        world.close();
    }
    let mut world = World::new(6, 256);
    world.open(opening(None, 100));
    world.step(session::Event::Close { session: world.session.expect("the scenario supplied a value") });
    world.complete(recorded::called(), llm::Stop::EndTurn, recorded::USAGE);
    assert_eq!(world.turns[0].spent, 16);
    assert_eq!(world.turns[0].messages[1].content[0], llm::Block::Opaque { bytes: recorded::OPAQUE.into() });
    world.close();
}

#[test]
fn committed_call_results_after_a_yield_are_restored_without_tickets() {
    let mut world = World::new(7, 256);
    world.open(opening(None, 100));
    world.complete(recorded::called(), llm::Stop::EndTurn, recorded::USAGE);
    world.close();
    let history = transcript(&world);
    let tail = history.turns.last().expect("saved turn");
    let position = tail
        .messages
        .last()
        .expect("saved assistant")
        .content
        .iter()
        .position(|block| matches!(block, llm::Block::ToolCall { id, .. } if id.as_ref() == b"provider-call"))
        .expect("saved call");
    let sequence = tail.sequence;
    let mut opened = opening(Some(history), 100);
    opened.answered = Box::new([session::record::Answered {
        origin: session::record::Origin { sequence, position: u32::try_from(position).expect("position") },
        result: llm::Returned::Text { text: b"committed answer".as_slice().into(), error: false, replay: None },
    }]);
    let mut resumed = World::new(7, 256);
    resumed.open(opened);
    assert_eq!(resumed.prompts[0].messages.len(), 3);
    assert_eq!(
        resumed.prompts[0].messages[2].content[0],
        llm::Block::ToolResult {
            id: b"provider-call".as_slice().into(),
            result: llm::Returned::Text { text: b"committed answer".as_slice().into(), error: false, replay: None }
        }
    );
    resumed.close();
}

#[test]
fn an_answer_name_outside_the_last_turn_is_malformed() {
    let mut first = World::new(31, 256);
    first.open(opening(None, 100));
    first.complete(recorded::called(), llm::Stop::EndTurn, recorded::USAGE);
    first.close();
    let mut opened = opening(Some(transcript(&first)), 100);
    opened.answered = Box::new([session::record::Answered {
        origin: session::record::Origin { sequence: 1, position: 99 },
        result: llm::Returned::Text { text: b"wrong call".as_slice().into(), error: false, replay: None },
    }]);
    let mut resumed = World::new(31, 256);
    resumed.open(opened);
    assert_eq!(resumed.end, Some(session::End::TranscriptRefused { reason: record::Refusal::Malformed }));
    assert!(resumed.prompts.is_empty());
}

#[test]
fn replay_and_facts_capacity_change_no_decision() {
    let first = scenario(19, 256);
    let replayed = scenario(19, 256);
    assert_eq!(first.trace, replayed.trace);
    assert_eq!(first.snapshots, replayed.snapshots, "the complete frozen domain states replay too");
    assert_eq!(first.trace, scenario(19, session::max_facts(&smith_session_world::Settings::calm(0).agent)).trace);
}

#[test]
fn yielded_historical_calls_resume_with_concrete_not_run_results() {
    let mut world = World::new(29, 256);
    world.open(opening(None, 100));
    world.complete(recorded::called(), llm::Stop::EndTurn, recorded::USAGE);
    world.close();
    let mut resumed = World::new(29, 256);
    resumed.open(opening(Some(transcript(&world)), 100));
    assert_eq!(
        resumed.prompts[0].messages[2].content[0],
        llm::Block::ToolResult {
            id: b"provider-call".as_slice().into(),
            result: llm::Returned::NotRun { why: llm::NotRun::Stopped }
        }
    );
    resumed.close();
}

#[test]
fn closing_a_resting_batch_keeps_its_result_and_marks_only_unstarted_calls() {
    use smith_domain_tools::{Call, Part, Path};
    let mut world = World::new(30, 256);
    world.open(opening(None, 100));
    let mut blocks = recorded::called().into_vec();
    blocks.insert(
        0,
        llm::Block::ToolCall {
            id: b"denied-read".as_slice().into(),
            name: b"read".as_slice().into(),
            input: br#"{"path":"."}"#.as_slice().into(),
            call: llm::Decoded::Owned {
                call: Call::Read {
                    path: Path { absolute: false, parts: Box::new([Part::Current]) },
                    offset: 1,
                    window: smith_domain_tools::Budget::Default,
                },
            },
            replay: None,
        },
    );
    world.complete(blocks.into(), llm::Stop::ToolUse, recorded::USAGE);
    world.domain.reclaim();
    assert!(world.domain.is_ready(), "the tools' entrance refusal defers the next batch");
    assert!(world.delegated.is_empty());
    world.close();
    let results = &world.turns[0].messages[2].content;
    assert!(matches!(
        results[0],
        llm::Block::ToolResult {
            result: llm::Returned::Owned { outcome: smith_domain_tools::Outcome::NotGranted },
            ..
        }
    ));
    assert_eq!(
        results[1],
        llm::Block::ToolResult {
            id: b"provider-call".as_slice().into(),
            result: llm::Returned::NotRun { why: llm::NotRun::Stopped }
        }
    );
}

#[test]
fn cumulative_child_spend_overflow_is_a_typed_failure() {
    let mut world = World::new(31, 256);
    world.open(opening(None, u64::MAX));
    world.complete(recorded::called(), llm::Stop::ToolUse, recorded::USAGE);
    world.step(session::Event::Answered {
        owner: world.delegated[0],
        text: b"child".as_slice().into(),
        error: false,
        spent: u64::MAX,
    });
    assert_eq!(world.spend, [16]);
    assert_eq!(world.end, Some(session::End::PriceOverflow));
    world.close();
}

#[test]
fn owned_io_cancellation_keeps_terminal_results_in_the_turn() {
    use smith_domain_tools::{Authority, Call, Done, Grants, Name, Op, Outcome, Part, Path, Repo, Version};
    for wins in [false, true] {
        let mut world = World::new(32, 256);
        let mut spec = opening(None, 100);
        let name = || Name::new(b"repo".as_slice().into()).expect("a repository mount");
        spec.spec.authority = Authority {
            cwd: Box::new([name()]),
            repos: Box::new([Repo {
                host: Box::new([
                    smith_domain_tools::Name::new(Box::from(b"host".as_slice())).expect("fixture host name")
                ]),
                mount: Box::new([name()]),
                root: Token::new(7),
                writable: false,
            }]),
            grants: Grants { inspect: true, modify: false, shell: false },
        };
        world.open(spec);
        world.complete(
            Box::new([
                llm::Block::Opaque { bytes: recorded::OPAQUE.into() },
                llm::Block::ToolCall {
                    id: b"owned-read".as_slice().into(),
                    name: b"read".as_slice().into(),
                    input: br#"{"path":"data"}"#.as_slice().into(),
                    call: llm::Decoded::Owned {
                        call: Call::Read {
                            path: Path {
                                absolute: false,
                                parts: Box::new([Part::Name {
                                    name: Name::new(b"data".as_slice().into()).expect("a file name"),
                                }]),
                            },
                            offset: 1,
                            window: smith_domain_tools::Budget::Default,
                        },
                    },
                    replay: None,
                },
            ]),
            llm::Stop::ToolUse,
            recorded::USAGE,
        );
        let (owner, op) = &world.operations[0];
        let owner = *owner;
        let Op::Load { at, .. } = op else {
            panic!("a read asks io to load the file");
        };
        assert_eq!((at.root, at.path.as_ref()), (Token::new(7), b"data".as_slice()));
        world.step(session::Event::Close { session: world.session.expect("admitted session") });
        assert_eq!(world.cancelled_operations, [owner]);
        assert!(world.turns.is_empty(), "the turn waits for the owned io terminal");
        let done = if wins {
            Done::Loaded { content: b"file bytes\n".as_slice().into(), version: Version::new([1, 0, 0, 0]) }
        } else {
            Done::Cancelled
        };
        world.step(session::Event::Done { owner, done });
        assert_eq!(world.end, Some(session::End::Closed));
        assert_eq!(world.turns[0].spent, 16);
        assert_eq!(world.turns[0].messages[1].content[0], llm::Block::Opaque { bytes: recorded::OPAQUE.into() });
        let llm::Block::ToolResult { result: llm::Returned::Owned { outcome }, .. } =
            &world.turns[0].messages[2].content[0]
        else {
            panic!("the actual owned result is recorded");
        };
        if wins {
            assert_eq!(
                *outcome,
                Outcome::Read {
                    content: b"file bytes\n".as_slice().into(),
                    first: 1,
                    lines: 1,
                    total: 1,
                    cut: smith_domain_tools::Cut::Whole,
                    next: smith_domain_tools::Next::End,
                    window: smith_domain_tools::BytesClamp::Within
                }
            );
        } else {
            assert_eq!(*outcome, Outcome::Cancelled);
        }
        let mut resumed = World::new(32, session::max_facts(&smith_session_world::Settings::calm(0).agent));
        resumed.open(opening(Some(transcript(&world)), 100));
        assert!(resumed.end.is_none(), "the settled owned terminal is resumable");
        resumed.close();
        world.close();
    }
}

#[test]
fn repeated_provider_ids_keep_distinct_origins_and_restore_includes_history_prefix() {
    let mut first = World::new(61, 256);
    first.open(opening(None, 1000));
    first.complete(recorded::called(), llm::Stop::ToolUse, llm::Usage::ZERO);
    let owner = first.delegated[0];
    first.step(session::Event::Answered { owner, text: b"first".as_slice().into(), error: false, spent: 0 });
    first.complete(recorded::called(), llm::Stop::ToolUse, llm::Usage::ZERO);
    let owner = first.delegated[0];
    first.step(session::Event::Answered { owner, text: b"second".as_slice().into(), error: false, spent: 0 });
    assert_eq!(
        first.origins,
        [record::Origin { sequence: 1, position: 1 }, record::Origin { sequence: 2, position: 1 }]
    );
    first.close();
    let mut resumed = World::new(62, 256);
    resumed.open(opening(Some(transcript(&first)), 1000));
    resumed.complete(recorded::called(), llm::Stop::ToolUse, llm::Usage::ZERO);
    assert_eq!(resumed.origins, [record::Origin { sequence: 3, position: 1 }]);
    let owner = resumed.delegated[0];
    resumed.step(session::Event::Answered { owner, text: b"third".as_slice().into(), error: false, spent: 0 });
    resumed.close();
}

// Receiving-credit controls use the actual V2 domain and its observed turns,
// rather than supplied answers as evidence. Contract: domain/session.md, 3/5.
fn initial_owned(opening: &record::Opening) -> u64 {
    let spec = &opening.spec;
    u64::try_from(
        spec.model.len() + spec.system.len() + 2 * spec.prompt.len() + spec.summary.len() + spec.preamble.len(),
    )
    .expect("small fixture")
        + u64::try_from(core::mem::size_of::<llm::Block>()).expect("block size")
        + u64::try_from(spec.delegated.len() * core::mem::size_of::<llm::Descriptor>()).expect("descriptors")
}

fn fill_initial(spec: &mut record::Opening, total: u64) {
    let current = initial_owned(spec);
    assert!(total >= current);
    let add = usize::try_from(total - current).expect("small fixture");
    let mut system = spec.spec.system.clone().into_vec();
    system.extend(vec![b's'; add]);
    spec.spec.system = system.into();
    assert_eq!(initial_owned(spec), total);
}

#[test]
fn cap_filled_provider_credit_preserves_replay_completion_that_wins_cancel() {
    let mut world = World::new(101, session::max_facts(&smith_session_world::Settings::calm(0).agent));
    world.env.limits.history_bytes = 16_384;
    world.env.limits.completion_bytes = 2048;
    world.env.limits.completion_blocks = 3;
    let reserve = session::completion_reserve(&world.env.limits).expect("bounded caps");
    let mut spec = opening(None, u64::MAX);
    fill_initial(
        &mut spec,
        world.env.limits.history_bytes
            - reserve
            - u64::try_from(core::mem::size_of::<llm::Block>()).expect("block")
            - 25
            - 256 * u64::from(world.env.limits.completion_blocks),
    );
    world.open(spec);
    assert_eq!(world.prompts.len(), 1);
    let call = recorded::called().into_vec().remove(1);
    // called() starts with opaque reasoning; the second block is its actual call.
    let call_payload = match &call {
        llm::Block::ToolCall { id, name, input, .. } => {
            assert_eq!(id.as_ref(), b"provider-call");
            assert_eq!(name.as_ref(), b"sub_agent");
            assert_eq!(input.as_ref(), br#"{"task":"review"}"#);
            id.len() + name.len() + input.len()
        }
        llm::Block::Text { .. }
        | llm::Block::Refusal { .. }
        | llm::Block::Opaque { .. }
        | llm::Block::ToolResult { .. } => panic!("fixture call"),
    };
    let fixed = 3 * core::mem::size_of::<llm::Block>() + call_payload + b"refused".len() + 8;
    let opaque = vec![0xff; 2048 - fixed].into_boxed_slice();
    let completion: Box<[llm::Block]> = Box::new([
        llm::Block::Opaque { bytes: opaque },
        llm::Block::Refusal {
            text: b"refused".as_slice().into(),
            replay: Some(llm::Replay { bytes: b"\0tag\xffext".as_slice().into() }),
        },
        call,
    ]);
    // The replay envelope is eight bytes and the full owned content reaches C.
    assert_eq!(b"\0tag\xffext".len(), 8);
    world.step(session::Event::Close { session: world.session.expect("admitted") });
    assert!(world.end.is_none(), "Cancel emission does not surrender receiving credit");
    world.complete(completion.clone(), llm::Stop::ToolUse, recorded::USAGE);
    assert_eq!(world.end, Some(session::End::Closed));
    assert_eq!(world.turns.len(), 1);
    assert_eq!(world.turns[0].usage, recorded::USAGE);
    let mut expected = completion.into_vec();
    match &mut expected[2] {
        llm::Block::ToolCall { call, .. } => *call = llm::Decoded::Historical,
        llm::Block::Text { .. }
        | llm::Block::Refusal { .. }
        | llm::Block::Opaque { .. }
        | llm::Block::ToolResult { .. } => panic!("fixture call"),
    }
    assert_eq!(world.turns[0].messages[1].content.as_ref(), expected.as_slice());
    assert_eq!(
        world.turns[0].messages[2].content.as_ref(),
        &[llm::Block::ToolResult {
            id: b"provider-call".as_slice().into(),
            result: llm::Returned::NotRun { why: llm::NotRun::Stopped }
        }]
    );
    world.close();
}

#[test]
fn full_history_batch_credit_keeps_maximum_late_results() {
    use smith_domain_tools::Effect;
    let mut world = World::new(103, session::max_facts(&smith_session_world::Settings::calm(0).agent));
    world.env.limits.history_bytes = 16_384;
    world.env.limits.completion_bytes = 512;
    world.env.limits.answer_bytes = 512;
    world.env.limits.completion_blocks = 2;
    world.env.limits.delegated_result_bytes = 2048;
    world.env.limits.parallel_tools = 2;
    let mut spec = opening(None, u64::MAX);
    spec.spec.delegated[0].effect = Effect::Read;
    let mut calls = vec![];
    let mut held = 0_u64;
    for id in [b"first".as_slice(), b"second".as_slice()] {
        calls.push(llm::Block::ToolCall {
            id: id.into(),
            name: b"served".as_slice().into(),
            input: b"{}".as_slice().into(),
            call: llm::Decoded::Delegated {
                descriptor: Token::new(19),
                source: smith_domain_session::ToolSource::Run,
                ticket: Token::new(19),
                effect: Effect::Read,
            },
            replay: Some(llm::Replay { bytes: b"\0token\xff".as_slice().into() }),
        });
        held += u64::try_from(
            2 * core::mem::size_of::<llm::Block>()
                + 2 * id.len()
                + b"served".len()
                + b"{}".len()
                + b"\0token\xff".len(),
        )
        .expect("small fixture");
    }
    fill_initial(
        &mut spec,
        world.env.limits.history_bytes
            - held
            - 4096
            - u64::try_from(core::mem::size_of::<llm::Block>()).expect("block")
            - 25,
    );
    world.open(spec);
    assert_eq!(world.prompts.len(), 1);
    world.complete(calls.into(), llm::Stop::ToolUse, llm::Usage::ZERO);
    assert_eq!(world.delegated.len(), 2);
    world.step(session::Event::Close { session: world.session.expect("admitted") });
    assert!(world.end.is_none());
    for byte in [b'a', b'b'] {
        let owner = world.delegated[0];
        world.step(session::Event::Answered { owner, text: vec![byte; 2048].into(), error: byte == b'b', spent: 0 });
    }
    assert_eq!(world.end, Some(session::End::Closed));
    let turn = world.turns.last().expect("actual final turn survives full history");
    for (index, id) in [b"first".as_slice(), b"second".as_slice()].into_iter().enumerate() {
        let result = llm::Returned::Text {
            text: vec![if index == 0 { b'a' } else { b'b' }; 2048].into(),
            error: index == 1,
            replay: None,
        };
        assert_eq!(turn.messages[2].content[index], llm::Block::ToolResult { id: id.into(), result });
    }
    world.close();
}

#[test]
fn exact_failure_classes_and_transport_evidence_survive_policy_without_diagnostic_retention() {
    for failure in [llm::Failure::ApplicationLimit, llm::Failure::Protocol, llm::Failure::Cancelled] {
        for evidence in [llm::Evidence::Unsent, llm::Evidence::Unknown, llm::Evidence::Response { status: None }] {
            for closing in [false, true] {
                let mut world = World::new(104, 256);
                world.env.limits.history_bytes = 16_384;
                world.env.limits.completion_bytes = 128;
                world.env.limits.completion_blocks = 1;
                world.env.limits.failure_bytes = 512;
                let reserve = session::completion_reserve(&world.env.limits).expect("bounded terminal cap");
                let mut spec = opening(None, u64::MAX);
                fill_initial(
                    &mut spec,
                    world.env.limits.history_bytes
                        - reserve
                        - u64::try_from(core::mem::size_of::<llm::Block>()).expect("block")
                        - 25
                        - 256 * u64::from(world.env.limits.completion_blocks),
                );
                world.open(spec);
                assert_eq!(world.prompts.len(), 1);
                if closing {
                    world.step(session::Event::Close { session: world.session.expect("admitted") });
                    assert!(world.end.is_none());
                }
                world.step(session::Event::Failed {
                    retry_ok: true,
                    owner: world.completing.expect("one actual provider terminal"),
                    failure,
                    evidence,
                    detail: vec![0xff; 512].into(),
                });
                let expected = if closing {
                    session::End::Closed
                } else {
                    session::End::Failed { detail: vec![0xff; 512].into(), failure, evidence, attempts: 1 }
                };
                assert_eq!(world.end, Some(expected));
                assert_eq!(world.prompts.len(), 1, "new classes cannot start retry");
                assert!(world.turns.is_empty(), "failure does not fabricate assistant content");
                assert!(!format!("{:?}", world.domain).contains("255, 255"), "policy drops actual detail");
                world.close();
            }
        }
    }
}

#[test]
fn fullest_history_keeps_maximum_owned_read_list_search_and_shell_after_cancel() {
    use smith_domain_tools::{self as tools, Authority, Call, Grants, Name, Repo};
    for kind in 0..4 {
        let mut world = World::new(110 + kind, session::max_facts(&smith_session_world::Settings::calm(0).agent));
        world.env.limits.history_bytes = 16_384;
        world.env.limits.completion_bytes = 512;
        world.env.limits.answer_bytes = 512;
        world.env.limits.completion_blocks = 1;
        world.env.limits.tools.read_bytes = 4096;
        world.env.limits.tools.list_entries = 8;
        world.env.limits.tools.list_bytes = 4096;
        world.env.limits.tools.search_hits = 4;
        world.env.limits.tools.search_bytes = 4096;
        world.env.limits.tools.shell_head = 2048;
        world.env.limits.tools.shell_tail = 2048;
        let (call, done, expected) = maximum_owned_result(kind);
        let credit = tools::result_worst_case(&call, &world.env.limits.tools).expect("checked complete result cap");
        let id = b"actual-owned".as_slice();
        let name = b"owned".as_slice();
        let input = b"{}".as_slice();
        let skeleton = u64::try_from(2 * core::mem::size_of::<llm::Block>() + 2 * id.len() + name.len() + input.len())
            .expect("bounded cells")
            + call.owned_bytes().expect("checked complete call")
            - u64::try_from(core::mem::size_of::<Call>()).expect("inline call");
        let mut spec = opening(None, u64::MAX);
        let mount = || Name::new(b"repo".as_slice().into()).expect("mount");
        spec.spec.authority = Authority {
            cwd: Box::new([mount()]),
            repos: Box::new([Repo {
                host: Box::new([
                    smith_domain_tools::Name::new(Box::from(b"host".as_slice())).expect("fixture host name")
                ]),
                mount: Box::new([mount()]),
                root: Token::new(7),
                writable: true,
            }]),
            grants: Grants { inspect: true, modify: true, shell: true },
        };
        fill_initial(
            &mut spec,
            world.env.limits.history_bytes
                - skeleton
                - credit
                - u64::try_from(core::mem::size_of::<llm::Block>()).expect("block")
                - 25,
        );
        world.open(spec);
        assert_eq!(world.prompts.len(), 1, "provider credit secured before actual call");
        world.complete(
            Box::new([llm::Block::ToolCall {
                id: id.into(),
                name: name.into(),
                input: input.into(),
                call: llm::Decoded::Owned { call },
                replay: None,
            }]),
            llm::Stop::ToolUse,
            llm::Usage::ZERO,
        );
        assert_eq!(world.operations.len(), 1);
        let owner = world.operations[0].0;
        world.step(session::Event::Close { session: world.session.expect("actual admitted handle") });
        assert_eq!(world.cancelled_operations, [owner]);
        assert!(world.turns.is_empty(), "close retains the actual pending IO/result right");
        world.step(session::Event::Done { owner, done });
        assert_eq!(world.end, Some(session::End::Closed));
        assert_eq!(
            world.turns[0].messages[2].content[0],
            llm::Block::ToolResult { id: id.into(), result: llm::Returned::Owned { outcome: expected } }
        );
        world.close();
    }
}

/// Exact maximum lower terminal and independently expected semantic result.
fn maximum_owned_result(
    kind: u64,
) -> (smith_domain_tools::Call, smith_domain_tools::Done, smith_domain_tools::Outcome) {
    use smith_domain_tools::{Call, Done, Entry, Exit, Hit, Kind, Name, Outcome, Part, Path, Version};
    let path = || Path {
        absolute: false,
        parts: Box::new([Part::Name { name: Name::new(b"file".as_slice().into()).expect("file name") }]),
    };
    match kind {
        0 => (
            Call::Read { path: path(), offset: 1, window: smith_domain_tools::Budget::Default },
            Done::Loaded { content: vec![b'r'; 4096].into(), version: Version::new([1; 4]) },
            Outcome::Read {
                content: vec![b'r'; 4096].into(),
                first: 1,
                lines: 1,
                total: 1,
                cut: smith_domain_tools::Cut::Whole,
                next: smith_domain_tools::Next::End,
                window: smith_domain_tools::BytesClamp::Within,
            },
        ),
        1 => {
            let each = (4096 - 8 * core::mem::size_of::<Entry>()) / 8;
            let entries: Box<[Entry]> =
                (0..8).map(|index| Entry { name: vec![b'a' + index; each].into(), kind: Kind::File }).collect();
            assert_eq!(8 * core::mem::size_of::<Entry>() + 8 * each, 4096);
            (
                Call::List { path: path(), depth: smith_domain_tools::Depth::Whole, glob: None },
                Done::Listed { entries: entries.clone(), more: 17 },
                Outcome::Listed { entries, more: 17 },
            )
        }
        2 => {
            let hits: Box<[Hit]> = (0..4)
                .map(|line| Hit { path: b"f".as_slice().into(), line: line + 1, text: vec![b's'; 1023].into() })
                .collect();
            (
                Call::Search { path: path(), pattern: b"s".as_slice().into(), glob: None },
                Done::Found { hits: hits.clone(), more: 19, timed_out: true },
                Outcome::Found { hits, more: 19, timed_out: true },
            )
        }
        3 => (
            Call::Shell {
                after: smith_domain_tools::Yield::Never,
                command: b"echo owned".as_slice().into(),
                timeout: smith_domain_tools::Timeout::Default,
                output: smith_domain_tools::Budget::Asked(4096),
            },
            Done::Exited {
                exit: Exit::Code { code: 7 },
                head: vec![b'h'; 2048].into(),
                tail: vec![b't'; 2048].into(),
                dropped: 23,
            },
            Outcome::Exited {
                exit: Exit::Code { code: 7 },
                head: vec![b'h'; 2048].into(),
                tail: vec![b't'; 2048].into(),
                dropped: 23,
                timeout: smith_domain_tools::TimeoutClamp::Within,
                output: smith_domain_tools::BytesClamp::Within,
            },
        ),
        _ => unreachable!("four owning result kinds"),
    }
}

#[test]
fn a_delegated_result_can_yield_after_its_actual_turn_before_the_next_provider_request() {
    let mut world = World::new(26, 256);
    world.open(opening(None, 100));
    world.complete(recorded::called(), llm::Stop::ToolUse, recorded::USAGE);
    let owner = world.delegated[0];
    world.step(session::Event::AnsweredAndYield {
        owner,
        text: b"messages-arrived".as_slice().into(),
        error: false,
        spent: 0,
    });
    assert_eq!(world.turns.len(), 1, "the result settles in its original concrete turn");
    assert_eq!(world.prompts.len(), 1, "the opener chooses the next user work");
    assert!(world.completing.is_none() && world.delegated.is_empty());
    world.step(session::Event::Continue {
        session: world.session.expect("yielded session"),
        content: b"person: late work".as_slice().into(),
    });
    assert_eq!(world.prompts.len(), 2);
    let prompt = &world.prompts[1];
    assert!(prompt.messages.iter().flat_map(|message| &message.content).any(|block| matches!(block,
        llm::Block::ToolResult { result: llm::Returned::Text { text, error: false, .. }, .. } if text.as_ref() == b"messages-arrived")));
    assert!(matches!(prompt.messages.last().expect("new work").content.as_ref(),
        [llm::Block::Text { text, .. }] if text.as_ref() == b"person: late work"));
    world.close();
}

fn partial_wait_mutations_are_refused(history: &record::Transcript) {
    for own_name in [true, false] {
        let mut changed = history.clone();
        for message in &mut changed.turns[0].messages {
            let mut content = message.content.to_vec();
            if own_name {
                for block in &mut content {
                    if let llm::Block::ToolCall { id, name, .. } = block
                        && id.as_ref() == b"held"
                    {
                        *name = b"another_tool".as_slice().into();
                    }
                }
            } else {
                content.retain(|block| !matches!(block, llm::Block::ToolResult { id, .. } if id.as_ref() == b"later"));
            }
            message.content = content.into();
        }
        let mut refused = World::new(30, 256);
        refused.open(opening(Some(changed), 100));
        assert_eq!(refused.end, Some(session::End::TranscriptRefused { reason: record::Refusal::Malformed }));
        assert!(
            refused.prompts.is_empty() && refused.delegated.is_empty(),
            "a second missing result or a non-own missing call starts no work"
        );
    }
}

#[test]
fn dropped_reasoning_has_one_fact_per_item_and_never_enters_history() {
    let mut world = World::new(61, 256);
    world.open(opening(None, 100));
    let sizes: Box<[u64]> = (0..world.env.limits.completion_blocks - 1).map(|index| u64::from(index) + 512).collect();
    world.step(session::Event::Completed {
        owner: world.completing.expect("pending completion"),
        completion: llm::Completion {
            reasoning_dropped: sizes.clone(),
            content: Box::new([llm::Block::Text { text: b"answer".as_slice().into(), replay: None }]),
            stop: llm::Stop::EndTurn,
            usage: llm::Usage::ZERO,
        },
    });
    let observed: Vec<u64> = world
        .facts
        .iter()
        .filter_map(|fact| match fact.kind {
            session::FactKind::ReasoningDropped { opener, bytes } => {
                assert_eq!(opener, Token::new(31));
                assert!(fact.response.is_some());
                Some(bytes)
            }
            session::FactKind::CompactionStarted { .. }
            | session::FactKind::CompactionEnded { .. }
            | session::FactKind::Opened { .. }
            | session::FactKind::CompletionStarted { .. }
            | session::FactKind::CompletionAnswered { .. }
            | session::FactKind::CompletionFailed { .. }
            | session::FactKind::CompletionCancelled { .. }
            | session::FactKind::CompletionRetried { .. }
            | session::FactKind::Tools { .. }
            | session::FactKind::BuiltinStarted { .. }
            | session::FactKind::BuiltinAnswered { .. }
            | session::FactKind::DelegateStarted { .. }
            | session::FactKind::DelegateAnswered { .. }
            | session::FactKind::DelegateWithheld { .. }
            | session::FactKind::DelegateCancelled { .. }
            | session::FactKind::Yielded { .. }
            | session::FactKind::Used { .. }
            | session::FactKind::JobEnded { .. }
            | session::FactKind::Ended { .. } => None,
        })
        .collect();
    assert_eq!(observed, sizes.as_ref());
    assert_eq!(
        world.turns[0].messages[1].content.as_ref(),
        &[llm::Block::Text { text: b"answer".as_slice().into(), replay: None }]
    );
    world.close();
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one corrective completion, initial write and ordered edit batch stay in boundary order"
)]
fn a_write_refused_as_too_large_is_made_in_pieces_and_accepted() {
    use smith_domain_tools::{Authority, Call, Done, Grants, Name, Op, Outcome, Part, Path, Repo, Version};
    let mut world = World::new(62, 256);
    world.env.limits.tools.tool_payload = 256;
    world.env.limits.tools.file_bytes = 320;
    world.env.limits.tools.read_bytes = 128;
    world.domain = session::Domain::new(&world.env.limits, 62);
    let mut spec = opening(None, 100);
    let name = || Name::new(b"repo".as_slice().into()).expect("repository mount");
    spec.spec.authority = Authority {
        cwd: Box::new([name()]),
        repos: Box::new([Repo {
            host: Box::new([smith_domain_tools::Name::new(Box::from(b"host".as_slice())).expect("fixture host name")]),
            mount: Box::new([name()]),
            root: Token::new(7),
            writable: true,
        }]),
        grants: Grants { inspect: true, modify: true, shell: false },
    };
    world.open(spec);
    world.complete(
        Box::new([llm::Block::ToolCall {
            id: b"large".as_slice().into(),
            name: b"write".as_slice().into(),
            input: b"{}".as_slice().into(),
            call: llm::Decoded::Invalid {
                descriptor: None,
                problem: llm::Problem::Oversize { bytes: 1024, bound: 256 },
            },
            replay: None,
        }]),
        llm::Stop::ToolUse,
        llm::Usage::ZERO,
    );
    world.resume();
    assert!(world.operations.is_empty(), "the invalid call makes no write");
    let answer = &world.prompts.last().expect("corrective prompt").messages[2].content[0];
    assert!(matches!(
        answer,
        llm::Block::ToolResult {
            result: llm::Returned::Invalid { problem: llm::Problem::Oversize { bytes: 1024, bound: 256 } },
            ..
        }
    ));
    let path = || Path {
        absolute: false,
        parts: Box::new([Part::Name { name: Name::new(b"data".as_slice().into()).expect("file name") }]),
    };
    let initial = [vec![b'a'; 188], b"END!".to_vec()].concat();
    let first_input =
        format!("{{\"path\":\"data\",\"content\":\"{}\"}}", std::str::from_utf8(&initial).expect("ASCII first part"))
            .into_bytes();
    assert!(first_input.len() <= 256, "the first part fits one call");
    world.complete(
        Box::new([llm::Block::ToolCall {
            id: b"piece-0".as_slice().into(),
            name: b"write".as_slice().into(),
            input: first_input.into(),
            call: llm::Decoded::Owned { call: Call::Write { path: path(), content: initial.clone().into() } },
            replay: None,
        }]),
        llm::Stop::ToolUse,
        llm::Usage::ZERO,
    );
    let [(owner, Op::Store { content, expect, .. })] = world.operations.as_slice() else { panic!("one initial store") };
    assert_eq!(content.as_ref(), initial);
    assert_eq!(*expect, smith_domain_tools::Expect::Absent);
    let first_version = Version::new([1, 0, 0, 0]);
    world.step(session::Event::Done { owner: *owner, done: Done::Stored { version: first_version } });
    world.resume();
    let first_extension = [vec![b'b'; 64], b"END!".to_vec()].concat();
    let second_extension = [vec![b'c'; 64], b"END!".to_vec()].concat();
    let edit_input = format!(
        "{{\"path\":\"data\",\"edits\":[{{\"old\":\"END!\",\"new\":\"{}\"}},{{\"old\":\"END!\",\"new\":\"{}\"}}]}}",
        std::str::from_utf8(&first_extension).expect("ASCII first extension"),
        std::str::from_utf8(&second_extension).expect("ASCII second extension")
    )
    .into_bytes();
    assert!(edit_input.len() <= 256, "both edits fit one call");
    world.complete(
        Box::new([llm::Block::ToolCall {
            id: b"piece-1".as_slice().into(),
            name: b"edit".as_slice().into(),
            input: edit_input.into(),
            call: llm::Decoded::Owned {
                call: Call::Edit {
                    path: path(),
                    edits: Box::new([
                        smith_domain_tools::Replacement {
                            old: b"END!".as_slice().into(),
                            new: first_extension.into(),
                            occurrences: smith_domain_tools::Occurrences::Once,
                        },
                        smith_domain_tools::Replacement {
                            old: b"END!".as_slice().into(),
                            new: second_extension.into(),
                            occurrences: smith_domain_tools::Occurrences::Once,
                        },
                    ]),
                },
            },
            replay: None,
        }]),
        llm::Stop::ToolUse,
        llm::Usage::ZERO,
    );
    let [(owner, Op::Load { max, .. })] = world.operations.as_slice() else { panic!("the edit loads once") };
    assert_eq!(*max, 320, "the edit loads at the whole-file cap");
    world.step(session::Event::Done {
        owner: *owner,
        done: Done::Loaded { content: initial.into(), version: first_version },
    });
    let expected = [vec![b'a'; 188], vec![b'b'; 64], vec![b'c'; 64], b"END!".to_vec()].concat();
    assert!(expected.len() > usize::try_from(world.env.limits.tools.tool_payload).expect("payload"));
    assert_eq!(expected.len(), usize::try_from(world.env.limits.tools.file_bytes).expect("whole file cap"));
    let [(owner, Op::Store { content, expect, .. })] = world.operations.as_slice() else {
        panic!("the whole edit batch stores once")
    };
    assert_eq!(content.as_ref(), expected);
    assert_eq!(*expect, smith_domain_tools::Expect::Is { version: first_version });
    world.step(session::Event::Done { owner: *owner, done: Done::Stored { version: Version::new([2, 0, 0, 0]) } });
    world.resume();
    assert!(matches!(
        world.turns[1].messages[1].content[0],
        llm::Block::ToolResult { result: llm::Returned::Owned { outcome: Outcome::Written { .. } }, .. }
    ));
    assert!(matches!(
        world.turns[2].messages[1].content[0],
        llm::Block::ToolResult { result: llm::Returned::Owned { outcome: Outcome::Edited { .. } }, .. }
    ));
    assert_eq!(world.turns.len(), 3);
    world.close();
}

#[test]
fn a_parked_wait_preserves_other_results_and_resumes_only_the_missing_result() {
    use smith_domain_tools::Effect;
    for earlier in [false, true] {
        let mut first = World::new(27, 256);
        let mut spec = opening(None, 100);
        spec.spec.delegated = Box::new([llm::Descriptor { ticket: Token::new(19), effect: Effect::Write }]);
        first.open(spec);
        let make = |id: &[u8], name: &[u8]| llm::Block::ToolCall {
            id: id.into(),
            name: name.into(),
            input: b"{}".as_slice().into(),
            replay: None,
            call: llm::Decoded::Delegated {
                descriptor: Token::new(19),
                source: session::ToolSource::Run,
                ticket: Token::new(19),
                effect: Effect::Write,
            },
        };
        let mut content = Vec::new();
        if earlier {
            content.push(make(b"earlier", b"finish"));
        }
        content.push(make(b"held", b"wait"));
        content.push(make(b"later", b"finish"));
        first.complete(content.into(), llm::Stop::ToolUse, recorded::USAGE);
        if earlier {
            let owner = first.delegated[0];
            first.step(session::Event::Answered {
                owner,
                text: b"actual earlier result".as_slice().into(),
                error: false,
                spent: 0,
            });
        }
        if earlier {
            first.resume();
        }
        let owner = first.delegated[0];
        first.step(session::Event::AnswerWithheld { owner });
        assert_eq!(first.turns.len(), 1);
        assert_eq!(first.prompts.len(), 1);
        let tail = first.turns[0].messages.last().expect("closing partial results");
        assert!(
            tail.content
                .iter()
                .any(|block| matches!(block, llm::Block::ToolResult { id, result: llm::Returned::NotRun { why: llm::NotRun::Stopped } }
            if id.as_ref() == b"later"))
        );
        assert!(
            !tail
                .content
                .iter()
                .any(|block| matches!(block, llm::Block::ToolResult { id, .. } if id.as_ref() == b"held"))
        );
        first.close();
        partial_wait_mutations_are_refused(&transcript(&first));
        let mut resumed = World::new(28, 256);
        resumed.open(opening(Some(transcript(&first)), 100));
        assert!(resumed.end.is_none(), "one own wait may be missing in the last partial results");
        assert!(matches!(resumed.prompts[0].messages.last().expect("missing wait supplied").content.as_ref(),
            [llm::Block::ToolResult { id, result: llm::Returned::Text { text, error: false, .. } }]
            if id.as_ref() == b"held" && text.as_ref() == b"wake"));
        resumed.complete(
            Box::new([llm::Block::Text { text: b"resumed".as_slice().into(), replay: None }]),
            llm::Stop::EndTurn,
            llm::Usage::ZERO,
        );
        resumed.close();
        let mut all = transcript(&first);
        all.turns = first.turns.iter().cloned().chain(resumed.turns.iter().cloned()).collect::<Vec<_>>().into();
        let mut again = World::new(29, 256);
        again.open(opening(Some(all), 100));
        assert!(again.end.is_none(), "the late result remains valid on a second restore");
        again.close();
    }
}

#[test]
fn a_generic_yield_with_two_unstarted_wait_calls_restores_both_as_not_run() {
    let mut first = World::new(31, 256);
    first.open(opening(None, 100));
    let content = [b"one".as_slice(), b"two"].map(|id| llm::Block::ToolCall {
        id: id.into(),
        name: b"wait".as_slice().into(),
        input: b"{}".as_slice().into(),
        call: llm::Decoded::Delegated {
            descriptor: Token::new(991),
            source: session::ToolSource::Run,
            ticket: Token::new(991),
            effect: smith_domain_tools::Effect::Write,
        },
        replay: None,
    });
    first.complete(Box::new(content), llm::Stop::EndTurn, llm::Usage::ZERO);
    assert!(first.delegated.is_empty(), "the actual generic yield starts neither call");
    first.close();
    let mut restored = World::new(32, 256);
    restored.open(opening(Some(transcript(&first)), 100));
    assert!(restored.end.is_none());
    assert!(matches!(restored.prompts[0].messages.last().expect("unstarted calls supplied").content.as_ref(),
        [llm::Block::ToolResult { id: one, result: llm::Returned::NotRun { why: llm::NotRun::Stopped } },
         llm::Block::ToolResult { id: two, result: llm::Returned::NotRun { why: llm::NotRun::Stopped } }, llm::Block::Text { text, .. }]
        if one.as_ref() == b"one" && two.as_ref() == b"two" && text.as_ref() == b"wake"));
    restored.close();
}

#[test]
fn opening_affinity_is_fixed_and_the_recorded_window_overrides_a_fresh_spec() {
    let affinity = llm::Affinity { key: [0x42; 16], thread: 3 };
    let mut first = World::new(17, 256);
    let mut fresh = opening(None, 100);
    fresh.spec.affinity = affinity;
    first.open(fresh);
    assert_eq!(first.prompts[0].affinity, affinity);
    first.complete(
        Box::new([llm::Block::Text { text: b"answer".as_slice().into(), replay: None }]),
        llm::Stop::EndTurn,
        llm::Usage::ZERO,
    );
    assert_eq!(
        first.turns[0].kind,
        record::Kind::First(Box::new(record::Opener { affinity, account: Some(0), backend: record::Backend::Direct }))
    );
    assert!(first.turns[0].kind.opens_window());
    first.close();
    let mut resumed = World::new(18, 256);
    let mut next = opening(Some(transcript(&first)), 100);
    next.spec.affinity = llm::Affinity { key: [0x17; 16], thread: 99 };
    resumed.open(next);
    assert_eq!(resumed.prompts[0].affinity, affinity);
    resumed.complete(
        Box::new([llm::Block::Text { text: b"continued".as_slice().into(), replay: None }]),
        llm::Stop::EndTurn,
        llm::Usage::ZERO,
    );
    assert_eq!(resumed.turns[0].kind, record::Kind::Ordinary);
    assert!(!resumed.turns[0].kind.opens_window());
    resumed.close();
}

#[test]
fn a_saved_window_starts_at_its_recorded_place_and_keeps_its_affinity() {
    let affinity = llm::Affinity { key: [0x25; 16], thread: 7 };
    let messages = Box::new([llm::Message {
        role: llm::Role::User,
        content: Box::new([
            llm::Block::Text { text: b"task".as_slice().into(), replay: None },
            llm::Block::Text { text: b"Earlier window summary:".as_slice().into(), replay: None },
            llm::Block::Text { text: b"summary".as_slice().into(), replay: None },
        ]),
    }]);
    let window = record::Turn {
        model: b"model".as_slice().into(),
        effort: None,
        version: record::VERSION,
        kind: record::Kind::Window(Box::new(record::Opener {
            affinity,
            account: Some(0),
            backend: record::Backend::Direct,
        })),
        endpoint: Some(llm::Endpoint(7)),
        dialect: 23,
        sequence: 19,
        usage: llm::Usage::ZERO,
        spent: 0,
        jobs: 0,
        messages: messages.clone(),
    };
    let history = record::Transcript {
        version: record::VERSION,
        endpoint: window.endpoint,
        dialect: window.dialect,
        turns: Box::new([window]),
    };
    let mut world = World::new(19, 256);
    world.open(opening(Some(history), 100));
    assert_eq!(world.prompts[0].affinity, affinity);
    assert_eq!(world.prompts[0].messages[0], messages[0]);
    world.complete(
        Box::new([llm::Block::Text { text: b"continued".as_slice().into(), replay: None }]),
        llm::Stop::EndTurn,
        llm::Usage::ZERO,
    );
    assert_eq!(world.turns[0].sequence, 20);
    assert_eq!(world.turns[0].kind, record::Kind::Ordinary);
    world.close();
}

#[path = "recorded/openings.rs"]
mod openings;
