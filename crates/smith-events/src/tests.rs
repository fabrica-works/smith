#![allow(
    clippy::disallowed_methods,
    reason = "programming-model.md section 10.2: deterministic fixtures use ordinary Rust"
)]

use crate::{Capture, Error, Event, Limits, read, write};

#[test]
fn another_version_is_named() {
    let limits = Limits { string: 64, content: 1024, items: 4 };
    assert_eq!(read(b"{\"v\":2,\"type\":\"future\",\"t_ms\":0}\n", &limits), Err(Error::Version(2)));
}

const END_WITH_UNREAD_USAGE: &[u8] = b"{\"v\":1,\"type\":\"session.ended\",\"t_ms\":7,\"exit\":0,\"answered\":true,\"teardown_ms\":2,\"emitted\":{\"session.ended\":1},\"loss\":{\"events\":0,\"channel\":null},\"cpu_ms\":null,\"peak_rss_bytes\":null}\n";

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
