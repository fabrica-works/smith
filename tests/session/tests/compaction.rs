use skein_lib::Token;
use smith_domain_session::{self as session, llm, record};
use smith_session_world::recorded::{World, opening, transcript};

const SIMULATION_ITERATIONS: u32 = 10_000;

fn said(text: &[u8]) -> Box<[llm::Block]> {
    Box::new([llm::Block::Text { text: text.into(), replay: None }])
}

fn usage() -> llm::Usage {
    llm::Usage { input_tokens: Some(1_300), output_tokens: Some(300), ..llm::Usage::ZERO }
}

fn started(seed: u64) -> World {
    let mut world = World::new(seed, 256);
    world.env.limits.spend = 10_000;
    let mut start = opening(None, world.env.limits.spend);
    start.spec.window = 2_000;
    start.spec.output = 512;
    world.open(start);
    world
}

fn continue_with(world: &mut World, text: &[u8]) {
    let session = world.session.expect("opened");
    world.step(session::Event::Continue { session, content: text.into() });
}

fn asked(seed: u64) -> World {
    let mut world = started(seed);
    world.complete(said(b"first answer"), llm::Stop::EndTurn, usage());
    continue_with(&mut world, b"go on");
    world
}

fn context_too_long(world: &mut World) {
    world.step(session::Event::Failed {
        retry_ok: true,
        owner: world.completing.expect("pending request"),
        failure: llm::Failure::ContextTooLong,
        evidence: llm::Evidence::Response { status: None },
        detail: Box::new([]),
    });
}

fn is_summary(world: &World) -> bool {
    world.prompts.last().expect("a prompt").messages.last().expect("a message").content
        == said(b"Summarise the work so far.")
}

#[test]
fn a_compaction_is_asked_with_none_and_the_windows_tools() {
    let mut world = World::new(4840, 256);
    let mut start = opening(None, world.env.limits.spend);
    start.spec.window = 2_000;
    start.spec.output = 512;
    start.spec.authority.grants.inspect = true;
    world.open(start);
    world.complete(said(b"answer"), llm::Stop::EndTurn, usage());
    continue_with(&mut world, b"go on");
    let before = &world.prompts[0];
    let summary = world.prompts.last().expect("compaction");
    assert!(is_summary(&world));
    assert_eq!(summary.choice, llm::ToolChoice::None);
    assert_eq!(summary.system, before.system);
    assert_eq!(summary.tools, before.tools);
    assert_eq!(summary.namespace, before.namespace);
    assert_eq!(summary.delegated, before.delegated);
    assert_eq!(summary.messages[0], before.messages[0]);
    world.close();
}

#[test]
fn a_conversation_narrowed_to_only_asks_only_again_after_its_compaction() {
    let mut world = started(4841);
    world.step(session::Event::Narrowed {
        owner: world.completing.expect("first request"),
        note: b"Finish now.".as_slice().into(),
        choice: llm::ToolChoice::Only(Box::new([Token::new(19)])),
        names: Some(Box::new([b"finish".as_slice().into()])),
    });
    world.complete(said(b"answer"), llm::Stop::EndTurn, usage());
    continue_with(&mut world, b"go on");
    assert!(is_summary(&world));
    assert_eq!(world.prompts.last().expect("compaction").choice, llm::ToolChoice::None);
    world.complete(said(b"summary"), llm::Stop::EndTurn, llm::Usage::ZERO);
    assert_eq!(world.prompts.last().expect("new window").choice, llm::ToolChoice::Only(Box::new([Token::new(19)])));
    world.close();
}

#[test]
fn a_childs_final_turn_falling_due_during_a_compaction_waits_for_it() {
    let mut world = World::new(4842, 256);
    let mut start = opening(None, world.env.limits.spend);
    start.spec.window = 2_000;
    start.spec.output = 512;
    start.spec.budget.time = skein_lib::Duration::from_secs(4);
    start.spec.final_turn = Some(session::FinalTurn {
        note: b"Answer now.".as_slice().into(),
        allowance: skein_lib::Duration::from_secs(1),
    });
    world.open(start);
    world.complete(said(b"answer"), llm::Stop::EndTurn, usage());
    continue_with(&mut world, b"go on");
    world.env.now = skein_lib::Time::ZERO.saturating_add(skein_lib::Duration::from_secs(3));
    assert_eq!(world.prompts.len(), 2);
    let compaction = world.prompts.last().expect("compaction");
    assert_eq!(compaction.choice, llm::ToolChoice::None);
    assert!(!compaction.messages.iter().any(|message| message.content == said(b"Answer now.")));
    world.complete(said(b"summary"), llm::Stop::EndTurn, llm::Usage::ZERO);
    let final_prompt = world.prompts.last().expect("final answer");
    assert_eq!(final_prompt.choice, llm::ToolChoice::None);
    assert_eq!(final_prompt.messages.last().expect("final note").content, said(b"Answer now."));
    world.complete(said(b"done"), llm::Stop::EndTurn, llm::Usage::ZERO);
    assert_eq!(world.yielded.last().expect("final yield").0, session::Yield::Budget);
    world.close();
}

#[test]
fn a_continue_past_the_threshold_compacts_and_holds_its_message_back() {
    let mut world = asked(4701);
    assert!(is_summary(&world));
    let asked = world.prompts.last().expect("summary request");
    assert!(!asked.messages.iter().any(|message| message.content == said(b"go on")));
    world.complete(said(b"the summary"), llm::Stop::EndTurn, llm::Usage { output_tokens: Some(40), ..usage() });
    assert!(matches!(world.turns.last().expect("window turn").kind, record::Kind::Window(_)));
    let opened = world.prompts.last().expect("new window request");
    let [message] = &*opened.messages else { panic!("one opening message: {opened:?}") };
    let expected =
        [b"wake".as_slice(), b"Earlier window summary:".as_slice(), b"the summary".as_slice(), b"go on".as_slice()];
    let texts: Vec<&[u8]> = message
        .content
        .iter()
        .map(|block| {
            let llm::Block::Text { text, .. } = block else { panic!("a window opens with text: {block:?}") };
            &**text
        })
        .collect();
    assert_eq!(texts, expected);
    world.close();
}

#[test]
fn a_context_too_long_compacts_a_used_window() {
    let mut world = started(4702);
    world.complete(said(b"first"), llm::Stop::EndTurn, llm::Usage::ZERO);
    continue_with(&mut world, b"go on");
    context_too_long(&mut world);
    assert!(is_summary(&world));
    world.close();
}

#[test]
fn context_too_long_on_the_compaction_fails_naming_the_window() {
    let mut world = asked(4703);
    context_too_long(&mut world);
    assert!(matches!(
        world.end,
        Some(session::End::Failed {
            detail: _,
            failure: llm::Failure::Limit { which: llm::Limit::Window, bound: 2_000 },
            evidence: llm::Evidence::Response { status: None },
            attempts: 1
        })
    ));
    assert_eq!(world.turns.len(), 1);
}

#[test]
fn a_fresh_window_that_fails_its_check_fails_naming_the_window() {
    let mut world = World::new(4704, 256);
    let mut start = opening(None, world.env.limits.spend);
    start.spec.window = 1;
    world.open(start);
    assert!(world.prompts.is_empty());
    assert!(matches!(
        world.end,
        Some(session::End::Failed { failure: llm::Failure::Limit { which: llm::Limit::Window, bound: 1 }, .. })
    ));
}

#[test]
fn calls_in_a_compactions_answer_are_not_run() {
    let mut world = asked(4705);
    let content = Box::new([
        llm::Block::Text { text: b"summary".as_slice().into(), replay: None },
        llm::Block::ToolCall {
            id: b"ignored".as_slice().into(),
            name: b"finish".as_slice().into(),
            input: b"{}".as_slice().into(),
            call: llm::Decoded::Delegated {
                source: session::ToolSource::Run,
                ticket: Token::new(19),
                effect: smith_domain_tools::Effect::Write,
                descriptor: Token::new(19),
            },
            replay: None,
        },
    ]);
    world.complete(content, llm::Stop::ToolUse, llm::Usage::ZERO);
    assert!(world.delegated.is_empty() && world.operations.is_empty());
    assert!(matches!(world.turns.last().expect("window turn").kind, record::Kind::Window(_)));
    world.close();
}

#[test]
fn a_cut_summary_opens_its_window_as_written() {
    let mut world = asked(4706);
    world.complete(said(b"cut here"), llm::Stop::MaxTokens, llm::Usage::ZERO);
    assert_eq!(world.turns.last().expect("window turn").messages[0].content[2], said(b"cut here")[0]);
    assert!(world.end.is_none());
    world.close();
}

#[test]
fn a_refused_compaction_fails_as_refused_and_tells_no_turn() {
    for block in [false, true] {
        let mut world = asked(4707);
        let content = if block {
            Box::new([llm::Block::Refusal { text: b"no".as_slice().into(), replay: None }])
        } else {
            said(b"some text")
        };
        world.complete(content, if block { llm::Stop::EndTurn } else { llm::Stop::Refusal }, llm::Usage::ZERO);
        assert_eq!(world.end, Some(session::End::NoSummary { fault: session::NoSummary::Refused }));
        assert_eq!(world.turns.len(), 1);
        assert_eq!(world.used.len(), 2);
    }
}

#[test]
fn a_compaction_with_no_text_fails_as_truncated_or_malformed() {
    for (stop, fault) in
        [(llm::Stop::MaxTokens, session::NoSummary::Truncated), (llm::Stop::EndTurn, session::NoSummary::Malformed)]
    {
        let mut world = asked(4708);
        world.complete(said(b" \n\t\r"), stop, llm::Usage::ZERO);
        assert_eq!(world.end, Some(session::End::NoSummary { fault }));
        assert_eq!(world.turns.len(), 1);
    }
}

#[test]
fn a_resumed_window_that_fails_its_check_compacts_first() {
    let mut old = started(4709);
    old.complete(said(b"first answer"), llm::Stop::EndTurn, usage());
    old.close();
    let mut world = World::new(4710, 256);
    let mut start = opening(Some(transcript(&old)), 10_000);
    start.spec.window = 2_000;
    start.spec.output = 512;
    world.open(start);
    assert!(is_summary(&world));
    world.close();
}

#[test]
fn a_budget_with_no_room_for_a_compaction_ends_the_session() {
    let mut world = started(4711);
    world.complete(said(b"first answer"), llm::Stop::EndTurn, usage());
    world.step(session::Event::Continue {
        session: world.session.expect("opened"),
        content: b"go on".as_slice().into(),
    });
    world.step(session::Event::BudgetDenied {
        owner: world.completing.expect("compaction"),
        reason: session::BudgetDenial::Spend,
    });
    assert_eq!(world.end, Some(session::End::Budget { spent: session::Dimension::Unit }));
}

#[test]
fn a_child_at_its_threshold_with_two_turns_left_compacts_then_answers() {
    let mut world = World::new(4712, 256);
    world.env.limits.spend = 10_000;
    let mut start = opening(None, 10_000);
    start.spec.window = 2_000;
    start.spec.output = 512;
    start.spec.budget.turns = 3;
    start.spec.final_turn = Some(session::FinalTurn {
        note: b"Answer now.".as_slice().into(),
        allowance: skein_lib::Duration::from_secs(1),
    });
    world.open(start);
    world.complete(said(b"first answer"), llm::Stop::EndTurn, usage());
    continue_with(&mut world, b"go on");
    assert!(is_summary(&world));
    world.complete(said(b"the summary"), llm::Stop::EndTurn, llm::Usage::ZERO);
    assert_eq!(world.prompts.last().expect("last completion").choice, llm::ToolChoice::None);
    world.complete(said(b"final answer"), llm::Stop::EndTurn, llm::Usage::ZERO);
    assert_eq!(world.yielded.last().expect("final yield").0, session::Yield::Budget);
    assert_eq!(world.used.len(), 3);
    world.close();
}

#[test]
fn an_oversized_summary_is_charged_without_telling_a_turn() {
    let mut world = World::new(4713, 256);
    world.env.limits.spend = 10_000;
    world.env.limits.completion_bytes = 256;
    let mut start = opening(None, world.env.limits.spend);
    start.spec.window = 2_000;
    start.spec.output = 512;
    world.open(start);
    world.complete(said(b"answer"), llm::Stop::EndTurn, usage());
    continue_with(&mut world, b"go on");
    world.complete(said(&[b's'; 512]), llm::Stop::EndTurn, usage());
    assert_eq!(
        world.end,
        Some(session::End::Failed {
            detail: Box::default(),
            failure: llm::Failure::Limit { which: llm::Limit::Path(llm::Cap::Answer), bound: 256 },
            evidence: llm::Evidence::Response { status: None },
            attempts: 1
        })
    );
    assert_eq!(world.turns.len(), 1);
    assert_eq!(world.used.len(), 2);
    assert!(
        world.facts.iter().any(|fact| matches!(
            fact.kind,
            session::FactKind::CompactionEnded { end: session::Compacted::Failed, .. }
        ))
    );
}

#[test]
fn a_history_at_its_message_bound_compacts_instead_of_ending() {
    let mut world = World::new(4714, 256);
    world.env.limits.history_messages = 4;
    world.open(opening(None, world.env.limits.spend));
    world.complete(said(b"answer"), llm::Stop::EndTurn, llm::Usage::ZERO);
    continue_with(&mut world, b"go on");
    assert!(is_summary(&world));
    world.complete(said(b"summary"), llm::Stop::EndTurn, llm::Usage::ZERO);
    assert!(matches!(world.turns.last().expect("window turn").kind, record::Kind::Window(_)));
    assert!(world.end.is_none());
    let prompt = world.prompts.last().expect("ordinary prompt after compaction");
    assert!(
        prompt.messages[0]
            .content
            .iter()
            .any(|block| matches!(block, llm::Block::Text { text, .. } if text.as_ref() == b"go on"))
    );
    world.close();
}

#[test]
fn a_fresh_window_at_its_history_bound_fails_naming_history() {
    let mut world = World::new(4715, 256);
    world.env.limits.history_messages = 2;
    world.open(opening(None, world.env.limits.spend));
    assert!(world.prompts.is_empty());
    assert_eq!(
        world.end,
        Some(session::End::Failed {
            detail: Box::default(),
            failure: llm::Failure::Limit { which: llm::Limit::History, bound: 2 },
            evidence: llm::Evidence::Unsent,
            attempts: 0
        })
    );
}

#[test]
fn a_childs_final_answer_at_its_message_bound_names_that_bound() {
    let mut world = World::new(4720, 256);
    world.env.limits.history_messages = 2;
    let mut start = opening(None, world.env.limits.spend);
    start.spec.budget.turns = 1;
    start.spec.final_turn = Some(session::FinalTurn {
        note: b"answer now".as_slice().into(),
        allowance: skein_lib::Duration::from_secs(1),
    });
    world.open(start);
    assert!(world.prompts.is_empty());
    assert_eq!(
        world.end,
        Some(session::End::Failed {
            detail: Box::default(),
            failure: llm::Failure::Limit { which: llm::Limit::History, bound: 2 },
            evidence: llm::Evidence::Unsent,
            attempts: 0,
        })
    );
}

#[test]
fn a_later_batch_runs_within_what_is_left_of_the_history_then_compacts() {
    let mut world = World::new(4716, 256);
    world.env.limits.history_bytes = 32_768;
    world.open(opening(None, world.env.limits.spend));
    world.complete(smith_session_world::recorded::written_calls(3), llm::Stop::ToolUse, llm::Usage::ZERO);
    for _ in 0..3 {
        let owner = *world.delegated.last().expect("every call runs");
        world.step(session::Event::Answered {
            owner,
            text: vec![b'r'; 16_384].into_boxed_slice(),
            error: false,
            spent: 0,
        });
    }
    assert!(is_summary(&world));
    let results = &world.prompts.last().expect("summary request").messages;
    let returned = &results[results.len() - 2].content;
    assert_eq!(returned.len(), 3);
    assert!(returned.iter().any(|block| matches!(block, llm::Block::ToolResult { result: llm::Returned::Text { text, .. }, .. } if text.ends_with(b" bytes omitted]"))));
    world.complete(said(b"summary"), llm::Stop::EndTurn, llm::Usage::ZERO);
    assert!(world.end.is_none());
    world.close();
}

#[test]
fn a_continue_past_the_byte_bound_keeps_its_message_and_answers_stopped_calls() {
    let mut world = World::new(4718, 256);
    world.env.limits.history_bytes = 16_384;
    world.env.limits.completion_bytes = 1024;
    world.env.limits.answer_bytes = 512;
    world.env.limits.completion_blocks = 2;
    world.open(opening(None, world.env.limits.spend));
    for _ in 0..9 {
        world.complete(said(&[b'a'; 600]), llm::Stop::EndTurn, llm::Usage::ZERO);
        continue_with(&mut world, b"go on");
    }
    world.complete(smith_session_world::recorded::written_calls(1), llm::Stop::EndTurn, llm::Usage::ZERO);
    continue_with(&mut world, &[b'h'; 7000]);
    assert!(is_summary(&world));
    let request = world.prompts.last().expect("compaction");
    let results = &request.messages[request.messages.len() - 2].content;
    assert_eq!(
        results.as_ref(),
        &[llm::Block::ToolResult {
            id: b"call-0".as_slice().into(),
            result: llm::Returned::NotRun { why: llm::NotRun::Stopped },
        }]
    );
    world.complete(said(b"summary"), llm::Stop::EndTurn, llm::Usage::ZERO);
    let request = world.prompts.last().expect("new ordinary request");
    assert_eq!(request.messages[0].content.last(), said(&[b'h'; 7000]).first());
    assert!(world.end.is_none());
    world.close();
}

#[test]
fn a_final_note_due_during_a_compaction_retry_waits_for_the_new_window() {
    let mut world = World::new(4719, 256);
    let mut start = opening(None, world.env.limits.spend);
    start.spec.window = 2_000;
    start.spec.output = 512;
    start.spec.budget.time = skein_lib::Duration::from_secs(4);
    start.spec.final_turn = Some(session::FinalTurn {
        note: b"Answer now.".as_slice().into(),
        allowance: skein_lib::Duration::from_secs(1),
    });
    world.open(start);
    world.complete(said(b"answer"), llm::Stop::EndTurn, usage());
    continue_with(&mut world, b"go on");
    assert!(is_summary(&world));
    assert_eq!(world.prompts.last().expect("compaction").choice, llm::ToolChoice::None);
    world.env.now = skein_lib::Time::ZERO.saturating_add(skein_lib::Duration::from_secs(2));
    world.step(session::Event::Failed {
        retry_ok: true,
        owner: world.completing.expect("compaction"),
        failure: llm::Failure::Overloaded,
        evidence: llm::Evidence::Response { status: None },
        detail: Box::new([]),
    });
    world.env.now = world.domain.next_deadline().expect("retry due");
    world.fire();
    assert!(is_summary(&world));
    let request = world.prompts.last().expect("retry");
    assert_eq!(request.choice, llm::ToolChoice::None);
    assert!(!request.messages.iter().any(|message| message.content == said(b"Answer now.")));
    world.complete(said(b"summary"), llm::Stop::EndTurn, llm::Usage::ZERO);
    let request = world.prompts.last().expect("final answer request");
    assert_eq!(request.messages[0].content.last(), said(b"Answer now.").first());
    assert_eq!(request.choice, llm::ToolChoice::None);
    world.complete(said(b"done"), llm::Stop::EndTurn, llm::Usage::ZERO);
    assert_eq!(world.yielded.last().expect("final yield").0, session::Yield::Budget);
    world.close();
}

fn saved_window(seed: u64) -> record::Transcript {
    let mut world = World::new(seed, 256);
    let mut start = opening(None, world.env.limits.spend);
    start.spec.effort = Some(b"low".as_slice().into());
    world.open(start);
    world.complete(said(b"saved answer"), llm::Stop::EndTurn, llm::Usage::ZERO);
    world.close();
    transcript(&world)
}

fn model_change(seed: u64) -> World {
    let mut world = World::new(seed, 256);
    let mut start = opening(Some(saved_window(seed)), world.env.limits.spend);
    start.window_llm = Some(record::Llm {
        model: b"model".as_slice().into(),
        effort: Some(b"low".as_slice().into()),
        window: 65_536,
        output: 128,
        prices: start.prices,
    });
    start.prices = record::Prices { input: 13, cached: 3, cache_write: None, output: 17, unit: 10 };
    start.spec.model = b"another-model".as_slice().into();
    start.spec.effort = Some(b"high".as_slice().into());
    start.spec.window = 131_072;
    start.spec.output = 256;
    world.open(start);
    world
}

#[test]
fn a_session_resumed_with_another_model_compacts_first_with_the_windows_model() {
    let mut world = model_change(4821);
    assert!(is_summary(&world));
    assert_eq!(world.prompts[0].model.as_ref(), b"model");
    assert_eq!(world.prompts[0].effort.as_deref(), Some(b"low".as_slice()));
    assert_eq!(world.prompts[0].max_tokens, 128);
    world.complete(said(b"summary"), llm::Stop::EndTurn, llm::Usage::ZERO);
    assert_eq!(world.prompts[1].model.as_ref(), b"another-model");
    assert_eq!(world.prompts[1].effort.as_deref(), Some(b"high".as_slice()));
    assert_eq!(world.prompts[1].max_tokens, 256);
    let turn = world.turns.last().expect("window turn");
    assert!(matches!(turn.kind, record::Kind::Window(_)));
    assert_eq!(turn.model.as_ref(), b"another-model");
    assert_eq!(turn.effort.as_deref(), Some(b"high".as_slice()));
    world.close();
}

#[test]
fn a_later_activation_with_another_effort_keeps_its_window() {
    let history = saved_window(4822);
    let prefix = history.turns[0].messages.clone();
    let mut world = World::new(4822, 256);
    let mut start = opening(Some(history), world.env.limits.spend);
    start.spec.effort = Some(b"high".as_slice().into());
    world.open(start);
    assert!(!is_summary(&world));
    assert_eq!(&world.prompts[0].messages[..prefix.len()], &*prefix);
    assert_eq!(world.prompts[0].effort.as_deref(), Some(b"high".as_slice()));
    world.complete(said(b"another answer"), llm::Stop::EndTurn, llm::Usage::ZERO);
    assert_eq!(world.turns[0].kind, record::Kind::Ordinary);
    assert_eq!(world.turns[0].effort.as_deref(), Some(b"high".as_slice()));
    world.close();
}

#[test]
fn a_resume_after_a_model_change_compaction_does_not_compact_again() {
    let mut first = model_change(4823);
    first.complete(said(b"summary"), llm::Stop::EndTurn, llm::Usage::ZERO);
    first.close();
    let mut world = World::new(4823, 256);
    let mut start = opening(Some(transcript(&first)), world.env.limits.spend);
    start.spec.model = b"another-model".as_slice().into();
    start.spec.effort = Some(b"high".as_slice().into());
    start.spec.window = 131_072;
    start.spec.output = 256;
    world.open(start);
    assert!(!is_summary(&world));
    assert_eq!(world.prompts.len(), 1);
    assert_eq!(world.prompts[0].model.as_ref(), b"another-model");
    world.close();
}

#[test]
fn a_model_change_compaction_is_charged_at_the_windows_prices() {
    let mut world = model_change(4824);
    let usage = llm::Usage { input_tokens: Some(10), output_tokens: Some(5), ..llm::Usage::ZERO };
    world.complete(said(b"summary"), llm::Stop::EndTurn, usage);
    assert_eq!(world.spend, [13]);
    assert_eq!(world.turns[0].spent, 13);
    world.complete(said(b"new answer"), llm::Stop::EndTurn, usage);
    assert_eq!(world.spend, [13, 35]);
    world.close();
}

#[test]
fn a_summary_of_several_text_blocks_is_joined_by_blank_lines() {
    let mut world = asked(4850);
    world.complete(
        Box::new([
            llm::Block::Text { text: b"first notes".as_slice().into(), replay: None },
            llm::Block::Opaque { bytes: b"reasoning".as_slice().into() },
            llm::Block::Text { text: b"next steps".as_slice().into(), replay: None },
            llm::Block::Text { text: b"exact values".as_slice().into(), replay: None },
        ]),
        llm::Stop::EndTurn,
        llm::Usage { output_tokens: Some(40), ..usage() },
    );
    assert_eq!(
        world.turns.last().expect("window turn").messages[0].content[2],
        said(b"first notes\n\nnext steps\n\nexact values")[0]
    );
    world.close();
}

#[test]
fn the_new_window_opens_with_the_task_the_preamble_and_the_summary() {
    let mut world = asked(4851);
    world.complete(said(b"summary"), llm::Stop::EndTurn, llm::Usage { output_tokens: Some(40), ..usage() });
    assert_eq!(
        world.turns.last().expect("window turn").messages,
        Box::new([llm::Message {
            role: llm::Role::User,
            content: Box::new([
                llm::Block::Text { text: b"wake".as_slice().into(), replay: None },
                llm::Block::Text { text: b"Earlier window summary:".as_slice().into(), replay: None },
                llm::Block::Text { text: b"summary".as_slice().into(), replay: None },
                llm::Block::Text { text: b"go on".as_slice().into(), replay: None },
            ]),
        }]) as Box<[llm::Message]>,
    );
    world.close();
}

#[test]
fn a_restored_window_reads_its_summary_after_the_preamble() {
    let mut old = asked(4852);
    old.complete(said(&[b's'; 2_500]), llm::Stop::EndTurn, llm::Usage { output_tokens: Some(40), ..usage() });
    old.close();
    let mut world = World::new(4853, 256);
    let mut history = transcript(&old);
    history.turns = Box::new([old.turns.last().expect("window turn").clone()]);
    let mut start = opening(Some(history), 10_000);
    start.spec.window = 2_000;
    start.spec.output = 512;
    world.open(start);
    assert_eq!(world.prompts.len(), 1);
    assert_eq!(world.prompts[0].messages[0].content[2], said(&[b's'; 2_500])[0]);
    assert!(!is_summary(&world));
    world.close();
}

#[test]
fn the_guards_refusal_compacts_the_window_once() {
    let mut world = started(4799);
    world.complete(said(b"first"), llm::Stop::EndTurn, llm::Usage::ZERO);
    continue_with(&mut world, b"go on");
    let detail = b"past the line: last prompt 1000 + new part 600 > line 1488";
    for _ in 0..2 {
        world.step(session::Event::Failed {
            retry_ok: true,
            owner: world.completing.expect("ordinary request then compaction"),
            failure: llm::Failure::ContextTooLong,
            evidence: llm::Evidence::Unsent,
            detail: detail.as_slice().into(),
        });
    }
    assert_eq!(
        world.end,
        Some(session::End::Failed {
            detail: detail.as_slice().into(),
            failure: llm::Failure::Limit { which: llm::Limit::Window, bound: 2_000 },
            evidence: llm::Evidence::Unsent,
            attempts: 1,
        })
    );
    assert_eq!(world.prompts.len(), 3);
    assert_eq!(world.turns.len(), 1);
}

#[test]
fn a_turn_counted_a_byte_a_token_is_refused_by_the_guard_then_compacted() {
    use smith_session_world::{Settings, World as Simulated, spec};
    const LINE: u64 = 200;
    let mut world = Simulated::new(Settings::calm(123));
    let opener = world.submit(skein_lib::Time::ZERO, session::Spec { line: LINE, ..spec(b"fix the build") });
    world.run(SIMULATION_ITERATIONS);
    let measures = &world.session(opener).measures;

    assert_eq!(measures[0].blocks.iter().map(|block| block.ratio).collect::<Vec<_>>(), [4, 3]);
    assert_eq!(measures[1].blocks[3].ratio, 1);
    assert_eq!(measures.iter().filter(|prompt| !prompt.sent).count(), 1);
    assert!(!measures[2].ordinary && measures[2].sent);
}

#[test]
fn a_new_part_just_within_the_line_is_sent() {
    use smith_session_world::{Settings, World as Simulated, spec};
    const LINE: u64 = 521;
    let mut world = Simulated::new(Settings::calm(104));
    let opener = world.submit(skein_lib::Time::ZERO, session::Spec { line: LINE, ..spec(b"fix the build") });
    world.run(SIMULATION_ITERATIONS);
    let prompt = &world.session(opener).measures[1];
    assert_eq!(prompt.new_part, 481);
    assert_eq!(prompt.guard.expect("guarded request").last_prompt, 40);
    assert!(prompt.sent);
}

#[test]
fn the_most_output_just_within_the_line_then_compacts_within_the_window() {
    use smith_session_world::{Settings, World as Simulated, spec};
    const LINE: u64 = 451;
    const WINDOW: u32 = 1_520;
    const OUTPUT: u32 = 1_024;
    const SUMMARY_TOKENS: u64 = 8;
    use skein_fake_llm_domain::api::{Finish, Line, Script, Turn};
    let read = Turn {
        finish: Finish::ToolCalls,
        tokens: u64::from(OUTPUT),
        lines: Box::new([Line::Call {
            name: b"read".as_slice().into(),
            arguments: br#"{"path":"src/lib.rs"}"#.as_slice().into(),
        }]),
    };
    let summary = Turn {
        finish: Finish::Stop,
        tokens: SUMMARY_TOKENS,
        lines: Box::new([Line::Text { text: b"summary".as_slice().into() }]),
    };
    let script =
        Script { cue: b"You are a coding agent.".as_slice().into(), turns: Box::new([read.clone(), read, summary]) };
    let mut world = Simulated::with_scripts(Settings::calm(104), Box::new([script]));
    let opener = world.submit(
        skein_lib::Time::ZERO,
        session::Spec { line: LINE, window: WINDOW, output: OUTPUT, ..spec(b"fix the build") },
    );
    world.run(SIMULATION_ITERATIONS);
    let session = world.session(opener);
    assert_eq!(session.records[0].usage.output_tokens, Some(u64::from(OUTPUT)));
    assert_eq!(session.measures[1].new_part, 411);
    assert!(session.measures[1].sent);
    assert!(!session.measures[2].ordinary && session.measures[2].sent);
}
