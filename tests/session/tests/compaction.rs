use skein_lib::Token;
use smith_domain_session::{self as session, llm, record};
use smith_session_world::recorded::{World, opening, transcript};

fn said(text: &[u8]) -> Box<[llm::Block]> {
    Box::new([llm::Block::Text { text: text.into(), replay: None }])
}

fn usage() -> llm::Usage {
    llm::Usage { input_tokens: Some(900), output_tokens: Some(300), ..llm::Usage::ZERO }
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
        owner: world.completing.expect("pending request"),
        failure: llm::Failure::ContextTooLong,
        evidence: llm::Evidence::Response,
        detail: Box::new([]),
    });
}

fn is_summary(world: &World) -> bool {
    world.prompts.last().expect("a prompt").messages.last().expect("a message").content
        == said(b"Summarise the work so far.")
}

#[test]
fn a_continue_past_the_threshold_compacts_and_holds_its_message_back() {
    let mut world = asked(4701);
    assert!(is_summary(&world));
    let asked = world.prompts.last().expect("summary request");
    assert!(!asked.messages.iter().any(|message| message.content == said(b"go on")));
    world.complete(said(b"the summary"), llm::Stop::EndTurn, llm::Usage { output_tokens: Some(40), ..usage() });
    assert!(matches!(world.turns.last().expect("window turn").kind, record::Kind::Window { .. }));
    let opened = world.prompts.last().expect("new window request");
    let [message] = &*opened.messages else { panic!("one opening message: {opened:?}") };
    let expected = [b"wake".as_slice(), b"the summary".as_slice(), b"go on".as_slice()];
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
            failure: llm::Failure::Limit { which: llm::Limit::Window, bound: 2_000 },
            evidence: llm::Evidence::Response
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
    assert!(matches!(world.turns.last().expect("window turn").kind, record::Kind::Window { .. }));
    world.close();
}

#[test]
fn a_cut_summary_opens_its_window_as_written() {
    let mut world = asked(4706);
    world.complete(said(b"cut here"), llm::Stop::MaxTokens, llm::Usage::ZERO);
    assert_eq!(world.turns.last().expect("window turn").messages[0].content[1], said(b"cut here")[0]);
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
    assert_eq!(world.end, Some(session::End::TranscriptFull));
    assert_eq!(world.turns.len(), 1);
    assert_eq!(world.used.len(), 2);
    assert!(
        world.facts.iter().any(|fact| matches!(
            fact.kind,
            session::FactKind::CompactionEnded { end: session::Compacted::Failed, .. }
        ))
    );
}
