//! Offline cache triplets retain repetitions, pooled misses and real evidence gaps.

use std::path::PathBuf;

use smith_bench::{CacheObservation, CheckVerdict, End, EventCheck, Judgment, Measure};
use smith_events::{Block, Event};

pub mod common;

#[test]
fn the_crossing_variant_waits_once_at_medium_effort() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let base = smith_bench::check_task(&root.join("tasks/probes/cache-reuse/task.toml"), &root.join("../docs/design"))
        .expect("cache task");
    let task = base.with_variant("crossing").expect("crossing");
    assert_eq!(task.effort.as_deref(), Some("medium"));
    assert!(task.prompt.contains("After the sixth read, call wait exactly once"));
    assert_eq!(task.person.len(), 1);
    assert_eq!(task.person[0].when, smith_bench::agent::Trigger::ToolCall { tool: "wait".into() });
    assert!(task.event.contains(&EventCheck::ToolCall {
        tool: "wait".into(),
        count: Some(smith_bench::Count::Exactly(1)),
        input_bytes_at_least: None,
        verdict: None,
        exit: None,
        clamped: None,
    }));
    assert!(task.event.contains(&EventCheck::Completions {
        count: smith_bench::Count::Exactly(14),
        before: smith_bench::Before::Answer
    }));
    let cache =
        base.event.iter().find(|check| matches!(check, EventCheck::CacheReuse { .. })).expect("base cache check");
    assert!(task.event.contains(cache));
}

fn repetition(position: u32, read: u64, misses: u64) -> smith_bench::AttemptResult {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut value = smith_bench::read_result(&root.join("tests/recorded/results/attempt.json")).expect("fixture");
    value.identity.task = "probes/cache-reuse".into();
    value.identity.attempt = format!("{position:02}");
    value.outcome.end = End::Completed;
    value.outcome.grade = None;
    value.outcome.deciding = vec!["answer".into(), "cache-reuse".into()];
    value.outcome.checks = vec![
        CheckVerdict { check: "answer".into(), passed: Measure::Observed { value: true }, evidence: vec![] },
        CheckVerdict { check: "cache-reuse".into(), passed: Measure::unavailable("triplet pending"), evidence: vec![] },
    ];
    value.cache = Some(Measure::Observed {
        value: CacheObservation {
            cache_read: read,
            predecessor_prompt: 12000,
            eligible: 10,
            full_misses: misses,
            median_at_least: 0.85,
            full_misses_per_ten_at_most: 1,
        },
    });
    value
}

#[test]
fn cache_triplets_judge_the_median_and_pool_misses_without_passing_a_partial_triplet() {
    let mut values = vec![repetition(1, 8400, 3), repetition(2, 10800, 0), repetition(3, 11400, 0)];
    assert_eq!(smith_bench::judge(&values[0]), Judgment::Incomplete);
    assert_eq!(smith_bench::judge_suite(&values[..2], false), Judgment::Incomplete);
    assert_eq!(smith_bench::judge_suite(&values, false), Judgment::Pass, "median .9 and 3/30 misses");
    let source = smith_bench::render_result(&values[0]).expect("typed metrics serialize");
    assert_eq!(
        smith_bench::parse_result(std::path::Path::new("result.json"), &source).expect("strict reader"),
        values[0]
    );
    let summary = smith_bench::summarise(&values, "cache-reuse", "small", smith_bench::Design::Single, 1, None, None)
        .expect("summary");
    let group = &summary.groups[0];
    assert_eq!(group.passes.passed, 1, "one logical attempt, three physical repetitions");
    assert_eq!(group.cache_triplets[0].median_reuse, Measure::Observed { value: 0.9 });
    assert_eq!(group.cache_triplets[0].full_misses, Measure::Observed { value: 3 });
    assert_eq!(group.metrics["cache/reuse"].observed, 3);
    if let Some(Measure::Observed { value }) = &mut values[1].cache {
        value.cache_read = 9600;
    }
    assert_eq!(smith_bench::judge_suite(&values, false), Judgment::Fail, "median .8");
    values[1] = repetition(2, 10800, 0);
    if let Some(Measure::Observed { value }) = &mut values[0].cache {
        value.full_misses = 4;
    }
    assert_eq!(smith_bench::judge_suite(&values, false), Judgment::Fail, "four pooled misses");
    values[0] = repetition(1, 8400, 3);
    values[2].cache = Some(Measure::unavailable("provider omitted usage"));
    assert_eq!(smith_bench::judge_suite(&values, false), Judgment::Incomplete);
    values[2] = repetition(3, 11400, 0);
    values[2].identity.prompt_sha256 = "different".into();
    assert_eq!(smith_bench::judge_suite(&values, false), Judgment::Incomplete);
    values[2] = values[1].clone();
    assert_eq!(smith_bench::judge_suite(&values, false), Judgment::Incomplete, "duplicate is not a repetition");
}

#[test]
fn cache_task_freezes_twelve_pointer_reads_and_two_sequential_children() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let directory = root.join("tasks/probes/cache-reuse");
    let task =
        smith_bench::check_task(&directory.join("task.toml"), &root.join("../docs/design")).expect("authored task");
    assert_eq!(task.repetitions, Some(3));
    let mut file = "node-cobalt.txt".to_owned();
    let mut seen = std::collections::BTreeSet::new();
    let mut sum = 0_u32;
    for _ in 0..12 {
        assert!(seen.insert(file.clone()));
        let text = std::fs::read_to_string(directory.join("seed").join(&file)).expect("pointer");
        let mut lines = text.lines();
        sum += lines.next().expect("value").strip_prefix("value=").expect("field").parse::<u32>().expect("integer");
        file = lines.next().expect("next").strip_prefix("next=").expect("field").to_owned();
    }
    assert_eq!((file.as_str(), sum), ("END", 197));
    assert!(task.event.contains(&EventCheck::Completions {
        count: smith_bench::Count::Exactly(13),
        before: smith_bench::Before::Answer
    }));
    let children = task.with_variant("children").expect("variant");
    assert!(children.event.contains(&EventCheck::ChildrenInTurn { count: 2 }));
    assert!(children.event.contains(&EventCheck::Completions {
        count: smith_bench::Count::Exactly(29),
        before: smith_bench::Before::Answer
    }));
}

#[test]
fn one_read_each_rejects_batched_calls_and_retains_missing_capture() {
    let mut records = common::events::balanced();
    let check = EventCheck::CallsPerResponse { tool: "read".into(), at_most: 1 };
    for count in [1, 2] {
        if let Event::ResponseCompleted(value) = &mut records[4].event {
            let mut blocks = skein_lib::List::with_capacity(2);
            for position in 0..count {
                blocks
                    .push(Block::Call {
                        call: position.to_string().as_bytes().into(),
                        tool: b"read".as_slice().into(),
                        input: b"{}".as_slice().into(),
                    })
                    .expect("bounded");
            }
            value.completion = Some(smith_events::Completion { blocks });
        }
        assert_eq!(observe(&records).check(&check, "task").passed, Measure::Observed { value: count == 1 });
    }
    if let Event::ResponseCompleted(value) = &mut records[4].event {
        value.completion = None;
    }
    assert!(matches!(observe(&records).check(&check, "task").passed, Measure::Unavailable { .. }));
}

fn observe(records: &[smith_events::Record]) -> smith_bench::agent::smith::events::Observer {
    let mut value = smith_bench::agent::smith::events::Observer::new(&common::events::LIMITS).expect("reader");
    value.feed(&common::events::bytes(records)).expect("outside codec records");
    value.finish().expect("complete lines");
    value
}

#[test]
fn sequential_children_wait_for_the_parent_return_and_require_both_returns() {
    let baseline = common::events::balanced();
    let mut records = baseline[..5].to_vec();
    for child in 1..=2 {
        let call = format!("child{child}");
        let mut opened = baseline[2].clone();
        if let Event::ConversationOpened(value) = &mut opened.event {
            value.conversation = child;
            value.thread = child;
            value.parent = Some(0);
            value.call = Some(call.as_bytes().into());
            value.kind = smith_events::ConversationKind::Child;
        }
        let mut closed = baseline[5].clone();
        if let Event::ConversationClosed(value) = &mut closed.event {
            value.conversation = child;
        }
        let mut returned =
            common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_completed_everything.jsonl"));
        if let Event::ToolCompleted(value) = &mut returned.event {
            value.run = 0;
            value.conversation = 0;
            value.call = call.as_bytes().into();
            value.child = Some(smith_events::ChildCompleted {
                conversation: child,
                stop: smith_events::ChildStop::End,
                turns: 1,
                time_ms: 1,
                spent: 0,
                usage: smith_events::Usage {
                    input_tokens: Some(1),
                    cache_read_tokens: Some(0),
                    cache_write_tokens: Some(0),
                    output_tokens: Some(1),
                    reasoning_tokens: Some(0),
                },
            });
        }
        records.extend([opened, closed, returned]);
    }
    records.extend_from_slice(&baseline[5..]);
    let check = EventCheck::ChildrenInTurn { count: 2 };
    assert_eq!(ordered_observe(&mut records).check(&check, "task").passed, Measure::Observed { value: true });
    let valid = records.clone();
    records.swap(7, 8);
    assert_eq!(
        ordered_observe(&mut records).check(&check, "task").passed,
        Measure::Observed { value: false },
        "closing is not the parent's return"
    );
    records = valid;
    records.remove(10);
    assert_eq!(ordered_observe(&mut records).check(&check, "task").passed, Measure::Observed { value: false });
}

fn ordered_observe(records: &mut [smith_events::Record]) -> smith_bench::agent::smith::events::Observer {
    for (position, record) in records.iter_mut().enumerate() {
        record.t_ms = u64::try_from(position).expect("small fixture");
    }
    observe(records)
}

#[test]
fn codex_cache_uses_observed_response_order_and_never_a_child_as_its_parent_predecessor() {
    use smith_bench::agent::codex::{ChildThread, Link, Rollouts, parse_rollout};
    let mut lines = vec![serde_json::json!({"type":"session_meta","payload":{"id":"root"}}).to_string()];
    for (response, cached) in [("z", 0), ("a", 1200), ("m", 1200), ("a", 1200)] {
        lines.push(
            serde_json::json!({"type":"token_usage_record","payload":{
            "thread_id":"root", "response_id":response,
            "usage":{"input_tokens":1200,"cached_input_tokens":cached,"cache_write_input_tokens":null,
                "output_tokens":0,"reasoning_output_tokens":null,"total_tokens":1200}}})
            .to_string(),
        );
    }
    let (_, responses) = parse_rollout(lines.join("\n").as_bytes(), "root").expect("pinned records");
    assert_eq!(responses.iter().map(|value| value.response.as_str()).collect::<Vec<_>>(), ["z", "a", "m"]);
    let mut rollouts = Rollouts { responses, children: vec![], files: vec![] };
    let Measure::Observed { value } = rollouts.cache_reuse("root", 0.85, 1) else {
        panic!("native counts");
    };
    assert_eq!((value.cache_read, value.predecessor_prompt, value.eligible, value.full_misses), (2400, 2400, 2, 0));
    rollouts.children.push(ChildThread {
        thread: "child".into(),
        link: Link::Linked { parent: "root".into() },
        responses: vec![rollouts.responses[0].clone()],
    });
    let Measure::Observed { value } = rollouts.cache_reuse("root", 0.85, 1) else {
        panic!("independent child");
    };
    assert_eq!((value.eligible, value.full_misses), (2, 0), "child's first request has no predecessor");
    rollouts.children[0].link = Link::Unlinked;
    assert!(matches!(rollouts.cache_reuse("root", 0.85, 1), Measure::Unavailable { .. }));
}
