//! Production rendering of typed workspace outcomes in live and saved prompts.

use smith_domain::{llm, tools};
use smith_protocol_llm_world::adapter::{self as adapter, Limits};

fn limits() -> Limits {
    Limits {
        client: skein_llm_world::limits(),
        tool_bytes: 32768,
        history_bytes: u64::from(skein_llm_world::limits().request),
        rendered_result: skein_llm_world::limits().strings,
        stated: smith_protocol_llm_world::adapter::stated(),
    }
}

#[test]
fn a_refused_write_names_its_payload_bound_and_the_piece_then_edits_route() {
    use smith_tools_world::{Settings, Step, World, fixture::Fixture};

    let fixture = Fixture::new();
    let authority = fixture.authority(tools::Grants { inspect: true, modify: true, shell: false });
    let mut settings = Settings::calm(42020);
    settings.tools.tool_payload = 40_960;
    settings.tools.file_bytes = 1_048_576;
    let maximum = adapter::render_worst_case(&settings.tools, 4096, 4096).expect("derived result bound");
    let mut world = World::new(settings, fixture.checkout);
    let session = world.session(
        skein_lib::Time::ZERO,
        authority,
        vec![Step::Calls(vec![tools::Call::Write {
            path: smith_tools_world::translate::path(b"payload.txt"),
            content: vec![b'x'; 50_008].into(),
        }])],
    );
    world.run(1000);
    assert_eq!(world.stats().ops, 0, "oversized content never reaches file IO");
    let answers = world.answers(session);
    assert_eq!(answers.len(), 1, "the refused call has one terminal");
    let (text, error) = adapter::render_outcome(answers[0], &tools::Namespace::One, maximum).expect("live refusal");
    let text = String::from_utf8(text.into_vec()).expect("rendered UTF-8");
    assert!(error);
    assert!(text.contains("write content"), "what was too large: {text}");
    assert!(text.contains("50008 bytes"), "actual size: {text}");
    assert!(text.contains("bound 40960 bytes"), "declared bound: {text}");
    assert!(text.contains("first part within the bound"), "bounded first piece: {text}");
    assert!(text.contains("extend it with edits"), "extend the file: {text}");
    assert!(text.contains("as many edits in one call as fit"), "batch the edits: {text}");
    assert!(!text.contains("omitted"), "the full refusal fits its derived result bound");
}

#[test]
fn workspace_successes_render_the_retained_content_and_counts() {
    let cases = [
        (
            tools::Outcome::Read {
                content: b"alpha\nbeta\n".as_slice().into(),
                first: 3,
                lines: 2,
                total: 9,
                cut: tools::Cut::Whole,
                next: if 2 + 2 < 9 { tools::Next::Line(2 + 2 + 1) } else { tools::Next::End },
                window: tools::BytesClamp::Within,
            },
            b"3: alpha\n4: beta\n(2 of 9 lines; skipped 2) [next line 5]".as_slice(),
        ),
        (
            tools::Outcome::Listed {
                entries: Box::new([tools::Entry { name: b"src".as_slice().into(), kind: tools::Kind::Directory }]),
                more: 3,
            },
            b"src\tdirectory\n... 3 more entries",
        ),
        (
            tools::Outcome::Found {
                hits: Box::new([tools::Hit {
                    path: b"a.rs".as_slice().into(),
                    line: 5,
                    text: b"found".as_slice().into(),
                }]),
                more: 2,
                timed_out: false,
            },
            b"a.rs:5: found\n... 2 more matches\n",
        ),
        (tools::Outcome::Written { created: true }, b"file created"),
        (tools::Outcome::Edited { replaced: 2 }, b"replaced 2 occurrence(s)"),
        (
            tools::Outcome::Exited {
                exit: tools::Exit::Code { code: 0 },
                head: b"first".as_slice().into(),
                tail: b"last".as_slice().into(),
                dropped: 42,
                timeout: tools::TimeoutClamp::Within,
                output: tools::BytesClamp::Within,
            },
            b"exit code 0\nfirst\n... 42 bytes dropped ...\nlast",
        ),
    ];
    for (outcome, expected) in cases {
        let (text, error) = adapter::render_outcome(&outcome, &tools::Namespace::One, 4096).expect("bounded rendering");
        assert_eq!(text.as_ref(), expected);
        assert!(!error);
    }
}

#[test]
fn workspace_failures_and_invalid_bytes_stay_visible_and_bounded() {
    let failures = [
        tools::Outcome::NotGranted,
        tools::Outcome::Outside { path: tools::Resolved::Working { path: b"gone.rs".as_slice().into() } },
        tools::Outcome::ReadOnly { path: tools::Resolved::Working { path: b"gone.rs".as_slice().into() } },
        tools::Outcome::TooLong,
        tools::Outcome::NotFound { path: tools::Resolved::Working { path: b"gone.rs".as_slice().into() } },
        tools::Outcome::NotFile,
        tools::Outcome::Linked,
        tools::Outcome::Protected,
        tools::Outcome::NotDirectory { path: tools::Resolved::Working { path: b"gone.rs".as_slice().into() } },
        tools::Outcome::TooLarge { what: tools::SizePart::File, size: 10_000, bound: 4096 },
        tools::Outcome::NotRead,
        tools::Outcome::Stale,
        tools::Outcome::EditFailed { edit: 2, why: tools::Mismatch::NoMatch },
        tools::Outcome::EditFailed { edit: 2, why: tools::Mismatch::Ambiguous { count: 2, lines: Box::new([3, 7]) } },
        tools::Outcome::EditFailed { edit: 2, why: tools::Mismatch::Unchanged },
        tools::Outcome::Failed { fault: tools::Fault::Denied },
        tools::Outcome::Failed { fault: tools::Fault::NoSpace },
        tools::Outcome::Failed { fault: tools::Fault::Other },
        tools::Outcome::TimedOut,
        tools::Outcome::Cancelled,
        tools::Outcome::Busy,
        tools::Outcome::NulByte,
        tools::Outcome::Exited {
            exit: tools::Exit::Code { code: 3 },
            head: b"failure\xff".as_slice().into(),
            tail: Box::new([]),
            dropped: 0,
            timeout: tools::TimeoutClamp::Within,
            output: tools::BytesClamp::Within,
        },
    ];
    for outcome in failures {
        let (text, error) =
            adapter::render_outcome(&outcome, &tools::Namespace::One, 4096).expect("bounded failure rendering");
        assert!(error, "{outcome:?}");
        assert!(!text.is_empty());
        assert!(text.iter().all(u8::is_ascii), "{text:?}");
    }
}

#[test]
fn the_same_typed_result_renders_after_a_saved_turn_is_replayed() {
    let outcome = tools::Outcome::Read {
        content: "saved – 界\n".as_bytes().into(),
        first: 5,
        lines: 1,
        total: 6,
        cut: tools::Cut::Whole,
        next: tools::Next::Line(6),
        window: tools::BytesClamp::Clamped { asked: 99, ceiling: 32 },
    };
    let (live_text, live_error) =
        adapter::render_outcome(&outcome, &tools::Namespace::One, 4096).expect("live feedback");
    assert!(live_text.ends_with(b"[next line 6] [window clamped: 99 bytes to 32 bytes]"));
    let history = llm::Prompt {
        choice: smith_domain::llm::ToolChoice::Auto,
        namespace: smith_domain::tools::Namespace::One,
        affinity: smith_domain::llm::Affinity { key: [0; 16], thread: 0 },
        asker: llm::Asker::Main,
        endpoint: llm::Endpoint(1),
        model: b"model".as_slice().into(),
        system: Box::new([]),
        tools: tools::Grants { inspect: false, modify: false, shell: false },
        served: Box::new([]),
        messages: Box::new([llm::Message {
            role: llm::Role::User,
            content: Box::new([llm::Block::ToolResult {
                id: b"saved-id".as_slice().into(),
                result: llm::Returned::Owned { outcome },
            }]),
        }]),
        max_tokens: 32,
    };
    let translated = adapter::prompt(history, &[], &limits()).expect("saved typed result translated");
    let [message] = translated.messages.as_ref() else { panic!("one saved message") };
    let [skein_llm::Block::ToolResult { id, text, is_error }] = message.content.as_ref() else {
        panic!("one saved result");
    };
    assert_eq!(id.as_ref(), b"saved-id");
    assert_eq!(text, &live_text);
    assert_eq!(*is_error, live_error);
}

#[test]
fn written_text_and_mechanical_feedback_keep_their_error_flags() {
    let history = llm::Prompt {
        choice: smith_domain::llm::ToolChoice::Auto,
        namespace: smith_domain::tools::Namespace::One,
        affinity: smith_domain::llm::Affinity { key: [0; 16], thread: 0 },
        asker: llm::Asker::Main,
        endpoint: llm::Endpoint(1),
        model: b"model".as_slice().into(),
        system: Box::new([]),
        tools: tools::Grants { inspect: false, modify: false, shell: false },
        served: Box::new([]),
        messages: Box::new([llm::Message {
            role: llm::Role::User,
            content: Box::new([
                llm::Block::ToolResult {
                    id: b"text".as_slice().into(),
                    result: llm::Returned::Text { text: b"domain words".as_slice().into(), error: false, replay: None },
                },
                llm::Block::ToolResult {
                    id: b"invalid".as_slice().into(),
                    result: llm::Returned::Invalid {
                        problem: llm::Problem::Missing { field: b"path".as_slice().into() },
                    },
                },
                llm::Block::ToolResult {
                    id: b"not-run".as_slice().into(),
                    result: llm::Returned::NotRun { why: llm::NotRun::Stopped },
                },
                llm::Block::ToolResult { id: b"withdrawn".as_slice().into(), result: llm::Returned::Withdrawn },
            ]),
        }]),
        max_tokens: 32,
    };
    let translated = adapter::prompt(history, &[], &limits()).expect("bounded feedback translated");
    let [message] = translated.messages.as_ref() else { panic!("one message") };
    let [first, second, third, fourth] = message.content.as_ref() else { panic!("four result blocks") };
    let skein_llm::Block::ToolResult { text, is_error, .. } = first else { panic!("text result") };
    assert_eq!(text.as_ref(), b"domain words");
    assert!(!is_error);
    for (block, expected) in
        [(second, b"missing field \"path\"".as_slice()), (third, b"not run"), (fourth, b"withdrawn")]
    {
        let skein_llm::Block::ToolResult { text, is_error, .. } = block else { panic!("error result") };
        assert_eq!(text.as_ref(), expected);
        assert!(*is_error);
    }
}

#[test]
fn oversized_write_and_edit_replay_the_same_concrete_piecewise_advice() {
    for name in [b"write".as_slice(), b"edit"] {
        let history = llm::Prompt {
            choice: smith_domain::llm::ToolChoice::Auto,
            namespace: tools::Namespace::One,
            affinity: llm::Affinity { key: [0; 16], thread: 0 },
            asker: llm::Asker::Main,
            endpoint: llm::Endpoint(1),
            model: b"model".as_slice().into(),
            system: Box::new([]),
            tools: tools::Grants { inspect: false, modify: true, shell: false },
            served: Box::new([]),
            max_tokens: 32,
            messages: Box::new([
                llm::Message {
                    role: llm::Role::Assistant,
                    content: Box::new([llm::Block::ToolCall {
                        id: b"large".as_slice().into(),
                        name: name.into(),
                        input: b"{}".as_slice().into(),
                        replay: None,
                    }]),
                },
                llm::Message {
                    role: llm::Role::User,
                    content: Box::new([llm::Block::ToolResult {
                        id: b"large".as_slice().into(),
                        result: llm::Returned::Invalid { problem: llm::Problem::Oversize { bytes: 1024, bound: 256 } },
                    }]),
                },
            ]),
        };
        let bounds = limits();
        let offered = adapter::schemas(&history, &bounds.stated);
        let translated = adapter::prompt(history, &offered, &bounds).expect("corrective prompt");
        let [skein_llm::Block::ToolCall { name: actual, arguments, .. }] = translated.messages[0].content.as_ref()
        else {
            panic!("replayable call")
        };
        assert_eq!(actual.as_ref(), name);
        assert_eq!(arguments.as_ref(), b"{}");
        let [skein_llm::Block::ToolResult { id, text, is_error }] = translated.messages[1].content.as_ref() else {
            panic!("corrective result")
        };
        assert_eq!(id.as_ref(), b"large");
        assert_eq!(text.as_ref(), b"input too large: 1024 bytes, bound 256 bytes; make a smaller write or an edit. For a larger file, write its first part within the bound, then extend it with edits; put as many edits in one call as fit the bound");
        assert!(*is_error);
    }
}

#[test]
fn command_clamps_are_rendered_without_losing_the_exit_or_omitted_output() {
    let outcome = tools::Outcome::Exited {
        exit: tools::Exit::Code { code: 1 },
        head: b"first".as_slice().into(),
        tail: b"last".as_slice().into(),
        dropped: 100,
        timeout: tools::TimeoutClamp::Clamped {
            asked: skein_lib::Duration::from_secs(9),
            ceiling: skein_lib::Duration::from_secs(5),
        },
        output: tools::BytesClamp::Clamped { asked: 99, ceiling: 12 },
    };
    let (text, error) = adapter::render_outcome(&outcome, &tools::Namespace::One, 4096).expect("command feedback");
    assert_eq!(text.as_ref(), b"exit code 1\nfirst\n... 100 bytes dropped ...\nlast\n[timeout clamped: 9000 ms to 5000 ms]\n[output clamped: 99 bytes to 12 bytes]");
    assert!(error);
}
