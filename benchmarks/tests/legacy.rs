use sha2::{Digest, Sha256};
use smith_bench::agent::smith::legacy::{Event, Observation, Observer, Value, parse_debug, parse_line};
use smith_bench::{BudgetLimit, End, Exit, Forced, Measure};

const COMPLETED: &str = include_str!("recorded/smith-legacy/completed.jsonl");
const TOOLS: &str = include_str!("recorded/smith-legacy/tools.jsonl");

fn observed(stream: &str) -> (Observer, Vec<Observation>) {
    let mut observer = Observer::default();
    let records = stream.lines().filter_map(|line| observer.observe_line(line).expect("fixture")).collect();
    (observer, records)
}

fn fact(text: &str) -> String {
    serde_json::json!({"type":"fact","at_ns":17,"fact":text}).to_string()
}

fn debug_notation_preserves_nested_named_fields_tuple_variants_bytes_and_unicode() {
    let tree = parse_debug(
        r#"Wrapper { token: Token(5), result: Some((Accepted, [1, 2])), data: b"\0\xff\n", label: "a\u{2764}\"\\" }"#,
    )
    .expect("Debug");
    let Value::Struct { name, fields } = tree else { panic!("struct") };
    assert_eq!(name, "Wrapper");
    assert_eq!(fields["data"], Value::Bytes(vec![0, 255, b'\n']));
    assert_eq!(fields["label"], Value::String("a❤\"\\".into()));
    assert_eq!(fields["token"], Value::Tuple { name: "Token".into(), values: vec![Value::Number(5)] });
    assert_eq!(
        parse_debug("Some(7,)").expect("trailing comma"),
        Value::Tuple { name: "Some".into(), values: vec![Value::Number(7)] }
    );
    for invalid in [
        "Token(1) junk",
        "Foo { renamed: 1, renamed: 2 }",
        "18446744073709551616",
        "b\"\\u{1234}\"",
        "\"\\u{d800}\"",
        "[1",
        "\"\\x80\"",
    ] {
        assert!(parse_debug(invalid).is_err(), "{invalid}");
    }
    assert!(parse_debug(&format!("{}0{}", "Some(".repeat(80), ")".repeat(80))).is_err());
}

fn baseline_recordings_map_calls_completion_attempts_tools_and_main_terminals() {
    for stream in [COMPLETED, TOOLS] {
        let (observer, records) = observed(stream);
        assert_eq!(observer.classify(Exit::Code(0), Forced::No).end, End::Completed);
        assert!(observer.accepted().is_some());
        assert!(matches!(observer.usage(), Measure::Unavailable { .. }));
        assert!(matches!(observer.loss(), Measure::Unavailable { .. }));
        assert!(records.iter().any(|record| matches!(
            record,
            Observation::Fact { event: Event::CompletionStarted { attempt: 0, .. }, .. }
        )));
        assert!(records.iter().any(
            |record| matches!(record, Observation::Call { name, input, .. } if name == b"finish" && !input.is_empty())
        ));
    }
    let (_, records) = observed(TOOLS);
    assert!(records.iter().any(|record| matches!(record, Observation::Fact { event: Event::ToolAnswered { verdict: Value::Struct { name, .. }, .. }, .. } if name == "Written")));
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("recorded/smith-legacy/manifest.json")).expect("manifest");
    for (record, stream) in manifest["records"].as_array().expect("records").iter().zip([COMPLETED, TOOLS]) {
        assert_eq!(record["provenance"], "recorded");
        assert_eq!(record["sha256"], format!("{:x}", Sha256::digest(stream)));
        assert!(!stream.contains("refresh-new") && !stream.contains("access-new"));
    }
}

fn child_finish_is_never_main_and_authoritative_run_answers_survive_missing_call_facts() {
    let mut child = Observer::default();
    for text in [
        "Run { fact: Opened { run: Token(1), conversation: Token(2), child: false } }",
        "Run { fact: Opened { run: Token(1), conversation: Token(3), child: true } }",
        "Run { fact: Called { run: Token(1), conversation: Token(3), call: Token(4), ask: Finish } }",
        "Run { fact: Returned { run: Token(1), call: Token(4), result: Accepted } }",
    ] {
        child.observe_line(&fact(text)).expect("child");
    }
    assert!(child.accepted().is_none());
    assert!(matches!(child.classify(Exit::Code(0), Forced::No).end, End::HarnessError { .. }));
    child.observe_line(&fact("Run { fact: Answered { run: Token(1), answer: Accepted } }")).expect("run terminal");
    assert_eq!(child.classify(Exit::Code(0), Forced::No).end, End::Completed);
    for (answer, end) in [
        ("Failed(Budget(Turns))", End::Budget { which: BudgetLimit::Turns }),
        ("Failed(Budget(Time))", End::Budget { which: BudgetLimit::Time }),
        ("Failed(Budget(Spend))", End::Budget { which: BudgetLimit::Spend }),
    ] {
        let mut observer = Observer::default();
        observer
            .observe_line(&fact(&format!("Run {{ fact: Answered {{ run: Token(1), answer: {answer} }} }}")))
            .expect("budget");
        assert_eq!(observer.classify(Exit::Code(0), Forced::No).end, end);
    }
}

fn emission_metadata_shapes_preserve_legacy_run_and_child_observations() {
    let mut observer = Observer::default();
    for text in [
        "Run { fact: Admitted { run: Token(1), resumed: true } }",
        "Run { fact: Admitted { run: Token(1), resumed: false } }",
        "Run { fact: Opened { run: Token(1), conversation: Token(2), child: false, parent: None, call: None } }",
        "Run { fact: Opened { run: Token(1), conversation: Token(3), child: true, parent: Some(Token(2)), call: Some(Token(4)) } }",
        "Run { fact: Called { run: Token(1), conversation: Token(3), call: Token(4), ask: Finish } }",
        "Run { fact: Returned { run: Token(1), call: Token(4), result: Accepted } }",
    ] {
        observer.observe_line(&fact(text)).expect("metadata shape");
    }
    assert!(observer.accepted().is_none());
    observer.observe_line(&fact("Run { fact: Answered { run: Token(1), answer: Accepted } }")).expect("terminal");
    assert_eq!(observer.classify(Exit::Code(0), Forced::No).end, End::Completed);
}

fn emission_metadata_shapes_reject_partial_fields_wrong_types_and_unknown_fields() {
    for text in [
        "Run { fact: Admitted { run: Token(1), resumed: 1 } }",
        "Run { fact: Admitted { run: Token(1), resumed: true, unknown: 0 } }",
        "Run { fact: Opened { run: Token(1), conversation: Token(2), child: false, parent: None } }",
        "Run { fact: Opened { run: Token(1), conversation: Token(2), child: false, call: None } }",
        "Run { fact: Opened { run: Token(1), conversation: Token(2), child: false, parent: Token(1), call: None } }",
        "Run { fact: Opened { run: Token(1), conversation: Token(2), child: false, parent: Some(1), call: None } }",
        "Run { fact: Opened { run: Token(1), conversation: Token(2), child: false, parent: Some(Token(true)), call: None } }",
        "Run { fact: Opened { run: Token(1), conversation: Token(2), child: false, parent: None, call: Some(Token(4), Token(5)) } }",
        "Run { fact: Opened { run: Token(1), conversation: Token(2), child: false, parent: None, call: None, unknown: 0 } }",
    ] {
        assert!(parse_line(&fact(text)).is_err(), "{text}");
    }
}

fn known_shape_changes_poison_classification_and_unknown_record_types_are_skipped() {
    assert!(parse_line(r#"{"type":"future","unexpected":2}"#).expect("unknown").is_none());
    for (text, field) in [
        ("Run { fact: Opened { run: Token(1), renamed: Token(2), child: false } }", "conversation"),
        ("Run { fact: Opened { run: Token(1), conversation: Token(2), child: No } }", "child"),
        ("Run { fact: Called { run: Token(1), conversation: Token(2), call: Token(4), ask: Finish(2) } }", "ask"),
        ("Run { fact: Answered { run: Token(1), answer: Accepted { unknown: 0 } } }", "answer"),
        (
            "Session { fact: Used { opener: Token(1), usage: Usage { input_tokens: 1, output_tokens: 2, cache_read_tokens: 3, renamed: 4 } } }",
            "cache_write_tokens",
        ),
    ] {
        let mut observer = observed(COMPLETED).0;
        assert!(observer.observe_line(&fact(text)).expect_err("shape").to_string().contains(field));
        assert!(matches!(observer.classify(Exit::Code(0), Forced::No).end, End::HarnessError { .. }));
    }
    for line in [
        r#"{"type":"prompt","at_ns":0,"owner":1,"encoding":"utf8","bytes":"aa"}"#,
        r#"{"type":"tool","at_ns":0,"owner":1}"#,
        r#"{"type":"call","at_ns":0,"owner":1,"encoding":"hex","id":"a","name":"66","input":""}"#,
    ] {
        assert!(parse_line(line).is_err());
    }
}

fn everything_content_preserves_bytes_and_legacy_exit_uses_observed_terminals() {
    assert!(
        matches!(parse_line(r#"{"type":"prompt","at_ns":2,"owner":0,"encoding":"hex","bytes":"00ff"}"#).expect("prompt"), Some(Observation::Prompt { bytes, .. }) if bytes == [0, 255])
    );
    assert!(matches!(parse_line(r#"{"type":"usage","at_ns":2,"owner":0,"usage":"Usage { input_tokens: 3, output_tokens: 2, cache_read_tokens: 1, cache_write_tokens: 0 }"}"#).expect("usage"), Some(Observation::Usage { .. })));
    assert!(
        matches!(parse_line(r#"{"type":"completion","at_ns":2,"owner":0,"text":"done"}"#).expect("text"), Some(Observation::Completion { text, .. }) if text == "done")
    );
    let observer = observed(COMPLETED).0;
    assert_eq!(observer.classify(Exit::Signal(9), Forced::Killed).end, End::Completed);
    assert_eq!(observer.classify(Exit::Signal(9), Forced::Killed).warnings.len(), 1);
    assert!(matches!(observer.classify(Exit::Code(1), Forced::No).end, End::HarnessError { .. }));
    assert_eq!(Observer::default().classify(Exit::Signal(9), Forced::Killed).end, End::Timeout);
    assert!(matches!(Observer::default().classify(Exit::Code(1), Forced::No).end, End::Failed { .. }));
}

fn message_facts_keep_opaque_names_and_fences_without_changing_run_classification() {
    // Explicit synthetic renderings of the current run fact variants; the frozen
    // baseline recordings remain untouched.
    for text in [
        "MessageReceived { run: Token(1), name: Token(99), bytes: 18 }",
        "MessageRead { run: Token(1), name: Token(99), turn: 1 }",
        "MessageFence { run: Token(1), turn: 1, read: Some(Token(99)) }",
        "MessageReceived { run: Token(1), name: Token(0), bytes: 16 }",
        "MessageUnread { run: Token(1), name: Token(0) }",
        "MessageFence { run: Token(1), turn: 2, read: None }",
        "MessageRefused { run: Token(1), name: Token(7), bytes: 4194304, reason: TooLarge }",
        "MessageRefused { run: Token(1), name: Token(7), bytes: 16, reason: Full }",
        "MessageRefused { run: Token(1), name: Token(7), bytes: 16, reason: NameInUse }",
        "MessageRefused { run: Token(1), name: Token(7), bytes: 16, reason: Ending }",
    ] {
        let mut observer = observed(COMPLETED).0;
        let event = observer.observe_line(&fact(&format!("Run {{ fact: {text} }}"))).expect("known message fact");
        assert!(matches!(event, Some(Observation::Fact { event: Event::Other { domain, value }, .. })
            if domain == "Run" && value == parse_debug(text).expect("exact neutral payload")));
        assert_eq!(observer.classify(Exit::Code(0), Forced::No).end, End::Completed);
    }
}

fn malformed_message_facts_poison_the_legacy_observer() {
    for text in [
        "MessageReceived { run: Token(1), name: 0, bytes: 16 }",
        "MessageReceived { run: Token(1), name: Token(0), bytes: Many }",
        "MessageReceived { run: Token(1), name: Token(0), bytes: 16, text: 4 }",
        "MessageRead { run: Token(1), name: Token(0), turn: 0 }",
        "MessageRead { run: Token(1), name: Token(0), turn: 4294967296 }",
        "MessageUnread { run: Token(1) }",
        "MessageFence { run: Token(1), turn: 1, read: Some(0) }",
        "MessageFence { run: Token(1), turn: 1, read: Some(Token(0), Token(1)) }",
        "MessageRefused { run: Token(1), name: Token(0), bytes: 16, reason: Future }",
        "MessageRefused { run: Token(1), name: Token(0), bytes: 16, reason: Full(0) }",
        "MessageLost { run: Token(1), name: Token(0) }",
    ] {
        let mut observer = observed(COMPLETED).0;
        assert!(observer.observe_line(&fact(&format!("Run {{ fact: {text} }}"))).is_err(), "{text}");
        assert!(matches!(observer.classify(Exit::Code(0), Forced::No).end, End::HarnessError { .. }));
    }
}

fn optional_usage_preserves_absence_and_zero_alongside_numeric_baseline_records() {
    // Synthetic renderings of both supported binary faces; frozen recordings
    // remain the evidence for the baseline face.
    for usage in [
        "Usage { input_tokens: 11, output_tokens: 4, cache_read_tokens: 5, cache_write_tokens: 3 }",
        "Usage { input_tokens: Some(11), output_tokens: Some(4), cache_read_tokens: Some(5), cache_write_tokens: Some(3), reasoning_tokens: None }",
        "Usage { input_tokens: None, output_tokens: Some(0), cache_read_tokens: None, cache_write_tokens: Some(0), reasoning_tokens: Some(0) }",
    ] {
        let expected = parse_debug(usage).expect("exact usage");
        let line = serde_json::json!({"type":"usage", "at_ns":17, "owner":1, "usage":usage}).to_string();
        assert!(
            matches!(parse_line(&line).expect("usage record"), Some(Observation::Usage { value, .. }) if value == expected)
        );
        let line = fact(&format!("Session {{ fact: Used {{ opener: Token(1), usage: {usage} }} }}"));
        assert!(
            matches!(parse_line(&line).expect("usage fact"), Some(Observation::Fact { event: Event::SessionUsed { usage: value, .. }, .. }) if value == expected)
        );
    }
}

fn optional_usage_rejects_mixed_partial_and_malformed_shapes() {
    for usage in [
        "Usage { input_tokens: Some(1), output_tokens: Some(2), cache_read_tokens: None, cache_write_tokens: None }",
        "Usage { input_tokens: 1, output_tokens: Some(2), cache_read_tokens: None, cache_write_tokens: None, reasoning_tokens: None }",
        "Usage { input_tokens: Some(true), output_tokens: None, cache_read_tokens: None, cache_write_tokens: None, reasoning_tokens: None }",
        "Usage { input_tokens: Some(1, 2), output_tokens: None, cache_read_tokens: None, cache_write_tokens: None, reasoning_tokens: None }",
        "Usage { input_tokens: None, output_tokens: None, cache_read_tokens: None, cache_write_tokens: None, reasoning_tokens: Token(0) }",
        "Usage { input_tokens: None, output_tokens: None, cache_read_tokens: None, reasoning_tokens: None }",
        "Usage { input_tokens: None, output_tokens: None, cache_read_tokens: None, cache_write_tokens: None, reasoning_tokens: None, extra: 0 }",
    ] {
        let line = serde_json::json!({"type":"usage", "at_ns":17, "owner":1, "usage":usage}).to_string();
        assert!(parse_line(&line).is_err(), "{usage}");
        let mut observer = observed(COMPLETED).0;
        let line = fact(&format!("Session {{ fact: Used {{ opener: Token(1), usage: {usage} }} }}"));
        assert!(observer.observe_line(&line).is_err(), "{usage}");
        assert!(matches!(observer.classify(Exit::Code(0), Forced::No).end, End::HarnessError { .. }));
    }
}

fn native_kind_facts_are_selected_without_changing_frozen_flag_records() {
    use skein_lib::{Duration, Token};
    use smith_bench::agent::smith::legacy::{FactShape, parse_line_for};
    use smith_domain_run::facts::{FactKind, Kind, Return, Standing, Used};
    use smith_domain_run::{Granted, Set, Share, Stage};
    let run = Token::new(0);
    let main = FactKind::Opened { run, conversation: Token::new(0), kind: Kind::Main };
    let child = FactKind::Opened {
        run,
        conversation: Token::new(2),
        kind: Kind::Child {
            parent: Token::new(0),
            call: Token::new(9),
            asked: Share { turns: Some(4), time: Some(Duration::from_millis(1250)) },
            granted: Granted {
                turns: 3,
                time: Duration::from_millis(750),
                spend: 20,
                turns_set: Set::Reserve,
                time_set: Set::Deadline,
            },
        },
    };
    let standing = FactKind::Budget {
        run,
        standing: Standing {
            stage: Stage::WindDown,
            turns: 4,
            turns_limit: 6,
            spend: 9,
            spend_limit: 100,
            time: Duration::from_millis(500),
            time_limit: Duration::from_secs(3),
            reserve: smith_domain_run::charter::Reserve { turns: 2, time: Duration::from_secs(1) },
            reserve_spend: 30,
        },
    };
    let returned = FactKind::Returned {
        run,
        call: Token::new(9),
        result: Return::Answered { used: Used { turns: 1, time: Duration::from_millis(250), spend: 5 }, cut: 0 },
    };
    let mut observer = Observer::new(FactShape::Kinds);
    for native in [main, child, standing] {
        let line = fact(&format!("Run {{ fact: {native:?} }}"));
        let observation = observer.observe_line(&line).expect("actual native Debug layout");
        assert!(observation.is_some());
        if matches!(native, FactKind::Opened { kind: Kind::Child { .. }, .. }) {
            assert!(matches!(observation, Some(Observation::Fact { event: Event::ConversationOpened {
                child: true, parent: Some(0), kind: Some(Value::Struct { name, .. }), .. }, .. }) if name == "Child"));
        }
        if matches!(native, FactKind::Opened { .. } | FactKind::Budget { .. }) {
            assert!(parse_line(&line).is_err(), "frozen flags must not guess a later layout");
        }
        assert!(
            parse_line_for(&line.replace("turns_set: Reserve", "turns_set: Deadline"), FactShape::Kinds).is_err()
                || !line.contains("turns_set")
        );
    }
    // The events-era carrier adds cut. The frozen Debug face still reads its
    // original literal layout, and refuses the new field rather than guessing.
    let current = fact(&format!("Run {{ fact: {returned:?} }}"));
    assert!(parse_line_for(&current, FactShape::Kinds).is_err());
    let frozen = fact(
        "Run { fact: Returned { run: Token(0), call: Token(9), result: Answered { used: Used { turns: 1, time: Duration(250000000), spend: 5 } } } }",
    );
    let observation = observer.observe_line(&frozen).expect("frozen native return layout");
    assert!(matches!(&observation, Some(Observation::Fact { event: Event::RunReturned {
        value: Value::Struct { name, fields }, .. }, .. }) if name == "Answered" && fields.contains_key("used")));
    let legacy = fact("Run { fact: Opened { run: Token(0), conversation: Token(0), child: false } }");
    assert!(parse_line_for(&legacy, FactShape::Kinds).is_err());
    assert!(parse_line(&legacy).is_ok());
    assert!(observer.accepted().is_none(), "child return never accepts main");
}

// Related offline assertions share a test process to keep startup overhead
// inside the one-second harness budget; each named story keeps its assertions.

#[test]
fn legacy_recordings_terminals_and_emission_metadata() {
    baseline_recordings_map_calls_completion_attempts_tools_and_main_terminals();
    legacy_predicates_prove_only_trace_visible_counts_and_counterexamples();
    legacy_passive_checks_name_missing_evidence_and_acceptance_exit_agreement();
    child_finish_is_never_main_and_authoritative_run_answers_survive_missing_call_facts();
    emission_metadata_shapes_preserve_legacy_run_and_child_observations();
    everything_content_preserves_bytes_and_legacy_exit_uses_observed_terminals();
    message_facts_keep_opaque_names_and_fences_without_changing_run_classification();
    optional_usage_preserves_absence_and_zero_alongside_numeric_baseline_records();
    native_kind_facts_are_selected_without_changing_frozen_flag_records();
    choice_intents_are_ancestry_selected_and_never_provider_attempt_counts();
    actual_everything_recording_counts_response_usage_once_and_preserves_acceptance();
}

#[test]
fn legacy_debug_and_strict_shape_contracts() {
    debug_notation_preserves_nested_named_fields_tuple_variants_bytes_and_unicode();
    emission_metadata_shapes_reject_partial_fields_wrong_types_and_unknown_fields();
    known_shape_changes_poison_classification_and_unknown_record_types_are_skipped();
    malformed_message_facts_poison_the_legacy_observer();
    optional_usage_rejects_mixed_partial_and_malformed_shapes();
}

fn legacy_observer(stream: &str) -> smith_bench::agent::smith::Observing {
    use smith_bench::agent::smith::legacy::{FactShape, Shape};
    use smith_bench::agent::smith::{Credential, CredentialShape, Face, FaceSince, Observing, Prepared};
    let face = Face::Legacy(Shape::LocalJson);
    let mut observer = Observing::new(Prepared {
        face,
        settings: "unissued-settings".into(),
        credential: Credential::Record(None),
        trace: None,
        registry: FaceSince {
            commit: "30259d89b88834ed3edaaa08b3f320d2252bbb9a".into(),
            face,
            credential: CredentialShape::Converted(smith_bench::credential::TokenShape::Legacy),
            facts: FactShape::Flags,
            receiving: None,
        },
        rendered: serde_json::json!({}),
        program: "/bin/false".into(),
        state: "unissued-state".into(),
        teardown: skein_lib::Duration::from_millis(40),
    })
    .expect("frozen outside face");
    for line in stream.lines() {
        observer.feed_line(line).expect("recorded trace");
    }
    observer
}

fn checked_legacy(stream: &str, checks: &[smith_bench::EventCheck]) -> Vec<smith_bench::CheckVerdict> {
    legacy_observer(stream).checks(checks, "task")
}

fn legacy_predicates_prove_only_trace_visible_counts_and_counterexamples() {
    use smith_bench::{Before, Count, EventCheck};
    let checks = [
        EventCheck::Completions { count: Count::AtLeast(1), before: Before::Finish },
        EventCheck::Completions { count: Count::Exactly(0), before: Before::Answer },
        EventCheck::Completions { count: Count::AtMost(100), before: Before::Answer },
        EventCheck::ConversationOpened { kind: "main".into(), count: Count::AtLeast(1) },
        EventCheck::ToolCall {
            tool: "read".into(),
            count: Some(Count::AtLeast(1)),
            input_bytes_at_least: None,
            verdict: None,
            exit: None,
            clamped: None,
        },
        EventCheck::ToolCall {
            tool: "read".into(),
            count: None,
            input_bytes_at_least: Some(1_000_000),
            verdict: None,
            exit: None,
            clamped: None,
        },
        EventCheck::NoVerdict { verdicts: vec!["written".into()] },
        EventCheck::NoVerdict { verdicts: vec!["bad_path".into()] },
        EventCheck::NoStop { stop: "tools".into() },
        EventCheck::NoStop { stop: "max_tokens".into() },
        EventCheck::RunCompleted { status: "failed".into(), class: None },
        EventCheck::RunCompleted { status: "accepted".into(), class: None },
        EventCheck::NoRequest,
    ];
    let values = checked_legacy(TOOLS, &checks);
    for index in [0, 3, 4] {
        assert_eq!(values[index].passed, Measure::Observed { value: true }, "{}", values[index].check);
    }
    for index in [1, 5, 6, 8] {
        assert_eq!(values[index].passed, Measure::Observed { value: false }, "{}", values[index].check);
    }
    for index in [2, 7, 9, 10, 11, 12] {
        assert!(matches!(values[index].passed, Measure::Unavailable { .. }), "{}", values[index].check);
    }
    let failed = fact("Session { fact: CompletionFailed { opener: Token(0), failure: Limit, evidence: Unknown } }");
    let values = checked_legacy(
        &failed,
        &[EventCheck::NoFailure { class: "limit".into() }, EventCheck::NoFailure { class: "unauthorized".into() }],
    );
    assert_eq!(values[0].passed, Measure::Observed { value: false });
    assert!(matches!(values[1].passed, Measure::Unavailable { .. }));
    let accepted = fact("Run { fact: Answered { run: Token(0), answer: Accepted } }");
    let values = checked_legacy(&accepted, &[EventCheck::RunCompleted { status: "failed".into(), class: None }]);
    assert_eq!(values[0].passed, Measure::Observed { value: false });
    let values = checked_legacy("", &checks);
    assert!(values.iter().all(|value| matches!(value.passed, Measure::Unavailable { .. })));
}

fn legacy_passive_checks_name_missing_evidence_and_acceptance_exit_agreement() {
    let observer = legacy_observer(COMPLETED);
    let good = observer.health(Exit::Code(0), Forced::No).expect("legacy health");
    assert_eq!(good.headroom.len(), 7);
    assert!(matches!(good.loss_per_sink["trace"], Measure::Unavailable { .. }));
    for check in &good.passive_checks {
        if check.check == "exit-agrees" {
            assert_eq!(check.passed, Measure::Observed { value: true });
        } else {
            assert!(matches!(check.passed, Measure::Unavailable { .. }), "{}", check.check);
        }
    }
    let bad = observer.health(Exit::Code(1), Forced::No).expect("legacy health");
    assert_eq!(
        bad.passive_checks.iter().find(|check| check.check == "exit-agrees").expect("exit").passed,
        Measure::Observed { value: false }
    );
    let missing = legacy_observer("").health(Exit::Code(1), Forced::No).expect("missing health");
    assert!(missing.passive_checks.iter().all(|check| matches!(check.passed, Measure::Unavailable { .. })));
}

fn choice_intents_are_ancestry_selected_and_never_provider_attempt_counts() {
    use smith_bench::agent::smith::legacy::{FactShape, parse_line_for};
    use smith_domain_session::{Choice, FactKind};
    let mut observer = Observer::new(FactShape::Choices);
    for choice in [Choice::Auto, Choice::Only, Choice::None] {
        let native = FactKind::CompletionStarted {
            opener: skein_lib::Token::new(0),
            attempt: 0,
            messages: 1,
            max_tokens: 4096,
            choice,
        };
        let line = fact(&format!("Session {{ fact: {native:?} }}"));
        let seen = observer.observe_line(&line).expect("current logical intent");
        assert!(matches!(
            seen,
            Some(Observation::Fact {
                event: Event::CompletionStarted { attempt: 0, choice: Some(Value::Name(_)), .. },
                ..
            })
        ));
        assert!(parse_line_for(&line, FactShape::Kinds).is_err(), "frozen kind layout rejects choice");
        assert!(parse_line(&line).is_err(), "frozen flag layout rejects choice");
    }
    let old =
        fact("Session { fact: CompletionStarted { opener: Token(0), attempt: 0, messages: 1, max_tokens: 4096 } }");
    assert!(parse_line_for(&old, FactShape::Choices).is_err(), "current choice is required");
    let counts = legacy_observer(TOOLS).counts();
    assert!(matches!(counts.provider_attempts, Measure::Unavailable { .. }));
    assert!(matches!(counts.retries, Measure::Unavailable { .. }));
    assert!(matches!(counts.responses, Measure::LowerBound { value, .. } if value > 0));
}

fn actual_everything_recording_counts_response_usage_once_and_preserves_acceptance() {
    use sha2::{Digest, Sha256};
    use smith_bench::agent::smith::legacy::FactShape;
    use smith_bench::agent::smith::{Credential, CredentialShape, Face, FaceSince, Observing, Prepared};
    let stream = include_str!("recorded/smith-current/e548-everything.jsonl");
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("recorded/smith-current/manifest.json")).expect("recording provenance");
    assert_eq!(manifest["sha256"], format!("{:x}", Sha256::digest(stream)));
    assert_eq!(manifest["provenance"], "recorded");
    let face = Face::Legacy(smith_bench::agent::smith::legacy::Shape::LocalJson);
    let mut observer = Observing::new(Prepared {
        face,
        settings: "unissued-recorded-settings".into(),
        credential: Credential::Record(None),
        trace: None,
        registry: FaceSince {
            commit: "e548d6329d17f7b141143a5957f8c95626f354f4".into(),
            face,
            credential: CredentialShape::Converted(smith_bench::credential::TokenShape::Legacy),
            facts: FactShape::Choices,
            receiving: None,
        },
        rendered: serde_json::json!({}),
        program: "/bin/false".into(),
        state: "unissued-recorded-state".into(),
        teardown: skein_lib::Duration::from_millis(40),
    })
    .expect("recorded outside face");
    for line in stream.lines() {
        observer.feed_line(line).expect("actual current Debug record");
    }
    let smith_bench::agent::smith::Observer::Legacy(native) = &observer.observer else { panic!("legacy") };
    assert!(native.accepted().is_some());
    assert_eq!(observer.observer.classify(smith_bench::Exit::Code(0), Forced::No).end, End::Completed);
    let counts = observer.counts();
    assert!(matches!(counts.responses, Measure::LowerBound { value: 3, .. }));
    assert!(matches!(counts.provider_attempts, Measure::Unavailable { .. }));
    assert!(matches!(counts.retries, Measure::Unavailable { .. }));
    let records = observer.tokens().expect("per-response usage joins");
    let total = records.iter().find(|record| record.role == smith_bench::ScopeKind::Total).expect("total");
    assert!(matches!(total.usage.fresh, Measure::LowerBound { value: 4796, .. }));
    assert!(matches!(total.usage.output, Measure::LowerBound { value: 67, .. }));
    assert!(matches!(total.usage.cache_read, Measure::LowerBound { value: 0, .. }));
    assert!(matches!(total.usage.cache_write, Measure::LowerBound { value: 0, .. }));
}
