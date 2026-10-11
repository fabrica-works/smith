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
#[expect(clippy::wildcard_enum_match_arm, reason = "select content from an independently checked complete stream")]
fn request_new_parts_and_completions_reconstruct_every_native_request() {
    let mut world = World::new(37, &charter());
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
                assert_eq!(query.max_tokens, 4096);
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
                        assert!(deltas.remove(&key).is_none());
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
