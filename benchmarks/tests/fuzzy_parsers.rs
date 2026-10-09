use skein_lib::Rng;
use smith_bench::agent::codex::Observer;

#[test]
fn codex_lines_survive_seeded_truncations_corruptions_and_repetition() {
    let mut rng = Rng::new(0xc0de_0005);
    for stream in [
        include_str!("recorded/codex/completed.jsonl"),
        include_str!("recorded/codex/children.jsonl"),
        include_str!("recorded/codex/failed.jsonl"),
        include_str!("recorded/codex/repeated.jsonl"),
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
fn legacy_lines_survive_seeded_truncations_corruptions_and_repetition() {
    use smith_bench::agent::smith::legacy::{Observer as Legacy, parse_debug};
    let mut rng = Rng::new(0x1e6a_0007);
    for stream in
        [include_str!("recorded/smith-legacy/completed.jsonl"), include_str!("recorded/smith-legacy/tools.jsonl")]
    {
        for line in stream.lines() {
            for end in 0..line.len() {
                if line.is_char_boundary(end) {
                    assert!(Legacy::default().observe_line(&line[..end]).is_err());
                }
            }
            for _ in 0..32 {
                let mut bytes = line.as_bytes().to_vec();
                let index =
                    usize::try_from(rng.below(u64::try_from(bytes.len()).expect("line length"))).expect("index");
                bytes[index] = u8::try_from(rng.below(128)).expect("ASCII");
                if let Ok(corrupt) = String::from_utf8(bytes) {
                    let _ = Legacy::default().observe_line(&corrupt);
                    let _ = parse_debug(&corrupt);
                }
            }
        }
        let mut observer = Legacy::default();
        for line in stream.lines() {
            observer.observe_line(line).expect("recording");
        }
        let before = observer.accepted();
        let terminal = stream.lines().last().expect("terminal return");
        for _ in 0..128 {
            observer.observe_line(terminal).expect("identical return");
        }
        assert_eq!(observer.accepted(), before);
    }
    for length in 0..256 {
        let bytes: Vec<u8> = (0..length).map(|_| u8::try_from(rng.below(128)).expect("ASCII")).collect();
        let text = String::from_utf8(bytes).expect("ASCII");
        let _ = parse_debug(&text);
    }
}

#[test]
fn named_rollouts_survive_seeded_truncation_corruption_and_response_repetition() {
    use smith_bench::agent::codex::parse_rollout;
    let mut rng = Rng::new(0xc0de_0010);
    let metadata = serde_json::json!({"type":"session_meta","payload":{"id":"root"}});
    let response = serde_json::json!({"type":"token_usage_record","payload":{"thread_id":"root","response_id":"response",
        "usage":{"input_tokens":20,"cached_input_tokens":5,"cache_write_input_tokens":0,"output_tokens":8,
            "reasoning_output_tokens":3,"total_tokens":28}}});
    let stream = format!("{metadata}\n{response}\n");
    for end in 0..stream.len() {
        let _ = parse_rollout(&stream.as_bytes()[..end], "root");
    }
    for _ in 0..512 {
        let mut bytes = stream.as_bytes().to_vec();
        let index = usize::try_from(rng.below(u64::try_from(bytes.len()).expect("bounded stream"))).expect("index");
        bytes[index] = u8::try_from(rng.below(256)).expect("byte");
        let _ = parse_rollout(&bytes, "root");
    }
    let (_, before) = parse_rollout(stream.as_bytes(), "root").expect("pinned shape");
    let repeated = format!("{stream}{}", format!("{response}\n").repeat(128));
    let (_, after) = parse_rollout(repeated.as_bytes(), "root").expect("immutable repeats");
    assert_eq!(before, after);
}
