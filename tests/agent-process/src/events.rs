//! Smith's event conformance observations over the independently read file.
//! This referee owns only decoded record identities and sums; Skein owns the
//! scheduling, machine effects, resource accounting and settlement.

use smith_events::{Event, Record};

#[expect(
    clippy::too_many_lines,
    reason = "one exhaustive Smith record referee keeps all18 pairings and actual count/charge checks together"
)]
pub(super) fn check(records: &[Record]) {
    assert!(matches!(records.first().expect("stream header").event, Event::SessionStarted(_)));
    let Event::SessionEnded(end) = &records.last().expect("stream end").event else { panic!("session.ended is last") };
    assert_eq!(end.loss.events, 0, "complete destination loses no record");
    let mut observed = std::collections::BTreeMap::<&[u8], u64>::new();
    let mut reported = std::collections::BTreeMap::<&[u8], u64>::new();
    let mut responses = Vec::new();
    let mut conversations = Vec::new();
    let mut calls = Vec::new();
    let mut checks = Vec::new();
    let mut runs = Vec::new();
    let mut answered = Vec::new();
    let mut spent = 0_u64;
    let mut usage = [None; 5];
    let mut reported_usage = false;
    for record in records {
        *observed.entry(name(&record.event)).or_default() += 1;
        match &record.event {
            Event::SessionStarted(_) => assert_eq!(observed.get(b"session.started".as_slice()), Some(&1)),
            Event::SessionEnded(_) => assert_eq!(observed.get(b"session.ended".as_slice()), Some(&1)),
            Event::RunStarted(run) => {
                assert!(!runs.contains(&run.run));
                runs.push(run.run);
            }
            Event::RunCompleted(run) => {
                assert!(!answered.contains(&run.run));
                answered.push(run.run);
                assert_eq!(run.spent, spent, "actual run spend equals its response charges");
                assert_eq!(
                    [
                        run.usage.input_tokens,
                        run.usage.cache_read_tokens,
                        run.usage.cache_write_tokens,
                        run.usage.output_tokens,
                        run.usage.reasoning_tokens
                    ],
                    usage,
                    "run totals equal reported response usage; no observation stays unknown"
                );
            }
            Event::ConversationOpened(conversation) => {
                let identity = (conversation.run, conversation.conversation);
                assert!(!conversations.contains(&identity));
                conversations.push(identity);
            }
            Event::ConversationClosed(conversation) => {
                let identity = (conversation.run, conversation.conversation);
                let index = conversations
                    .iter()
                    .position(|opened| *opened == identity)
                    .expect("one actual opening before closure");
                conversations.remove(index);
            }
            Event::ResponseStarted(response) => {
                let identity = (response.run, response.conversation, response.response);
                assert!(!responses.contains(&identity));
                responses.push(identity);
            }
            Event::ResponseCompleted(response) => {
                let identity = (response.run, response.conversation, response.response);
                let index = responses
                    .iter()
                    .position(|started| *started == identity)
                    .expect("one actual provider handoff before terminal");
                responses.remove(index);
                spent = spent.checked_add(response.spent).expect("bounded actual charges");
                let actual = [
                    response.usage.input_tokens,
                    response.usage.cache_read_tokens,
                    response.usage.cache_write_tokens,
                    response.usage.output_tokens,
                    response.usage.reasoning_tokens,
                ];
                if reported_usage {
                    for (sum, value) in usage.iter_mut().zip(actual) {
                        *sum = match (*sum, value) {
                            (Some(sum), Some(value)) => Some(sum.checked_add(value).expect("bounded usage")),
                            (None, _) | (_, None) => None,
                        };
                    }
                } else {
                    usage = actual;
                    reported_usage = true;
                }
                assert_eq!(response.request_bytes, None, "transport measures belong to02.6");
                assert_eq!(response.first_byte_ms, None);
                assert_eq!(response.largest_gap_ms, None);
            }
            Event::ToolStarted(call) => {
                let identity = (call.run, call.conversation, call.call.clone());
                assert!(!calls.contains(&identity));
                calls.push(identity);
                assert_eq!(call.input, None, "call content belongs to02.6");
            }
            Event::ToolCompleted(call) => {
                let identity = (call.run, call.conversation, call.call.clone());
                let index = calls
                    .iter()
                    .position(|started| *started == identity)
                    .expect("one actual call start before terminal");
                calls.remove(index);
                assert_eq!(call.result, None, "tool content belongs to02.6");
            }
            Event::CheckStarted(check) => {
                assert!(!checks.contains(&check.run));
                checks.push(check.run);
            }
            Event::CheckCompleted(check) => {
                let index =
                    checks.iter().position(|started| *started == check.run).expect("one check start before terminal");
                checks.remove(index);
            }
            Event::Budget(_)
            | Event::TextDelta(_)
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Notice(_) => {}
        }
    }
    assert!(
        responses.is_empty() && conversations.is_empty() && calls.is_empty() && checks.is_empty(),
        "every actual start settles once"
    );
    assert_eq!(answered.len(), 1, "one answer includes the entrance refusal");
    assert!(runs.iter().all(|run| answered.contains(run)), "every admitted run answers");
    for count in &end.emitted {
        assert!(reported.insert(count.record.as_ref(), count.count).is_none(), "one emitted counter per type");
    }
    assert_eq!(observed, reported, "every emitted type, including authoritative records, equals file observations");
}

fn name(event: &Event) -> &'static [u8] {
    match event {
        Event::Budget(_) => b"budget",
        Event::SessionStarted(_) => b"session.started",
        Event::SessionEnded(_) => b"session.ended",
        Event::RunStarted(_) => b"run.started",
        Event::RunCompleted(_) => b"run.completed",
        Event::ConversationOpened(_) => b"conversation.opened",
        Event::ConversationClosed(_) => b"conversation.closed",
        Event::ResponseStarted(_) => b"response.started",
        Event::ResponseCompleted(_) => b"response.completed",
        Event::TextDelta(_) => b"text.delta",
        Event::ToolStarted(_) => b"tool.started",
        Event::ToolCompleted(_) => b"tool.completed",
        Event::CheckStarted(_) => b"check.started",
        Event::CheckCompleted(_) => b"check.completed",
        Event::MessageReceived(_) => b"message.received",
        Event::MessageRead(_) => b"message.read",
        Event::MessageUnread(_) => b"message.unread",
        Event::Notice(_) => b"notice",
    }
}
