//! Task predicates are evaluated over typed records, with unavailable evidence
//! retained explicitly (benchmarks.md, sections3.2/11.1).

use std::collections::{BTreeMap, BTreeSet};

use smith_events::{Block, ConversationKind, Event, Record};

use crate::{Before, CheckVerdict, EventCheck, Measure};

use super::{Observer, Session, names};

fn measured(value: bool) -> Measure<bool> {
    Measure::Observed { value }
}

fn unavailable(reason: &str) -> Measure<bool> {
    Measure::unavailable(reason)
}

fn records(observer: &Observer) -> Vec<&Record> {
    observer.sessions.iter().flat_map(|session| &session.records).collect()
}

fn tool_call(observer: &Observer, check: &EventCheck) -> Measure<bool> {
    let EventCheck::ToolCall { tool, count, input_bytes_at_least, verdict, exit, clamped } = check else {
        return unavailable("not a tool predicate");
    };
    let mut starts = Vec::new();
    let mut ends = BTreeMap::new();
    for (index, session) in observer.sessions.iter().enumerate() {
        for record in &session.records {
            match &record.event {
                Event::ToolStarted(value) if value.tool.as_ref() == tool.as_bytes() => starts.push((index, value)),
                Event::ToolCompleted(value) => {
                    ends.insert((index, value.run, value.conversation, value.call.clone()), value);
                }
                Event::MessageReceived(_)
                | Event::MessageRead(_)
                | Event::MessageUnread(_)
                | Event::Budget(_)
                | Event::SessionStarted(_)
                | Event::SessionEnded(_)
                | Event::RunStarted(_)
                | Event::RunCompleted(_)
                | Event::ConversationOpened(_)
                | Event::ConversationClosed(_)
                | Event::ResponseStarted(_)
                | Event::ResponseCompleted(_)
                | Event::TextDelta(_)
                | Event::ToolStarted(_)
                | Event::CheckStarted(_)
                | Event::CheckCompleted(_)
                | Event::JobEnded(_)
                | Event::Notice(_) => {}
            }
        }
    }
    let number = u64::try_from(starts.len()).expect("tool count fits");
    if count.is_some_and(|count| !count.holds(number)) {
        return measured(false);
    }
    let metadata = input_bytes_at_least.is_some() || verdict.is_some() || exit.is_some() || clamped.is_some();
    if starts.is_empty() {
        return measured(!metadata && count.is_some());
    }
    for (index, start) in starts {
        if input_bytes_at_least.is_some_and(|bound| start.input_bytes < bound) {
            return measured(false);
        }
        if verdict.is_some() || exit.is_some() || clamped.is_some() {
            let Some(end) = ends.get(&(index, start.run, start.conversation, start.call.clone())) else {
                return measured(false);
            };
            if verdict.as_ref().is_some_and(|value| names::verdict(&end.verdict) != value.as_bytes())
                || clamped.is_some_and(|value| value != end.clamped)
            {
                return measured(false);
            }
            if let Some(expected) = exit {
                match &end.exit {
                    Some(smith_events::CommandExit::Code { code }) if code == expected => {}
                    Some(smith_events::CommandExit::Code { .. } | smith_events::CommandExit::Signal { .. }) | None => {
                        return measured(false);
                    }
                }
            }
        }
    }
    measured(true)
}

fn completions(session: &Session, before: Before) -> usize {
    let mut ids = BTreeSet::new();
    for record in &session.records {
        match &record.event {
            Event::ToolStarted(value) if before == Before::Finish && value.tool.as_ref() == b"finish" => break,
            Event::RunCompleted(_) if before == Before::Answer => break,
            Event::ResponseCompleted(value) if value.outcome == smith_events::Outcome::Completed => {
                ids.insert((value.run, value.response));
            }
            Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Budget(_)
            | Event::SessionStarted(_)
            | Event::SessionEnded(_)
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
            | Event::Notice(_) => {}
        }
    }
    ids.len()
}

fn first_prompt(observer: &Observer, task: &str) -> Measure<bool> {
    if task.trim_end().is_empty() {
        return unavailable("the task text is empty");
    }
    for record in records(observer) {
        if let Event::ResponseStarted(value) = &record.event {
            let Some(prompt) = &value.prompt else {
                return unavailable("first request prompt was not captured");
            };
            return measured(prompt.messages.iter().any(|message| {
                message.blocks.iter().any(|block| match block {
                    Block::Text { text } => String::from_utf8_lossy(text).contains(task.trim_end()),
                    Block::Unknown { .. }
                    | Block::Refusal { .. }
                    | Block::Call { .. }
                    | Block::Oversized { .. }
                    | Block::Cut { .. }
                    | Block::Result { .. }
                    | Block::Opaque { .. } => false,
                })
            }));
        }
    }
    measured(false)
}

fn calls_one_response(observer: &Observer, tool: &str, at_least: u32) -> Measure<bool> {
    let mut captured = false;
    for record in records(observer) {
        if let Event::ResponseCompleted(value) = &record.event
            && let Some(completion) = &value.completion
        {
            captured = true;
            let count = completion
                .blocks
                .iter()
                .filter(|block| matches!(block, Block::Call { tool: actual, .. } if actual.as_ref() == tool.as_bytes()))
                .count();
            if u64::try_from(count).expect("blocks fit") >= u64::from(at_least) {
                return measured(true);
            }
        }
    }
    if captured { measured(false) } else { unavailable("response call blocks were not captured") }
}

fn children_overlap(observer: &Observer, at_least: u32) -> Measure<bool> {
    let mut largest = 0_usize;
    for session in &observer.sessions {
        let mut openings = BTreeMap::new();
        let mut closures = BTreeMap::new();
        for record in &session.records {
            if let Event::ConversationOpened(value) = &record.event
                && value.kind == ConversationKind::Child
                && openings.insert((value.run, value.conversation), record.t_ms).is_some()
            {
                return unavailable("child conversation has duplicate openings");
            }
            if let Event::ConversationClosed(value) = &record.event {
                closures.insert((value.run, value.conversation), record.t_ms);
            }
        }
        let mut boundaries = BTreeMap::<u64, Vec<(u64, bool)>>::new();
        for ((run, conversation), start) in openings {
            let Some(end) = closures.get(&(run, conversation)) else {
                return unavailable("child closure is required to measure temporal overlap");
            };
            if *end < start {
                return unavailable("child closes before its opening clock");
            }
            if *end == start {
                continue;
            }
            let points = boundaries.entry(run).or_default();
            points.extend([(start, true), (*end, false)]);
        }
        for points in boundaries.values_mut() {
            // Equal-time closures precede openings: touching intervals do not overlap.
            points.sort_unstable();
            let mut active = 0_usize;
            for (_, opening) in points {
                if *opening {
                    active += 1;
                } else {
                    active = active.checked_sub(1).expect("each positive interval opened first");
                }
                largest = largest.max(active);
            }
        }
    }
    measured(u64::try_from(largest).expect("children fit") >= u64::from(at_least))
}

fn block_counts(observer: &Observer, oversized: Option<crate::Count>, cut: Option<crate::Count>) -> Measure<bool> {
    let mut oversized_total = 0_u128;
    let mut cut_total = 0_u128;
    for session in &observer.sessions {
        let pairing = super::pairs::check(session);
        if !pairing.violations.is_empty() || !pairing.missing.is_empty() {
            return unavailable("unpaired response records prevent exact block cardinalities");
        }
        for record in &session.records {
            if let Event::ResponseCompleted(value) = &record.event {
                let Some(next_oversized) = oversized_total.checked_add(u128::from(value.oversized)) else {
                    return unavailable("oversized block aggregate exceeds the exact counter range");
                };
                let Some(next_cut) = cut_total.checked_add(u128::from(value.cut)) else {
                    return unavailable("cut block aggregate exceeds the exact counter range");
                };
                oversized_total = next_oversized;
                cut_total = next_cut;
            }
        }
    }
    let holds = |count: Option<crate::Count>, total: u128| match count {
        Some(crate::Count::Exactly(value)) => total == u128::from(value),
        Some(crate::Count::AtMost(value)) => total <= u128::from(value),
        Some(crate::Count::AtLeast(value)) => total >= u128::from(value),
        None => true,
    };
    measured(holds(oversized, oversized_total) && holds(cut, cut_total))
}

pub(super) fn evaluate(observer: &Observer, check: &EventCheck, task: &str, currency: Option<u64>) -> CheckVerdict {
    if *check == EventCheck::NoRun {
        return CheckVerdict {
            check: check.name().into(),
            passed: measured(observer.ended_without_run()),
            evidence: vec!["Explicit startup-only predicate; no terminal is synthesized".into()],
        };
    }
    let passed = if observer.sessions.is_empty()
        || observer.fault.is_some()
        || observer.sessions.iter().any(|session| super::usage::incomplete(session).is_some())
    {
        unavailable("incomplete event stream prevents a complete task predicate")
    } else {
        if *check == EventCheck::CompactionCarried {
            return super::carried::check(observer);
        }
        if *check == EventCheck::CompactionChoice {
            return super::compaction_choice::check(observer);
        }
        if let EventCheck::CacheReuse { median_at_least, full_misses_per_ten_at_most } = check {
            return super::cache::check(observer, *median_at_least, *full_misses_per_ten_at_most);
        }
        if let EventCheck::ProviderFailure { status_at_least, status_at_most, detail_bytes_at_most } = check {
            return provider_failure(observer, *status_at_least, *status_at_most, *detail_bytes_at_most);
        }
        if let EventCheck::ToolResultBytes { tool, at_least } = check {
            return tool_result_bytes(observer, tool, *at_least);
        }
        if let EventCheck::ToolTimeout { requested_seconds, deadline_ms, clamped, grace_ms, command } = check {
            return tool_timeout(observer, *requested_seconds, *deadline_ms, *clamped, *grace_ms, command.as_deref());
        }
        if let EventCheck::SlowCompletion { longer_than_ms, head_ms, idle_ms, reasoning_bytes } = check {
            return super::slow::check(observer, *longer_than_ms, *head_ms, *idle_ms, *reasoning_bytes);
        }
        evaluate_complete(observer, check, task, currency)
    };
    let evidence = if let EventCheck::ToolPath { tool, path, aliases, verdict } = check {
        vec![format!("required {tool} path {path:?} or {aliases:?}, joined terminal {verdict}")]
    } else {
        Vec::new()
    };
    CheckVerdict { check: check.name().into(), passed, evidence }
}

fn evaluate_complete(observer: &Observer, check: &EventCheck, task: &str, currency: Option<u64>) -> Measure<bool> {
    let records = records(observer);
    match check {
        EventCheck::ToolPath { tool, path, aliases, verdict } => super::tool_path::check(observer, tool, path, aliases, verdict),
        EventCheck::WrittenFile { .. } => unavailable("written file comparison requires the settled workspace"),
        EventCheck::OversizedWrite { .. } => unavailable("oversized write pairing requires its workspace path"),
        EventCheck::FilePieces { .. } => unavailable("file piece comparison requires the settled workspace"),
        EventCheck::SlowCompletion { longer_than_ms, head_ms, idle_ms, reasoning_bytes } =>
            super::slow::check(observer, *longer_than_ms, *head_ms, *idle_ms, *reasoning_bytes).passed,
        EventCheck::UsageSums => super::usage_sums::check(observer),
        EventCheck::JobRunning | EventCheck::CompletionDuringJob | EventCheck::JobTerminal =>
            super::jobs::check(observer, check),
        EventCheck::ToolTimeout { requested_seconds, deadline_ms, clamped, grace_ms, command } =>
            tool_timeout(observer, *requested_seconds, *deadline_ms, *clamped, *grace_ms, command.as_deref()).passed,
        EventCheck::ToolCall { .. } => tool_call(observer, check),
        EventCheck::CallCount { tool, count } => measured(count.holds(u64::try_from(records.iter().filter(|record|
            matches!(&record.event, Event::ToolStarted(value) if value.tool.as_ref() == tool.as_bytes())).count()).expect("tool count fits"))),
        EventCheck::ToolResultBytes { tool, at_least } => tool_result_bytes(observer, tool, *at_least).passed,
        EventCheck::NoVerdict { verdicts } => measured(!records.iter().any(|record| matches!(&record.event,
            Event::ToolCompleted(value) if verdicts.iter().any(|verdict| verdict.as_bytes() == names::verdict(&value.verdict))))),
        EventCheck::Completions { count, before } => measured(count.holds(u64::try_from(observer.sessions.iter()
            .map(|session| completions(session, *before)).sum::<usize>()).expect("completions fit"))),
        EventCheck::FirstRequestHoldsTask => first_prompt(observer, task),
        EventCheck::NoStop { stop } => measured(!records.iter().any(|record| matches!(&record.event,
            Event::ResponseCompleted(value) if value.stop.as_ref().is_some_and(|value| names::stop(value) == stop.as_bytes())))),
        EventCheck::ConversationOpened { kind, count } => {
            let openings = u64::try_from(records.iter().filter(|record|
                matches!(&record.event, Event::ConversationOpened(value) if names::kind(&value.kind) == kind.as_bytes())).count()).expect("scopes fit");
            if kind == "compaction" && openings == 0 {
                match super::threshold::passed_line(observer) {
                    Measure::Observed { value: false } => return unavailable("not exercised: no request passed the compaction line"),
                    Measure::Unavailable { reason } => return unavailable(&reason),
                    Measure::LowerBound { .. } | Measure::Observed { value: true } => {}
                }
            }
            measured(count.holds(openings))
        }
        EventCheck::CallsInOneResponse { tool, at_least } => calls_one_response(observer, tool, *at_least),
        EventCheck::CallsPerResponse { tool, at_most } => super::cache_shape::calls_per_response(observer, tool, *at_most),
        EventCheck::ChildrenInTurn { count } => super::cache_shape::children_in_turn(observer, *count),
        EventCheck::ChildrenOverlap { at_least } => children_overlap(observer, *at_least),
        EventCheck::NoFailure { class } => measured(!records.iter().any(|record| matches!(&record.event,
            Event::ResponseCompleted(value) if value.failure.as_ref().is_some_and(|value| names::failure(&value.class) == class.as_bytes())))),
        EventCheck::RunCompleted { status, class, which, reason } => {
            let terminals: Vec<_> = records.iter().filter_map(|record| match &record.event {
                Event::RunCompleted(value) => Some(value),
                Event::MessageReceived(_) | Event::MessageRead(_) | Event::MessageUnread(_) | Event::Budget(_) | Event::SessionStarted(_) | Event::SessionEnded(_) | Event::RunStarted(_)
                | Event::ConversationOpened(_) | Event::ConversationClosed(_) | Event::ResponseStarted(_) | Event::ResponseCompleted(_)
                | Event::TextDelta(_) | Event::ToolStarted(_) | Event::JobEnded(_) | Event::ToolCompleted(_) | Event::CheckStarted(_)
                | Event::CheckCompleted(_) | Event::Notice(_) => None,
            }).collect();
            if let Some(expected) = which {
                for value in &terminals {
                    let Some(actual) = value.failure.as_ref().and_then(|failure| failure.which.as_ref()) else {
                        return unavailable("run failure typed identity was not retained");
                    };
                    if actual.as_ref() != expected.as_bytes() { return measured(false); }
                }
            }
            if let Some(pattern) = reason {
                let pattern = regex::Regex::new(pattern).expect("manifest regex checked");
                for value in &terminals {
                    let Some(reason) = value.failure.as_ref().and_then(|failure| failure.reason.as_ref()) else {
                        return unavailable("run failure reason was not captured");
                    };
                    let Ok(text) = std::str::from_utf8(reason) else {
                        return unavailable("run failure reason is not UTF-8");
                    };
                    if !pattern.is_match(text) { return measured(false); }
                }
            }
            measured(!terminals.is_empty() && terminals.iter().all(|value| names::status(&value.status) == status.as_bytes()
                && class.as_ref().is_none_or(|class| value.failure.as_ref().is_some_and(|value| names::answer(&value.class) == class.as_bytes()))))
        }
        EventCheck::ProviderFailure { status_at_least, status_at_most, detail_bytes_at_most } =>
            provider_failure(observer, *status_at_least, *status_at_most, *detail_bytes_at_most).passed,
        EventCheck::NoRequest => measured(!records.iter().any(|record| matches!(record.event, Event::ResponseStarted(_)))),
        EventCheck::NoRun => measured(false),
        EventCheck::NothingBeforeTask => super::messages::before_task(observer, task),
        EventCheck::HandledOnce => measured(records.iter().any(|record| matches!(record.event, Event::MessageReceived(_)))
            && observer.sessions.iter().all(|session| super::messages::terminals(session, true).is_empty())),
        EventCheck::CompletionPerExchange => super::messages::per_exchange(observer),
        EventCheck::Blocks { oversized, cut } => block_counts(observer, *oversized, *cut),
        EventCheck::PromptWithinWindow => super::scopes::prompt_within_window(observer),
        EventCheck::CacheReuse { median_at_least, full_misses_per_ten_at_most } =>
            super::cache::check(observer, *median_at_least, *full_misses_per_ten_at_most).passed,
        EventCheck::FinalTurn => super::final_turn::check(observer),
        EventCheck::CompactionCarried => super::carried::check(observer).passed,
        EventCheck::CompactionChoice => super::compaction_choice::check(observer).passed,
        EventCheck::BudgetVisible => super::budget::check(observer),
        EventCheck::BudgetTold => super::budget_report::check(observer, false),
        EventCheck::BudgetReport => super::budget_report::check(observer, true),
        EventCheck::MainAfterChild => super::scopes::main_after_child(observer),
        EventCheck::ChildShare { turns_at_most } => super::scopes::child_share(observer, *turns_at_most),
        EventCheck::SpendWithin { ceiling_usd } => match currency {
            Some(currency) => super::spend::check(observer, currency, *ceiling_usd),
            None => unavailable("dollar conversion requires the prepared arm's exact notional currency"),
        },
    }
}

// Consume the authoritative terminal; never infer a status from diagnostic text.
fn provider_failure(observer: &Observer, minimum: u16, maximum: u16, detail_bound: u32) -> CheckVerdict {
    let mut evidence = Vec::new();
    let mut found_terminal = false;
    let mut passed = true;
    for record in records(observer) {
        if let Event::RunCompleted(run) = &record.event {
            let Some(failure) = run.failure.as_ref().and_then(|value| value.completion.as_ref()) else {
                return CheckVerdict {
                    check: "provider-failure".into(),
                    passed: unavailable("typed completion failure was not retained"),
                    evidence,
                };
            };
            let (Some(status), Some(detail)) = (failure.status, failure.detail.as_ref()) else {
                return CheckVerdict {
                    check: "provider-failure".into(),
                    passed: unavailable("actual HTTP status or provider diagnostic was not captured"),
                    evidence,
                };
            };
            found_terminal = true;
            // The retired client fallback contains no provider text (benchmarks.md 11.1).
            let generated_detail = detail
                .strip_prefix(b"provider HTTP error ")
                .is_some_and(|digits| !digits.is_empty() && digits.iter().all(u8::is_ascii_digit));
            passed &= (u64::from(minimum)..=u64::from(maximum)).contains(&status)
                && !detail.is_empty()
                && detail.len() <= usize::try_from(detail_bound).expect("bound fits")
                && !generated_detail;
            let retained = &detail[..detail.len().min(usize::try_from(detail_bound).expect("bound fits"))];
            evidence.push(format!(
                "run={} HTTP={} detail_bytes={} bound={} detail={}",
                run.run,
                status,
                detail.len(),
                detail_bound,
                String::from_utf8_lossy(retained)
            ));
        }
    }
    CheckVerdict { check: "provider-failure".into(), passed: measured(found_terminal && passed), evidence }
}

fn tool_result_bytes(observer: &Observer, tool: &str, bound: u64) -> CheckVerdict {
    let mut found = false;
    let mut evidence = Vec::new();
    for session in &observer.sessions {
        for record in &session.records {
            if let Event::ToolStarted(start) = &record.event
                && start.tool.as_ref() == tool.as_bytes()
            {
                let Some(end) = session
                    .records
                    .iter()
                    .filter_map(|record| if let Event::ToolCompleted(end) = &record.event { Some(end) } else { None })
                    .find(|end| {
                        end.run == start.run
                            && end.conversation == start.conversation
                            && end.call == start.call
                            && end.tool == start.tool
                    })
                else {
                    return CheckVerdict {
                        check: "tool-result-bytes".into(),
                        passed: unavailable("tool result has no matching terminal"),
                        evidence,
                    };
                };
                evidence.push(format!(
                    "run={} conversation={} call={} payload_bytes={} at_least={bound}",
                    start.run,
                    start.conversation,
                    String::from_utf8_lossy(&start.call),
                    end.bytes
                ));
                found |= end.bytes >= bound;
            }
        }
    }
    CheckVerdict { check: "tool-result-bytes".into(), passed: measured(found), evidence }
}

// This probe configures the command bound equal to the emitted outer ceiling.
// An emitted outer ceiling alone never proves a shorter requested duration.
fn tool_timeout(
    observer: &Observer,
    requested: u32,
    bound: u64,
    clamped: bool,
    grace: u64,
    command: Option<&str>,
) -> CheckVerdict {
    let maximum = bound.checked_add(grace).expect("manifest duration checked");
    let mut count = 0_u64;
    let mut passed = true;
    let mut evidence = Vec::new();
    for session in &observer.sessions {
        for (start_at, record) in session.records.iter().enumerate() {
            if let Event::ToolStarted(start) = &record.event
                && start.tool.as_ref() == b"shell"
            {
                let Some(input) = &start.input else {
                    return CheckVerdict {
                        check: "tool-timeout".into(),
                        passed: unavailable("shell request input was not captured"),
                        evidence,
                    };
                };
                let Ok(input) = serde_json::from_slice::<serde_json::Value>(input) else {
                    return CheckVerdict { check: "tool-timeout".into(), passed: measured(false), evidence };
                };
                if input.get("timeout_seconds").and_then(serde_json::Value::as_u64) != Some(u64::from(requested)) {
                    continue;
                }
                if let Some(command) = command {
                    let Some(actual) = input.get("command").and_then(serde_json::Value::as_str) else {
                        return CheckVerdict {
                            check: "tool-timeout".into(),
                            passed: unavailable("shell command identity was not retained"),
                            evidence,
                        };
                    };
                    passed &= actual == command;
                    evidence.push(format!("command={actual}"));
                }
                count = count.checked_add(1).expect("retained shell count fits");
                let Some(deadline) = start.deadline_ms else {
                    return CheckVerdict {
                        check: "tool-timeout".into(),
                        passed: unavailable("actual shell call bound was not emitted"),
                        evidence,
                    };
                };
                let Some(end) = session
                    .records
                    .iter()
                    .skip(start_at + 1)
                    .filter_map(|record| if let Event::ToolCompleted(end) = &record.event { Some(end) } else { None })
                    .find(|end| {
                        end.run == start.run
                            && end.conversation == start.conversation
                            && end.call == start.call
                            && end.tool == start.tool
                    })
                else {
                    return CheckVerdict {
                        check: "tool-timeout".into(),
                        passed: unavailable("shell timeout lacks its matching terminal"),
                        evidence,
                    };
                };
                // An explicit output budget could also set the scalar clamp.
                // This case omits it, so the observed clamp belongs to time.
                passed &= input.get("output_bytes").is_none()
                    && deadline == bound
                    && end.clamped == clamped
                    && names::verdict(&end.verdict) == b"timed_out"
                    && end.duration_ms >= bound
                    && end.duration_ms <= maximum;
                evidence.push(format!("run={} conversation={} call={} requested_seconds={requested} emitted_bound_ms={deadline} duration_ms={} grace_ms={grace} clamped={}",
                    start.run, start.conversation, String::from_utf8_lossy(&start.call), end.duration_ms, end.clamped));
            }
        }
    }
    CheckVerdict { check: "tool-timeout".into(), passed: measured(passed && count == 1), evidence }
}
