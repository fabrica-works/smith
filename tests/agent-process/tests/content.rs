//! Rebuild actual native requests from append-file deltas and completion blocks.
//! The independent byte peer supplies the expected requests, never the service.
use std::collections::BTreeMap;

use skein_fake_llm_domain::api;
use skein_world::domain::heap::Counting;
use smith_agent_process_world::{TraceCase, World, charter};
use smith_events::{Block, Event, Message, Role};

#[global_allocator]
static HEAP: Counting = Counting;

fn part(block: &Block) -> api::Part {
    match block {
        Block::Text { text } | Block::Refusal { text } => api::Part::Text { text: text.clone() },
        Block::Call { call, tool, input } | Block::Cut { call, tool, input } => {
            api::Part::ToolCall { id: call.clone(), name: tool.clone(), arguments: input.clone() }
        }
        Block::Result { call, error, text } => {
            api::Part::ToolOutput { id: call.clone(), output: text.clone(), is_error: *error }
        }
        Block::Opaque { .. } | Block::Oversized { .. } | Block::Unknown { .. } => {
            panic!("this native text/call fixture has no opaque or oversized block")
        }
    }
}

fn message(message: &Message) -> api::Message {
    api::Message {
        role: match message.role {
            Role::User => api::Role::User,
            Role::Assistant => api::Role::Assistant,
            Role::Unknown(_) => panic!("known producer role"),
        },
        parts: message.blocks.iter().map(part).collect::<Vec<_>>().into_boxed_slice(),
    }
}

#[test]
fn request_new_parts_and_completions_reconstruct_every_native_request() {
    reconstruct(false);
}

#[test]
fn actual_anthropic_live_text_joins_the_native_completion_blocks() {
    reconstruct(true);
}

#[expect(clippy::wildcard_enum_match_arm, reason = "select content from an independently checked complete stream")]
fn reconstruct(anthropic: bool) {
    let mut world = World::new(37, &charter());
    world.private_grant();
    if anthropic {
        world.anthropic();
    }
    world.initial_message(902, b"task", b"Keep \"quotes\" and a newline.\nALPHA");
    world.observe_facts(TraceCase::ChildAnswered, 4);
    world.check_memory();
    assert!(world.settle());
    world.assert_event_conformance();
    let mut queries = world.peer_queries();
    assert_eq!(queries.len(), 3, "main, actual child, and main after rendered child feedback");
    let mut conversations = BTreeMap::<u64, (u64, Vec<u8>, Vec<Vec<u8>>, Vec<api::Message>)>::new();
    let mut deltas = BTreeMap::<(u64, u64, u64), Vec<u8>>::new();
    let mut requests = 0;
    let mut text_blocks = 0;
    for record in world.records() {
        match &record.event {
            Event::ResponseStarted(started) => {
                let prompt = started.prompt.as_ref().expect("Everything records the actual hand-off");
                let state = conversations.entry(started.conversation).or_default();
                if state.0 != started.window {
                    *state = (started.window, Vec::new(), Vec::new(), Vec::new());
                }
                if let Some(system) = &prompt.system {
                    state.1 = system.to_vec();
                    state.2 =
                        prompt.tools.as_ref().expect("first window tools").iter().map(|name| name.to_vec()).collect();
                } else {
                    assert!(prompt.tools.is_none(), "later new parts omit offered definitions");
                }
                state.3.extend(prompt.messages.iter().map(message));
                let found = queries
                    .iter()
                    .position(|query| {
                        query.model.as_ref() == started.model.model.as_ref()
                            && query.system.as_ref() == state.1.as_slice()
                            && query.messages.as_ref() == state.3.as_slice()
                    })
                    .expect("the rebuilt ordered content equals an independently decoded native request");
                let query = queries.remove(found);
                assert_eq!(query.tools.iter().map(|tool| tool.name.to_vec()).collect::<Vec<_>>(), state.2);
                // Codex sends no output cap. The independent OpenAI peer
                // supplies its configured model ceiling for that absent field;
                // this content reconstruction does not equate the two sources.
                assert_eq!(query.max_tokens, if anthropic { 512 } else { 4096 });
                assert_eq!(started.output_tokens, 512);
                requests += 1;
            }
            Event::TextDelta(delta) => {
                deltas
                    .entry((delta.conversation, delta.response, delta.block))
                    .or_default()
                    .extend_from_slice(&delta.text);
            }
            Event::ResponseCompleted(completed) => {
                let completion = completed.completion.as_ref().expect("native successful completion under Everything");
                for (index, block) in completion.blocks.iter().enumerate() {
                    if let Block::Text { text } | Block::Refusal { text } = block {
                        let key =
                            (completed.conversation, completed.response, u64::try_from(index).expect("finite blocks"));
                        // This independent Codex peer emits Added/Done with
                        // complete text, and no TextDelta SSE event. The native
                        // Anthropic component story separately joins live pieces.
                        if anthropic {
                            assert_eq!(deltas.remove(&key).expect("actual native live pieces"), text.to_vec());
                        } else {
                            assert!(deltas.remove(&key).is_none());
                        }
                        assert_eq!(text.as_ref(), b"actual child answer");
                        text_blocks += 1;
                    }
                }
                conversations.get_mut(&completed.conversation).expect("actual request precedes completion").3.push(
                    api::Message {
                        role: api::Role::Assistant,
                        parts: completion.blocks.iter().map(part).collect::<Vec<_>>().into_boxed_slice(),
                    },
                );
            }
            _ => {}
        }
    }
    assert_eq!(requests, 3);
    assert_eq!(text_blocks, 1, "the child supplies actual text while main calls tools");
    assert!(queries.is_empty());
    assert!(deltas.is_empty());
    let trace = std::str::from_utf8(world.trace_bytes()).expect("actual UTF-8 JSON lines");
    assert!(!trace.contains("native-private-bearer-02.6"), "actual granted bearer bytes never reach the stream");
    assert!(!trace.contains("native-private-account-02.6"), "actual granted account bytes never reach the stream");
    world.assert_agent_clean();
}

#[test]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "check capture fields across independently checked complete native streams"
)]
fn native_capture_policies_keep_execution_and_select_only_their_content() {
    use smith_agent_service::sinks::Capture;
    let mut queries = None;
    let mut answer = None;
    for capture in [Capture::None, Capture::Calls, Capture::Everything] {
        let mut world = World::new(38, &charter());
        world.initial_message(903, b"task", b"ALPHA");
        world.observe_facts(TraceCase::ChildAnswered, 4);
        world.capture(capture);
        world.check_memory();
        assert!(world.settle());
        world.assert_event_conformance();
        let actual_queries: Vec<_> = world.peer_queries().into_iter().cloned().collect();
        if let Some(expected) = &queries {
            assert_eq!(&actual_queries, expected, "capture grants no execution authority");
            assert_eq!(world.answer(), answer);
        } else {
            queries = Some(actual_queries);
            answer = world.answer();
        }
        let mut calls = 0;
        let mut starts = 0;
        for record in world.records() {
            match &record.event {
                Event::SessionStarted(started) => assert_eq!(started.capture, capture.encoded()),
                Event::MessageReceived(message) => {
                    assert_eq!(message.text.is_some(), capture == Capture::Everything);
                }
                Event::ResponseStarted(started) => {
                    assert_eq!(started.prompt.is_some(), capture == Capture::Everything);
                    starts += 1;
                }
                Event::ResponseCompleted(completed) => {
                    assert_eq!(completed.completion.is_some(), capture == Capture::Everything);
                    assert!(completed.request_bytes.is_some(), "transport is independent of capture");
                }
                Event::ToolStarted(started) => {
                    assert_eq!(started.input.is_some(), capture != Capture::None);
                    calls += 1;
                }
                Event::RunCompleted(completed) => {
                    assert_eq!(
                        completed.result.as_ref().expect("actual accepted report").text.is_some(),
                        capture == Capture::Everything
                    );
                }
                Event::TextDelta(_) => assert_eq!(capture, Capture::Everything),
                _ => {}
            }
        }
        assert_eq!(starts, 3);
        assert_eq!(calls, 2);
        world.assert_agent_clean();
    }
}

#[test]
#[expect(clippy::wildcard_enum_match_arm, reason = "select starts and terminals from the independently checked stream")]
fn three_conversations_starting_completions_in_one_pass_are_each_recorded() {
    let mut world = World::new(39, &charter());
    world.initial_message(904, b"task", b"ALPHA");
    world.observe_facts(TraceCase::ThreeChildren, 8);
    world.check_memory();
    assert!(world.settle());
    world.assert_event_conformance();
    assert_eq!(world.peer_queries().len(), 5, "main, three actual children, final main");
    let children: Vec<_> = world
        .records()
        .iter()
        .filter_map(|record| match &record.event {
            Event::ConversationOpened(opened) if opened.parent.is_some() => Some(opened.conversation),
            _ => None,
        })
        .collect();
    assert_eq!(children.len(), 3);
    let mut starts = Vec::new();
    let mut completions = Vec::new();
    for record in world.records() {
        match &record.event {
            Event::ResponseStarted(started) if children.contains(&started.conversation) => {
                assert!(started.prompt.is_some(), "each hand-off has its own owned new part");
                starts.push((started.conversation, started.response, record.t_ms));
            }
            Event::ResponseCompleted(completed) if children.contains(&completed.conversation) => {
                let content = completed.completion.as_ref().expect("each actual child completion has content");
                assert!(
                    content
                        .blocks
                        .iter()
                        .any(|block| matches!(block, Block::Text { text } if text.as_ref() == b"actual child answer"))
                );
                completions.push((completed.conversation, completed.response));
            }
            _ => {}
        }
    }
    assert_eq!(starts.len(), 3);
    assert_eq!(completions.len(), 3);
    assert!(starts.iter().all(|start| start.2 == starts[0].2), "one batch's request hand-offs share their pass clock");
    for (conversation, response, _) in starts {
        assert_eq!(completions.iter().filter(|pair| **pair == (conversation, response)).count(), 1);
    }
    world.assert_agent_clean();
}

#[test]
fn native_provider_failure_keeps_status_and_capture_selected_detail_in_every_terminal() {
    use smith_agent_service::sinks::Capture;
    for capture in [Capture::None, Capture::Calls, Capture::Everything] {
        let mut world = World::new(71, &charter());
        world.initial_message(907, b"task", b"ALPHA");
        world.observe_facts(TraceCase::ProviderRejected, 4);
        world.capture(capture);
        world.check_memory();
        assert!(world.settle());
        world.assert_event_conformance();
        let mut response = None;
        let mut conversation = None;
        let mut run = None;
        let mut started = None;
        for record in world.records() {
            match &record.event {
                Event::ResponseStarted(value) => started = Some(value),
                Event::ResponseCompleted(value) => response = value.failure.as_ref(),
                Event::ConversationClosed(value) => conversation = value.failure.as_ref(),
                Event::RunCompleted(value) => run = value.failure.as_ref(),
                Event::Budget(_)
                | Event::SessionStarted(_)
                | Event::SessionEnded(_)
                | Event::RunStarted(_)
                | Event::ConversationOpened(_)
                | Event::TextDelta(_)
                | Event::ToolStarted(_)
                | Event::JobEnded(_)
                | Event::ToolCompleted(_)
                | Event::CheckStarted(_)
                | Event::CheckCompleted(_)
                | Event::MessageReceived(_)
                | Event::MessageRead(_)
                | Event::MessageUnread(_)
                | Event::Notice(_) => {}
            }
        }
        let failure = response.expect("actual provider failure");
        assert_eq!(failure.class, smith_events::FailureClass::ContextTooLong);
        assert_eq!(failure.evidence, smith_events::Evidence::Response);
        assert_eq!(failure.status, Some(400));
        let mut window = failure.clone();
        window.class = smith_events::FailureClass::Limit;
        window.limit = Some(smith_events::FailureLimit { which: b"window".as_slice().into(), bound: 1_048_576 });
        assert_eq!(conversation, Some(&window));
        let run = run.expect("actual failed run");
        assert_eq!(run.class, smith_events::AnswerClass::Model);
        assert_eq!(run.completion.as_ref(), Some(&window));
        let model = &started.expect("actual request").model;
        assert_eq!(run.endpoint.as_deref(), Some(model.endpoint.as_ref()));
        assert_eq!(run.model.as_deref(), Some(model.model.as_ref()));
        assert_eq!(run.attempts, Some(1));
        match capture {
            Capture::None => assert!(failure.detail.is_none()),
            Capture::Calls | Capture::Everything => {
                let detail = failure.detail.as_ref().expect("clipped provider content");
                assert!(!detail.is_empty());
                assert!(detail.len() <= 512);
                assert!(std::str::from_utf8(detail).is_ok());
            }
        }
        world.assert_agent_clean();
    }
}

#[test]
fn no_grant_value_reaches_a_turn_fact_or_trace() {
    use smith_agent_process_world::{PRIVATE_GRANT, bearer, key};
    let charter = charter();
    let secret = PRIVATE_GRANT;
    let rows = [(bearer(secret), (true, false)), (key(secret), (true, false))];
    for (index, (value, expected)) in rows.into_iter().enumerate() {
        let mut world = World::new(39, &charter);
        world.private_grant();
        world.anthropic();
        world.granting(value);
        world.observe_facts(TraceCase::ChildAnswered, 4);
        let settled = world.settle();
        let leaked = world.observed().iter().any(|frame| frame.body.windows(secret.len()).any(|bytes| bytes == secret))
            || world.trace_bytes().windows(secret.len()).any(|bytes| bytes == secret);
        assert_eq!((settled, leaked), expected, "row {index}");
        world.assert_agent_clean();
    }
}

#[test]
fn a_runs_model_names_its_account_and_backend() {
    let mut world = World::new(37, &charter());
    world.observe_facts(TraceCase::ChildAnswered, 4);
    let settled = world.settle();
    let models = world
        .records()
        .iter()
        .filter_map(|record| match &record.event {
            Event::RunStarted(started) => Some((started.main.account, started.main.backend.clone())),
            Event::Budget(_)
            | Event::SessionStarted(_)
            | Event::SessionEnded(_)
            | Event::RunCompleted(_)
            | Event::ConversationOpened(_)
            | Event::ConversationClosed(_)
            | Event::ResponseStarted(_)
            | Event::ResponseCompleted(_)
            | Event::TextDelta(_)
            | Event::ToolStarted(_)
            | Event::ToolCompleted(_)
            | Event::JobEnded(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Notice(_) => None,
        })
        .collect::<Vec<_>>();
    assert_eq!((settled, models), (true, vec![(Some(0), smith_events::Backend::Direct)]));
    world.assert_agent_clean();
}
