//! Smith's event conformance observations over the independently read file.
//! This referee owns only decoded record identities and sums; Skein owns the
//! scheduling, machine effects, resource accounting and settlement.

use smith_events::{Event, Record};

/// Check the direct provider's stream identities, cardinalities and response sums.
pub fn check(records: &[Record]) {
    check_transport(records, false);
}

/// Check program stream conformance with the unknown request measures of events.md 3.5.
pub fn check_program(records: &[Record], received: &[serde_json::Value]) {
    check_transport(records, true);
    let mut captured = Vec::<Vec<Vec<u8>>>::new();
    for record in records {
        if let Event::ResponseStarted(started) = &record.event
            && started.model.backend == smith_events::Backend::ClaudeCode
            && let Some(prompt) = &started.prompt
        {
            for message in &prompt.messages {
                let mut blocks = Vec::new();
                assert_eq!(message.role, smith_events::Role::User);
                for block in &message.blocks {
                    let smith_events::Block::Text { text } = block else { panic!("sent text block") };
                    blocks.push(text.to_vec());
                }
                captured.push(blocks);
            }
        }
    }
    let expected: Vec<Vec<Vec<u8>>> = received
        .iter()
        .map(|message| {
            assert_eq!(message["type"], "user");
            assert_eq!(message["message"]["role"], "user");
            match &message["message"]["content"] {
                serde_json::Value::String(text) => vec![text.as_bytes().to_vec()],
                serde_json::Value::Array(blocks) => blocks
                    .iter()
                    .map(|block| {
                        assert_eq!(block["type"], "text");
                        block["text"].as_str().expect("recorded text block").as_bytes().to_vec()
                    })
                    .collect(),
                serde_json::Value::Null
                | serde_json::Value::Bool(_)
                | serde_json::Value::Number(_)
                | serde_json::Value::Object(_) => panic!("sent user content"),
            }
        })
        .collect();
    assert_eq!(captured, expected, "captured user messages equal independently received contents in order");
}

#[expect(
    clippy::too_many_lines,
    reason = "one exhaustive Smith record referee keeps all18 pairings and actual count/charge checks together"
)]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the inner lookback selects one preceding provider call; the outer referee checks every event"
)]
fn check_transport(records: &[Record], program: bool) {
    let record_limits = std::cell::OnceCell::new();
    let Event::SessionStarted(header) = &records.first().expect("stream header").event else {
        panic!("session.started is first");
    };
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
    for (position, record) in records.iter().enumerate() {
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
                if response.model.backend == smith_events::Backend::ClaudeCode
                    && let Some(prompt) = &response.prompt
                {
                    for message in &prompt.messages {
                        assert_eq!(message.role, smith_events::Role::User);
                        assert!(message.blocks.iter().all(|block| matches!(block, smith_events::Block::Text { .. })));
                    }
                    if prompt.system.is_some() {
                        assert!(!prompt.messages.is_empty(), "first request carries start text");
                    }
                }
            }
            Event::ResponseCompleted(response) => {
                if header.capture == smith_events::Capture::Everything
                    && response.outcome == smith_events::Outcome::Completed
                {
                    assert!(
                        response.completion.is_some(),
                        "everything capture includes the completed response's blocks"
                    );
                }
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
                            (None, Some(value)) => Some(value),
                            (sum, None) => sum,
                        };
                    }
                } else {
                    usage = actual;
                    reported_usage = true;
                }
                let started = records[..position]
                    .iter()
                    .find_map(|record| match &record.event {
                        Event::ResponseStarted(started) if started.response == response.response => Some(started),
                        _ => None,
                    })
                    .expect("response start");
                if program || started.model.backend == smith_events::Backend::ClaudeCode {
                    assert_eq!(
                        (response.request_bytes, response.first_byte_ms, response.largest_gap_ms),
                        (None, None, None)
                    );
                } else if response.outcome == smith_events::Outcome::Completed {
                    assert!(response.request_bytes.expect("completed request was encoded") > 0);
                    assert!(response.first_byte_ms.is_some(), "the native peer sent plaintext response bytes");
                    assert!(response.largest_gap_ms.is_some(), "the native peer sent multiple SSE events");
                }
            }
            Event::ToolStarted(call) => {
                let identity = (call.run, call.conversation, call.call.clone());
                assert!(!calls.contains(&identity));
                calls.push(identity);
                match header.capture {
                    smith_events::Capture::Calls | smith_events::Capture::Everything => {
                        let input = call.input.as_ref().expect("the actual call carries its provider arguments");
                        assert_eq!(u64::try_from(input.len()).expect("bounded input length"), call.input_bytes);
                        if header.capture == smith_events::Capture::Everything {
                            let arguments = records
                                .iter()
                                .take(position)
                                .rev()
                                .find_map(|record| match &record.event {
                                    Event::ResponseCompleted(response)
                                        if response.run == call.run && response.conversation == call.conversation =>
                                    {
                                        response.completion.as_ref().and_then(|completion| {
                                            completion.blocks.iter().find_map(|block| match block {
                                                smith_events::Block::Call { call: name, tool, input }
                                                    if name == &call.call && tool == &call.tool =>
                                                {
                                                    Some(input)
                                                }
                                                _ => None,
                                            })
                                        })
                                    }
                                    _ => None,
                                })
                                .expect("the independently decoded provider completion contains the actual call");
                            assert_eq!(input, arguments, "call inputs equal the provider's original arguments");
                        }
                    }
                    smith_events::Capture::None => assert!(call.input.is_none()),
                    smith_events::Capture::Unknown(_) => panic!("the fixture selects a known capture policy"),
                }
            }
            Event::ToolCompleted(call) => {
                let identity = (call.run, call.conversation, call.call.clone());
                let index = calls
                    .iter()
                    .position(|started| *started == identity)
                    .expect("one actual call start before terminal");
                calls.remove(index);
                let line = smith_events::write(
                    record,
                    &smith_events::Capture::Everything,
                    record_limits.get_or_init(|| crate::tracing::event_limits(&crate::limits()).sinks.records),
                )
                .expect("actual tool terminal fits its receiving bounds")
                .expect("tool terminal is emitted");
                assert!(
                    !line.windows(b"\"result\":".len()).any(|part| part == b"\"result\":"),
                    "tool.completed carries no pre-render result under the binding contract"
                );
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
            | Event::JobEnded(_)
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
        Event::JobEnded(_) => b"job.ended",
        Event::ToolCompleted(_) => b"tool.completed",
        Event::CheckStarted(_) => b"check.started",
        Event::CheckCompleted(_) => b"check.completed",
        Event::MessageReceived(_) => b"message.received",
        Event::MessageRead(_) => b"message.read",
        Event::MessageUnread(_) => b"message.unread",
        Event::Notice(_) => b"notice",
    }
}

/// Decode the actual closed append file under its producer's receiving ceiling.
#[must_use]
pub fn read(bytes: &[u8], events: &smith_agent_service::EventLimits) -> Vec<Record> {
    let ceiling = events.sinks.line_ceiling(events.capture).expect("producer line ceiling");
    let mut reader = smith_events::Reader::with_ceiling(&events.sinks.records, ceiling).expect("reader limits");
    let mut records = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let line = reader.feed(&bytes[offset..]).expect("valid event stream");
        assert!(line.consumed > 0);
        offset += line.consumed;
        if let Some(record) = line.record {
            records.push(record);
        }
    }
    reader.finish().expect("newline-terminated stream");
    records
}
