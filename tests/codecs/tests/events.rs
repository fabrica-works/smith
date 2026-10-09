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
        | Event::CheckCompleted(_) => panic!("notice"),
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
                    <= smith_events::largest_size(&record.event, &capture, &limits()).expect("bound")
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
