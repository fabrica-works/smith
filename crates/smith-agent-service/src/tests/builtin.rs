use crate::calls::Book;
use crate::sinks::tests::{drain_ready, grant, limits};
use crate::sinks::{Capture, Delivery, Destination, Sinks};
use alloc::boxed::Box;
use skein_lib::{List, Time, Token};
use smith_domain::{llm, session, tools};
use smith_events::{Event, Source, Verdict};

#[test]
fn a_builtin_call_is_a_workspace_tool_event() {
    for capture in [Capture::None, Capture::Calls, Capture::Everything] {
        let profile = crate::profile::standard();
        let source = crate::profile::derive(&profile, &super::declarations(0)).unwrap().domain.session;
        let mut book = Book::new(&source, capture);
        let mut sinks = Sinks::new(&limits(1), capture, 7, crate::sinks::tests::DEADLINE);
        let destination = Token::new(999);
        sinks.add(Destination::Trace, destination, Delivery::Complete).unwrap();
        grant(&mut sinks, destination);
        let opener = Token::new(91);
        sinks.conversation(
            Time::ZERO,
            Time::ZERO,
            opener,
            smith_events::ConversationOpened {
                run: 77,
                conversation: 0,
                kind: smith_events::ConversationKind::Main,
                parent: None,
                call: None,
                thread: 0,
                model: smith_events::Model {
                    account: None,
                    backend: smith_events::Backend::Direct,
                    endpoint: b"endpoint".as_slice().into(),
                    model: b"model".as_slice().into(),
                    window_tokens: 4096,
                    output_tokens: 32,
                    effort: None,
                },
                requested: None,
                share: None,
                clamped: List::with_capacity(0),
            },
        );
        assert_eq!(drain_ready(&mut sinks, destination).len(), 1);
        let input = br#"{"file_path":"file"}"#;
        book.received(
            opener,
            &llm::Completion {
                content: Box::new([llm::Said::ToolCall {
                    id: b"read".as_slice().into(),
                    name: b"Read".as_slice().into(),
                    input: input.as_slice().into(),
                    call: llm::Decoded::Builtin { effect: tools::Effect::Read },
                    replay: None,
                }]),
                reasoning_dropped: Box::default(),
                stop: llm::Stop::ToolUse,
                usage: llm::Usage::ZERO,
            },
        );
        let metadata = session::ToolCall {
            owner: Token::new(43),
            id: b"read".as_slice().into(),
            name: b"Read".as_slice().into(),
            source: session::ToolSource::Workspace,
            effect: tools::Effect::Read,
            deadline: Time::from_nanos(103_000_000),
            input_bytes: u64::try_from(input.len()).unwrap(),
        };
        book.fact(
            Time::ZERO,
            &session::Fact {
                at: Time::from_nanos(3_000_000),
                kind: session::FactKind::BuiltinStarted { opener, block: 0 },
                call: Some(metadata.clone()),
                response: None,
            },
            &mut sinks,
        );
        book.fact(
            Time::ZERO,
            &session::Fact {
                at: Time::from_nanos(14_000_000),
                kind: session::FactKind::BuiltinAnswered { opener, bytes: 17, error: false },
                call: Some(metadata),
                response: None,
            },
            &mut sinks,
        );
        let records = drain_ready(&mut sinks, destination);
        let [started, completed] = records.as_slice() else {
            panic!("one start and terminal: {records:?}");
        };
        let Event::ToolStarted(started) = &started.event else {
            panic!("tool start");
        };
        assert_eq!((started.run, started.conversation, started.source.clone()), (77, 0, Source::Workspace));
        assert_eq!(started.call.as_ref(), b"read");
        assert_eq!(started.tool.as_ref(), b"Read");
        assert_eq!(started.input_bytes, u64::try_from(input.len()).unwrap());
        assert_eq!(started.input.as_deref(), if capture == Capture::None { None } else { Some(input.as_slice()) });
        let Event::ToolCompleted(completed) = &completed.event else {
            panic!("tool terminal");
        };
        assert_eq!((completed.verdict.clone(), completed.bytes, completed.duration_ms), (Verdict::Result, 17, 11));
        assert_eq!(completed.call, started.call);
        assert_eq!(completed.tool, started.tool);
    }
}
