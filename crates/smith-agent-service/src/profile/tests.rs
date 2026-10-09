//! Named startup comparisons use independent declared values and exact refusals.

use alloc::boxed::Box;
use skein_lib::Duration;

use super::{
    Check, Configuration, Endpoint, Layer, LimitName, Model, OversizedReasoning, Per, Refusal, Value, standard,
    validate, words,
};

fn profile() -> super::Profile {
    let mut profile = standard();
    profile.declared.memory = Some(u64::MAX);
    profile.declared.tool_payload = 256;
    profile.declared.read_window = 256;
    profile.declared.calls_per_response = 2;
    profile
}

fn declarations() -> Configuration {
    Configuration {
        environment_bytes: 0,
        endpoints: Box::new([Endpoint {
            number: 7,
            provider: skein_llm::Provider::OpenAiCodex,
            option_bytes: 0,
            connect: Duration::from_secs(2),
            handshake: Duration::from_secs(3),
            models: Box::new([Model {
                name: Box::from(b"one".as_slice()),
                window: 65_536,
                output: 512,
                reasoning_item: 2048,
                oversized_reasoning: OversizedReasoning::Fail,
                head: Duration::from_secs(5),
                idle: Duration::from_secs(11),
            }]),
        }]),
    }
}

#[test]
fn read_past_payload_retains_both_names_values_and_units() {
    let mut profile = profile();
    profile.declared.read_window = 1025;
    profile.declared.tool_payload = 1024;
    let refusal = super::derive(&profile, &declarations()).expect_err("read exceeds payload");
    assert_eq!(refusal.check(), Some(Check::ReadInPayload));
    assert_eq!(
        words(&refusal).as_ref(),
        b"read-in-payload: declared.read_window=1025 bytes; declared.tool_payload=1024 bytes"
    );
}

#[test]
fn output_at_the_window_names_the_actual_endpoint_and_model() {
    let mut configuration = declarations();
    let model = configuration.endpoints.get_mut(0).expect("one endpoint").models.get_mut(0).expect("one model");
    model.window = 8192;
    model.output = 8192;
    let refusal = super::derive(&profile(), &configuration).expect_err("output must be below window");
    let per = Per::Model { endpoint: 7, model: 0 };
    assert_eq!(
        refusal,
        Refusal::Broken {
            check: Check::OutputInWindow,
            left: LimitName { layer: Layer::Declared, field: "output", per }.value(Value::Tokens(8192)),
            right: LimitName { layer: Layer::Declared, field: "window", per }.value(Value::Tokens(8192)),
        }
    );
    assert_eq!(
        words(&refusal).as_ref(),
        b"output-in-window: declared.output[endpoint=7,model=0]=8192 tokens; declared.window[endpoint=7,model=0]=8192 tokens"
    );
}

#[test]
fn zero_model_idle_is_a_named_deadline_refusal() {
    let mut configuration = declarations();
    configuration.endpoints.get_mut(0).expect("one endpoint").models.get_mut(0).expect("one model").idle =
        Duration::ZERO;
    let refusal = super::derive(&profile(), &configuration).expect_err("zero idle");
    assert_eq!(refusal.check(), Some(Check::Deadlines));
    assert_eq!(
        words(&refusal).as_ref(),
        b"deadlines: declared.idle[endpoint=7,model=0]=0 ns; policy.deadline_minimum=0 ns"
    );
}

#[test]
fn shell_default_cannot_pass_its_tool_deadline() {
    let mut profile = profile();
    profile.policy.shell_timeout = Duration::from_secs(11);
    profile.policy.tool_deadline = Duration::from_secs(10);
    let refusal = super::derive(&profile, &declarations()).expect_err("shell exceeds tool deadline");
    assert_eq!(
        words(&refusal).as_ref(),
        b"deadlines: policy.shell_timeout=11000000000 ns; policy.tool_deadline=10000000000 ns"
    );
}

#[test]
fn policy_fields_past_the_vocabulary_are_refused_before_deriving() {
    let mut profile = profile();
    profile.policy.fields = smith_charter::CEILINGS.run_result_fields.checked_add(1).expect("small ceiling");
    let refusal = super::derive(&profile, &declarations()).expect_err("field ceiling");
    assert_eq!(refusal.check(), Some(Check::CeilingsInVocabulary));
    assert_eq!(
        words(&refusal).as_ref(),
        b"ceilings-in-vocabulary: policy.fields=33 items; channel.text_rule_fields=32 items"
    );
}

#[test]
fn composed_machine_refuses_a_file_cap_below_the_tools() {
    let limits = super::derive(&profile(), &declarations()).expect("valid declarations");
    let mut machine = limits.machine;
    machine.file_bytes = limits.domain.session.tools.file_bytes.checked_sub(1).expect("positive payload");
    let refusal = validate::machine(&machine, &limits.domain.session.tools).expect_err("file cap");
    assert_eq!(refusal.check(), Some(Check::MachineHoldsTools));
    assert_eq!(
        words(&refusal).as_ref(),
        b"machine-holds-tools: tools.file_bytes=256 bytes; machine.file_bytes=255 bytes"
    );
}

#[test]
fn process_memory_refusal_retains_the_measured_and_declared_bytes() {
    let refusal = validate::memory(101, 100).expect_err("memory target");
    assert_eq!(refusal.check(), Some(Check::Memory));
    assert_eq!(words(&refusal).as_ref(), b"memory: service.worst_case=101 bytes; declared.memory=100 bytes");
    validate::memory(100, 100).expect("exact fit");
}

#[test]
fn a_larger_inbox_does_not_raise_the_rooms_offer_bound() {
    let mut profile = profile();
    let original = super::derive(&profile, &declarations()).expect("bounded room").domain.run;
    profile.policy.inbox = u32::MAX;
    let larger = super::derive(&profile, &declarations()).expect("bounded offer").domain.run;
    assert_eq!(larger.messages, u32::MAX);
    assert_eq!(larger.offer_messages, u32::MAX);
    assert_eq!(larger.offer_bytes, original.offer_bytes);
    assert_eq!(larger.message_bytes, original.message_bytes);
}

#[test]
fn native_connection_relationship_keeps_its_actual_values() {
    let refusal = Refusal::Connection(skein_llm_connection::EndpointError::HttpReadTlsRead { demand: 31, cap: 30 });
    assert_eq!(refusal.check(), None);
    assert_eq!(words(&refusal).as_ref(), b"http-read-tls-read: demand=31 bytes; cap=30 bytes");
}

#[test]
fn random_declarations_refuse_by_name_or_hold_the_checked_relationships() {
    let mut rng = skein_lib::Rng::new(19405);
    for _ in 0_u32..256 {
        let mut profile = profile();
        let mut configuration = declarations();
        profile.declared.tool_payload = u32::try_from(rng.next_u64() % 4096 + 1).expect("payload");
        profile.declared.read_window = u32::try_from(rng.next_u64() % 8192 + 1).expect("read");
        profile.declared.file_bytes = u32::try_from(rng.next_u64() % 8192 + 1).expect("whole file");
        let model = configuration.endpoints.get_mut(0).expect("one endpoint").models.get_mut(0).expect("one model");
        model.window = u32::try_from(rng.next_u64() % 8192 + 1).expect("window");
        model.output = u32::try_from(rng.next_u64() % 8192 + 1).expect("output");
        let output = model.output;
        let window = model.window;
        match super::derive(&profile, &configuration) {
            Ok(limits) => {
                assert!(profile.declared.read_window <= profile.declared.tool_payload, "default read fits payload");
                assert!(profile.declared.tool_payload <= profile.declared.file_bytes, "payload fits file");
                assert!(output < window, "output fits window");
                validate::machine(&limits.machine, &limits.domain.session.tools).expect("machine holds tools");
            }
            Err(Refusal::Llm { endpoint, violation: _ }) => assert_eq!(endpoint, 7, "native refusal retains its scope"),
            Err(refusal) => assert!(refusal.check().is_some(), "smith refusals retain their relationship"),
        }
    }
}

#[test]
fn a_tool_piece_must_fit_the_separately_declared_whole_file() {
    let mut profile = profile();
    profile.declared.read_window = 128;
    profile.declared.tool_payload = 256;
    profile.declared.file_bytes = 255;
    let refusal = super::derive(&profile, &declarations()).expect_err("payload past file");
    assert_eq!(refusal.check(), Some(Check::PayloadInFile));
    assert_eq!(
        words(&refusal).as_ref(),
        b"payload-in-file: declared.tool_payload=256 bytes; declared.file_bytes=255 bytes"
    );
    profile.declared.file_bytes = 256;
    validate::declarations(&profile, &declarations()).expect("payload at file ceiling");
    profile.declared.file_bytes = 1_u32 << 20_u32;
    let limits = super::derive(&profile, &declarations()).expect("independent file declaration");
    assert_eq!(limits.domain.session.tools.file_bytes, 1_u32 << 20_u32);
    assert_eq!(limits.domain.session.tools.tool_payload, 256);
    assert_eq!(limits.machine.file_bytes, 1_u32 << 20_u32);
}

#[test]
fn zero_child_wrap_up_is_a_named_deadline_refusal() {
    let mut selected = profile();
    selected.policy.wrap_up = Duration::ZERO;
    let refusal = super::derive(&selected, &declarations()).expect_err("zero child wrap-up");
    assert_eq!(refusal.check(), Some(Check::Deadlines));
    assert_eq!(words(&refusal).as_ref(), b"deadlines: policy.wrap_up=0 ns; policy.deadline_minimum=0 ns");
}

#[test]
fn window_first_refuses_each_history_dimension_by_its_name() {
    let profile = profile();
    let configuration = declarations();
    let original = super::derive(&profile, &configuration).expect("coherent tiny declarations").domain.session;
    for field in ["history_bytes", "history_messages", "history_blocks"] {
        let mut limits = original;
        match field {
            "history_bytes" => limits.history_bytes = 1,
            "history_messages" => limits.history_messages = 1,
            "history_blocks" => limits.history_blocks = 1,
            _ => unreachable!(),
        }
        let refusal = validate::window_first(&profile, &configuration, &limits).expect_err("too small before window");
        match refusal {
            Refusal::Broken { check: Check::WindowFirst, right, .. } => {
                assert_eq!(right.name.field, field);
                assert_eq!(right.name.per, Per::Model { endpoint: 7, model: 0 });
            }
            refusal @ (Refusal::Field { .. }
            | Refusal::Broken { .. }
            | Refusal::Derived { .. }
            | Refusal::Environment { .. }
            | Refusal::Connection(_)
            | Refusal::Llm { .. }) => panic!("unexpected refusal: {refusal:?}"),
        }
    }
}

#[test]
fn request_head_and_uncut_rendering_keep_the_actual_byte_sides() {
    validate::request_head(7, 100, 100).expect("head exactly fits");
    let refusal = validate::request_head(7, 101, 100).expect_err("head exceeds transport piece");
    assert_eq!(refusal.check(), Some(Check::RequestHead));
    assert!(words(&refusal).ends_with(b"llm.request_piece[endpoint=7]=100 bytes"));
    validate::render_in_result(100, 100).expect("whole result fits");
    let refusal = validate::render_in_result(101, 100).expect_err("cut cannot satisfy startup");
    assert_eq!(words(&refusal).as_ref(), b"render-in-result: render.uncut_result=101 bytes; render.result=100 bytes");
}

#[test]
fn transcript_blocks_hold_the_largest_completion_or_name_the_ceiling() {
    let profile = profile();
    let limits = super::derive(&profile, &declarations()).expect("coherent declarations");
    assert!(limits.channel.transcript.message_blocks >= limits.domain.session.completion_blocks);
    assert_eq!(limits.channel.transcript.turn_messages, 3);
    let refusal = validate::calls_in_transcript(65, 64).expect_err("one beyond the vocabulary");
    assert_eq!(refusal.check(), Some(Check::CallsInTranscript));
    assert_eq!(
        words(&refusal).as_ref(),
        b"calls-in-transcript: session.message_blocks=65 items; transcript.message_blocks=64 items"
    );
}
