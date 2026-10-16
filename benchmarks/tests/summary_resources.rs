//! Summary rows retain response counts and the boundary of each resident peak.

use smith_bench::{Agent, AttemptResult, Design, Measure, PeakScope, RunSummary};

fn result() -> AttemptResult {
    smith_bench::read_result(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/results/attempt.json"),
    )
    .expect("recorded attempt")
}

fn summary(result: AttemptResult) -> RunSummary {
    smith_bench::summarise(&[result], "sample", "working", Design::Single, 17, None, None).expect("summary")
}

fn observer(bytes: &[u8]) -> smith_bench::agent::smith::events::Observer {
    const STRING_BYTES: u32 = 524_288;
    const CONTENT_BYTES: u32 = 4096;
    const ITEMS: u32 = 512;
    const LINE_BYTES: u32 = 16_384;
    let limits = smith_events::Limits { string: STRING_BYTES, content: CONTENT_BYTES, items: ITEMS };
    let mut observer = smith_bench::agent::smith::events::Observer::with_ceiling(&limits, LINE_BYTES).expect("bounds");
    observer.feed(bytes).expect("recorded stream");
    observer.finish().expect("complete lines");
    observer
}

#[test]
fn a_row_keeps_the_attempts_responses() {
    const RESPONSES: u64 = 7;
    let mut result = result();
    result.counts.responses = Measure::Observed { value: RESPONSES };
    let summary = summary(result);
    assert_eq!(summary.attempts[0].responses, Measure::Observed { value: 7 });
    assert_eq!(summary.groups[0].metrics["responses"].median, Measure::Observed { value: 7.0 });
}

#[test]
fn smiths_own_peak_comes_from_session_ended() {
    let observer = observer(include_bytes!("recorded/retried-failure-17d63afd-02.jsonl"));
    let mut result = result();
    result.identity.agent = Agent::Smith;
    result.resources.own_peak_rss_bytes = observer.own_peak_rss_bytes();
    // The recorded session.ended's peak_rss_bytes.self is 17,321,984 bytes.
    assert_eq!(result.resources.own_peak_rss_bytes, Measure::Observed { value: 17_321_984 });
    let summary = summary(result);
    assert_eq!(summary.attempts[0].peak_rss_bytes, Measure::Observed { value: 17_321_984 });
    assert_eq!(summary.attempts[0].peak_rss_scope, PeakScope::Process);
    assert_eq!(summary.groups[0].metrics["peak_rss_bytes/process"].median, Measure::Observed { value: 17_321_984.0 });
}

#[test]
fn codexs_peak_is_its_trees_and_says_so() {
    let summary = summary(result());
    assert_eq!(summary.attempts[0].peak_rss_bytes, Measure::Observed { value: 4096 });
    assert_eq!(summary.attempts[0].peak_rss_scope, PeakScope::Tree);
    assert_eq!(summary.groups[0].metrics["peak_rss_bytes/tree"].median, Measure::Observed { value: 4096.0 });
}

#[test]
fn a_null_peak_record_leaves_smiths_own_unavailable() {
    let stream = include_str!("recorded/retried-failure-17d63afd-02.jsonl")
        .replace("\"peak_rss_bytes\":{\"self\":17321984,\"children\":157687808}", "\"peak_rss_bytes\":null");
    assert_eq!(
        observer(stream.as_bytes()).own_peak_rss_bytes(),
        Measure::unavailable("session.ended reported peak_rss_bytes null: resource usage unread")
    );
}

#[test]
fn parity_reports_the_medians_without_changing_its_criteria() {
    const RESPONSES: u64 = 10;
    const PEAK_BYTES: u64 = 8192;
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/parity/on-par.json");
    let mut summary = smith_bench::read_summary(&file).expect("recorded parity");
    let before = smith_bench::parity::from_summary(&summary).expect("verdict");
    for row in &mut summary.attempts {
        row.responses = Measure::Observed { value: RESPONSES };
        row.peak_rss_bytes = Measure::Observed { value: PEAK_BYTES };
    }
    let after = smith_bench::parity::from_summary(&summary).expect("reported medians");
    assert_eq!(after.criteria, before.criteria);
    assert_eq!(after.verdict, before.verdict);
    assert_eq!(after.reported["a-smith"]["responses"], Measure::Observed { value: 10.0 });
    assert_eq!(after.reported["a-smith"]["peak_rss_bytes/process"], Measure::Observed { value: 8192.0 });
    assert_eq!(after.reported["z-reference"]["peak_rss_bytes/tree"], Measure::Observed { value: 8192.0 });
}
