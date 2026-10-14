use fixture::{Decoder, Event};

#[test]
fn complete_plain_lines() {
    let mut d = Decoder::new(20);
    assert_eq!(
        d.push(b"one\ntwo\n"),
        vec![Event::Line(b"one".to_vec()), Event::Line(b"two".to_vec())]
    );
    assert!(d.finish().is_empty());
}

#[test]
fn complete_oversized_line() {
    let mut d = Decoder::new(2);
    assert_eq!(d.push(b"abcd\n"), vec![Event::TooLong]);
}

#[test]
fn a_split_crlf_accepts_a_payload_at_the_bound() {
    let mut decoder = Decoder::new(2);
    assert!(decoder.push(b"ab\r").is_empty());
    assert_eq!(decoder.push(b"\n"), vec![Event::Line(b"ab".to_vec())]);
    assert_eq!(decoder.buffered_len(), 0);
}
