use fixture::{Decoder, Event};

fn line(bytes: &[u8]) -> Event { Event::Line(bytes.to_vec()) }

#[test]
fn waits_for_line_and_finishes_once() {
    let mut d = Decoder::new(10);
    assert!(d.push(b"ab").is_empty());
    assert_eq!(d.buffered_len(), 2);
    assert_eq!(d.push(b"c\nxy"), vec![line(b"abc")]);
    assert_eq!(d.finish(), vec![line(b"xy")]);
    assert!(d.finish().is_empty());
    assert_eq!(d.push(b"z\n"), vec![line(b"z")]);
}

#[test]
fn crlf_and_lone_cr() {
    let mut d = Decoder::new(3);
    assert!(d.push(b"abc\r").is_empty());
    assert!(d.buffered_len() <= 4);
    assert_eq!(d.push(b"\n\r\n\n"), vec![line(b"abc"), line(b""), line(b"")]);
    assert!(d.push(b"a\rb").is_empty());
    assert_eq!(d.finish(), vec![line(b"a\rb")]);
    assert!(d.push(b"ab\r").is_empty());
    assert_eq!(d.finish(), vec![line(b"ab\r")]);
}

#[test]
fn oversized_recovery_emits_one_error() {
    let mut d = Decoder::new(3);
    let mut got = d.push(b"abcd");
    got.extend(d.push(&vec![b'x'; 100_000]));
    assert!(d.buffered_len() <= 4);
    got.extend(d.push(b"\nok\n"));
    assert_eq!(got, vec![Event::TooLong, line(b"ok")]);
    assert!(d.finish().is_empty());
}

#[test]
fn pending_cr_can_overflow_at_finish_or_next_byte() {
    let mut d = Decoder::new(2);
    assert!(d.push(b"ab\r").is_empty());
    assert_eq!(d.finish(), vec![Event::TooLong]);
    assert_eq!(d.push(b"ab\rx\nq\n"), vec![Event::TooLong, line(b"q")]);
}

#[test]
fn zero_capacity_and_binary_data() {
    let mut d = Decoder::new(0);
    assert_eq!(d.push(b"\n\r\nx\n"), vec![line(b""), line(b""), Event::TooLong]);
    let mut d = Decoder::new(3);
    assert_eq!(d.push(&[0, 255, 128, 10]), vec![line(&[0, 255, 128])]);
}

#[test]
fn all_chunk_partitions_produce_same_events() {
    let bytes = b"\na\r\nabcdef\nxy\rz\nend";
    let expected = vec![line(b""), line(b"a"), Event::TooLong, Event::TooLong, line(b"end")];
    for mask in 0..(1u32 << 18) {
        let mut d = Decoder::new(3);
        let mut out = Vec::new();
        let mut start = 0;
        for i in 1..bytes.len() {
            if mask & (1 << (i - 1)) != 0 {
                out.extend(d.push(&bytes[start..i]));
                assert!(d.buffered_len() <= 4);
                start = i;
            }
        }
        out.extend(d.push(&bytes[start..]));
        out.extend(d.finish());
        assert_eq!(out, expected, "partition {mask}");
    }
}

// Extended adversarial validation


fn oracle(bytes: &[u8], max: usize) -> Vec<Event> {
    let mut result = Vec::new();
    let mut start = 0;
    for (position, &byte) in bytes.iter().enumerate() {
        if byte == b'\n' {
            let mut end = position;
            if end > start && bytes[end - 1] == b'\r' { end -= 1; }
            result.push(if end - start > max { Event::TooLong } else { line(&bytes[start..end]) });
            start = position + 1;
        }
    }
    if start < bytes.len() {
        result.push(if bytes.len() - start > max { Event::TooLong } else { line(&bytes[start..]) });
    }
    result
}

#[test]
fn deterministic_binary_transcripts_match_whole_stream_oracle() {
    for seed in 1..129u64 {
        let mut state = seed;
        let mut bytes = Vec::new();
        for _ in 0..500 {
            state ^= state << 13; state ^= state >> 7; state ^= state << 17;
            bytes.push(match state % 8 { 0 => b'\r', 1 => b'\n', _ => (state >> 8) as u8 });
        }
        for max in 0..8 {
            let mut d = Decoder::new(max);
            let mut got = Vec::new();
            let mut position = 0;
            while position < bytes.len() {
                state ^= state << 13; state ^= state >> 7; state ^= state << 17;
                let end = (position + 1 + (state as usize % 23)).min(bytes.len());
                got.extend(d.push(&bytes[position..end]));
                assert!(d.buffered_len() <= max + 1, "seed {seed}, max {max}");
                position = end;
            }
            got.extend(d.finish());
            assert_eq!(got, oracle(&bytes, max), "seed {seed}, max {max}");
        }
    }
}
