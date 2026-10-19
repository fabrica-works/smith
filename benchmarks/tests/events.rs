use smith_bench::agent::smith::events::Observer;
use smith_bench::{Convention, End, Exit, Forced, Measure, ScopeKind};
use smith_events::{Event, Limits, Record};

pub mod common;

#[path = "events/delegation.rs"]
mod delegation;

#[path = "events/compaction_choice.rs"]
mod compaction_choice;

#[path = "events/carried.rs"]
mod carried;

use common::events::{LIMITS, balanced, bytes, record};

fn observe(records: &[Record]) -> Observer {
    static ORIGINAL: std::sync::LazyLock<Visible> = std::sync::LazyLock::new(|| Visible::new(balanced()));
    observe_against(records, Some(&*ORIGINAL))
}

fn passed(observer: &Observer, name: &str) -> Measure<bool> {
    observer
        .health(Exit::Code(0), Forced::No, 10)
        .passive_checks
        .iter()
        .find(|check| check.check == name)
        .expect("passive check")
        .passed
        .clone()
}

fn captured(records: &[Record], capture: &smith_events::Capture, limits: &Limits) -> Observer {
    let lines: Vec<_> = records
        .iter()
        .map(|record| smith_events::write(record, capture, limits).expect("bounded record").expect("metadata retained"))
        .collect();
    let ceiling = u32::try_from(lines.iter().map(|line| line.len()).max().expect("outside records"))
        .expect("bounded encoded lines");
    let mut observer = Observer::with_ceiling(limits, ceiling).expect("actual writer ceiling");
    for line in &lines {
        observer.feed(line).expect("outside captured reader");
    }
    observer.finish().expect("complete stream");
    observer
}

fn event_chunks_keep_exact_usage_and_authoritative_exits() {
    let values = balanced();
    let mut observer = Observer::new(&LIMITS).expect("receiving bounds");
    for chunk in bytes(&values).chunks(7) {
        observer.feed(chunk).expect("seven-byte chunks");
    }
    observer.finish().expect("complete framing");
    assert_eq!(observer.sessions()[0].records, values);
    assert_eq!(observer.classify(Exit::Code(0), Forced::No).end, End::Completed);
    assert!(matches!(observer.classify(Exit::Code(1), Forced::No).end, End::HarnessError { .. }));
    let forced = observer.classify(Exit::Signal(15), Forced::Terminated);
    assert_eq!(forced.end, End::Completed);
    assert_eq!(forced.warnings.len(), 1);
    let tokens = observer.tokens().expect("scoped usage");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].role, ScopeKind::Root);
    assert_eq!(tokens[0].convention, Convention::SmithEvents);
    assert_eq!(tokens[1].usage.fresh, Measure::Observed { value: 13 });
    assert_eq!(tokens[1].usage.output, Measure::Observed { value: 5 });
    assert_eq!(observer.counts().responses, Measure::Observed { value: 1 });
    for name in ["stream-whole", "exit-agrees", "nothing-lost", "paired", "spend-sums"] {
        assert_eq!(passed(&observer, name), Measure::Observed { value: true }, "{name}");
    }
    assert!(
        matches!(passed(&observer, "teardown"), Measure::Unavailable { reason } if reason.starts_with("not yet: 06-teardown"))
    );
}

fn unread_resources_stay_null_and_do_not_change_terminals_or_response_usage() {
    let mut values = balanced();
    let original = observe(&values).sessions()[0].ended.clone().expect("authoritative terminal");
    assert!(original.cpu_ms.is_some() && original.peak_rss_bytes.is_some());
    if let Event::SessionEnded(ended) = &mut values[7].event {
        ended.cpu_ms = None;
        ended.peak_rss_bytes = None;
    }
    let limits = Limits { items: original.emitted.len(), ..LIMITS };
    for capture in [smith_events::Capture::None, smith_events::Capture::Calls, smith_events::Capture::Everything] {
        let observer = captured(&values, &capture, &limits);
        let ended = observer.sessions()[0].ended.as_ref().expect("authoritative end retained");
        assert_eq!(ended.cpu_ms, None);
        assert_eq!(ended.peak_rss_bytes, None);
        assert_eq!(ended.exit, original.exit);
        assert_eq!(observer.classify(Exit::Code(0), Forced::No).end, End::Completed);
        assert_eq!(
            observer.tokens().expect("response usage independent")[1].usage.output,
            Measure::Observed { value: 5 }
        );
        assert!(
            matches!(passed(&observer, "headroom/peak-rss"), Measure::Unavailable { reason } if reason.contains("null") && reason.contains("unread"))
        );
    }
    values = balanced();
    assert_eq!(observe(&values).sessions()[0].ended.as_ref().expect("read end").cpu_ms, original.cpu_ms);
    assert_eq!(
        observe(&values).sessions()[0].ended.as_ref().expect("read end").peak_rss_bytes,
        original.peak_rss_bytes
    );
    values.pop();
    assert!(matches!(passed(&observe(&values), "teardown"), Measure::Unavailable { .. }));
}

fn repeated_usage_is_immutable_but_duplicate_terminals_break_pairing() {
    let mut values = balanced();
    values.insert(5, values[4].clone());
    let observer = observe(&values);
    assert_eq!(observer.tokens().expect("deduplicated")[1].usage.output, Measure::Observed { value: 5 });
    assert_eq!(passed(&observer, "paired"), Measure::Observed { value: false });
    if let Event::ResponseCompleted(value) = &mut values[5].event {
        value.usage.output_tokens = Some(6);
    }
    assert!(observe(&values).tokens().is_err());
}

fn agent_exit_tracks_the_sent_answer_and_forced_pending_stays_unknown() {
    let mut values = balanced();
    if let Event::SessionStarted(value) = &mut values[0].event {
        value.mode = smith_events::Mode::Agent;
    }
    let failed = record(include_str!("../../crates/smith-events/golden/v4/status_failed.jsonl"));
    if let Event::RunCompleted(source) = failed.event
        && let Event::RunCompleted(value) = &mut values[6].event
    {
        value.status = smith_events::Status::Failed;
        value.failure = source.failure;
        value.result = None;
    }
    let observer = observe(&values);
    assert!(matches!(observer.classify(Exit::Code(0), Forced::No).end, End::Failed { .. }));
    assert!(matches!(observer.classify(Exit::Code(1), Forced::No).end, End::HarnessError { .. }));
    assert_eq!(passed(&observer, "exit-agrees"), Measure::Observed { value: true });
    if let Event::SessionEnded(value) = &mut values[7].event {
        value.answered = false;
        value.exit = 1;
    }
    assert!(matches!(observe(&values).classify(Exit::Code(1), Forced::No).end, End::Failed { .. }));
    assert!(matches!(observe(&values).classify(Exit::Code(0), Forced::No).end, End::HarnessError { .. }));
    values.truncate(6);
    let pending = observe(&values);
    assert_eq!(pending.classify(Exit::Signal(15), Forced::Terminated).end, End::Timeout);
    let health = pending.health(Exit::Signal(15), Forced::Terminated, 10);
    assert!(matches!(
        health.passive_checks.iter().find(|check| check.check == "exit-agrees").expect("exit check").passed,
        Measure::Unavailable { .. }
    ));
    let failure = record(include_str!("../../crates/smith-events/golden/v4/failure_class_limit.jsonl"));
    if let Event::RunCompleted(source) = failure.event
        && let Event::ResponseCompleted(value) = &mut values[4].event
    {
        value.failure = source.failure.expect("limit failure").completion;
    }
    assert_eq!(passed(&observe(&values), "no-limit-failure"), Measure::Observed { value: false });
}

fn child_and_compaction_usage_belongs_to_its_native_scope_and_run() {
    let mut values = balanced();
    let mut child = values[2].clone();
    if let Event::ConversationOpened(value) = &mut child.event {
        value.conversation = 1;
        value.kind = smith_events::ConversationKind::Child;
        value.parent = Some(0);
    }
    let mut child_response = values[4].clone();
    if let Event::ResponseCompleted(value) = &mut child_response.event {
        value.conversation = 1;
        value.response = 1;
        value.usage.output_tokens = Some(8);
    }
    child.t_ms = 4;
    child_response.t_ms = 4;
    values.splice(5..5, [child, child_response]);
    let records = observe(&values).tokens().expect("actual parent evidence");
    assert_eq!(records.len(), 3);
    assert!(matches!(&records[1].role, ScopeKind::Child { parent } if parent == "session-0/run-0/conversation-0"));
    assert_eq!(records[2].usage.output, Measure::Observed { value: 13 });
    if let Event::ConversationOpened(value) = &mut values[5].event {
        value.kind = smith_events::ConversationKind::Compaction;
    }
    let compacted = observe(&values).tokens().expect("actual compaction parent");
    assert!(
        matches!(&compacted[1].role, ScopeKind::Compaction { parent } if parent == "session-0/run-0/conversation-0")
    );
    // A response's ordinal is local to its process/run, not a global provider id.
    let mut second = balanced();
    for record in &mut second {
        if let Event::ResponseCompleted(value) = &mut record.event {
            value.usage.output_tokens = Some(9);
        }
    }
    values.extend(second);
    let records = observe(&values).tokens().expect("appended independent process");
    assert_eq!(records.last().expect("total").usage.output, Measure::Observed { value: 22 });
}

fn unknown_types_are_skipped_and_version_or_partial_records_fault() {
    let mut observer = Observer::new(&LIMITS).expect("bounds");
    observer.feed(b"{\"v\":4,\"type\":\"future.record\",\"t_ms\":0}\n").expect("unknown prefix skipped");
    observer.feed(&bytes(&balanced()[..1])).expect("startup");
    observer.feed(b"{\"v\":4,\"type\":\"future.record\",\"t_ms\":1,\"field\":true}\n").expect("unknown type");
    assert_eq!(observer.sessions()[0].records.len(), 1);
    let error = observer.feed(b"{\"v\":5,\"type\":\"future.record\",\"t_ms\":1}\n").expect_err("version");
    assert!(error.what.contains("version 5"));
    assert!(observer.feed(&bytes(&balanced())).is_err(), "a fault cannot be silently cleared");
    let mut partial = Observer::new(&LIMITS).expect("bounds");
    partial.feed(b"{\"v\":4").expect("partial bytes held");
    assert!(partial.finish().is_err());
    let invalid = Limits { string: 128, content: 65_536, items: 128 };
    assert!(Observer::new(&invalid).is_err(), "receiving cardinalities must fit the codec bound");
}

fn writer_ceiling_is_independent_and_preserves_every_semantic_cap() {
    use smith_bench::agent::smith::Receiving;

    let records = balanced();
    let encoded = bytes(&records);
    let ceiling = u32::try_from(encoded.split_inclusive(|byte| *byte == b'\n').map(<[u8]>::len).max().expect("lines"))
        .expect("bounded writer bytes");
    let receiving = Receiving { string: 524_288, content: 4096, items: 32, encoded_line: ceiling };
    let limits = receiving.limits().expect("joint aggregate caps");
    assert!(smith_events::largest_record(&limits).is_none(), "independent-maxima overflow is not this writer's bound");
    assert_eq!(
        limits,
        smith_events::Limits { string: receiving.string, content: receiving.content, items: receiving.items }
    );
    let mut observer = Observer::with_ceiling(&limits, receiving.encoded_line).expect("actual writer ceiling");
    for chunk in encoded.chunks(17) {
        observer.feed(chunk).expect("chunked bounded stream");
    }
    observer.finish().expect("complete stream");
    assert_eq!(observer.sessions()[0].records, records);
    let mut short = Observer::with_ceiling(&limits, ceiling - 1).expect("smaller nonzero cap");
    assert!(short.feed(&encoded).is_err(), "one byte below actual longest line must refuse");
    assert!(Observer::with_ceiling(&limits, 0).is_err());
    for bounded in [
        smith_events::Limits { string: 1, ..limits },
        smith_events::Limits { content: 1, ..limits },
        smith_events::Limits { items: 1, ..limits },
    ] {
        let mut observer = Observer::with_ceiling(&bounded, ceiling).expect("semantic cap independent of byte cap");
        assert!(observer.feed(&encoded).is_err(), "encoded room cannot bypass a semantic cap");
    }
    for missing in ["string", "content", "items", "encoded_line"] {
        let mut value = serde_json::to_value(receiving).expect("typed receiving record");
        value.as_object_mut().expect("object").remove(missing);
        assert!(
            serde_json::from_value::<Receiving>(value).is_err(),
            "missing {missing} never receives a guessed default"
        );
    }
    for zero in [
        Receiving { string: 0, ..receiving },
        Receiving { content: 0, ..receiving },
        Receiving { items: 0, ..receiving },
        Receiving { encoded_line: 0, ..receiving },
    ] {
        assert!(zero.limits().is_err());
    }
    let mut overflow = serde_json::to_value(receiving).expect("typed receiving record");
    overflow["encoded_line"] = serde_json::json!(u64::from(u32::MAX) + 1);
    assert!(serde_json::from_value::<Receiving>(overflow).is_err(), "ceiling never narrows");
}

fn passive_negatives_preserve_loss_scope_and_exact_spend() {
    let mut values = balanced();
    if let Event::RunCompleted(value) = &mut values[6].event {
        value.spent = 4;
    }
    assert_eq!(passed(&observe(&values), "spend-sums"), Measure::Observed { value: false });
    let mut values = balanced();
    if let Event::SessionEnded(value) = &mut values[7].event {
        value.loss.events = 1;
        value.loss.channel = Some(2);
    }
    let observer = observe(&values);
    assert_eq!(passed(&observer, "nothing-lost"), Measure::Observed { value: false });
    assert!(matches!(passed(&observer, "paired"), Measure::Unavailable { .. }));
    assert!(matches!(observer.tokens().expect("loss retained")[1].usage.output, Measure::LowerBound { value: 5, .. }));
    let health = observer.health(Exit::Code(0), Forced::No, 10);
    assert_eq!(health.loss_per_sink["events"], Measure::Observed { value: 1 });
    assert_eq!(health.loss_per_sink["channel"], Measure::Observed { value: 2 });
    assert!(matches!(passed(&observer, "message-terminals"), Measure::Unavailable { .. }));
    let mut values = balanced();
    values.remove(5);
    assert_eq!(passed(&observe(&values), "paired"), Measure::Observed { value: false });
    values.pop();
    assert_eq!(passed(&observe(&values), "stream-whole"), Measure::Observed { value: false });
}

fn observed_passive_contradictions_survive_loss_but_missing_records_do_not() {
    let mut values = balanced();
    if let Event::SessionEnded(value) = &mut values[7].event {
        value.loss.events = 1;
    }
    if let Event::RunCompleted(value) = &mut values[6].event {
        value.spent = 2;
    }
    assert_eq!(passed(&observe(&values), "spend-sums"), Measure::Observed { value: false });
    if let Event::RunCompleted(value) = &mut values[6].event {
        value.spent = 4;
    }
    assert!(matches!(passed(&observe(&values), "spend-sums"), Measure::Unavailable { .. }));
    values.insert(5, values[4].clone());
    assert_eq!(passed(&observe(&values), "paired"), Measure::Observed { value: false });
    values.remove(5);
    values.remove(5);
    assert!(matches!(passed(&observe(&values), "paired"), Measure::Unavailable { .. }));
    let failed = record(include_str!("../../crates/smith-events/golden/v4/failure_class_limit.jsonl"));
    if let Event::RunCompleted(source) = failed.event
        && let Event::RunCompleted(value) = &mut values[5].event
    {
        value.status = smith_events::Status::Failed;
        value.failure = source.failure;
        value.result = None;
    }
    assert_eq!(passed(&observe(&values), "no-limit-failure"), Measure::Observed { value: false });
}

fn every_codec_golden_remains_readable_as_outside_recorded_bytes() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/smith-events/golden/v4");
    let mut files: Vec<_> =
        std::fs::read_dir(directory).expect("codec goldens").map(|entry| entry.expect("entry").path()).collect();
    files.sort();
    // The codec's independent maximum records have unrelated timestamps and
    // cannot form one native process stream. Read each unchanged line through
    // the adapter's actual bounded Reader; ordered native Observer behavior is
    // covered by the coherent exchanges below.
    let mut reader = smith_events::Reader::new(&LIMITS).expect("bounded receiving");
    for file in files {
        if file.extension().is_none_or(|extension| extension != "jsonl") {
            continue;
        }
        let captured = std::fs::read(&file).expect("golden bytes");
        let line = reader.feed(&captured).unwrap_or_else(|error| panic!("{}: {error:?}", file.display()));
        assert!(line.complete, "{}", file.display());
        assert_eq!(line.consumed, captured.len(), "{}", file.display());
        assert!(line.record.is_some(), "{}", file.display());
        reader.finish().expect("golden framing");
    }
}

fn author_predicates_use_typed_records_and_missing_content_stays_unavailable() {
    use smith_bench::{Before, Count, EventCheck};
    let mut values = balanced();
    if let Event::ResponseCompleted(value) = &mut values[4].event {
        value.completion = None;
    }
    let observer = observe(&values);
    for check in [
        EventCheck::Completions { count: Count::Exactly(1), before: Before::Answer },
        EventCheck::ConversationOpened { kind: "main".into(), count: Count::Exactly(1) },
        EventCheck::NoFailure { class: "limit".into() },
        EventCheck::RunCompleted { status: "accepted".into(), class: None, which: None, reason: None },
        EventCheck::NoStop { stop: "max_tokens".into() },
    ] {
        assert_eq!(observer.check(&check, "task").passed, Measure::Observed { value: true });
    }
    assert_eq!(observer.check(&EventCheck::NoRequest, "task").passed, Measure::Observed { value: false });
    assert!(matches!(
        observer.check(&EventCheck::CallsInOneResponse { tool: "read".into(), at_least: 1 }, "task").passed,
        Measure::Unavailable { .. }
    ));
    assert!(Count::AtLeast(2).holds(u64::MAX));
    assert!(!Count::AtMost(2).holds(3));
    let mut later = values[4].clone();
    later.t_ms = values[6].t_ms;
    if let Event::ResponseCompleted(response) = &mut later.event {
        response.run = 1;
        response.response = 1;
    }
    values.insert(7, later);
    let observer = observe(&values);
    assert_eq!(
        observer.check(&EventCheck::Completions { count: Count::Exactly(1), before: Before::Answer }, "task").passed,
        Measure::Observed { value: true }
    );
    assert_eq!(
        observer.check(&EventCheck::Completions { count: Count::Exactly(2), before: Before::Answer }, "task").passed,
        Measure::Observed { value: false }
    );
}

fn block_cardinalities_use_completion_metadata_under_every_capture_policy() {
    use smith_bench::{Count, EventCheck};
    let mut values = balanced();
    if let Event::ResponseCompleted(value) = &mut values[4].event {
        value.oversized = 2;
        value.cut = 3;
        // The content deliberately disagrees: cardinalities are metadata,
        // retained independently of captured blocks by events.md section3.5.
        value.completion = None;
    }
    for capture in [smith_events::Capture::None, smith_events::Capture::Calls, smith_events::Capture::Everything] {
        let observer = captured(&values, &capture, &LIMITS);
        assert_eq!(
            observer
                .check(&EventCheck::Blocks { oversized: Some(Count::Exactly(2)), cut: Some(Count::AtLeast(3)) }, "task")
                .passed,
            Measure::Observed { value: true }
        );
        assert_eq!(
            observer.check(&EventCheck::Blocks { oversized: Some(Count::AtMost(1)), cut: None }, "task").passed,
            Measure::Observed { value: false }
        );
        assert_eq!(
            observer.check(&EventCheck::Blocks { oversized: None, cut: Some(Count::Exactly(2)) }, "task").passed,
            Measure::Observed { value: false }
        );
    }
    if let Event::ResponseCompleted(value) = &mut values[4].event {
        value.oversized = u64::MAX;
        value.cut = u64::MAX;
    }
    let check = EventCheck::Blocks { oversized: Some(Count::AtLeast(u32::MAX)), cut: Some(Count::AtLeast(u32::MAX)) };
    assert_eq!(observe(&values).check(&check, "task").passed, Measure::Observed { value: true });
    values.insert(5, values[4].clone());
    assert!(matches!(observe(&values).check(&check, "task").passed, Measure::Unavailable { .. }));
    values.remove(5);
    values.remove(4);
    assert!(matches!(observe(&values).check(&check, "task").passed, Measure::Unavailable { .. }));
    let mut lossy = balanced();
    if let Event::SessionEnded(value) = &mut lossy[7].event {
        value.loss.events = 1;
    }
    assert!(matches!(observe(&lossy).check(&check, "task").passed, Measure::Unavailable { .. }));
}

fn message_refusals_and_read_unread_each_settle_one_admission_attempt() {
    use smith_bench::EventCheck;
    let mut values = balanced();
    if let Event::RunStarted(value) = &mut values[1].event {
        value.messages = 1;
    }
    if let Event::RunCompleted(value) = &mut values[6].event {
        value.read = Some(7);
        value.unread = 0;
    }
    let received = Record {
        t_ms: 2,
        event: Event::MessageReceived(smith_events::MessageReceived {
            run: 0,
            name: 7,
            bytes: 4,
            refused: None,
            label: Some(Box::from(&b"person"[..])),
            text: Some(Box::from(&b"task"[..])),
        }),
    };
    let mut rejected = received.clone();
    if let Event::MessageReceived(value) = &mut rejected.event {
        value.refused = Some(smith_events::MessageRefusal::NameInUse);
    }
    values.splice(2..2, [received, rejected]);
    values.insert(
        7,
        Record { t_ms: 7, event: Event::MessageRead(smith_events::MessageRead { run: 0, name: 7, turn: 1 }) },
    );
    for (index, record) in values.iter_mut().enumerate() {
        record.t_ms = u64::try_from(index).expect("index");
    }
    let observer = observe(&values);
    assert_eq!(passed(&observer, "message-terminals"), Measure::Observed { value: true });
    assert_eq!(observer.check(&EventCheck::NothingBeforeTask, "task").passed, Measure::Observed { value: true });
    assert_eq!(observer.check(&EventCheck::HandledOnce, "task").passed, Measure::Observed { value: true });
    assert_eq!(observer.check(&EventCheck::CompletionPerExchange, "task").passed, Measure::Observed { value: true });
    assert_eq!(observer.counts().message_terminals["refused"], Measure::Observed { value: 1 });
    assert_eq!(observer.counts().message_terminals["read"], Measure::Observed { value: 1 });
    for record in &mut values {
        if let Event::RunCompleted(value) = &mut record.event {
            value.read = Some(8);
        }
    }
    assert_eq!(passed(&observe(&values), "message-terminals"), Measure::Observed { value: false });
    for record in &mut values {
        if let Event::RunCompleted(value) = &mut record.event {
            value.read = Some(7);
            value.unread = 1;
        }
    }
    assert_eq!(passed(&observe(&values), "message-terminals"), Measure::Observed { value: false });
    values[7].event = Event::MessageUnread(smith_events::MessageUnread { run: 0, name: 7 });
    assert_eq!(passed(&observe(&values), "message-terminals"), Measure::Observed { value: false });
    for record in &mut values {
        if let Event::RunCompleted(value) = &mut record.event {
            value.read = None;
        }
    }
    let observer = observe(&values);
    assert_eq!(passed(&observer, "message-terminals"), Measure::Observed { value: true });
    assert_eq!(observer.check(&EventCheck::HandledOnce, "task").passed, Measure::Observed { value: false });
    values.remove(7);
    assert_eq!(passed(&observe(&values), "message-terminals"), Measure::Observed { value: false });
}

fn prompt_windows_use_exact_disjoint_usage_and_the_actual_request_model() {
    use smith_bench::EventCheck;
    let mut values = balanced();
    for record in &mut values {
        match &mut record.event {
            Event::ConversationOpened(value) => value.model.window_tokens = 20,
            Event::ResponseStarted(value) => value.model.window_tokens = 20,
            Event::Budget(_)
            | Event::SessionStarted(_)
            | Event::SessionEnded(_)
            | Event::RunStarted(_)
            | Event::RunCompleted(_)
            | Event::ConversationClosed(_)
            | Event::ResponseCompleted(_)
            | Event::TextDelta(_)
            | Event::ToolStarted(_)
            | Event::JobEnded(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Notice(_) => {}
        }
    }
    assert_eq!(
        observe(&values).check(&EventCheck::PromptWithinWindow, "task").passed,
        Measure::Observed { value: true }
    );
    if let Event::ResponseCompleted(value) = &mut values[4].event {
        value.usage.cache_write_tokens = Some(3);
    }
    assert_eq!(
        observe(&values).check(&EventCheck::PromptWithinWindow, "task").passed,
        Measure::Observed { value: false }
    );
    if let Event::ResponseCompleted(value) = &mut values[4].event {
        value.usage.cache_write_tokens = None;
    }
    assert!(matches!(
        observe(&values).check(&EventCheck::PromptWithinWindow, "task").passed,
        Measure::Unavailable { .. }
    ));
    if let Event::ResponseCompleted(value) = &mut values[4].event {
        value.usage.input_tokens = Some(u64::MAX);
        value.usage.cache_read_tokens = Some(u64::MAX);
        value.usage.cache_write_tokens = Some(u64::MAX);
    }
    assert_eq!(
        observe(&values).check(&EventCheck::PromptWithinWindow, "task").passed,
        Measure::Observed { value: false }
    );
    if let Event::ResponseStarted(value) = &mut values[3].event {
        value.model.endpoint = Box::from(&b"foreign"[..]);
    }
    assert!(matches!(
        observe(&values).check(&EventCheck::PromptWithinWindow, "task").passed,
        Measure::Unavailable { .. }
    ));
}

fn child_run() -> Vec<Record> {
    let mut values = balanced();
    let mut opened = values[2].clone();
    if let Event::ConversationOpened(value) = &mut opened.event {
        value.conversation = 1;
        value.kind = smith_events::ConversationKind::Child;
        value.parent = Some(0);
        value.call = Some(Box::from(&b"child-call"[..]));
        value.share = Some(smith_events::Share { turns: 2, time_ms: 1000, spend: 10 });
    }
    let mut returned = record(include_str!("../../crates/smith-events/golden/v4/tool_completed_everything.jsonl"));
    if let Event::ToolCompleted(value) = &mut returned.event {
        value.run = 0;
        value.conversation = 0;
        value.call = Box::from(&b"child-call"[..]);
        value.child = Some(smith_events::ChildCompleted {
            conversation: 1,
            stop: smith_events::ChildStop::Budget,
            turns: 2,
            time_ms: 100,
            spent: 0,
            usage: smith_events::Usage {
                input_tokens: Some(0),
                cache_read_tokens: Some(0),
                cache_write_tokens: Some(0),
                output_tokens: Some(0),
                reasoning_tokens: None,
            },
        });
    }
    values.splice(4..4, [opened, returned]);
    for (index, value) in values.iter_mut().enumerate() {
        value.t_ms = u64::try_from(index).expect("small emission position");
    }
    values
}

fn child_share_and_main_return_checks_join_the_parent_call_and_emission_order() {
    use smith_bench::EventCheck;
    let mut values = child_run();
    for check in [EventCheck::ChildShare { turns_at_most: 2 }, EventCheck::MainAfterChild] {
        assert_eq!(observe(&values).check(&check, "task").passed, Measure::Observed { value: true });
    }
    if let Event::ToolCompleted(value) = &mut values[5].event {
        value.child.as_mut().expect("child return").turns = 3;
    }
    assert_eq!(
        observe(&values).check(&EventCheck::ChildShare { turns_at_most: 2 }, "task").passed,
        Measure::Observed { value: false }
    );
    if let Event::ToolCompleted(value) = &mut values[5].event {
        value.call = Box::from(&b"foreign-call"[..]);
    }
    assert!(matches!(
        observe(&values).check(&EventCheck::ChildShare { turns_at_most: 2 }, "task").passed,
        Measure::Unavailable { .. }
    ));
    if let Event::ToolCompleted(value) = &mut values[5].event {
        value.call = Box::from(&b"child-call"[..]);
    }
    values.swap(5, 6);
    for (index, value) in values.iter_mut().enumerate() {
        value.t_ms = u64::try_from(index).expect("small emission position");
    }
    assert_eq!(observe(&values).check(&EventCheck::MainAfterChild, "task").passed, Measure::Observed { value: false });
}

fn a_capped_child_final_turn_requires_no_tools_and_its_captured_native_answer() {
    use smith_bench::EventCheck;
    let mut values = child_run();
    let mut request = values[3].clone();
    if let Event::ResponseStarted(value) = &mut request.event {
        value.conversation = 1;
        value.response = 1;
        value.tool_choice = smith_events::ToolChoice::None;
    }
    let mut response = values[6].clone();
    if let Event::ResponseCompleted(value) = &mut response.event {
        value.conversation = 1;
        value.response = 1;
        let mut blocks = skein_lib::List::with_capacity(1);
        blocks.push(smith_events::Block::Text { text: Box::from(&b"survey done"[..]) }).expect("one answer block");
        value.completion = Some(smith_events::Completion { blocks });
    }
    values.splice(5..5, [request, response]);
    for (index, value) in values.iter_mut().enumerate() {
        value.t_ms = u64::try_from(index).expect("small emission position");
    }
    assert_eq!(observe(&values).check(&EventCheck::FinalTurn, "task").passed, Measure::Observed { value: true });
    if let Event::ResponseStarted(value) = &mut values[5].event {
        value.tool_choice = smith_events::ToolChoice::Auto;
    }
    assert_eq!(observe(&values).check(&EventCheck::FinalTurn, "task").passed, Measure::Observed { value: false });
    if let Event::ResponseStarted(value) = &mut values[5].event {
        value.tool_choice = smith_events::ToolChoice::None;
    }
    if let Event::ResponseCompleted(value) = &mut values[6].event {
        value.completion = None;
    }
    assert!(matches!(observe(&values).check(&EventCheck::FinalTurn, "task").passed, Measure::Unavailable { .. }));
    let mut values = child_run();
    values.remove(5);
    assert_eq!(observe(&values).check(&EventCheck::MainAfterChild, "task").passed, Measure::Observed { value: false });
}

fn cache_series() -> Vec<Record> {
    let mut values = balanced();
    if let Event::ResponseStarted(value) = &mut values[3].event {
        value.window = 1;
    }
    if let Event::ResponseCompleted(value) = &mut values[4].event {
        value.usage.input_tokens = Some(1200);
        value.usage.cache_read_tokens = Some(0);
        value.usage.cache_write_tokens = Some(0);
    }
    let mut pairs = Vec::new();
    for response in 1..=10 {
        let mut start = values[3].clone();
        let mut end = values[4].clone();
        if let Event::ResponseStarted(value) = &mut start.event {
            value.response = response;
            value.turn = response + 1;
        }
        if let Event::ResponseCompleted(value) = &mut end.event {
            value.response = response;
            value.usage.input_tokens = Some(0);
            value.usage.cache_read_tokens = Some(1200);
        }
        pairs.extend([start, end]);
    }
    values.splice(5..5, pairs);
    for (position, value) in values.iter_mut().enumerate() {
        value.t_ms = u64::try_from(position).expect("small position");
    }
    values
}

fn cache_reuse_sums_eligible_predecessors_with_exact_misses_and_window_boundaries() {
    let check = smith_bench::EventCheck::CacheReuse { median_at_least: 0.85, full_misses_per_ten_at_most: 1 };
    let mut values = cache_series();
    let measured = observe(&values).check(&check, "task");
    assert!(matches!(measured.passed, Measure::Unavailable { .. }), "one repetition cannot decide a triplet");
    assert!(measured.evidence[0].contains("cache_read=12000; predecessor_prompt=12000; eligible=10; full_misses=0"));
    for position in [6, 8] {
        if let Event::ResponseCompleted(value) = &mut values[position].event {
            value.usage.input_tokens = Some(1200);
            value.usage.cache_read_tokens = Some(0);
        }
        let Measure::Observed { value } = observe(&values).cache_reuse(0.85, 1) else {
            panic!("exact counts");
        };
        assert_eq!(value.full_misses, if position == 6 { 1 } else { 2 });
    }
    // The first request of a new window never inherits the old predecessor.
    if let Event::ResponseStarted(value) = &mut values[7].event {
        value.window = 2;
    }
    for record in &mut values[9..] {
        if let Event::ResponseStarted(value) = &mut record.event {
            value.window = 2;
        }
    }
    let Measure::Observed { value } = observe(&values).cache_reuse(0.85, 1) else {
        panic!("exact counts");
    };
    assert_eq!((value.eligible, value.full_misses), (9, 1));
}

fn cache_reuse_does_not_invent_usage_or_join_other_conversations() {
    let mut values = cache_series();
    if let Event::ResponseCompleted(value) = &mut values[4].event {
        value.usage.cache_write_tokens = None;
    }
    assert!(matches!(observe(&values).cache_reuse(0.85, 1), Measure::Unavailable { .. }));
    let mut values = cache_series();
    if let Event::ResponseStarted(value) = &mut values[5].event {
        value.conversation = 7;
    }
    if let Event::ResponseCompleted(value) = &mut values[6].event {
        value.conversation = 7;
    }
    assert!(matches!(observe(&values).cache_reuse(0.85, 1), Measure::Unavailable { .. }));
    // A child on a separate thread supplies no predecessor to main.
    let mut values = balanced();
    let mut child = values[2].clone();
    if let Event::ConversationOpened(value) = &mut child.event {
        value.conversation = 7;
        value.thread = 7;
        value.kind = smith_events::ConversationKind::Child;
        value.parent = Some(0);
    }
    let mut request = values[3].clone();
    let mut response = values[4].clone();
    if let Event::ResponseStarted(value) = &mut request.event {
        value.conversation = 7;
        value.response = 7;
    }
    if let Event::ResponseCompleted(value) = &mut response.event {
        value.conversation = 7;
        value.response = 7;
        value.usage.input_tokens = Some(1200);
    }
    values.splice(5..5, [child, request, response]);
    for (position, value) in values.iter_mut().enumerate() {
        value.t_ms = u64::try_from(position).expect("small position");
    }
    assert!(matches!(observe(&values).cache_reuse(0.85, 1), Measure::Unavailable { .. }));
}

fn spend_ceiling_uses_exact_currency_and_never_charges_terminals_again() {
    use smith_bench::EventCheck;
    let mut values = balanced();
    let check = EventCheck::SpendWithin { ceiling_usd: 3.0 };
    assert!(matches!(observe(&values).check(&check, "task").passed, Measure::Unavailable { .. }));
    let judged = observe(&values).check_with_currency(&check, "task", 1);
    assert_eq!(judged.passed, Measure::Observed { value: true });
    values.insert(5, values[4].clone());
    assert_eq!(observe(&values).check_with_currency(&check, "task", 1).passed, Measure::Observed { value: true });
    let below = f64::from_bits(3.0_f64.to_bits() - 1);
    assert_eq!(
        observe(&values).check_with_currency(&EventCheck::SpendWithin { ceiling_usd: below }, "task", 1).passed,
        Measure::Observed { value: false }
    );
    assert!(matches!(observe(&values).check_with_currency(&check, "task", 0).passed, Measure::Unavailable { .. }));
    let mut values = balanced();
    let large = (1_u64 << 53) + 1;
    if let Event::ResponseCompleted(value) = &mut values[4].event {
        value.spent = large;
    }
    if let Event::RunCompleted(value) = &mut values[6].event {
        value.spent = large;
    }
    let check = EventCheck::SpendWithin { ceiling_usd: 9_007_199_254_740_992.0 };
    assert_eq!(observe(&values).check_with_currency(&check, "task", 1).passed, Measure::Observed { value: false });
    let check = EventCheck::SpendWithin { ceiling_usd: f64::MAX };
    assert_eq!(observe(&values).check_with_currency(&check, "task", 1).passed, Measure::Observed { value: true });
    let check = EventCheck::SpendWithin { ceiling_usd: f64::from_bits(1) };
    assert_eq!(observe(&values).check_with_currency(&check, "task", 1).passed, Measure::Observed { value: false });
}

fn spend_ceiling_retains_over_budget_snapshots_and_multiple_run_totals() {
    let check = smith_bench::EventCheck::SpendWithin { ceiling_usd: 3.0 };
    let mut values = balanced();
    let mut budget = record(include_str!("../../crates/smith-events/golden/v4/budget_ordinary.jsonl"));
    if let Event::Budget(value) = &mut budget.event {
        value.run = 0;
        value.spend.used = 4;
    }
    values.insert(5, budget);
    for (position, value) in values.iter_mut().enumerate() {
        value.t_ms = u64::try_from(position).expect("small position");
    }
    assert_eq!(observe(&values).check_with_currency(&check, "task", 1).passed, Measure::Observed { value: false });
    let mut values = balanced();
    values.extend(balanced());
    assert_eq!(observe(&values).check_with_currency(&check, "task", 1).passed, Measure::Observed { value: false });
    let check = smith_bench::EventCheck::SpendWithin { ceiling_usd: 6.0 };
    assert_eq!(observe(&values).check_with_currency(&check, "task", 1).passed, Measure::Observed { value: true });
}

// Encode each unchanged synthetic outside record once. Every control still
// goes through Reader; mutations are encoded anew instead of borrowing bytes
// whose fields no longer match. This keeps all cases within the offline budget.
struct Visible {
    records: Vec<Record>,
    lines: Vec<Vec<u8>>,
}

impl Visible {
    fn new(records: Vec<Record>) -> Self {
        let lines = records.iter().map(|record| bytes(std::slice::from_ref(record))).collect();
        Self { records, lines }
    }
}

fn changed_line(record: &Record) -> Vec<u8> {
    bytes(std::slice::from_ref(record))
}

static VISIBLE_MAIN: std::sync::LazyLock<Visible> = std::sync::LazyLock::new(|| Visible::new(build_visible_budget()));

static VISIBLE_CHILD: std::sync::LazyLock<Visible> = std::sync::LazyLock::new(|| Visible::new(build_visible_child()));

fn visible_budget() -> Vec<Record> {
    VISIBLE_MAIN.records.clone()
}

fn visible_child() -> Vec<Record> {
    VISIBLE_CHILD.records.clone()
}

fn observe_visibility(records: &[Record]) -> Observer {
    let original = match records.len() {
        9 => Some(&*VISIBLE_MAIN),
        15 => Some(&*VISIBLE_CHILD),
        _ => None,
    };
    observe_against(records, original)
}

fn observe_against(records: &[Record], original: Option<&Visible>) -> Observer {
    let mut reported = records.to_vec();
    common::events::inventory(&mut reported);
    let records = reported.as_slice();
    // These synthetic exchanges use at least four entries per list, plus their
    // complete emitted inventory. Reserve
    // their real receiving envelope instead of each maximum codec fixture's
    // quadratic list envelope. Every case still enters the outside Reader;
    // the full golden sweep above retains its original maximum receiving cap.
    let items = records
        .iter()
        .filter_map(|record| match &record.event {
            Event::SessionEnded(ended) => Some(ended.emitted.len()),
            Event::Budget(_)
            | Event::SessionStarted(_)
            | Event::RunStarted(_)
            | Event::RunCompleted(_)
            | Event::ConversationOpened(_)
            | Event::ConversationClosed(_)
            | Event::ResponseStarted(_)
            | Event::ResponseCompleted(_)
            | Event::TextDelta(_)
            | Event::ToolStarted(_)
            | Event::JobEnded(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Notice(_) => None,
        })
        .max()
        .unwrap_or(4)
        .max(4);
    let limits = Limits { items, ..LIMITS };
    let mut lines = Vec::with_capacity(records.len());
    for record in records {
        let unchanged = original.and_then(|original| {
            original.records.iter().zip(&original.lines).find_map(|(value, line)| (value == record).then_some(line))
        });
        if let Some(line) = unchanged {
            lines.push(line.clone());
        } else {
            // Equality includes every field and timestamp. A changed value
            // always gets a fresh codec line; repeated identical controls only
            // reuse those outside bytes, never the Reader's parsed records.
            lines.push(changed_line(record));
        }
    }
    // The fixture's complete codec lines provide their joint receiving bound.
    // Keep all three semantic caps and feed every outside byte through Reader;
    // independent maxima need not allocate a Cartesian envelope for each case.
    let ceiling =
        u32::try_from(lines.iter().map(Vec::len).max().expect("fixture records")).expect("bounded fixture writer");
    let mut observer = Observer::with_ceiling(&limits, ceiling).expect("writer-derived receiving bounds");
    for line in &lines {
        observer.feed(line).expect("exact outside record");
    }
    observer.finish().expect("complete framing");
    observer
}

fn build_visible_budget() -> Vec<Record> {
    let mut values = balanced();
    if let Event::RunStarted(value) = &mut values[1].event {
        value.budget = smith_events::Budget {
            turns: 10,
            spend: 100,
            time_ms: 10_000,
            reserve: smith_events::Reserve { turns: 2, time_ms: 2000, spend: 10 },
        };
    }
    if let Event::ResponseStarted(value) = &mut values[3].event {
        value.tool_choice = smith_events::ToolChoice::Auto;
        value.prompt.as_mut().expect("captured prompt").system = Some(Box::from(&b"Host instructions\n## Budget\n\nAt open, you have 10 turns and 9 seconds of wall time. Spend is bounded too.\nKeep 2 turns and 2 seconds to finish. When the ordinary budget is spent, live children close and you finish on that reserve.\n"[..]));
    }
    if let Event::RunCompleted(value) = &mut values[6].event {
        value.wind_down = false;
    }
    let budget = Record {
        t_ms: 5,
        event: Event::Budget(smith_events::BudgetStanding {
            call: None,
            run: 0,
            stage: smith_events::BudgetStage::Ordinary,
            turns: smith_events::Quantity { used: 1, limit: 10 },
            spend: smith_events::Quantity { used: 3, limit: 100 },
            time_ms: smith_events::Quantity { used: 4, limit: 10_000 },
            reserve: smith_events::Reserve { turns: 2, time_ms: 2000, spend: 10 },
        }),
    };
    values.insert(5, budget);
    for (index, value) in values.iter_mut().enumerate() {
        value.t_ms = u64::try_from(index).expect("small index");
    }
    values
}

fn build_visible_child() -> Vec<Record> {
    let mut values = visible_budget();
    let mut opened = values[2].clone();
    if let Event::ConversationOpened(value) = &mut opened.event {
        value.kind = smith_events::ConversationKind::Child;
        value.conversation = 1;
        value.parent = Some(0);
        value.call = Some(Box::from(&b"child-call"[..]));
        value.share = Some(smith_events::Share { turns: 2, time_ms: 1000, spend: 10 });
    }
    let mut request = values[3].clone();
    if let Event::ResponseStarted(value) = &mut request.event {
        value.conversation = 1;
        value.response = 1;
        value.prompt.as_mut().expect("child prompt").system = Some(Box::from(&b"Child instructions\n## Your share\n\nAt open, your share is 2 turns and 1 second. Your answer may hold at most 1024 bytes; anything past it is cut with a marker. Your last turn has no tools: answer with what you have.\n"[..]));
    }
    let mut response = values[4].clone();
    if let Event::ResponseCompleted(value) = &mut response.event {
        value.conversation = 1;
        value.response = 1;
    }
    let mut snapshot = values[5].clone();
    if let Event::Budget(value) = &mut snapshot.event {
        value.turns.used = 2;
        value.time_ms.used = 8;
    }
    let mut returned = record(include_str!("../../crates/smith-events/golden/v4/tool_completed_everything.jsonl"));
    if let Event::ToolCompleted(value) = &mut returned.event {
        value.run = 0;
        value.conversation = 0;
        value.call = Box::from(&b"child-call"[..]);
        value.child = Some(smith_events::ChildCompleted {
            conversation: 1,
            stop: smith_events::ChildStop::Budget,
            turns: 2,
            time_ms: 100,
            spent: 0,
            usage: smith_events::Usage {
                input_tokens: Some(0),
                cache_read_tokens: Some(0),
                cache_write_tokens: Some(0),
                output_tokens: Some(0),
                reasoning_tokens: None,
            },
        });
    }
    let mut closed = values[6].clone();
    if let Event::ConversationClosed(value) = &mut closed.event {
        value.conversation = 1;
    }
    let mut rendered = values[3].clone();
    if let Event::ResponseStarted(value) = &mut rendered.event {
        value.response = 2;
        let prompt = value.prompt.as_mut().expect("rendered parent request");
        prompt.system = None;
        let mut blocks = skein_lib::List::with_capacity(1);
        blocks.push(smith_events::Block::Result {
                call: Box::from(&b"child-call"[..]),
                error: false,
                text: Box::from(&b"stop=budget\nSurvey complete.\n\nChild usage: used 2 turns and less than a second; stopped for budget: yes; ordinary left 6 turns and 7 seconds."[..]),
            }).expect("child result");
        prompt.messages = skein_lib::List::with_capacity(1);
        prompt
            .messages
            .push(smith_events::Message { role: smith_events::Role::User, blocks })
            .expect("rendered message");
    }
    let mut answered = values[4].clone();
    if let Event::ResponseCompleted(value) = &mut answered.event {
        value.response = 2;
        value.spent = 0;
        value.usage = smith_events::Usage {
            input_tokens: Some(0),
            cache_read_tokens: Some(0),
            cache_write_tokens: Some(0),
            output_tokens: Some(0),
            reasoning_tokens: None,
        };
    }
    values.splice(6..6, [opened, request, response, snapshot, returned, closed, rendered, answered]);
    for (index, value) in values.iter_mut().enumerate() {
        value.t_ms = u64::try_from(index).expect("small index");
    }
    values
}

fn budget_visibility_joins_immutable_openings_and_latest_child_usage_without_reserve_credit() {
    let check = smith_bench::EventCheck::BudgetVisible;
    assert_eq!(observe_visibility(&visible_budget()).check(&check, "task").passed, Measure::Observed { value: true });
    let mut values = visible_child();
    assert_eq!(observe_visibility(&values).check(&check, "task").passed, Measure::Observed { value: true });
    if let Event::ResponseStarted(value) = &mut values[12].event {
        let message = value.prompt.as_mut().expect("prompt").messages.get_mut(0).expect("rendered message");
        if let smith_events::Block::Result { text, .. } = message.blocks.get_mut(0).expect("rendered block") {
            *text = Box::from(&b"\n\nChild usage: used 2 turns and less than a second; stopped for budget: yes; ordinary left 7 turns and 7 seconds."[..]);
        }
    }
    assert_eq!(
        observe_visibility(&values).check(&check, "task").passed,
        Measure::Observed { value: false },
        "an older snapshot is not the returned child's report"
    );
    let mut values = visible_child();
    if let Event::ResponseStarted(value) = &mut values[12].event {
        value.prompt.as_mut().expect("prompt").messages = skein_lib::List::with_capacity(0);
    }
    assert!(matches!(observe_visibility(&values).check(&check, "task").passed, Measure::Unavailable { .. }));
    let mut values = visible_child();
    if let Event::ResponseStarted(value) = &mut values[12].event {
        let message = value.prompt.as_mut().expect("prompt").messages.get_mut(0).expect("rendered message");
        if let smith_events::Block::Result { call, .. } = message.blocks.get_mut(0).expect("rendered block") {
            *call = Box::from(&b"foreign-rendered-call"[..]);
        }
    }
    assert!(
        matches!(observe_visibility(&values).check(&check, "task").passed, Measure::Unavailable { .. }),
        "rendered text must join the actual parent call"
    );
    let mut values = visible_child();
    let mut later = values[9].clone();
    later.t_ms = values[12].t_ms;
    if let Event::Budget(value) = &mut later.event {
        value.turns.used = 3;
        value.time_ms.used = 2000;
    }
    values.insert(12, later);
    assert_eq!(
        observe_visibility(&values).check(&check, "task").passed,
        Measure::Observed { value: true },
        "the suffix keeps the return-time budget even when a later request follows another snapshot"
    );
    for capture in [smith_events::Capture::None, smith_events::Capture::Calls] {
        let mut values = visible_child();
        common::events::inventory(&mut values);
        assert!(
            matches!(captured(&values, &capture, &LIMITS).check(&check, "task").passed, Measure::Unavailable { .. }),
            "without captured request content, budget visibility is unavailable"
        );
    }
    let mut values = visible_child();
    if let Event::ToolCompleted(value) = &mut values[10].event {
        value.call = Box::from(&b"foreign"[..]);
    }
    assert!(matches!(observe_visibility(&values).check(&check, "task").passed, Measure::Unavailable { .. }));
    for part in 0..4 {
        let mut values = visible_child();
        if let Event::ConversationOpened(value) = &mut values[6].event {
            let share = value.share.as_mut().expect("effective share");
            match part {
                0 => share.turns = 8,
                1 => share.spend = 88,
                2 => share.time_ms = 8000,
                _ => share.spend = 0,
            }
        }
        assert_eq!(
            observe_visibility(&values).check(&check, "task").passed,
            Measure::Observed { value: false },
            "reserve part {part}"
        );
    }
    let mut values = visible_child();
    if let Event::ConversationOpened(value) = &mut values[6].event {
        value.share.as_mut().expect("effective share").time_ms = 7995;
    }
    assert!(
        matches!(observe_visibility(&values).check(&check, "task").passed, Measure::Unavailable { .. }),
        "submillisecond boundary is not silently exact"
    );
    let mut values = visible_child();
    if let Event::SessionEnded(value) = &mut values.last_mut().expect("terminal").event {
        value.loss.events = 1;
    }
    assert!(matches!(observe_visibility(&values).check(&check, "task").passed, Measure::Unavailable { .. }));
    let mut values = visible_child();
    if let Event::ResponseStarted(value) = &mut values[3].event {
        value.prompt.as_mut().expect("prompt").system = None;
    }
    assert!(matches!(observe_visibility(&values).check(&check, "task").passed, Measure::Unavailable { .. }));
}

fn budget_visibility_preserves_wind_down_metadata_and_native_request_choice() {
    let check = smith_bench::EventCheck::BudgetVisible;
    let mut values = visible_budget();
    if let Event::Budget(value) = &mut values[5].event {
        value.stage = smith_events::BudgetStage::WindDown;
    }
    if let Event::RunCompleted(value) = &mut values[7].event {
        value.wind_down = true;
    }
    let mut request = values[3].clone();
    if let Event::ResponseStarted(value) = &mut request.event {
        value.response = 1;
        value.tool_choice = smith_events::ToolChoice::Only;
        let prompt = value.prompt.as_mut().expect("prompt");
        prompt.system = None;
        let mut blocks = skein_lib::List::with_capacity(1);
        blocks.push(smith_events::Block::Text { text: Box::from(&b"The ordinary budget is spent. Finish now with what you have. You have at most 2 final completions and 2 seconds to finish. Use only finish, or deliver when offered."[..]) }).expect("note");
        prompt.messages = skein_lib::List::with_capacity(1);
        prompt.messages.push(smith_events::Message { role: smith_events::Role::User, blocks }).expect("note message");
    }
    values.insert(6, request);
    for (index, value) in values.iter_mut().enumerate() {
        value.t_ms = u64::try_from(index).expect("small index");
    }
    assert_eq!(observe_visibility(&values).check(&check, "task").passed, Measure::Observed { value: true });
    if let Event::ResponseStarted(value) = &mut values[6].event {
        value.tool_choice = smith_events::ToolChoice::Auto;
    }
    assert_eq!(observe_visibility(&values).check(&check, "task").passed, Measure::Observed { value: false });
    if let Event::ResponseStarted(value) = &mut values[6].event {
        value.tool_choice = smith_events::ToolChoice::Only;
        value.prompt.as_mut().expect("prompt").messages = skein_lib::List::with_capacity(0);
    }
    assert_eq!(observe_visibility(&values).check(&check, "task").passed, Measure::Observed { value: false });
    let mut values = visible_budget();
    if let Event::Budget(value) = &mut values[5].event {
        value.reserve.turns = 3;
    }
    assert_eq!(observe_visibility(&values).check(&check, "task").passed, Measure::Observed { value: false });
    let mut values = visible_budget();
    if let Event::Budget(value) = &mut values[5].event {
        value.stage = smith_events::BudgetStage::Unknown(Box::from(&b"future"[..]));
    }
    assert!(matches!(observe_visibility(&values).check(&check, "task").passed, Measure::Unavailable { .. }));
}

fn health(records: &[Record]) -> smith_bench::Health {
    observe_visibility(records).health(Exit::Code(0), Forced::No, 10)
}

fn health_passed(health: &smith_bench::Health, quantity: &str) -> Measure<bool> {
    health
        .passive_checks
        .iter()
        .find(|check| check.check == format!("headroom/{quantity}"))
        .expect("named headroom check")
        .passed
        .clone()
}

fn headroom_uses_actual_model_and_admission_bounds_with_exact_half_warnings() {
    let result = health(&visible_budget());
    for (name, fraction) in [("prompt-tokens", 0.002), ("turns", 0.1), ("spend", 0.03)] {
        assert_eq!(result.headroom[name].fraction_used, Measure::Observed { value: fraction });
        assert!(!result.headroom[name].warning);
        assert_eq!(health_passed(&result, name), Measure::Observed { value: true });
    }
    for name in ["request-bytes", "first-byte", "largest-gap", "peak-rss"] {
        assert!(matches!(result.headroom[name].fraction_used, Measure::Unavailable { .. }));
    }
    let mut values = visible_budget();
    if let Event::RunCompleted(value) = &mut values[7].event {
        value.turns = u64::MAX;
    }
    assert_eq!(
        health(&values).headroom["turns"].fraction_used,
        Measure::Observed { value: 0.1 },
        "durable activation turns are not charged budget turns"
    );
    // A ratio can round to 0.5 even below half; the warning uses integer evidence.
    for (used, warning) in [(u64::MAX / 2, false), (u64::MAX / 2 + 1, true)] {
        let mut values = visible_budget();
        for record in &mut values {
            match &mut record.event {
                Event::RunStarted(value) => {
                    value.budget.turns = u64::MAX;
                    value.budget.spend = u64::MAX;
                }
                Event::ConversationOpened(value) => value.model.window_tokens = u64::MAX,
                Event::ResponseStarted(value) => value.model.window_tokens = u64::MAX,
                Event::ResponseCompleted(value) => {
                    value.usage.input_tokens = Some(used);
                    value.usage.cache_read_tokens = Some(0);
                    value.usage.cache_write_tokens = Some(0);
                }
                Event::Budget(value) => {
                    value.turns = smith_events::Quantity { used, limit: u64::MAX };
                    value.spend = smith_events::Quantity { used, limit: u64::MAX };
                }
                Event::SessionStarted(_)
                | Event::SessionEnded(_)
                | Event::RunCompleted(_)
                | Event::ConversationClosed(_)
                | Event::TextDelta(_)
                | Event::ToolStarted(_)
                | Event::JobEnded(_)
                | Event::ToolCompleted(_)
                | Event::CheckStarted(_)
                | Event::CheckCompleted(_)
                | Event::MessageReceived(_)
                | Event::MessageRead(_)
                | Event::MessageUnread(_)
                | Event::Notice(_) => {}
            }
        }
        let result = health(&values);
        for name in ["prompt-tokens", "turns", "spend"] {
            assert_eq!(result.headroom[name].warning, warning, "{name}, used={used}");
            assert_eq!(health_passed(&result, name), Measure::Observed { value: true });
        }
    }
}

fn headroom_preserves_known_violations_and_missing_scope_uncertainty() {
    let mut values = visible_budget();
    if let Event::ResponseCompleted(value) = &mut values[4].event {
        value.usage.input_tokens = Some(u64::MAX);
        value.usage.cache_read_tokens = Some(u64::MAX);
        value.usage.cache_write_tokens = Some(u64::MAX);
    }
    if let Event::Budget(value) = &mut values[5].event {
        value.turns.used = 11;
        value.spend.used = 101;
    }
    if let Event::SessionEnded(value) = &mut values[8].event {
        value.loss.events = 1;
    }
    let result = health(&values);
    for name in ["prompt-tokens", "turns", "spend"] {
        assert_eq!(health_passed(&result, name), Measure::Observed { value: false });
        assert!(matches!(result.headroom[name].fraction_used, Measure::LowerBound { .. }));
        assert!(result.headroom[name].warning);
    }
    let mut values = visible_budget();
    if let Event::SessionEnded(value) = &mut values[8].event {
        value.loss.events = 1;
    }
    let result = health(&values);
    for name in ["prompt-tokens", "turns", "spend"] {
        assert!(matches!(health_passed(&result, name), Measure::Unavailable { .. }));
        assert!(matches!(result.headroom[name].fraction_used, Measure::LowerBound { .. }));
    }
    for mutation in 0..6 {
        let mut values = visible_budget();
        match mutation {
            0 => {
                if let Event::ResponseCompleted(value) = &mut values[4].event {
                    value.usage.cache_write_tokens = None;
                }
            }
            1 => {
                if let Event::ResponseCompleted(value) = &mut values[4].event {
                    value.conversation = 99;
                }
            }
            2 => {
                if let Event::ResponseStarted(value) = &mut values[3].event {
                    value.model.window_tokens += 1;
                }
            }
            3 => {
                values.remove(4);
            }
            4 => {
                if let Event::ConversationOpened(value) = &mut values[2].event {
                    value.model.window_tokens = 0;
                }
            }
            5 => {
                values.remove(3);
            }
            _ => unreachable!("fixed controls"),
        }
        assert!(
            matches!(health_passed(&health(&values), "prompt-tokens"), Measure::Unavailable { .. }),
            "mutation {mutation}"
        );
    }
    let mut values = visible_budget();
    if let Event::Budget(value) = &mut values[5].event {
        value.turns.limit = 9;
        value.spend.limit = 99;
    }
    let result = health(&values);
    assert!(matches!(health_passed(&result, "turns"), Measure::Unavailable { .. }));
    assert!(matches!(health_passed(&result, "spend"), Measure::Unavailable { .. }));
    values.remove(1);
    let result = health(&values);
    assert!(matches!(health_passed(&result, "turns"), Measure::Unavailable { .. }));
    assert!(matches!(health_passed(&result, "spend"), Measure::Unavailable { .. }));
}

fn headroom_maxima_compare_each_conversation_ratio_instead_of_aggregating_usage() {
    let mut values = visible_child();
    if let Event::ConversationOpened(value) = &mut values[6].event {
        value.model.endpoint = Box::from(&b"another-endpoint"[..]);
        value.model.window_tokens = 100;
    }
    if let Event::ResponseStarted(value) = &mut values[7].event {
        value.model.endpoint = Box::from(&b"another-endpoint"[..]);
        value.model.window_tokens = 100;
    }
    if let Event::ResponseCompleted(value) = &mut values[8].event {
        value.usage.input_tokens = Some(41);
    }
    let result = health(&values);
    assert_eq!(result.headroom["prompt-tokens"].fraction_used, Measure::Observed { value: 0.5 });
    assert!(result.headroom["prompt-tokens"].warning);
    assert_eq!(health_passed(&result, "prompt-tokens"), Measure::Observed { value: true });
    assert!(
        result
            .passive_checks
            .iter()
            .find(|check| check.check == "headroom/prompt-tokens")
            .expect("prompt headroom")
            .evidence
            .iter()
            .any(|item| item.contains("conversation-1/response-1"))
    );
    // Same very large windows: three disjoint counts exceed u64 but stay exact
    // through ratio ordering; multiplying complete numerators would overflow.
    for record in &mut values {
        match &mut record.event {
            Event::ConversationOpened(value) => value.model.window_tokens = u64::MAX,
            Event::ResponseStarted(value) => value.model.window_tokens = u64::MAX,
            Event::ResponseCompleted(value) => {
                value.usage.input_tokens = Some(u64::MAX);
                value.usage.cache_read_tokens = Some(u64::MAX);
                value.usage.cache_write_tokens = Some(u64::MAX - u64::from(value.conversation == 0));
            }
            Event::Budget(_)
            | Event::SessionStarted(_)
            | Event::SessionEnded(_)
            | Event::RunStarted(_)
            | Event::RunCompleted(_)
            | Event::ConversationClosed(_)
            | Event::TextDelta(_)
            | Event::ToolStarted(_)
            | Event::JobEnded(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Notice(_) => {}
        }
    }
    let result = health(&values);
    assert_eq!(result.headroom["prompt-tokens"].fraction_used, Measure::Observed { value: 3.0 });
    assert_eq!(health_passed(&result, "prompt-tokens"), Measure::Observed { value: false });
    assert!(
        result
            .passive_checks
            .iter()
            .find(|check| check.check == "headroom/prompt-tokens")
            .expect("largest exact ratio")
            .evidence
            .iter()
            .any(|item| item.contains("conversation-1/response-1"))
    );
}

fn smoke_usage_sums_compare_optional_raw_counts_once_and_do_not_fill_null_with_zero() {
    let mut values = balanced();
    let usage = match &values[4].event {
        Event::ResponseCompleted(value) => value.usage.clone(),
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
        | Event::JobEnded(_)
        | Event::ToolCompleted(_)
        | Event::CheckStarted(_)
        | Event::CheckCompleted(_)
        | Event::MessageReceived(_)
        | Event::MessageRead(_)
        | Event::MessageUnread(_)
        | Event::Notice(_) => panic!("response fixture"),
    };
    if let Event::RunCompleted(value) = &mut values[6].event {
        value.usage = usage.clone();
    }
    let predicate = smith_bench::EventCheck::UsageSums;
    assert_eq!(observe(&values).check(&predicate, "task").passed, Measure::Observed { value: true });
    if let Event::RunCompleted(value) = &mut values[6].event {
        value.usage.output_tokens = Some(6);
    }
    assert_eq!(observe(&values).check(&predicate, "task").passed, Measure::Observed { value: false });
    if let Event::RunCompleted(value) = &mut values[6].event {
        value.usage.output_tokens = Some(5);
    }
    if let Event::ResponseCompleted(value) = &mut values[4].event {
        value.usage.cache_read_tokens = None;
    }
    assert!(matches!(observe(&values).check(&predicate, "task").passed, Measure::Unavailable { .. }));
    values = balanced();
    if let Event::RunCompleted(value) = &mut values[6].event {
        value.usage = usage;
    }
    values.insert(5, values[4].clone());
    assert_eq!(observe(&values).check(&predicate, "task").passed, Measure::Observed { value: true });
    if let Event::ResponseCompleted(value) = &mut values[5].event {
        value.conversation += 1;
    }
    assert_eq!(observe(&values).check(&predicate, "task").passed, Measure::Observed { value: false });
}

#[test]
fn typed_event_stream_classification_usage_and_author_predicates() {
    delegation::a_childs_share_stop_turns_and_time_are_recorded();
    delegation::a_clamped_share_names_its_part_and_bound();
    delegation::parent_blocked_time_is_the_union_of_sub_agent_spans();
    delegation::event_loss_leaves_delegation_unavailable();
    delegation::an_unknown_child_stop_leaves_children_unavailable();
    smoke_usage_sums_compare_optional_raw_counts_once_and_do_not_fill_null_with_zero();
    expected_entrance_refusal_requires_captured_reason_and_no_request();
    every_codec_golden_remains_readable_as_outside_recorded_bytes();
    unread_resources_stay_null_and_do_not_change_terminals_or_response_usage();
    emitted_inventory_compares_received_types_with_each_destination_loss();
    event_chunks_keep_exact_usage_and_authoritative_exits();
    agent_exit_tracks_the_sent_answer_and_forced_pending_stays_unknown();
    repeated_usage_is_immutable_but_duplicate_terminals_break_pairing();
    child_and_compaction_usage_belongs_to_its_native_scope_and_run();
    unknown_types_are_skipped_and_version_or_partial_records_fault();
    writer_ceiling_is_independent_and_preserves_every_semantic_cap();
    passive_negatives_preserve_loss_scope_and_exact_spend();
    observed_passive_contradictions_survive_loss_but_missing_records_do_not();
    author_predicates_use_typed_records_and_missing_content_stays_unavailable();
    block_cardinalities_use_completion_metadata_under_every_capture_policy();
    message_refusals_and_read_unread_each_settle_one_admission_attempt();
    prompt_windows_use_exact_disjoint_usage_and_the_actual_request_model();
    child_share_and_main_return_checks_join_the_parent_call_and_emission_order();
    a_capped_child_final_turn_requires_no_tools_and_its_captured_native_answer();
    cache_reuse_sums_eligible_predecessors_with_exact_misses_and_window_boundaries();
    cache_reuse_does_not_invent_usage_or_join_other_conversations();
    spend_ceiling_uses_exact_currency_and_never_charges_terminals_again();
    spend_ceiling_retains_over_budget_snapshots_and_multiple_run_totals();
    budget_visibility_joins_immutable_openings_and_latest_child_usage_without_reserve_credit();
    budget_visibility_preserves_wind_down_metadata_and_native_request_choice();
    headroom_uses_actual_model_and_admission_bounds_with_exact_half_warnings();
    headroom_preserves_known_violations_and_missing_scope_uncertainty();
    headroom_maxima_compare_each_conversation_ratio_instead_of_aggregating_usage();
}

fn emitted_inventory_compares_received_types_with_each_destination_loss() {
    let mut values = balanced();
    if let Event::SessionEnded(ended) = &mut values[7].event {
        ended.emitted.get_mut(0).expect("one type").count += 1;
    }
    let mut observer = Observer::new(&LIMITS).expect("bounds");
    observer.feed(&bytes(&values)).expect("mismatched typed inventory");
    assert_eq!(passed(&observer, "nothing-lost"), Measure::Observed { value: false });
    if let Event::SessionStarted(started) = &mut values[0].event {
        started.delivery = smith_events::Delivery::BestEffort;
    }
    if let Event::SessionEnded(ended) = &mut values[7].event {
        ended.loss.events = 1;
    }
    let mut observer = Observer::new(&LIMITS).expect("bounds");
    observer.feed(&bytes(&values)).expect("reported best-effort loss");
    assert_eq!(passed(&observer, "nothing-lost"), Measure::Observed { value: true }, "{:?}", observer.sessions()[0]);
    let mut observer = Observer::new(&LIMITS).expect("bounds");
    for value in &balanced() {
        if matches!(value.event, Event::SessionEnded(_)) {
            observer.feed(b"{\"v\":4,\"type\":\"future.record\",\"t_ms\":6}\n").expect("future record");
        }
        observer.feed(&bytes(std::slice::from_ref(value))).expect("typed stream");
    }
    assert!(matches!(passed(&observer, "nothing-lost"), Measure::Unavailable { .. }));
}

fn expected_entrance_refusal_requires_captured_reason_and_no_request() {
    let original = balanced();
    let mut values = vec![original[0].clone(), original[6].clone(), original[7].clone()];
    if let Event::RunCompleted(value) = &mut values[1].event {
        value.status = smith_events::Status::Refused;
        value.result = None;
        value.failure = Some(smith_events::AnswerFailure {
            state: None,
            code: None,
            program: None,
            exit: None,
            detail: None,
            reset_ms: None,
            used: None,
            bound: None,
            dropped: None,
            ended: None,
            turn: None,
            endpoint: None,
            model: None,
            attempts: None,
            class: smith_events::AnswerClass::Invalid,
            completion: None,
            account: None,
            which: Some(Box::from(&b"budget reserve"[..])),
            reason: Some(Box::from(&b"budget reserve turns 2 must be below total turns 1"[..])),
        });
    }
    if let Event::SessionEnded(value) = &mut values[2].event {
        value.exit = 2;
    }
    let predicate = smith_bench::EventCheck::RunCompleted {
        status: "refused".into(),
        class: Some("invalid".into()),
        which: None,
        reason: Some("budget.*reserve".into()),
    };
    let observer = observe_against(&values, None);
    assert_eq!(
        observer.classify(Exit::Code(2), Forced::No).end,
        End::Refused { setup: "budget reserve turns 2 must be below total turns 1".into() }
    );
    assert_eq!(observer.check(&predicate, "task").passed, Measure::Observed { value: true });
    assert_eq!(observer.check(&smith_bench::EventCheck::NoRequest, "task").passed, Measure::Observed { value: true });
    if let Event::RunCompleted(value) = &mut values[1].event {
        value.failure.as_mut().expect("invalid refusal").reason = Some(Box::from(&b"unrelated setting"[..]));
    }
    assert_eq!(observe_against(&values, None).check(&predicate, "task").passed, Measure::Observed { value: false });
    if let Event::RunCompleted(value) = &mut values[1].event {
        value.failure.as_mut().expect("invalid refusal").reason = None;
    }
    assert!(matches!(observe_against(&values, None).check(&predicate, "task").passed, Measure::Unavailable { .. }));
}

#[test]
fn the_resolved_window_comes_from_each_agents_stream() {
    use smith_bench::agent::codex::{RolloutFile, Rollouts};
    let mut identity = smith_bench::read_result(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/results/attempt.json"),
    )
    .expect("recorded identity")
    .identity
    .model;
    observe(&balanced()).resolve_model(&mut identity);
    assert_eq!(identity.resolved_model, Measure::Observed { value: "model".into() });
    assert_eq!(identity.resolved_window, Measure::Observed { value: 10_000 });
    let rollouts = Rollouts {
        responses: vec![],
        children: vec![],
        files: vec![RolloutFile {
            thread: "root".into(),
            bytes: include_bytes!("recorded/codex/model-window.jsonl").to_vec(),
        }],
    };
    rollouts.resolve_model("root", &mut identity).expect("root model metadata");
    assert_eq!(identity.resolved_model, Measure::Observed { value: "gpt-6-astra".into() });
    assert_eq!(identity.resolved_window, Measure::Observed { value: 258_400 });
}
