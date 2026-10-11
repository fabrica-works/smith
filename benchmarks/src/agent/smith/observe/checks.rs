//! Frozen records prove observed violations and lower counts, never absence.
//! No trace loss, response identity, command clamp or native HTTP start is known.
//! Entry `evaluate` answers each author predicate using only retained records.
//! Contract: benchmarks.md sections 6.2/11.1; reliability00 interface sketches.

use crate::{Before, CheckVerdict, Count, EventCheck, Measure};

use super::{Event, Observation};

mod names;

fn observed(value: bool) -> Measure<bool> {
    Measure::Observed { value }
}

fn missing(reason: &str) -> Measure<bool> {
    Measure::unavailable(reason)
}

fn lower_count(count: Count, number: u64) -> Measure<bool> {
    match count {
        Count::AtLeast(bound) if number >= u64::from(bound) => observed(true),
        Count::Exactly(bound) | Count::AtMost(bound) if number > u64::from(bound) => observed(false),
        Count::Exactly(_) | Count::AtMost(_) | Count::AtLeast(_) => {
            missing("legacy trace loss makes this count a lower bound")
        }
    }
}

fn completions(records: &[Observation], before: Before) -> u64 {
    let mut number = 0;
    for record in records {
        match record {
            Observation::Fact { event: Event::RunCalled { ask, .. }, .. }
                if before == Before::Finish && ask == "Finish" =>
            {
                break;
            }
            Observation::Fact { event: Event::RunAnswered { .. }, .. } if before == Before::Answer => break,
            Observation::Fact { event: Event::CompletionAnswered { .. }, .. } => number += 1,
            Observation::Fact { .. }
            | Observation::Usage { .. }
            | Observation::Tool { .. }
            | Observation::Prompt { .. }
            | Observation::Completion { .. }
            | Observation::Call { .. } => {}
        }
    }
    number
}

fn tool(records: &[Observation], check: &EventCheck) -> Measure<bool> {
    let EventCheck::ToolCall { tool, count, input_bytes_at_least, verdict, exit, clamped } = check else {
        return missing("not a tool predicate");
    };
    let calls: Vec<_> = records
        .iter()
        .filter_map(|record| match record {
            Observation::Call { name, input, .. } if name == tool.as_bytes() => Some(input),
            Observation::Fact { .. }
            | Observation::Call { .. }
            | Observation::Tool { .. }
            | Observation::Prompt { .. }
            | Observation::Usage { .. }
            | Observation::Completion { .. } => None,
        })
        .collect();
    if calls.iter().any(|input| {
        input_bytes_at_least.is_some_and(|bound| u64::try_from(input.len()).expect("captured bytes fit") < bound)
    }) {
        return observed(false);
    }
    if let Some(count) = count {
        let measured = lower_count(*count, u64::try_from(calls.len()).expect("captured count fits"));
        if measured == observed(false) {
            return measured;
        }
        if verdict.is_none() && exit.is_none() && clamped.is_none() && input_bytes_at_least.is_none() {
            return measured;
        }
    }
    missing("legacy content has no loss count or call-to-verdict/exit/clamp identity")
}

fn terminal(records: &[Observation], status: &str, class: Option<&str>) -> Measure<bool> {
    for record in records {
        if let Observation::Fact { event: Event::RunAnswered { answer, .. }, .. } = record {
            if let Some((actual_status, actual_class)) = names::answer(answer) {
                if actual_status != status || class.is_some_and(|class| Some(class) != actual_class) {
                    return observed(false);
                }
            } else {
                return missing("legacy answer classification is not in this frozen vocabulary");
            }
        }
    }
    missing("observed run terminals cannot prove that no terminal was lost")
}

fn evaluate_one(records: &[Observation], check: &EventCheck) -> Measure<bool> {
    match check {
        EventCheck::ToolPath { .. } => missing("legacy content has no authoritative path-to-terminal identity"),
        EventCheck::ToolCall { .. } => tool(records, check),
        EventCheck::WrittenFile { .. } => missing("legacy facts omit exact captured write content"),
        EventCheck::OversizedWrite { .. } => missing("legacy facts omit paired oversized write content"),
        EventCheck::FilePieces { .. } => missing("legacy facts omit oversized call identity and captured edit pieces"),
        EventCheck::Completions { count, before } => lower_count(*count, completions(records, *before)),
        EventCheck::ConversationOpened { kind, count } if kind == "main" || kind == "child" => {
            let number = records.iter().filter(|record| matches!(record,
                Observation::Fact { event: Event::ConversationOpened { child, .. }, .. } if *child == (kind == "child")
            )).count();
            lower_count(*count, u64::try_from(number).expect("captured count fits"))
        }
        EventCheck::NoVerdict { verdicts } => {
            if records.iter().any(|record| {
                matches!(record,
                    Observation::Fact { event: Event::ToolAnswered { verdict, .. }, .. }
                    if names::verdict(verdict).is_some_and(|name| verdicts.iter().any(|value| value == name))
                )
            }) {
                observed(false)
            } else {
                missing("legacy loss prevents proving no forbidden tool verdict")
            }
        }
        EventCheck::NoStop { stop } => {
            if records.iter().any(|record| {
                matches!(record,
                    Observation::Fact { event: Event::CompletionAnswered { stop: actual, .. }, .. }
                    if names::stop(actual) == Some(stop.as_str())
                )
            }) {
                observed(false)
            } else {
                missing("legacy loss prevents proving no forbidden stop")
            }
        }
        EventCheck::NoFailure { class } => {
            if records.iter().any(|record| {
                matches!(record,
                    Observation::Fact { event: Event::CompletionFailed { failure, .. }, .. }
                    if names::failure(failure) == Some(class.as_str())
                )
            }) {
                observed(false)
            } else {
                missing("legacy loss prevents proving no forbidden failure")
            }
        }
        EventCheck::ProviderFailure { .. } => missing("legacy terminal lacks typed HTTP status and provider detail"),
        EventCheck::RunCompleted { which: Some(_), .. } => {
            missing("legacy terminal has no authoritative typed failure identity")
        }
        EventCheck::RunCompleted { reason: Some(_), .. } => {
            missing("legacy terminal has no authoritative failure reason")
        }
        EventCheck::RunCompleted { status, class, which: None, reason: None } => {
            terminal(records, status, class.as_deref())
        }
        EventCheck::ToolTimeout { .. } => missing("legacy tool terminal has no native deadline or exact input join"),
        EventCheck::ToolResultBytes { .. } => missing("legacy tool result has no authoritative payload byte count"),
        EventCheck::UsageSums => missing("legacy raw run usage totals do not prove complete per-response sums"),
        EventCheck::NoRequest => missing("legacy CompletionStarted is logical intent, not an actual native HTTP start"),
        EventCheck::NoRun => missing("legacy trace cannot prove an empty native startup event capture"),
        EventCheck::FirstRequestHoldsTask
        | EventCheck::NothingBeforeTask
        | EventCheck::HandledOnce
        | EventCheck::CompletionPerExchange => {
            missing("legacy content lacks typed new-message admission and request boundaries")
        }
        EventCheck::Blocks { .. } => missing("legacy completion facts omit oversized and cut numeric totals"),
        EventCheck::ConversationOpened { .. } | EventCheck::PromptWithinWindow | EventCheck::CacheReuse { .. } => {
            missing("legacy trace lacks window/model bounds and immutable response/window identities")
        }
        EventCheck::MainAfterChild
        | EventCheck::FinalTurn
        | EventCheck::ChildShare { .. }
        | EventCheck::BudgetVisible
        | EventCheck::SpendWithin { .. } => {
            missing("legacy trace lacks complete typed charge/share/reserve and final-request metadata")
        }
        EventCheck::CallsInOneResponse { .. }
        | EventCheck::CallsPerResponse { .. }
        | EventCheck::ChildrenInTurn { .. }
        | EventCheck::ChildrenOverlap { .. } => {
            missing("legacy trace lacks immutable response joins or complete child lifetime terminals")
        }
    }
}

pub(super) fn evaluate(records: &[Observation], check: &EventCheck) -> CheckVerdict {
    CheckVerdict {
        check: check.name().into(),
        passed: evaluate_one(records, check),
        evidence: vec![format!("{} retained legacy records; loss and HTTP starts are unobservable", records.len())],
    }
}
