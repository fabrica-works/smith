//! Profile construction stays with the service whose limits it selects.

#![expect(
    clippy::disallowed_types,
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    reason = "profile regressions use ordinary Rust to construct and collect bounded provider fixtures"
)]

use alloc::boxed::Box;
use skein_http::{self as http, Header};
use skein_lib::List;
use skein_llm::openai;
use smith_protocol_llm as llm;

fn standard_limits() -> Result<crate::Limits, crate::profile::ProfileError> {
    let mut profile = crate::profile::standard();
    profile.declared.memory = Some(u64::MAX);
    crate::profile::derive(&profile, &crate::profile::Configuration { environment_bytes: 0, endpoints: Box::new([]) })
}

#[test]
fn standard_profile_builds_a_complete_bounded_service() {
    let limits = crate::profile::derive(
        &{
            let mut profile = crate::profile::standard();
            profile.declared.memory = Some(u64::MAX);
            profile
        },
        &crate::profile::Configuration { environment_bytes: 0, endpoints: Box::new([]) },
    )
    .expect("standard profile");
    let configuration = crate::Config {
        limits,
        domain: smith_domain::Config { endpoints: Box::new([]), models: Box::new([]) },
        channel_endpoints: smith_protocol_channel::Endpoints::new(List::with_capacity(0)),
        llm_endpoints: smith_protocol_llm::Endpoints::new(
            Box::new([]),
            crate::profile::ENDPOINTS,
            crate::profile::ACCOUNTS,
        )
        .expect("empty endpoints"),
        environment: Box::new([]),
        stream_mode: skein_channel::StreamMode::Two,
        capture_prompts: false,
    };
    let bound = crate::worst_case(&configuration.limits, &configuration.llm_endpoints).expect("checked bound");
    assert!(bound > 0);
    let service = crate::Service::new(configuration, 1).expect("service");
    assert!(crate::done(&service).is_none());
}

#[test]
fn random_tool_render_caps_derive_a_machine_that_holds_them() {
    let mut rng = skein_lib::Rng::new(941);
    for _ in 0_u32..256 {
        let mut profile = crate::profile::standard();
        profile.declared.memory = Some(u64::MAX);
        profile.declared.tool_payload = u32::try_from(rng.next_u64() % 65536 + 1).expect("bounded payload");
        profile.declared.read_window =
            u32::try_from(rng.next_u64() % u64::from(profile.declared.tool_payload) + 1).expect("bounded read");
        profile.declared.shell_head = u32::try_from(rng.next_u64() % 4096 + 1).expect("bounded head");
        profile.declared.shell_tail = u32::try_from(rng.next_u64() % 4096 + 1).expect("bounded tail");
        profile.declared.search_hits = u32::try_from(rng.next_u64() % 32 + 1).expect("bounded hits");
        profile.declared.search_bytes = u32::try_from(rng.next_u64() % 4096 + 1).expect("bounded search");
        profile.declared.list_entries = u32::try_from(rng.next_u64() % 64 + 1).expect("bounded listing");
        let configuration = crate::profile::Configuration { environment_bytes: 37, endpoints: Box::new([]) };
        let limits = crate::profile::derive(&profile, &configuration).expect("bounded declarations derive");
        let tools = limits.domain.session.tools;
        let machine = limits.machine;
        assert!(machine.output_bytes >= tools.shell_head);
        assert!(machine.output_bytes >= tools.shell_tail);
        assert!(machine.file_bytes >= tools.file_bytes);
        assert!(machine.entries >= tools.list_entries);
        assert!(machine.entry_bytes >= tools.list_bytes);
        assert!(machine.search_hits >= tools.search_hits);
        assert!(machine.search_bytes >= tools.search_bytes);
        assert!(machine.path_bytes >= tools.path_bytes);
        assert_eq!(machine.env_bytes, configuration.environment_bytes);
    }
    let mut profile = crate::profile::standard();
    profile.declared.memory = Some(u64::MAX);
    profile.declared.search_bytes = u32::MAX;
    assert_eq!(
        crate::profile::derive(
            &profile,
            &crate::profile::Configuration { environment_bytes: 0, endpoints: Box::new([]) }
        )
        .err(),
        Some(crate::profile::ProfileError::Receiving)
    );
}

#[test]
fn standard_profile_accepts_full_workspace_read_results() {
    let limits = standard_limits().expect("standard limits");
    let read_bytes = limits.domain.session.tools.read_bytes;
    let outcome = smith_domain::tools::Outcome::Read {
        content: vec![0xff; usize::try_from(read_bytes).expect("read bytes fit usize")].into_boxed_slice(),
        skipped: 0,
        lines: 1,
        total: 1,
        cut: false,
    };
    let (rendered, error) = llm::render_outcome(&outcome, limits.llm.adapter.client.dialect.string_bytes)
        .expect("full read fits after escaping and adding line metadata");
    assert!(!error);
    assert!(rendered.len() > 16_384, "escaped bytes and metadata exceed the old string bound");
}

#[test]
fn standard_profile_accepts_conversation_history_with_workspace_results() {
    let limits = standard_limits().expect("standard limits").llm.adapter.client.dialect;
    let mut input = Vec::new();
    for index in 0_u32..12 {
        let call_id = format!("read_{index}").into_bytes().into_boxed_slice();
        input.push(openai::Input::FunctionCall {
            call_id: call_id.clone(),
            item_id: None,
            name: b"read".as_slice().into(),
            arguments: br#"{"path":"README.md"}"#.as_slice().into(),
        });
        input.push(openai::Input::FunctionOutput { call_id, output: vec![b'x'; 1024].into_boxed_slice() });
    }
    let request = openai::Request {
        model: b"provider-model".as_slice().into(),
        instructions: b"Read the workspace and report.".as_slice().into(),
        tools: Box::new([]),
        input: input.into_boxed_slice(),
        effort: None,
        prompt_cache_key: None,
        choice: skein_llm::ToolChoice::Auto,
    };
    let encoded = openai::encode_request(&request, &limits).expect("workspace results fit the request");
    assert!(encoded.len() > 8192, "workspace history exceeds the old request bound");
}

#[test]
fn standard_profile_accepts_provider_heads_with_many_metadata_fields() {
    use skein_lib::stream::{Down, Up};
    use skein_lib::{Env, Queue, Time, Wall};

    let limits = standard_limits().expect("standard limits").llm.adapter.client.http;
    let env = Env { now: Time::ZERO, wall: Wall::EPOCH, limits };
    let mut client = http::client::Client::new(&limits);
    let mut above = Queue::with_capacity(4);
    let mut below = Queue::with_capacity(4);
    http::client::down(
        &mut client,
        &env,
        http::client::Request::Call(http::client::Call {
            method: http::Method::Get,
            target: b"/".as_slice().into(),
            headers: Box::new([Header { name: b"Host".as_slice().into(), value: b"provider.test".as_slice().into() }]),
            body: http::client::Body::None,
            close: false,
        }),
        &mut above,
        &mut below,
    );
    let Some(Down::Demand { .. }) = below.pop() else { panic!("the client demands response bytes") };
    http::client::up(&mut client, &env, Up::Room, &mut above, &mut below);
    let Some(Down::Send(_)) = below.pop() else { panic!("the client sends its request") };

    let mut lines = vec![b"HTTP/1.1 200 OK\r\n".to_vec()];
    for index in 0_u32..40 {
        lines.push(format!("X-Provider-Metadata-{index}: value\r\n").into_bytes());
    }
    lines.push(b"Content-Type: text/event-stream\r\n".to_vec());
    lines.push(b"Content-Length: 0\r\n".to_vec());
    lines.push(b"\r\n".to_vec());
    for line in lines {
        let Some(Down::Demand { .. }) = below.pop() else { panic!("each head line answers a read demand") };
        http::client::up(&mut client, &env, Up::Bytes(line.into()), &mut above, &mut below);
    }
    let Some(http::client::Event::Response(response)) = above.pop() else {
        panic!("the standard profile accepts the provider's response head");
    };
    assert_eq!(response.status, 200);
    assert_eq!(response.headers.len(), 42);
}

#[test]
fn standard_profile_accepts_provider_events_that_echo_tool_schemas() {
    use skein_lib::stream::{Down, Up};
    use skein_lib::{Env, Queue, Time, Wall};

    let limits = standard_limits().expect("standard limits").llm.adapter.client;
    let tools: Vec<_> = (0_u32..32)
        .map(|index| {
            serde_json::json!({
                "type": "function",
                "name": format!("tool_{index}"),
                "description": "x".repeat(300),
                "parameters": { "type": "object", "properties": {} }
            })
        })
        .collect();
    let payload = serde_json::to_vec(&serde_json::json!({
        "type": "response.created",
        "response": { "id": "response", "status": "in_progress", "tools": tools }
    }))
    .expect("provider event JSON");
    assert!(payload.len() > 8192, "echoed schemas exceed the old event and document bounds");
    let mut data = b"data: ".to_vec();
    data.extend_from_slice(&payload);
    data.push(b'\n');

    let env = Env { now: Time::ZERO, wall: Wall::EPOCH, limits: limits.sse };
    let mut reader = http::sse::Reader::new(&limits.sse);
    let mut above = Queue::with_capacity(http::sse::UP_MAX_OUT.above);
    let mut below = Queue::with_capacity(http::sse::UP_MAX_OUT.below);
    let mut received = Vec::new();
    let mut dispatched = false;
    http::sse::down(&mut reader, &env, http::sse::Request::Next, &mut above, &mut below);
    for line in [b"event: response.created\n".as_slice(), data.as_slice(), b"\n"] {
        for chunk in line.chunks(usize::try_from(limits.sse.chunk).expect("chunk fits usize")) {
            let Some(Down::Demand { .. }) = below.pop() else { panic!("each chunk answers a read demand") };
            http::sse::up(&mut reader, &env, Up::Bytes(chunk.into()), &mut above, &mut below);
            while let Some(event) = above.pop() {
                match event {
                    http::sse::Event::Opened | http::sse::Event::Data(Up::Bytes(_)) => {
                        if let http::sse::Event::Data(Up::Bytes(bytes)) = event {
                            received.extend_from_slice(&bytes);
                        }
                        http::sse::down(
                            &mut reader,
                            &env,
                            http::sse::Request::Data(Down::Demand { read: skein_lib::stream::Read::Fill(1), room: 0 }),
                            &mut above,
                            &mut below,
                        );
                    }
                    http::sse::Event::Data(Up::End) => {}
                    http::sse::Event::Dispatched(dispatch) => {
                        assert_eq!(dispatch.name.as_ref(), b"response.created");
                        dispatched = true;
                    }
                    other @ (http::sse::Event::Data(Up::Room | Up::Failed(_))
                    | http::sse::Event::Ended
                    | http::sse::Event::Failed(_)
                    | http::sse::Event::Closed) => panic!("the standard profile accepts the provider event: {other:?}"),
                }
            }
        }
    }
    assert!(dispatched);
    assert_eq!(received, payload);
    let json = openai::Json::from_bytes(&received, &limits.dialect).expect("provider event document");
    assert_eq!(openai::decode_event(&json, &limits.dialect), Ok(openai::Event::Created { echo: None }));
}
