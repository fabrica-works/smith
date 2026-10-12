use fixture::{Decoder, Event};

#[test]
fn complete_plain_lines() {
    let mut d = Decoder::new(20);
    assert_eq!(d.push(b"one\ntwo\n"), vec![Event::Line(b"one".to_vec()), Event::Line(b"two".to_vec())]);
    assert!(d.finish().is_empty());
}

#[test]
fn complete_oversized_line() {
    let mut d = Decoder::new(2);
    assert_eq!(d.push(b"abcd\n"), vec![Event::TooLong]);
}
