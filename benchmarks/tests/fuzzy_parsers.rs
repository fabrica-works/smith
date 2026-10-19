use skein_lib::Rng;
use smith_bench::agent::codex::Observer;

#[test]
fn codex_lines_survive_seeded_truncations_corruptions_and_repetition() {
    let mut rng = Rng::new(0xc0de_0005);
    for stream in [
        include_str!("recorded/codex/completed.jsonl"),
        include_str!("recorded/codex/children.jsonl"),
        include_str!("recorded/codex/failed.jsonl"),
        include_str!("recorded/codex/live-children.jsonl"),
    ] {
        for line in stream.lines() {
            for end in 0..line.len() {
                if line.is_char_boundary(end) {
                    let _ = Observer::default().observe_line(&line[..end]);
                }
            }
            for _ in 0..64 {
                let mut bytes = line.as_bytes().to_vec();
                let index =
                    usize::try_from(rng.below(u64::try_from(bytes.len()).expect("line length"))).expect("index");
                bytes[index] = u8::try_from(rng.below(128)).expect("ASCII");
                if let Ok(corrupt) = String::from_utf8(bytes) {
                    let _ = Observer::default().observe_line(&corrupt);
                }
            }
        }
        let mut observer = Observer::default();
        for line in stream.lines() {
            observer.observe_line(line).expect("fixture");
        }
        if let Some(terminal) = stream.lines().last().filter(|line| line.contains("turn.completed")) {
            let before = observer.token_records().expect("before");
            for _ in 0..128 {
                observer.observe_line(terminal).expect("repeat");
            }
            assert_eq!(observer.token_records().expect("after"), before);
        }
    }
}

#[test]
fn named_rollouts_survive_seeded_truncation_corruption_and_response_repetition() {
    use smith_bench::agent::codex::parse_rollout;
    let mut rng = skein_lib::Rng::new(0xc0de_0010);
    for (thread, stream) in [
        ("01a121d2-5c3c-74b1-865b-66d8c6f41c3a", include_str!("recorded/codex/live-root-rollout.jsonl")),
        ("01a121d2-6e5a-7e30-abd0-133d08c8c960", include_str!("recorded/codex/live-child-rollout.jsonl")),
        ("01a121bc-04a8-75c2-8fba-d2694fa1ad22", include_str!("recorded/codex/repeated-rollout.jsonl")),
    ] {
        let original = parse_rollout(stream.as_bytes(), thread).expect("actual rollout");
        for end in 0..stream.len() {
            if stream.is_char_boundary(end) {
                let _ = parse_rollout(&stream.as_bytes()[..end], thread);
            }
        }
        for _ in 0..512 {
            let mut bytes = stream.as_bytes().to_vec();
            let index = usize::try_from(rng.below(u64::try_from(bytes.len()).expect("length"))).expect("index");
            bytes[index] = u8::try_from(rng.below(128)).expect("ASCII");
            let _ = parse_rollout(&bytes, thread);
        }
        let response = stream.lines().find(|line| line.contains("token_usage_record")).expect("actual usage");
        let repeated = format!("{stream}{}", format!("{response}\n").repeat(128));
        assert_eq!(parse_rollout(repeated.as_bytes(), thread).expect("immutable repeats"), original);
    }
}

#[test]
fn typed_event_chunks_preserve_records_under_seeded_cuts_and_corruption() {
    use smith_bench::agent::smith::events::Observer as Events;
    let limits = smith_events::Limits { string: 128, content: 1024, items: 16 };
    let stream = include_bytes!("../../crates/smith-events/golden/v4/session_started_effective_limits.jsonl");
    let mut rng = Rng::new(0xe7e0_0020);
    for _ in 0..128 {
        let mut observer = Events::new(&limits).expect("bounds");
        let mut at = 0_usize;
        while at < stream.len() {
            let length = usize::try_from(rng.below(31) + 1).expect("small chunk");
            let end = at.saturating_add(length).min(stream.len());
            observer.feed(&stream[at..end]).expect("arbitrary byte boundary");
            at = end;
        }
        observer.finish().expect("framing");
        assert_eq!(observer.sessions().len(), 1);
        let cut = usize::try_from(rng.below(u64::try_from(stream.len()).expect("length"))).expect("index");
        let mut partial = Events::new(&limits).expect("bounds");
        let _ = partial.feed(&stream[..cut]);
        if cut > 0 {
            assert!(partial.finish().is_err());
        }
        let mut corrupt = stream.to_vec();
        corrupt[cut] = u8::try_from(rng.below(128)).expect("ASCII");
        let mut observer = Events::new(&limits).expect("bounds");
        let _ = observer.feed(&corrupt);
        let _ = observer.finish();
    }
}
