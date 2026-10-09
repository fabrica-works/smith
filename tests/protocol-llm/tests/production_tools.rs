//! Smith's production schema and decoder, exercised independently of the
//! scripted provider's finite fixture language.

use smith_domain::{llm, run, tools};
use smith_protocol_llm_world::adapter::{self as adapter, Limits, ToolKind};

fn limits() -> Limits {
    Limits {
        client: skein_llm_world::limits(),
        tool_bytes: 32768,
        rendered_result: skein_llm_world::limits().dialect.string_bytes,
        stated: smith_protocol_llm_world::adapter::stated(),
    }
}

fn grants() -> tools::Grants {
    tools::Grants { inspect: true, modify: true, shell: true }
}

fn prompt() -> llm::Prompt {
    llm::Prompt {
        namespace: smith_domain::tools::Namespace::One,
        affinity: smith_domain::llm::Affinity { key: [0; 16], thread: 0 },
        asker: llm::Asker::Main,
        endpoint: llm::Endpoint(1),
        model: b"test".as_slice().into(),
        system: Box::new([]),
        tools: grants(),
        served: Box::new([llm::Served::Wait, llm::Served::SubAgent]),
        messages: Box::new([]),
        max_tokens: 16,
    }
}

#[test]
fn production_schemas_cover_every_offered_workspace_and_run_tool() {
    let schemas = adapter::schemas(&prompt(), &limits().stated);
    assert_eq!(schemas.len(), 8);
    for kind in [
        ToolKind::Owned(tools::Tool::Read),
        ToolKind::Owned(tools::Tool::List),
        ToolKind::Owned(tools::Tool::Search),
        ToolKind::Owned(tools::Tool::Write),
        ToolKind::Owned(tools::Tool::Edit),
        ToolKind::Owned(tools::Tool::Shell),
        ToolKind::Wait,
        ToolKind::SubAgent,
    ] {
        let schema = schemas.iter().find(|schema| schema.kind == kind).expect("offered schema");
        assert!(schema.description.len() > 20);
        let document = skein_llm::Json::from_bytes(&schema.schema, &limits().client.dialect)
            .expect("production schema is bounded valid JSON");
        assert_eq!(document.document().token(0).map(|record| record.kind), Some(skein_json::Kind::ObjectStart));
    }
}

#[test]
fn each_workspace_call_decodes_escaped_input_and_ignores_unknown_members() {
    let limits = limits();
    let cases = [
        (
            b"read".as_slice(),
            br#"{"path":"src/a.rs","first_line":3,"lines":3,"future":[1,{"x":true}]}"#.as_slice(),
            tools::Tool::Read,
        ),
        (b"list", br#"{"path":".","future":false}"#, tools::Tool::List),
        (b"search", br#"{"pattern":"a\\nb","glob":"*.rs"}"#, tools::Tool::Search),
        (b"write", br#"{"path":"a","content":"line\nnext"}"#, tools::Tool::Write),
        (b"edit", br#"{"path":"a","old":"before","new":"after","all":true}"#, tools::Tool::Edit),
        (b"shell", br#"{"command":"printf ok","timeout":5}"#, tools::Tool::Shell),
    ];
    for (name, input, expected) in cases {
        let decoded = adapter::decode(name, input, grants(), &[], llm::Asker::Main, &limits);
        match decoded {
            llm::Decoded::Owned { call } => {
                assert_eq!(tools::tool(&call), expected, "{}", String::from_utf8_lossy(name));
            }
            other @ (llm::Decoded::Served { .. } | llm::Decoded::Invalid { .. }) => {
                panic!("{}: {other:?}", String::from_utf8_lossy(name));
            }
        }
    }
}

#[test]
fn malformed_workspace_calls_have_field_named_typed_problems_then_correct() {
    let limits = limits();
    let cases = [
        (b"read".as_slice(), br#"{"path":12}"#.as_slice(), b"path".as_slice()),
        (b"list", br"{}", b"path"),
        (b"search", br#"{"pattern":false}"#, b"pattern"),
        (b"write", br#"{"path":"a"}"#, b"content"),
        (b"edit", br#"{"path":"a","old":"x"}"#, b"new"),
        (b"shell", br#"{"command":"x","timeout":-1}"#, b"timeout"),
    ];
    for (name, input, field) in cases {
        let decoded = adapter::decode(name, input, grants(), &[], llm::Asker::Main, &limits);
        match decoded {
            llm::Decoded::Invalid {
                problem:
                    llm::Problem::Missing { field: found }
                    | llm::Problem::WrongType { field: found }
                    | llm::Problem::BadValue { field: found },
            } => assert_eq!(found.as_ref(), field),
            other @ (llm::Decoded::Owned { .. } | llm::Decoded::Served { .. } | llm::Decoded::Invalid { .. }) => {
                panic!("{}: {other:?}", String::from_utf8_lossy(name));
            }
        }
    }
    assert!(matches!(
        adapter::decode(b"read", b"{bad}", grants(), &[], llm::Asker::Main, &limits),
        llm::Decoded::Invalid { problem: llm::Problem::NotAnObject }
    ));
    assert!(matches!(
        adapter::decode(b"unknown", b"{}", grants(), &[], llm::Asker::Main, &limits),
        llm::Decoded::Invalid { problem: llm::Problem::UnknownTool }
    ));
}

#[test]
fn wait_and_sub_agent_decode_into_run_asks() {
    let limits = limits();
    let served = [llm::Served::Wait, llm::Served::SubAgent];
    assert!(matches!(
        adapter::decode(b"wait", b"{}", grants(), &served, llm::Asker::Main, &limits),
        llm::Decoded::Served { ask: run::Ask::Wait }
    ));
    let decoded = adapter::decode(
        b"sub_agent",
        br#"{"brief":"look","tools":["inspect","shell"],"llm":"small"}"#,
        grants(),
        &served,
        llm::Asker::Main,
        &limits,
    );
    match decoded {
        llm::Decoded::Served { ask: run::Ask::SubAgent { brief, families, llm, share } } => {
            assert_eq!(share, run::Share::UNASKED);
            assert_eq!(brief.as_ref(), b"look");
            assert!(families.tools.inspect && families.tools.shell && !families.tools.modify && !families.agents);
            assert_eq!(llm.as_deref(), Some(b"small".as_slice()));
        }
        other @ (llm::Decoded::Owned { .. } | llm::Decoded::Served { .. } | llm::Decoded::Invalid { .. }) => {
            panic!("{other:?}");
        }
    }
}

#[test]
fn optional_child_turns_and_seconds_decode_without_a_schema_maximum() {
    use skein_lib::Duration;

    let limits = limits();
    for (input, turns, time) in [
        (br#"{"brief":"look","tools":[],"max_turns":2,"max_seconds":3}"#.as_slice(), Some(2), Some(3)),
        (br#"{"brief":"look","tools":[],"max_turns":4294967295}"#, Some(u32::MAX), None),
        (br#"{"brief":"look","tools":[],"max_seconds":4294967295}"#, None, Some(u32::MAX)),
        (br#"{"brief":"look","tools":[]}"#, None, None),
    ] {
        let decoded =
            adapter::decode(b"sub_agent", input, grants(), &[llm::Served::SubAgent], llm::Asker::Main, &limits);
        let llm::Decoded::Served { ask: run::Ask::SubAgent { share, .. } } = decoded else {
            panic!("expected a child ask: {decoded:?}");
        };
        assert_eq!(share, run::Share { turns, time: time.map(|seconds| Duration::from_secs(u64::from(seconds))) });
    }
    for field in ["max_turns", "max_seconds"] {
        for value in ["0", "-1", "1.5", "1e2", "4294967296"] {
            let input = format!("{{\"brief\":\"look\",\"tools\":[],\"{field}\":{value}}}");
            assert_eq!(
                adapter::decode(
                    b"sub_agent",
                    input.as_bytes(),
                    grants(),
                    &[llm::Served::SubAgent],
                    llm::Asker::Main,
                    &limits
                ),
                llm::Decoded::Invalid { problem: llm::Problem::BadValue { field: field.as_bytes().into() } },
                "{input}",
            );
        }
    }
    let schemas = adapter::schemas(&prompt(), &limits.stated);
    let schema = schemas.iter().find(|schema| schema.kind == ToolKind::SubAgent).expect("main tool");
    let document = skein_llm::Json::from_bytes(&schema.schema, &limits.client.dialect).expect("bounded schema");
    let keys: Vec<_> = document
        .as_tokens()
        .iter()
        .filter_map(|token| if let skein_json::Token::Key(key) = token { Some(key.as_ref()) } else { None })
        .collect();
    assert!(keys.contains(&b"max_turns".as_slice()) && keys.contains(&b"max_seconds".as_slice()));
    assert!(!keys.contains(&b"maximum".as_slice()));
    assert!(schema.description.windows(b"finishing reserve".len()).any(|part| part == b"finishing reserve"));
}

#[test]
fn nesting_is_refused_for_any_agents_field_and_any_child_sub_agent_call() {
    let limits = limits();
    for input in [
        br#"{"brief":"look","tools":["inspect"],"agents":true}"#.as_slice(),
        br#"{"agents":true}"#,
        br#"{"brief":null,"agents":false}"#,
        br#"{"brief":"look","tools":[],"agents":false}"#,
        br#"{"brief":"look","tools":[],"agents":null}"#,
        br#"{"brief":"look","tools":[],"agents":{"future":[]}}"#,
    ] {
        assert_eq!(
            adapter::decode(b"sub_agent", input, grants(), &[llm::Served::SubAgent], llm::Asker::Main, &limits),
            llm::Decoded::Invalid { problem: llm::Problem::Nesting },
        );
    }
    for input in [b"{}".as_slice(), b"{invalid", br#"{"brief":"look","tools":[]}"#] {
        assert_eq!(
            adapter::decode(b"sub_agent", input, grants(), &[], llm::Asker::Child, &limits),
            llm::Decoded::Invalid { problem: llm::Problem::Nesting },
        );
    }
    assert_eq!(
        adapter::decode(b"sub_agent", b"{}", grants(), &[], llm::Asker::Main, &limits),
        llm::Decoded::Invalid { problem: llm::Problem::UnknownTool },
    );
    let schemas = adapter::schemas(&prompt(), &limits.stated);
    let child = schemas.iter().find(|schema| schema.kind == ToolKind::SubAgent).expect("main tool");
    let document = skein_llm::Json::from_bytes(&child.schema, &limits.client.dialect).expect("valid schema");
    let document = document.document();
    assert!(!(0..document.len()).any(|index| {
        let token = document.token(index).expect("within document");
        token.kind == skein_json::Kind::Key && document.text(token) == Some(b"agents".as_slice())
    }));
}
