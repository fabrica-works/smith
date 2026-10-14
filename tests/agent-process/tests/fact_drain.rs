//! Native trace observations remain complete across host output pressure and
//! every admitted terminal. The independent channel peer supplies the answer.
use skein_world::domain::heap::Counting;
use smith_agent_process_world::{TraceCase, World, charter};
use smith_channel::RunResult;
use smith_events::{Event, RunCompleted, Status};

#[global_allocator]
static HEAP: Counting = Counting;

fn terminal(world: &World) -> &RunCompleted {
    world.assert_event_conformance();
    let terminals: Vec<_> = world
        .records()
        .iter()
        .filter_map(|record| match &record.event {
            Event::RunCompleted(answer) => Some(answer),
            Event::Budget(_)
            | Event::SessionStarted(_)
            | Event::SessionEnded(_)
            | Event::RunStarted(_)
            | Event::ConversationOpened(_)
            | Event::ConversationClosed(_)
            | Event::ResponseStarted(_)
            | Event::ResponseCompleted(_)
            | Event::TextDelta(_)
            | Event::ToolStarted(_)
            | Event::JobEnded(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Notice(_) => None,
        })
        .collect();
    assert_eq!(terminals.len(), 1, "one typed actual answer, including an entrance refusal");
    assert_eq!(world.trace_loss(), 0, "trace room is independent of channel room");
    assert!(matches!(world.records().first().expect("header").event, Event::SessionStarted(_)));
    let Event::SessionEnded(end) = &world.records().last().expect("end").event else { panic!("end is last") };
    assert_eq!(
        end.emitted.iter().map(|count| count.count).sum::<u64>(),
        u64::try_from(world.records().len()).expect("finite records")
    );
    terminals[0]
}

#[test]
fn a_host_slow_to_acknowledge_loses_projected_facts_and_the_trace_keeps_them() {
    let mut narrow = World::new(7, &charter());
    narrow.observe_facts(TraceCase::Failed, 1);
    assert!(narrow.settle());
    let mut wide = World::new(7, &charter());
    wide.observe_facts(TraceCase::Failed, 8);
    assert!(wide.settle());
    assert_eq!(narrow.answer(), wide.answer(), "projection cannot change the run's answer");
    assert_eq!(terminal(&narrow).status, terminal(&wide).status);
    assert_eq!(
        &narrow.records()[..narrow.records().len() - 1],
        &with_narrow_channel_header(wide.records())[..wide.records().len() - 1],
        "each typed emission survives the constrained output window"
    );
    let Event::SessionEnded(narrow_end) = &narrow.records().last().expect("narrow end").event else { panic!("end") };
    let Event::SessionEnded(wide_end) = &wide.records().last().expect("wide end").event else { panic!("end") };
    assert_eq!(narrow_end.loss.channel, Some(narrow.channel_loss()));
    assert_eq!(wide_end.loss.channel, Some(wide.channel_loss()));
    let mut narrow_end = narrow_end.clone();
    let mut wide_end = wide_end.clone();
    narrow_end.loss.channel = None;
    wide_end.loss.channel = None;
    assert_eq!(narrow_end, wide_end, "only separately asserted channel projection loss changes");
    assert!(narrow.channel_loss() > 0);
    let projected = narrow.observed().iter().filter(|frame| frame.kind == 0x010f).count();
    let wide_projected = wide.observed().iter().filter(|frame| frame.kind == 0x010f).count();
    assert!(
        projected < wide_projected,
        "the constrained channel loses actual projected facts: narrow={projected}/loss{} wide={wide_projected}/loss{}",
        narrow.channel_loss(),
        wide.channel_loss()
    );
}

fn with_narrow_channel_header(records: &[smith_events::Record]) -> Vec<smith_events::Record> {
    let mut expected = records.to_vec();
    let Event::SessionStarted(start) = &mut expected[0].event else { panic!("header") };
    let index = start
        .limits
        .iter()
        .position(|entry| entry.name.as_ref() == b"channel.framing.output_frames")
        .expect("named channel output frame bound");
    start.limits.get_mut(u32::try_from(index).expect("bounded header")).expect("named bound").value =
        smith_events::LimitValue::Integer(1);
    expected
}

#[test]
fn every_answer_kind_reaches_the_trace_with_its_terminal_fact_once() {
    for case in [TraceCase::Accepted, TraceCase::Failed, TraceCase::Refused, TraceCase::Parked, TraceCase::Cancelled] {
        let mut world = World::new(8, &charter());
        world.observe_facts(case, 4);
        world.check_memory();
        assert!(world.settle(), "{case:?}");
        let answer = world.answer().expect("one answer crossed the host channel");
        let ended = &terminal(&world).status;
        match case {
            TraceCase::Accepted | TraceCase::ChildAnswered | TraceCase::ThreeChildren | TraceCase::StoppedReasoning => {
                assert!(
                    matches!(answer.result(), RunResult::Accepted(_)),
                    "answer={:?}; records={:?}",
                    answer.result(),
                    world.records()
                );
                assert_eq!(ended, &Status::Accepted);
            }
            TraceCase::Failed | TraceCase::ProviderRejected => {
                assert!(matches!(answer.result(), RunResult::Failed(_)));
                assert_eq!(ended, &Status::Failed);
            }
            TraceCase::Refused => {
                assert!(matches!(answer.result(), RunResult::Refused(_)));
                assert_eq!(ended, &Status::Refused);
            }
            TraceCase::Parked => {
                assert!(matches!(answer.result(), RunResult::Parked));
                assert_eq!(ended, &Status::Parked);
            }
            TraceCase::Cancelled => {
                assert!(matches!(answer.result(), RunResult::Failed(_)));
                assert_eq!(ended, &Status::Failed);
                assert_eq!(
                    terminal(&world).failure.as_ref().expect("cancelled failure").class,
                    smith_events::AnswerClass::Cancelled
                );
            }
        }
        world.assert_agent_clean();
    }
}

#[test]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the test selects three typed records from a checked complete stream"
)]
fn a_real_child_joins_its_actual_opening_response_usage_and_enclosing_tool_terminal() {
    let mut world = World::new(19, &charter());
    world.observe_facts(TraceCase::ChildAnswered, 4);
    world.check_memory();
    assert!(world.settle());
    assert!(matches!(world.answer().expect("actual host answer").result(), RunResult::Accepted(_)));
    world.assert_event_conformance();
    let children: Vec<_> = world
        .records()
        .iter()
        .filter_map(|record| match &record.event {
            Event::ConversationOpened(opened) if opened.kind == smith_events::ConversationKind::Child => Some(opened),
            _ => None,
        })
        .collect();
    assert_eq!(children.len(), 1, "one actual delegated opening");
    let child = children[0];
    assert_eq!(child.parent, Some(0));
    assert!(child.call.is_some());
    assert_eq!(child.requested.as_ref().expect("actual child ask").turns, Some(2));
    assert_eq!(child.requested.as_ref().expect("actual child ask").seconds, Some(30));
    let completions: Vec<_> = world
        .records()
        .iter()
        .filter_map(|record| match &record.event {
            Event::ToolCompleted(completed) if completed.child.is_some() => Some(completed),
            _ => None,
        })
        .collect();
    assert_eq!(completions.len(), 1);
    let completed = completions[0];
    assert_eq!(Some(&completed.call), child.call.as_ref());
    let actual = completed.child.as_ref().expect("actual child terminal");
    assert_eq!(actual.conversation, child.conversation);
    assert_eq!(actual.stop, smith_events::ChildStop::End);
    assert_eq!(actual.turns, 1);
    let responses: Vec<_> = world
        .records()
        .iter()
        .filter_map(|record| match &record.event {
            Event::ResponseCompleted(response) if response.conversation == child.conversation => Some(response),
            _ => None,
        })
        .collect();
    assert_eq!(responses.len(), 1);
    assert_eq!(actual.usage, responses[0].usage);
    assert_eq!(actual.spent, responses[0].spent);
    assert_eq!(completed.omitted_bytes, 0, "short actual answer fits its tool cap");
    world.assert_agent_clean();
}

#[test]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the native source control selects one message receipt from the complete stream"
)]
fn initial_task_receipt_carries_the_actual_channel_label_and_text() {
    let mut world = World::new(31, &charter());
    world.initial_message(901, b"task", b"Read the guide. Keep \"quotes\" and a newline.\nALPHA");
    world.observe_facts(TraceCase::Accepted, 4);
    world.check_memory();
    assert!(world.settle());
    world.assert_event_conformance();
    let receipts: Vec<_> = world
        .records()
        .iter()
        .filter_map(|record| match &record.event {
            Event::MessageReceived(message) => Some(message),
            _ => None,
        })
        .collect();
    assert_eq!(receipts.len(), 1);
    let message = receipts[0];
    assert_eq!(message.name, 901);
    assert_eq!(message.label.as_deref(), Some(b"task".as_slice()));
    assert_eq!(message.text.as_deref(), Some(b"Read the guide. Keep \"quotes\" and a newline.\nALPHA".as_slice()));
    assert_eq!(
        message.bytes,
        u64::try_from(b"task: Read the guide. Keep \"quotes\" and a newline.\nALPHA".len())
            .expect("finite observed message bytes")
    );
    assert!(message.refused.is_none());
    assert_eq!(terminal(&world).status, Status::Accepted);
    world.assert_agent_clean();
}

fn child_usage(world: &World, parent: u64, call: &[u8]) -> String {
    for record in world.records() {
        if let Event::ResponseStarted(response) = &record.event
            && response.conversation == parent
        {
            for message in &response.prompt.as_ref().expect("full capture").messages {
                for block in &message.blocks {
                    if let smith_events::Block::Result { call: result_call, text, .. } = block
                        && result_call.as_ref() == call
                    {
                        return std::str::from_utf8(text).expect("rendered child result is UTF-8").to_owned();
                    }
                }
            }
        }
    }
    panic!("child result in main's next actual request");
}

#[test]
fn a_child_usage_line_tells_the_budget_record_naming_its_call() {
    let mut world = World::new(19, &charter());
    world.capture(smith_agent_service::sinks::Capture::Everything);
    world.observe_facts(TraceCase::ChildAnswered, 4);
    assert!(world.settle());
    let child = world
        .records()
        .iter()
        .filter_map(|record| if let Event::ConversationOpened(child) = &record.event { Some(child) } else { None })
        .find(|child| child.kind == smith_events::ConversationKind::Child)
        .expect("one actual child");
    let call = child.call.as_ref().expect("provider call");
    let told: Vec<_> = world
        .records()
        .iter()
        .filter_map(|record| if let Event::Budget(budget) = &record.event { Some(budget) } else { None })
        .filter(|budget| budget.call.as_ref() == Some(call))
        .collect();
    assert_eq!(told.len(), 1, "one return snapshot for this provider call");
    let budget = told[0];
    let turns = budget.turns.limit.saturating_sub(budget.turns.used).saturating_sub(budget.reserve.turns);
    let seconds =
        budget.time_ms.limit.saturating_sub(budget.time_ms.used).saturating_sub(budget.reserve.time_ms) / 1000;
    let usage = child_usage(&world, child.parent.expect("the actual child names its parent"), call);
    let expected = format!("; ordinary left {turns} turns and {seconds} seconds.");
    assert!(usage.contains(&expected));
}
