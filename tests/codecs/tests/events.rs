//! The event vocabulary round trips exact integers and filters capture as specified.

#[path = "support/events.rs"]
mod fixtures;

use smith_events::{Capture, Error, Event, Limits, read, write};

fn limits() -> Limits {
    Limits { string: 64, content: 1024, items: 8 }
}

#[test]
fn every_record_and_listed_value_reads_back_to_its_written_values() {
    for (name, record, capture) in fixtures::samples() {
        let line = write(&record, &capture, &limits()).expect(name).expect("a recorded event");
        let decoded = read(&line, &limits()).expect(name).expect("a known record");
        if capture == Capture::Everything {
            assert_eq!(decoded, record, "{name}");
        }
        let rewritten = write(&decoded, &capture, &limits()).expect(name).expect("known record");
        assert_eq!(rewritten, line, "{name}");
        assert!(
            line.len()
                <= usize::try_from(smith_events::largest_size(&record.event, &capture, &limits()).expect("bound"))
                    .expect("usize")
        );
    }
}

#[test]
fn response_tool_choice_is_metadata_with_bounded_unknown_values_and_a_required_string() {
    let base = r#"{"v":1,"type":"response.started","t_ms":0,"run":1,"conversation":2,"response":3,"turn":1,"attempt":0,"window":1,"model":{"endpoint":"e","model":"m","window_tokens":1,"output_tokens":1,"effort":null},"messages":1,"output_tokens":1,"tool_choice":"auto"}"#;
    for (spelling, expected) in [
        ("auto", smith_events::ToolChoice::Auto),
        ("none", smith_events::ToolChoice::None),
        ("only", smith_events::ToolChoice::Only),
        ("future🌍", smith_events::ToolChoice::Unknown("future🌍".as_bytes().into())),
    ] {
        let line = base.replace("\"tool_choice\":\"auto\"", &format!("\"tool_choice\":\"{spelling}\""));
        let record = read(line.as_bytes(), &limits()).expect("independent wire").expect("response");
        let Event::ResponseStarted(response) = &record.event else { panic!("response") };
        assert_eq!(response.tool_choice, expected);
        for capture in [Capture::None, Capture::Calls, Capture::Everything] {
            let encoded = write(&record, &capture, &limits()).expect("bounded metadata").expect("response");
            assert_eq!(core::str::from_utf8(&encoded).expect("UTF-8").trim_end(), line);
            assert_eq!(read(&encoded, &limits()).expect("read"), Some(record.clone()));
        }
    }
    for replacement in ["", ",\"tool_choice\":null", ",\"tool_choice\":0", ",\"tool_choice\":[]"] {
        let malformed = base.replace(",\"tool_choice\":\"auto\"", replacement);
        assert_eq!(read(malformed.as_bytes(), &limits()), Err(Error::Shape));
    }
    let tiny = Limits { string: 1, ..limits() };
    let known = read(base.as_bytes(), &limits()).expect("known metadata").expect("response");
    assert!(write(&known, &Capture::None, &tiny).expect("known spelling is not payload").is_some());
    // The reader's string cap also admits the existing response.started kind.
    let bounded = Limits { string: 16, ..limits() };
    let exact = base.replace("\"tool_choice\":\"auto\"", "\"tool_choice\":\"xxxxxxxxxxxxxxxx\"");
    let record = read(exact.as_bytes(), &bounded).expect("exact unknown cap").expect("response");
    assert!(write(&record, &Capture::None, &bounded).expect("exact unknown cap").is_some());
    let too_large = exact.replace("xxxxxxxxxxxxxxxx", "xxxxxxxxxxxxxxxxx");
    assert_eq!(read(too_large.as_bytes(), &bounded), Err(Error::Limits));
    let mut record = record;
    let Event::ResponseStarted(response) = &mut record.event else { panic!("response") };
    response.tool_choice = smith_events::ToolChoice::Unknown(b"xxxxxxxxxxxxxxxxx".as_slice().into());
    assert_eq!(write(&record, &Capture::None, &bounded), Err(Error::Limits));
    let Event::ResponseStarted(response) = &mut record.event else { panic!("response") };
    response.tool_choice = smith_events::ToolChoice::Unknown([255].as_slice().into());
    assert_eq!(write(&record, &Capture::None, &bounded), Err(Error::Text));
}

#[test]
fn capture_filters_only_content_fields_and_live_text() {
    let records = fixtures::samples();
    for (_, record, _) in records {
        let none = write(&record, &Capture::None, &limits()).expect("bounded");
        match &record.event {
            Event::TextDelta(_) => assert!(none.is_none()),
            Event::ResponseCompleted(_) => {
                let line = none.expect("metadata");
                match read(&line, &limits()).expect("read").expect("event").event {
                    Event::ResponseCompleted(value) => assert!(value.completion.is_none()),
                    Event::Budget(_)
                    | Event::SessionStarted(_)
                    | Event::SessionEnded(_)
                    | Event::RunStarted(_)
                    | Event::RunCompleted(_)
                    | Event::ConversationOpened(_)
                    | Event::ConversationClosed(_)
                    | Event::ResponseStarted(_)
                    | Event::TextDelta(_)
                    | Event::ToolStarted(_)
                    | Event::ToolCompleted(_)
                    | Event::CheckStarted(_)
                    | Event::CheckCompleted(_)
                    | Event::MessageReceived(_)
                    | Event::MessageRead(_)
                    | Event::MessageUnread(_)
                    | Event::Notice(_) => panic!("response"),
                }
            }
            Event::Budget(_)
            | Event::SessionStarted(_)
            | Event::SessionEnded(_)
            | Event::RunStarted(_)
            | Event::RunCompleted(_)
            | Event::ConversationOpened(_)
            | Event::ConversationClosed(_)
            | Event::ResponseStarted(_)
            | Event::ToolStarted(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Notice(_) => {
                let line = String::from_utf8(none.expect("metadata").into_vec()).expect("UTF-8");
                for key in ["\"prompt\":", "\"input\":", "\"text\":", "\"fields\":"] {
                    assert!(!line.contains(key), "content removed under none: {key}");
                }
            }
        }
    }
}

#[test]
fn unknown_types_fields_and_listed_values_are_tolerated_within_this_version() {
    let limits = limits();
    assert_eq!(read(b"{\"v\":1,\"type\":\"future\",\"t_ms\":0,\"extra\":[{}]}\n", &limits), Ok(None));
    let (_, record, capture) = fixtures::samples()
        .into_iter()
        .find(|(_, record, _)| matches!(record.event, Event::Notice(_)))
        .expect("notice");
    let line = write(&record, &capture, &limits).expect("write").expect("record");
    let text = String::from_utf8(line.into_vec())
        .expect("UTF8")
        .replace("\"warning\"", "\"future\"")
        .replace("\"info\"", "\"future\"");
    let text = text.trim_end().strip_suffix('}').expect("object").to_owned() + ",\"extra\":{\"v\":9}}\n";
    let decoded = read(text.as_bytes(), &limits).expect("forward values").expect("notice");
    match decoded.event {
        Event::Notice(notice) => {
            assert_eq!(notice.level, smith_events::Level::Unknown(Box::from(b"future".as_slice())));
        }
        Event::Budget(_)
        | Event::SessionStarted(_)
        | Event::SessionEnded(_)
        | Event::RunStarted(_)
        | Event::RunCompleted(_)
        | Event::ConversationOpened(_)
        | Event::ConversationClosed(_)
        | Event::ResponseStarted(_)
        | Event::ResponseCompleted(_)
        | Event::TextDelta(_)
        | Event::ToolStarted(_)
        | Event::ToolCompleted(_)
        | Event::CheckStarted(_)
        | Event::CheckCompleted(_)
        | Event::MessageReceived(_)
        | Event::MessageRead(_)
        | Event::MessageUnread(_) => panic!("notice"),
    }
    assert_eq!(read(b"{\"v\":99,\"type\":\"future\",\"t_ms\":0}", &limits), Err(Error::Version(99)));
}

#[test]
fn malformed_known_shapes_and_overflowing_integers_are_refused() {
    for line in [
        b"{\"v\":1,\"v\":1,\"type\":\"notice\",\"t_ms\":0}".as_slice(),
        b"{\"v\":1,\"type\":\"notice\",\"t_ms\":18446744073709551616}",
        b"{\"v\":1,\"type\":\"notice\",\"t_ms\":-1}",
        b"{\"v\":1,\"type\":\"notice\",\"t_ms\":1.5}",
    ] {
        assert!(read(line, &limits()).is_err());
    }
}

#[test]
fn a_chunked_stream_checks_each_version_and_rejects_an_unterminated_tail() {
    let limits = limits();
    let (_, value, capture) = fixtures::samples().remove(0);
    let line = write(&value, &capture, &limits).expect("line").expect("record");
    let mut reader = smith_events::Reader::new(&limits).expect("bounded reader");
    for byte in line.as_ref() {
        let result = reader.feed(&[*byte]).expect("one byte");
        assert_eq!(result.consumed, 1);
        assert_eq!(result.complete, *byte == b'\n');
        if let Some(record) = result.record {
            assert_eq!(record.t_ms, u64::MAX);
        }
    }
    reader.finish().expect("whole stream");
    let unknown = reader.feed(b"{\"v\":1,\"type\":\"future\",\"t_ms\":0}\ntrailing").expect("skip unknown line");
    assert!(unknown.complete && unknown.record.is_none());
    assert_eq!(
        reader.feed(b"{\"v\":2,\"type\":\"future\",\"t_ms\":0}\n").expect_err("stream version checked"),
        Error::Version(2)
    );
    reader.feed(b"{").expect("partial");
    assert_eq!(reader.finish(), Err(Error::Malformed));
}

#[test]
fn a_frozen_line_of_another_version_is_refused_by_name() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/smith-events/golden/unsupported-v2.jsonl");
    let line = std::fs::read(path).expect("unsupported golden");
    assert_eq!(read(&line, &limits()), Err(Error::Version(2)));
}

#[test]
fn configured_boundaries_and_unknown_content_kinds_are_checked() {
    let (_, mut record, _) = fixtures::samples()
        .into_iter()
        .find(|(_, value, _)| matches!(value.event, Event::TextDelta(_)))
        .expect("delta");
    let limits = Limits { string: 32, content: 4, items: 2 };
    if let Event::TextDelta(value) = &mut record.event {
        value.text = Box::from(b"1234".as_slice());
    }
    let line = write(&record, &Capture::Everything, &limits).expect("exact content bound").expect("line");
    assert_eq!(read(&line, &limits).expect("reads"), Some(record.clone()));
    if let Event::TextDelta(value) = &mut record.event {
        value.text = Box::from(b"12345".as_slice());
    }
    assert_eq!(write(&record, &Capture::Everything, &limits), Err(Error::Limits));
    if let Event::TextDelta(value) = &mut record.event {
        value.text = Box::from([255_u8].as_slice());
    }
    assert_eq!(write(&record, &Capture::Everything, &limits), Err(Error::Text));
    let overflowing = Limits { string: u32::MAX, content: u32::MAX, items: u32::MAX };
    assert_eq!(smith_events::largest_record(&overflowing), None);
    assert_eq!(smith_events::worst_case(&overflowing), None);
    let (_, prompt, _) = fixtures::samples()
        .into_iter()
        .find(|(_, value, capture)| matches!(value.event, Event::ResponseStarted(_)) && *capture == Capture::Everything)
        .expect("prompt");
    let line = write(&prompt, &Capture::Everything, &self::limits()).expect("prompt").expect("line");
    let future = String::from_utf8(line.into_vec()).expect("UTF8").replace("\"type\":\"text\"", "\"type\":\"future\"");
    assert!(read(future.as_bytes(), &self::limits()).expect("unknown block kind").is_some());
}

#[test]
fn a_reasoning_drop_notice_keeps_its_size_under_every_capture() {
    let frozen = include_bytes!("../../../crates/smith-events/golden/v1/notice_kind_reasoning_dropped.jsonl");
    let record = read(frozen, &limits()).expect("drop notice").expect("known event");
    for capture in [Capture::None, Capture::Calls, Capture::Everything] {
        assert_eq!(write(&record, &capture, &limits()).expect("write").expect("notice").as_ref(), frozen);
    }
    assert_eq!(
        read(br#"{"v":1,"type":"notice","t_ms":0,"level":"warning","kind":"reasoning_dropped"}"#, &limits()),
        Err(Error::Shape)
    );
}

#[test]
fn startup_limits_preserve_scopes_and_exact_numbers_under_every_capture() {
    let record = fixtures::effective_record();
    for capture in [Capture::None, Capture::Calls, Capture::Everything] {
        let line = write(&record, &capture, &limits()).expect("bounded header").expect("authoritative record");
        let text = std::str::from_utf8(&line).expect("UTF8");
        assert!(text.contains("18446744073709551615"));
        assert!(text.contains("9007199254740993"));
        assert!(text.contains("llm.request[endpoint=7]"));
        assert!(text.contains("llm.request[endpoint=8]"));
        assert!(text.contains("declared.window[endpoint=7,model=0]"));
        assert!(text.contains("declared.window[endpoint=7,model=1]"));
        assert_eq!(read(&line, &limits()).expect("read").expect("header"), record);
    }
}

#[test]
fn startup_limits_refuse_duplicate_names_invalid_values_and_exceeded_bounds() {
    let original = fixtures::effective_record();
    for invalid in 0..5 {
        let mut record = original.clone();
        let Event::SessionStarted(header) = &mut record.event else { panic!("header fixture") };
        match invalid {
            0 => header.limits.get_mut(1).expect("second").name = header.limits.get(0).expect("first").name.clone(),
            1 => header.limits.get_mut(0).expect("first").name = Box::new([]),
            2 => header.limits.get_mut(0).expect("first").name = Box::new([0xff]),
            3 => header.limits.get_mut(0).expect("first").name = vec![b'a'; 65].into(),
            4 => {
                header.limits.get_mut(0).expect("first").value =
                    smith_events::LimitValue::Fraction { numerator: u64::MAX, denominator: 0 }
            }
            _ => unreachable!("five controls"),
        }
        assert!(write(&record, &Capture::Everything, &limits()).is_err(), "control {invalid}");
    }
    assert_eq!(write(&original, &Capture::Everything, &Limits { items: 5, ..limits() }), Err(Error::Limits));
}

#[test]
fn startup_limit_reader_refuses_ambiguous_names_and_inexact_numbers() {
    let line = write(&fixtures::effective_record(), &Capture::Everything, &limits()).expect("bounded").expect("header");
    let line = String::from_utf8(line.into_vec()).expect("UTF8");
    for malformed in [
        line.replace("llm.request[endpoint=8]", "llm.request[endpoint=7]"),
        line.replace("18446744073709551615", "18446744073709551616"),
        line.replace("18446744073709551615", "1.5"),
        line.replace("18446744073709551615", "-1"),
        line.replace("\"denominator\":5", "\"denominator\":0"),
    ] {
        assert!(read(malformed.as_bytes(), &limits()).is_err(), "{malformed}");
    }
    assert_eq!(read(line.as_bytes(), &Limits { items: 5, ..limits() }), Err(Error::Limits));
}

#[test]
fn child_and_budget_metadata_survive_capture_and_unknown_listed_values() {
    for (name, record, _) in
        fixtures::samples().into_iter().filter(|(name, _, _)| name.starts_with("child_") || name.starts_with("budget_"))
    {
        for capture in [Capture::None, Capture::Calls, Capture::Everything] {
            let line = write(&record, &capture, &limits())
                .expect("valid bounded event fixture")
                .expect("valid bounded event fixture");
            assert_eq!(read(&line, &limits()).expect("valid bounded event fixture"), Some(record.clone()), "{name}");
        }
        let text = String::from_utf8(
            write(&record, &Capture::Everything, &limits())
                .expect("valid bounded event fixture")
                .expect("valid bounded event fixture")
                .into_vec(),
        )
        .expect("valid bounded event fixture");
        let unknown = text
            .replace("\"ordinary\"", "\"future\"")
            .replace("\"wind_down\"", "\"future\"")
            .replace("\"reserve\"", "\"future\"")
            .replace("\"deadline\"", "\"future\"")
            .replace("\"stop\":\"end\"", "\"stop\":\"future\"");
        // Unknown enum values are retained; object keys keep their contract spelling.
        let unknown = unknown.replace("\"future\":{", "\"reserve\":{");
        let decoded = read(unknown.as_bytes(), &limits())
            .expect("valid bounded event fixture")
            .expect("valid bounded event fixture");
        assert_eq!(
            write(&decoded, &Capture::Everything, &limits())
                .expect("valid bounded event fixture")
                .expect("valid bounded event fixture")
                .as_ref(),
            unknown.as_bytes()
        );
    }
}

#[test]
fn startup_containment_preserves_known_and_forward_values_under_every_capture() {
    for containment in [
        smith_events::Containment::Groups,
        smith_events::Containment::Trees,
        smith_events::Containment::Unknown(Box::from("future🌍".as_bytes())),
    ] {
        let mut record = fixtures::effective_record();
        let Event::SessionStarted(header) = &mut record.event else { panic!("header fixture") };
        header.containment = containment;
        for capture in [Capture::None, Capture::Calls, Capture::Everything] {
            let line = write(&record, &capture, &limits()).expect("bounded header").expect("authoritative");
            assert_eq!(read(&line, &limits()).expect("known record").expect("header"), record);
        }
    }
}

#[test]
fn startup_containment_requires_a_string_and_bounds_unknown_values() {
    let mut record = fixtures::effective_record();
    let encoded = write(&record, &Capture::Everything, &limits()).expect("bounded header").expect("record");
    let encoded = std::str::from_utf8(&encoded).expect("UTF8");
    for malformed in [
        encoded.replace("\"containment\":\"groups\",", ""),
        encoded.replace("\"containment\":\"groups\"", "\"containment\":null"),
        encoded.replace("\"containment\":\"groups\"", "\"containment\":42"),
    ] {
        assert_eq!(read(malformed.as_bytes(), &limits()), Err(Error::Shape));
    }
    let Event::SessionStarted(header) = &mut record.event else { panic!("header fixture") };
    header.containment = smith_events::Containment::Unknown(vec![b'x'; 64].into());
    let boundary = write(&record, &Capture::Everything, &limits()).expect("exact unknown bound").expect("record");
    assert_eq!(read(&boundary, &limits()).expect("exact bound").expect("header"), record);
    let Event::SessionStarted(header) = &mut record.event else { panic!("header fixture") };
    header.containment = smith_events::Containment::Unknown(vec![b'x'; 65].into());
    assert_eq!(write(&record, &Capture::Everything, &limits()), Err(Error::Limits));
    let oversized = encoded.replace("\"groups\"", &format!("\"{}\"", "x".repeat(65)));
    assert_eq!(read(oversized.as_bytes(), &limits()), Err(Error::Shape));
    let Event::SessionStarted(header) = &mut record.event else { panic!("header fixture") };
    header.containment = smith_events::Containment::Unknown(Box::new([0xff]));
    assert!(write(&record, &Capture::Everything, &limits()).is_err());
}

#[test]
fn tool_budget_and_command_exit_metadata_survive_every_capture_policy() {
    for (name, mut record, _) in fixtures::samples().into_iter().filter(|(name, _, _)| {
        ["tool_completed_signal", "tool_completed_unclamped", "check_completed_signal"].contains(name)
    }) {
        if let Event::ToolCompleted(value) = &mut record.event {
            value.result = None;
        }
        for capture in [Capture::None, Capture::Calls, Capture::Everything] {
            let encoded = write(&record, &capture, &limits()).expect("bounded metadata").expect("terminal");
            assert_eq!(read(&encoded, &limits()).expect("valid metadata"), Some(record.clone()), "{name}");
            assert!(
                u64::try_from(encoded.len()).expect("size")
                    <= u64::from(smith_events::largest_size(&record.event, &capture, &limits()).expect("bound"))
            );
        }
    }
}

#[test]
fn tool_budget_metadata_is_required_and_exact() {
    let (_, record, _) =
        fixtures::samples().into_iter().find(|(name, _, _)| *name == "tool_completed_unclamped").expect("fixture");
    let encoded = write(&record, &Capture::Everything, &limits()).expect("bounded").expect("terminal");
    let text = std::str::from_utf8(&encoded).expect("UTF8");
    for malformed in [
        text.replace("\"omitted_bytes\":0,", ""),
        text.replace("\"omitted_bytes\":0", "\"omitted_bytes\":null"),
        text.replace("\"omitted_bytes\":0", "\"omitted_bytes\":-1"),
        text.replace("\"omitted_bytes\":0", "\"omitted_bytes\":1.5"),
        text.replace("\"omitted_bytes\":0", "\"omitted_bytes\":18446744073709551616"),
        text.replace("\"clamped\":false,", ""),
        text.replace("\"clamped\":false", "\"clamped\":null"),
        text.replace("\"clamped\":false", "\"clamped\":0"),
        text.replace("\"clamped\":false", "\"clamped\":\"false\""),
    ] {
        assert!(read(malformed.as_bytes(), &limits()).is_err(), "{malformed}");
    }
}

#[test]
fn command_exit_requires_exactly_one_code_or_unsigned_signal() {
    for name in ["tool_completed_signal", "check_completed_signal"] {
        let (_, record, _) = fixtures::samples().into_iter().find(|(sample, _, _)| *sample == name).expect("fixture");
        let encoded = write(&record, &Capture::Everything, &limits()).expect("bounded").expect("terminal");
        let text = std::str::from_utf8(&encoded).expect("UTF8");
        for replacement in [
            "{}",
            "{\"code\":0,\"signal\":9}",
            "{\"signal\":9,\"code\":0}",
            "{\"code\":null}",
            "{\"signal\":null}",
            "{\"signal\":-1}",
            "{\"signal\":1.5}",
            "{\"signal\":18446744073709551616}",
            "{\"code\":9223372036854775808}",
            "{\"signal\":9,\"signal\":9}",
        ] {
            let malformed = text.replace("{\"signal\":18446744073709551615}", replacement);
            assert!(read(malformed.as_bytes(), &limits()).is_err(), "{name}: {malformed}");
        }
        for replacement in
            ["{\"code\":0}", "{\"code\":-9223372036854775808}", "{\"signal\":0}", "{\"signal\":9,\"future\":true}"]
        {
            let valid = text.replace("{\"signal\":18446744073709551615}", replacement);
            assert!(read(valid.as_bytes(), &limits()).expect("valid exit").is_some());
        }
    }
}

fn received(refused: Option<smith_events::MessageRefusal>, label: &[u8], text: &[u8]) -> smith_events::Record {
    smith_events::Record {
        t_ms: u64::MAX,
        event: Event::MessageReceived(smith_events::MessageReceived {
            run: u64::MAX,
            name: 9_007_199_254_740_993,
            bytes: u64::MAX,
            refused,
            label: Some(label.into()),
            text: Some(text.into()),
        }),
    }
}

#[test]
fn message_capture_keeps_identity_size_and_refusal_and_removes_only_label_and_text() {
    use smith_events::MessageRefusal;
    for refused in [
        None,
        Some(MessageRefusal::TooLarge),
        Some(MessageRefusal::InboxFull),
        Some(MessageRefusal::NameInUse),
        Some(MessageRefusal::Ending),
        Some(MessageRefusal::StartRefused),
        Some(MessageRefusal::Unknown(Box::from("future🌍".as_bytes()))),
    ] {
        let record = received(refused, b"label", b"private text");
        for capture in [Capture::None, Capture::Calls, Capture::Everything] {
            let line = write(&record, &capture, &limits()).expect("bounded message").expect("always recorded");
            let mut expected = record.clone();
            let Event::MessageReceived(message) = &mut expected.event else { panic!("message") };
            if capture != Capture::Everything {
                message.label = None;
                message.text = None;
            }
            assert_eq!(read(&line, &limits()).expect("message reads"), Some(expected));
            let line = std::str::from_utf8(&line).expect("UTF8");
            assert_eq!(line.contains("\"label\":"), capture == Capture::Everything);
            assert_eq!(line.contains("\"text\":"), capture == Capture::Everything);
            assert!(line.contains("9007199254740993"), "message name remains an exact integer");
        }
    }
}

#[test]
fn message_label_text_and_unknown_refusal_obey_their_separate_bounds() {
    let limits = Limits { string: 4, content: 5, items: 0 };
    // Known refusal spellings are vocabulary, not caller metadata.
    let record = received(Some(smith_events::MessageRefusal::StartRefused), b"1234", b"12345");
    let line = write(&record, &Capture::Everything, &limits).expect("exact bounds").expect("message");
    assert_eq!(read(&line, &Limits { string: 32, ..limits }).expect("known type"), Some(record));
    for (label, text, refused) in [
        (b"12345".as_slice(), b"12345".as_slice(), None),
        (b"1234", b"123456", None),
        (b"1234", b"12345", Some(smith_events::MessageRefusal::Unknown(Box::from(b"12345".as_slice())))),
    ] {
        let record = received(refused, label, text);
        for capture in [Capture::None, Capture::Calls, Capture::Everything] {
            assert_eq!(write(&record, &capture, &limits), Err(Error::Limits));
        }
    }
    for record in [received(None, &[0xff], b"text"), received(None, b"label", &[0xff])] {
        assert_eq!(write(&record, &Capture::Everything, &self::limits()), Err(Error::Text));
    }
    let limits = Limits { string: 32, content: 5, items: 0 };
    let record = received(Some(smith_events::MessageRefusal::Unknown(vec![b'r'; 32].into())), &[b'l'; 32], b"12345");
    let line = write(&record, &Capture::Everything, &limits).expect("exact bounds").expect("message");
    assert_eq!(read(&line, &limits).expect("bounded text"), Some(record));
    let line = std::str::from_utf8(&line).expect("UTF8");
    for changed in [
        line.replace(&"l".repeat(32), &"l".repeat(33)),
        line.replace("12345", "123456"),
        line.replace(&"r".repeat(32), &"r".repeat(33)),
    ] {
        assert!(read(changed.as_bytes(), &limits).is_err());
    }
}

#[test]
fn message_shapes_reject_missing_null_duplicate_and_inexact_fields() {
    let line = br#"{"v":1,"type":"message.received","t_ms":0,"run":1,"name":2,"bytes":3}"#;
    let line = std::str::from_utf8(line).expect("UTF8");
    for changed in [
        line.replace("\"name\":2,", ""),
        line.replace("\"run\":1", "\"run\":null"),
        line.replace("\"name\":2", "\"name\":-2"),
        line.replace("\"name\":2", "\"name\":2.5"),
        line.replace("\"name\":2", "\"name\":18446744073709551616"),
        line.replace("\"bytes\":3", "\"bytes\":3,\"refused\":null"),
        line.replace("\"bytes\":3", "\"bytes\":3,\"label\":null"),
        line.replace("\"bytes\":3", "\"bytes\":3,\"text\":null"),
        line.replace("\"name\":2", "\"name\":2,\"name\":2"),
    ] {
        assert!(read(changed.as_bytes(), &limits()).is_err(), "{changed}");
    }
    let future = line.replace("\"bytes\":3", "\"bytes\":3,\"future\":{\"name\":4}");
    assert_eq!(read(future.as_bytes(), &limits()), read(line.as_bytes(), &limits()));
    for malformed in [
        br#"{"v":1,"type":"message.read","t_ms":0,"run":1,"name":2}"#.as_slice(),
        br#"{"v":1,"type":"message.read","t_ms":0,"run":1,"name":2,"turn":null}"#,
        br#"{"v":1,"type":"message.unread","t_ms":0,"run":1,"name":null}"#,
    ] {
        assert_eq!(read(malformed, &limits()), Err(Error::Shape));
    }
}

#[test]
fn chunked_message_records_preserve_the_independent_terminal_vocabulary() {
    let lines: [&[u8]; 4] = [
        b"{\"v\":1,\"type\":\"message.received\",\"t_ms\":0,\"run\":1,\"name\":2,\"bytes\":3}\n",
        b"{\"v\":1,\"type\":\"message.read\",\"t_ms\":1,\"run\":1,\"name\":2,\"turn\":4}\n",
        b"{\"v\":1,\"type\":\"message.unread\",\"t_ms\":2,\"run\":1,\"name\":3}\n",
        b"{\"v\":1,\"type\":\"message.received\",\"t_ms\":3,\"run\":2,\"name\":4,\"bytes\":5,\"refused\":\"start_refused\"}\n",
    ];
    let mut reader = smith_events::Reader::new(&limits()).expect("bounded reader");
    for line in lines {
        let expected = read(line, &limits()).expect("independent wire fixture").expect("message record");
        for chunk in line.chunks(3) {
            let result = reader.feed(chunk).expect("chunked record");
            assert_eq!(result.consumed, chunk.len());
            if result.complete {
                assert_eq!(result.record, Some(expected.clone()));
            } else {
                assert!(result.record.is_none());
            }
        }
    }
    reader.finish().expect("all records ended");
}

#[test]
fn run_message_counts_and_nullable_final_fence_are_required_exact_fields() {
    for (_, record, capture) in fixtures::samples().into_iter().filter(|(name, _, _)| {
        matches!(*name, "run_started_everything" | "run_completed_everything" | "run_completed_no_read")
    }) {
        for capture in [capture, Capture::None, Capture::Calls] {
            let line = write(&record, &capture, &limits()).expect("bounded run").expect("record");
            let decoded = read(&line, &limits()).expect("run").expect("known record");
            match (&record.event, &decoded.event) {
                (Event::RunStarted(expected), Event::RunStarted(actual)) => {
                    assert_eq!(actual.messages, expected.messages);
                }
                (Event::RunCompleted(expected), Event::RunCompleted(actual)) => {
                    assert_eq!(actual.read, expected.read);
                    assert_eq!(actual.unread, expected.unread);
                }
                _ => panic!("run fixture"),
            }
            let text = std::str::from_utf8(&line).expect("UTF8");
            for key in ["messages", "read", "unread"] {
                let field = if key == "read" && text.contains("\"read\":null") {
                    "\"read\":null,".to_owned()
                } else {
                    format!("\"{key}\":18446744073709551615,")
                };
                if text.contains(&field) {
                    assert_eq!(read(text.replace(&field, "").as_bytes(), &limits()), Err(Error::Shape));
                    let malformed = text.replace(&field, &format!("\"{key}\":-1,"));
                    assert!(read(malformed.as_bytes(), &limits()).is_err());
                }
            }
        }
    }
}

#[test]
fn model_window_is_exact_and_independent_of_output_under_every_capture() {
    for (_, mut record, _) in fixtures::samples() {
        let model = match &mut record.event {
            Event::RunStarted(value) => &mut value.main,
            Event::ConversationOpened(value) => &mut value.model,
            Event::ResponseStarted(value) => &mut value.model,
            Event::Budget(_)
            | Event::SessionStarted(_)
            | Event::SessionEnded(_)
            | Event::RunCompleted(_)
            | Event::ConversationClosed(_)
            | Event::ResponseCompleted(_)
            | Event::TextDelta(_)
            | Event::ToolStarted(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Notice(_) => continue,
        };
        model.window_tokens = u64::MAX;
        model.output_tokens = 9_007_199_254_740_993;
        for capture in [Capture::None, Capture::Calls, Capture::Everything] {
            let line = write(&record, &capture, &limits()).expect("bounded model").expect("model event");
            let decoded = read(&line, &limits()).expect("exact integers").expect("known event");
            let model = match &decoded.event {
                Event::RunStarted(value) => &value.main,
                Event::ConversationOpened(value) => &value.model,
                Event::ResponseStarted(value) => &value.model,
                Event::Budget(_)
                | Event::SessionStarted(_)
                | Event::SessionEnded(_)
                | Event::RunCompleted(_)
                | Event::ConversationClosed(_)
                | Event::ResponseCompleted(_)
                | Event::TextDelta(_)
                | Event::ToolStarted(_)
                | Event::ToolCompleted(_)
                | Event::CheckStarted(_)
                | Event::CheckCompleted(_)
                | Event::MessageReceived(_)
                | Event::MessageRead(_)
                | Event::MessageUnread(_)
                | Event::Notice(_) => panic!("model event preserved"),
            };
            assert_eq!(model.window_tokens, u64::MAX);
            assert_eq!(model.output_tokens, 9_007_199_254_740_993);
        }
    }
}

#[test]
fn model_window_requires_an_unsigned_integer() {
    let (_, record, _) = fixtures::samples()
        .into_iter()
        .find(|(_, record, _)| matches!(record.event, Event::ResponseStarted(_)))
        .expect("response fixture");
    let line = write(&record, &Capture::Everything, &limits()).expect("bounded").expect("record");
    let text = String::from_utf8(line.into_vec()).expect("JSON UTF-8");
    let field = "\"window_tokens\":18446744073709551615";
    assert!(text.contains(field));
    for invalid in ["null", "-1", "1.5", "18446744073709551616", "\"64\""] {
        let changed = text.replace(field, &format!("\"window_tokens\":{invalid}"));
        assert!(read(changed.as_bytes(), &limits()).is_err(), "{invalid}");
    }
    let missing = text.replace(&format!("{field},"), "");
    assert!(read(missing.as_bytes(), &limits()).is_err());
}

#[test]
fn conversation_thread_and_response_window_are_required_exact_metadata_under_every_capture() {
    let conversation = r#"{"v":1,"type":"conversation.opened","t_ms":0,"run":1,"conversation":2,"kind":"main","thread":0,"model":{"endpoint":"e","model":"m","window_tokens":1,"output_tokens":1,"effort":null}}"#;
    let response = r#"{"v":1,"type":"response.started","t_ms":0,"run":1,"conversation":2,"response":3,"turn":1,"attempt":0,"window":1,"model":{"endpoint":"e","model":"m","window_tokens":1,"output_tokens":1,"effort":null},"messages":1,"output_tokens":1,"tool_choice":"auto"}"#;
    for (base, field, initial) in [(conversation, "thread", 0), (response, "window", 1)] {
        let initial_field = format!("\"{field}\":{initial}");
        for value in [0, 1, 9_007_199_254_740_993, u64::MAX] {
            let line = base.replace(&initial_field, &format!("\"{field}\":{value}"));
            let record = read(line.as_bytes(), &limits()).expect("independent exact wire").expect("known record");
            let actual = match &record.event {
                Event::ConversationOpened(opened) => opened.thread,
                Event::ResponseStarted(started) => started.window,
                Event::Budget(_)
                | Event::SessionStarted(_)
                | Event::SessionEnded(_)
                | Event::RunStarted(_)
                | Event::RunCompleted(_)
                | Event::ConversationClosed(_)
                | Event::ResponseCompleted(_)
                | Event::TextDelta(_)
                | Event::ToolStarted(_)
                | Event::ToolCompleted(_)
                | Event::CheckStarted(_)
                | Event::CheckCompleted(_)
                | Event::MessageReceived(_)
                | Event::MessageRead(_)
                | Event::MessageUnread(_)
                | Event::Notice(_) => panic!("independent conversation or response record"),
            };
            assert_eq!(actual, value);
            for capture in [Capture::None, Capture::Calls, Capture::Everything] {
                let encoded = write(&record, &capture, &limits()).expect("metadata fits").expect("record");
                assert_eq!(core::str::from_utf8(&encoded).expect("JSON").trim_end(), line);
                assert_eq!(read(&encoded, &limits()).expect("round trip"), Some(record.clone()));
            }
        }
        assert_eq!(read(base.replace(&format!("{initial_field},"), "").as_bytes(), &limits()), Err(Error::Shape));
        for invalid in ["null", "-1", "1.5", "18446744073709551616", "\"1\""] {
            let line = base.replace(&initial_field, &format!("\"{field}\":{invalid}"));
            assert!(read(line.as_bytes(), &limits()).is_err(), "{field}={invalid}");
        }
    }
}

#[test]
fn entrance_reserve_and_terminal_wind_down_are_required_metadata_in_every_capture() {
    let started = r#"{"v":1,"type":"run.started","t_ms":0,"run":1,"resumed":false,"messages":1,"main":{"endpoint":"e","model":"m","window_tokens":1,"output_tokens":1,"effort":null},"budget":{"turns":4,"spend":8,"time_ms":1000,"reserve":{"turns":2,"time_ms":750,"spend":3}},"tools":{"families":[],"wait":false,"deliver":false,"host":[]},"contract":[]}"#;
    let completed = r#"{"v":1,"type":"run.completed","t_ms":0,"run":1,"wind_down":false,"status":"parked","turns":2,"spent":3,"usage":{"input_tokens":null,"cache_read_tokens":null,"cache_write_tokens":null,"output_tokens":null,"reasoning_tokens":null},"read":null,"unread":0,"duration_ms":750}"#;
    for wire in [started.to_owned(), completed.to_owned(), completed.replace("false", "true")] {
        let record = read(wire.as_bytes(), &limits()).expect("independent wire").expect("run event");
        for capture in [Capture::None, Capture::Calls, Capture::Everything] {
            let encoded = write(&record, &capture, &limits()).expect("metadata").expect("always recorded");
            assert_eq!(read(&encoded, &limits()).expect("metadata").expect("run"), record);
            if let Event::RunStarted(run) = &record.event {
                assert_eq!(run.budget.reserve, smith_events::Reserve { turns: 2, time_ms: 750, spend: 3 });
            } else if let Event::RunCompleted(run) = &record.event {
                assert_eq!(run.wind_down, wire.contains("\"wind_down\":true"));
            } else {
                panic!("run event");
            }
        }
    }
    for invalid in [
        started.replace(",\"reserve\":{\"turns\":2,\"time_ms\":750,\"spend\":3}", ""),
        completed.replace("\"wind_down\":false,", ""),
        completed.replace("\"wind_down\":false", "\"wind_down\":null"),
        completed.replace("\"wind_down\":false", "\"wind_down\":0"),
    ] {
        assert!(read(invalid.as_bytes(), &limits()).is_err(), "required typed field");
    }
}
