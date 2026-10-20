#![allow(
    clippy::disallowed_methods,
    reason = "programming-model.md section 10.2: deterministic fixtures use ordinary Rust"
)]

use crate::{Capture, Error, Event, Limits, read, write};

#[test]
#[expect(clippy::disallowed_macros, reason = "an independent frozen wire fixture reproduces the reader defect")]
fn provider_failure_metadata_and_capture_detail_survive_the_reader() {
    let limits = Limits { string: 256, content: 1024, items: 16 };
    let original = core::str::from_utf8(include_bytes!("../golden/v4/outcome_failed.jsonl")).unwrap();
    let source = original.replace(
        "\"class\":\"overloaded\"",
        "\"class\":\"invalid\",\"status\":404,\"phase\":\"head\",\"limit\":{\"which\":\"response_head\",\"bound\":1935},\"detail\":\"unknown model 🌍\"",
    );
    let record = crate::read_with_ceiling(source.as_bytes(), &limits, 4096).unwrap().unwrap();
    for capture in [Capture::None, Capture::Calls, Capture::Everything] {
        let encoded = crate::write_with_ceiling(&record, &capture, &limits, 4096).unwrap().unwrap();
        let line = core::str::from_utf8(&encoded).unwrap();
        assert!(line.contains("\"status\":404"), "observed HTTP status is typed metadata");
        assert!(line.contains("\"phase\":\"head\""));
        assert!(line.contains("\"limit\":{\"which\":\"response_head\",\"bound\":1935}"));
        assert_eq!(line.contains("\"detail\":\"unknown model 🌍\""), capture != Capture::None);
    }
}

#[test]
fn actual_producer_receiving_caps_construct_without_a_cartesian_product() {
    let limits = Limits { string: 524_288, content: 4096, items: 32 };
    let ceiling = u32::try_from(END_WITH_UNREAD_USAGE.len()).expect("terminal length");
    let mut reader = crate::Reader::with_ceiling(&limits, ceiling).expect("valid writer-derived receiving bounds");
    let line = reader.feed(END_WITH_UNREAD_USAGE).expect("actual bounded terminal");
    assert!(line.complete);
    assert!(line.record.is_some());
    reader.finish().expect("complete line");
}

#[test]
fn joint_line_ceiling_is_inclusive_and_one_more_byte_refuses_in_both_paths() {
    let limits = Limits { string: 524_288, content: 4096, items: 32 };
    let ceiling = u32::try_from(END_WITH_UNREAD_USAGE.len()).expect("terminal length");
    let expected = crate::read_with_ceiling(END_WITH_UNREAD_USAGE, &limits, ceiling).expect("exact ceiling");
    assert!(expected.is_some());
    for chunk in [1, 7, 64, END_WITH_UNREAD_USAGE.len()] {
        let mut reader = crate::Reader::with_ceiling(&limits, ceiling).expect("joint bound");
        let mut records = skein_lib::List::with_capacity(1);
        for bytes in END_WITH_UNREAD_USAGE.chunks(chunk) {
            let line = reader.feed(bytes).expect("exact ceiling in arbitrary chunks");
            assert_eq!(line.consumed, bytes.len());
            if line.complete {
                records.push(line.record).expect("one complete line");
            }
        }
        assert_eq!(records.as_slice(), core::slice::from_ref(&expected));
        reader.finish().expect("terminated line");
    }
    let over = core::str::from_utf8(END_WITH_UNREAD_USAGE).expect("fixture UTF-8").replace('\n', " \n");
    assert_eq!(crate::read_with_ceiling(over.as_bytes(), &limits, ceiling), Err(Error::Limits));
    let mut reader = crate::Reader::with_ceiling(&limits, ceiling).expect("joint bound");
    assert_eq!(reader.feed(over.as_bytes()).expect_err("one byte above line ceiling"), Error::Limits);
    reader.finish().expect("refused bytes released");
    let line = reader.feed(END_WITH_UNREAD_USAGE).expect("next bounded line");
    assert_eq!(line.record, expected);
    assert_eq!(crate::Reader::with_ceiling(&limits, 0).expect_err("zero ceiling"), Error::Limits);
    assert_eq!(crate::read_with_ceiling(END_WITH_UNREAD_USAGE, &limits, 0), Err(Error::Limits));
}

#[test]
fn joint_line_bound_never_relaxes_string_content_or_list_caps() {
    let limits = Limits { string: 32, content: 8, items: 1 };
    let base = "{\"v\":4,\"type\":\"message.received\",\"t_ms\":0,\"run\":1,\"name\":2,\"bytes\":8,\"label\":\"LABEL\",\"text\":\"12345678\"}\n";
    let at_cap = base.replace("LABEL", &"x".repeat(32));
    assert!(crate::read_with_ceiling(at_cap.as_bytes(), &limits, 1024).expect("semantic boundaries").is_some());
    let metadata_over = base.replace("LABEL", &"x".repeat(33));
    let content_over = base.replace("12345678", "123456789");
    let list_over = core::str::from_utf8(END_WITH_UNREAD_USAGE)
        .expect("fixture UTF-8")
        .replace("\"session.ended\":1", "\"session.ended\":1,\"run.completed\":1");
    for (line, expected) in [
        (metadata_over.as_bytes(), Error::Shape),
        (content_over.as_bytes(), Error::Shape),
        (list_over.as_bytes(), Error::Limits),
    ] {
        assert_eq!(crate::read_with_ceiling(line, &limits, 1024), Err(expected));
        let mut reader = crate::Reader::with_ceiling(&limits, 1024).expect("ample aggregate room");
        assert_eq!(reader.feed(line).expect_err("semantic cap still applies"), expected);
        reader.finish().expect("semantic refusal releases the partial line");
    }
}

#[test]
fn another_version_is_named() {
    let limits = Limits { string: 64, content: 1024, items: 4 };
    assert_eq!(read(b"{\"v\":5,\"type\":\"future\",\"t_ms\":0}\n", &limits), Err(Error::Version(5)));
}

const END_WITH_UNREAD_USAGE: &[u8] = b"{\"v\":4,\"type\":\"session.ended\",\"t_ms\":7,\"exit\":0,\"answered\":true,\"aborted\":false,\"teardown_ms\":2,\"emitted\":{\"session.ended\":1},\"loss\":{\"events\":0,\"channel\":null},\"cpu_ms\":null,\"peak_rss_bytes\":null}\n";

#[test]
fn joint_writer_ceiling_round_trips_and_never_weakens_semantic_caps() {
    let limits = Limits { string: 524_288, content: 13_056_000, items: 52_754 };
    let ceiling = u32::try_from(END_WITH_UNREAD_USAGE.len()).expect("terminal length");
    let record = crate::read_with_ceiling(END_WITH_UNREAD_USAGE, &limits, ceiling).unwrap().unwrap();
    assert_eq!(write(&record, &Capture::Everything, &limits), Err(Error::Limits));
    for capture in [Capture::None, Capture::Calls, Capture::Everything] {
        let line = crate::write_with_ceiling(&record, &capture, &limits, ceiling).unwrap().unwrap();
        assert_eq!(line.as_ref(), END_WITH_UNREAD_USAGE);
        assert_eq!(crate::write_with_ceiling(&record, &capture, &limits, ceiling - 1), Err(Error::Limits));
        assert_eq!(crate::write_with_ceiling(&record, &capture, &limits, 0), Err(Error::Limits));
    }
    let input = b"{\"v\":4,\"type\":\"message.received\",\"t_ms\":0,\"run\":1,\"name\":2,\"bytes\":2,\"label\":\"abc\",\"text\":\"ab\"}\n";
    let message = crate::read_with_ceiling(input, &limits, 1024).unwrap().unwrap();
    for restrictive in [Limits { string: 2, ..limits }, Limits { content: 1, ..limits }] {
        assert_eq!(crate::write_with_ceiling(&message, &Capture::Everything, &restrictive, 1024), Err(Error::Limits));
    }
    let restrictive = Limits { items: 0, ..limits };
    assert_eq!(crate::write_with_ceiling(&record, &Capture::Everything, &restrictive, 1024), Err(Error::Limits));
}

#[test]
fn unread_resource_usage_is_null_and_round_trips_without_changing_the_terminal() {
    let limits = Limits { string: 64, content: 1024, items: 4 };
    let record =
        read(END_WITH_UNREAD_USAGE, &limits).expect("unread usage is a valid observation").expect("end record");
    let Event::SessionEnded(end) = &record.event else { panic!("session end") };
    assert_eq!(end.exit, 0);
    assert!(end.answered);
    assert_eq!(end.teardown_ms, Some(2));
    assert_eq!(end.loss.events, 0);
    assert_eq!(end.emitted.get(0).expect("authoritative count").count, 1);
    for capture in [Capture::None, Capture::Calls, Capture::Everything] {
        let encoded = write(&record, &capture, &limits).expect("bounded end").expect("end retained");
        assert_eq!(encoded.as_ref(), END_WITH_UNREAD_USAGE);
    }
}

#[test]
fn measured_resource_objects_round_trip_with_exact_values() {
    let limits = Limits { string: 64, content: 1024, items: 4 };
    let known = core::str::from_utf8(END_WITH_UNREAD_USAGE)
        .expect("fixture UTF-8")
        .replace("\"cpu_ms\":null", "\"cpu_ms\":{\"user\":1,\"system\":2,\"children_user\":3,\"children_system\":4}")
        .replace("\"peak_rss_bytes\":null", "\"peak_rss_bytes\":{\"self\":5,\"children\":6}");
    let record = read(known.as_bytes(), &limits).expect("measured usage remains valid").expect("end record");
    for capture in [Capture::None, Capture::Calls, Capture::Everything] {
        let encoded = write(&record, &capture, &limits).expect("bounded end").expect("end retained");
        assert_eq!(encoded.as_ref(), known.as_bytes());
        assert_eq!(read(&encoded, &limits).expect("read usage"), Some(record.clone()));
    }
}

fn word_failure(class: crate::AnswerClass) -> crate::AnswerFailure {
    crate::AnswerFailure {
        state: None,
        code: None,
        program: None,
        exit: None,
        detail: None,
        reset_ms: None,
        used: None,
        bound: None,
        class,
        endpoint: None,
        model: None,
        attempts: None,
        completion: None,
        account: None,
        which: None,
        reason: None,
        dropped: None,
        ended: None,
        turn: None,
    }
}

#[test]
fn every_answer_class_has_its_words() {
    use crate::AnswerClass;
    let cases = [
        (AnswerClass::Model, b"model failed".as_slice()),
        (AnswerClass::Budget, b"out of budget".as_slice()),
        (AnswerClass::Policy, b"policy: unfinished result".as_slice()),
        (AnswerClass::Cancelled, b"the run was cancelled".as_slice()),
        (AnswerClass::Agent, b"the agent ended without answering".as_slice()),
        (AnswerClass::Host, b"host: operation failed".as_slice()),
        (AnswerClass::Stale, b"stale: workspace changed".as_slice()),
        (AnswerClass::Transcript, b"chat cannot be resumed: transcript refused".as_slice()),
        (AnswerClass::Busy, b"busy".as_slice()),
        (AnswerClass::Invalid, b"invalid: start refused".as_slice()),
        (AnswerClass::Unknown(b"future".as_slice().into()), b"unknown failure: future".as_slice()),
    ];
    for (class, expected) in cases {
        assert_eq!(crate::words(&word_failure(class), None, None).as_ref(), expected);
    }
    let mut turns = word_failure(AnswerClass::Budget);
    turns.which = Some(b"turns".as_slice().into());
    assert_eq!(crate::words(&turns, None, None).as_ref(), b"out of turns");
}

#[test]
fn every_provider_class_has_its_words() {
    use crate::FailureClass;
    let cases = [
        (FailureClass::Overloaded, b"overloaded".as_slice()),
        (FailureClass::RateLimited, b"rate limited".as_slice()),
        (FailureClass::Exhausted, b"limited".as_slice()),
        (FailureClass::Refused, b"refused".as_slice()),
        (FailureClass::Unsupported, b"unsupported".as_slice()),
        (FailureClass::Unavailable, b"unavailable".as_slice()),
        (FailureClass::TimedOut, b"timed out".as_slice()),
        (FailureClass::ContextTooLong, b"context too long".as_slice()),
        (FailureClass::Invalid, b"invalid".as_slice()),
        (FailureClass::Unauthorized, b"unauthorized".as_slice()),
        (FailureClass::Limit, b"limit".as_slice()),
        (FailureClass::Protocol, b"protocol failed".as_slice()),
        (FailureClass::Cancelled, b"cancelled".as_slice()),
        (FailureClass::Unknown(b"future".as_slice().into()), b"future".as_slice()),
    ];
    for (class, expected) in cases {
        let mut failure = word_failure(crate::AnswerClass::Model);
        failure.completion = Some(crate::CompletionFailure {
            field: None,
            code: None,
            request_id: None,
            class,
            evidence: crate::Evidence::Response,
            retry_after_ms: None,
            phase: None,
            limit: None,
            status: None,
            detail: None,
        });
        assert_eq!(crate::words(&failure, None, None).as_ref(), expected);
    }
}

#[test]
fn clip_stops_at_a_character_boundary() {
    let mut source = [b'x'; 515];
    source.get_mut(511..).unwrap().copy_from_slice("🌍".as_bytes());
    assert_eq!(crate::clip(&source), [b'x'; 511]);
    let mut source = [b'x'; 512];
    source.get_mut(508..).unwrap().copy_from_slice("🌍".as_bytes());
    assert_eq!(crate::clip(&source), source);
}

#[test]
fn clip_keeps_the_first_line() {
    assert_eq!(crate::clip(b"provider refused\nsecond line"), b"provider refused");
    assert_eq!(crate::clip(b"provider refused\r\nsecond line"), b"provider refused");
    assert_eq!(crate::clip(b""), b"");
}

#[test]
fn each_settled_budget_answer_names_its_used_and_bound() {
    for (which, used, bound, expected) in [
        (b"turns".as_slice(), 7, 8, b"out of turns: used 7 of 8".as_slice()),
        (b"spend".as_slice(), 103, 101, b"out of spend: used 103 of 101".as_slice()),
        (b"time".as_slice(), 1234, 1500, b"out of time: used 1234 of 1500".as_slice()),
    ] {
        let mut failure = word_failure(crate::AnswerClass::Budget);
        failure.which = Some(which.into());
        failure.used = Some(used);
        failure.bound = Some(bound);
        assert_eq!(crate::words(&failure, None, None).as_ref(), expected);
    }
}

#[test]
fn a_stream_of_version_two_is_refused() {
    let limits = Limits { string: 64, content: 1024, items: 4 };
    assert_eq!(read(b"{\"v\":2,\"type\":\"future\",\"t_ms\":0}\n", &limits), Err(Error::Version(2)));
}

#[test]
fn an_account_failure_has_its_words() {
    struct Row {
        name: &'static str,
        state: AccountState,
        reset: Option<u64>,
        code: Option<&'static [u8]>,
        account: Option<&'static [u8]>,
        expected: &'static [u8],
    }
    use crate::AccountState;
    for row in [
        Row {
            name: "limited reset",
            state: AccountState::Limited,
            reset: Some(120_000),
            code: None,
            account: Some(b"work"),
            expected: b"account work reached its usage limit, usable again at 00:01",
        },
        Row {
            name: "limited now",
            state: AccountState::Limited,
            reset: Some(0),
            code: None,
            account: Some(b"work"),
            expected: b"account work reached its usage limit, usable again at 23:59",
        },
        Row {
            name: "limited unknown",
            state: AccountState::Limited,
            reset: None,
            code: None,
            account: None,
            expected: b"account 7 reached its usage limit",
        },
        Row {
            name: "refused",
            state: AccountState::Refused,
            reset: None,
            code: Some(b"not_allowed"),
            account: Some(b"work"),
            expected: b"account work: the provider refused it: not_allowed",
        },
        Row {
            name: "unauthorized",
            state: AccountState::Unauthorized,
            reset: None,
            code: None,
            account: Some(b"work\nignored"),
            expected: b"account work unauthorized",
        },
    ] {
        let mut failure = word_failure(crate::AnswerClass::Model);
        failure.account = Some(7);
        failure.state = Some(row.state);
        failure.reset_ms = row.reset;
        failure.code = row.code.map(Box::from);
        assert_eq!(
            crate::words(&failure, row.account, Some(skein_lib::Wall::from_nanos(86_340_000_000_000))).as_ref(),
            row.expected,
            "{}",
            row.name
        );
    }
}

#[test]
fn program_failures_name_their_terminal_form() {
    for (exit, expected) in [
        (None, b"claude could not run: program detail".as_slice()),
        (Some(crate::AgentExit::Code(7)), b"claude exited before its answer: program detail".as_slice()),
    ] {
        let mut failure = word_failure(crate::AnswerClass::Model);
        failure.program = Some(b"claude_code".as_slice().into());
        failure.exit = exit;
        failure.detail = Some(b"program detail".as_slice().into());
        assert_eq!(crate::words(&failure, None, None).as_ref(), expected);
    }
}

#[test]
fn program_detail_stops_before_a_partial_character() {
    // The 512-byte detail cap cuts through the following three-byte character.
    const KEPT_ASCII: usize = 511;
    let mut failure = word_failure(crate::AnswerClass::Model);
    failure.program = Some(b"claude_code".as_slice().into());
    let mut detail = "x".repeat(KEPT_ASCII);
    detail.push_str("中tail");
    failure.detail = Some(detail.into_bytes().into());
    let mut expected = "claude could not run: ".to_owned();
    expected.push_str(&"x".repeat(KEPT_ASCII));
    assert_eq!(crate::words(&failure, None, None).as_ref(), expected.as_bytes());
}

#[test]
fn program_detail_keeps_only_its_first_line() {
    let mut failure = word_failure(crate::AnswerClass::Model);
    failure.program = Some(b"claude_code".as_slice().into());
    failure.detail = Some(b"first\nsecond".as_slice().into());
    assert_eq!(crate::words(&failure, None, None).as_ref(), b"claude could not run: first");
}

#[test]
fn program_detail_escapes_terminal_controls() {
    let mut failure = word_failure(crate::AnswerClass::Model);
    failure.program = Some(b"claude_code".as_slice().into());
    failure.detail = Some(b"first\x1b[2J\t\x7f\nignored\x1b".as_slice().into());
    assert_eq!(crate::words(&failure, None, None).as_ref(), b"claude could not run: first\\x1b[2J\\x09\\x7f");
}
