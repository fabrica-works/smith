//! Passive judgments use reported boundaries, actual pairs and per-response sums.
//! No skipped record, absent limit or unavailable field is proof of compliance.
//! Contract: benchmarks.md, section11.2; protocol/events.md, sections3/5.2.

use std::collections::BTreeMap;

use smith_events::{Delivery, Event, Status};

use crate::{CheckVerdict, Exit, Forced, Health, Measure};

use super::{Observer, Session};

fn verdict(check: &str, passed: Measure<bool>, evidence: Vec<String>) -> CheckVerdict {
    CheckVerdict { check: check.into(), passed, evidence }
}

fn observed(check: &str, passed: bool, evidence: Vec<String>) -> CheckVerdict {
    verdict(check, Measure::Observed { value: passed }, evidence)
}

fn unavailable(check: &str, reason: &str) -> CheckVerdict {
    verdict(check, Measure::unavailable(reason), Vec::new())
}

fn runs(session: &Session) -> bool {
    let mut started = BTreeMap::<u64, u32>::new();
    let mut completed = BTreeMap::<u64, (u32, bool)>::new();
    for record in &session.records {
        match &record.event {
            Event::RunStarted(value) => *started.entry(value.run).or_default() += 1,
            Event::RunCompleted(value) => {
                let entry = completed.entry(value.run).or_insert((0, false));
                entry.0 += 1;
                entry.1 = value.status == Status::Refused;
            }
            Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Budget(_)
            | Event::SessionStarted(_)
            | Event::SessionEnded(_)
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
    session.records.iter().filter_map(|record| super::run(&record.event)).all(|run| completed.contains_key(&run))
        && started.iter().all(|(run, count)| *count == 1 && completed.get(run).is_some_and(|(count, _)| *count == 1))
        && completed.iter().all(|(run, (count, refused))| *count == 1 && (started.contains_key(run) || *refused))
}

fn stream_whole(observer: &Observer) -> bool {
    !observer.sessions.is_empty()
        && observer.fault.is_none()
        && observer.sessions.iter().all(|session| session.ended.is_some() && runs(session))
}

fn spend_sums(session: &Session) -> Result<(bool, bool), String> {
    let mut sums = BTreeMap::<u64, u128>::new();
    let mut responses = BTreeMap::new();
    let mut runs = BTreeMap::new();
    for record in &session.records {
        match &record.event {
            Event::ResponseCompleted(value) => {
                let key = (value.run, value.response);
                match responses.insert(key, value.spent) {
                    Some(previous) if previous != value.spent => {
                        return Err("response spent changed on repetition".into());
                    }
                    Some(_) => {}
                    None => {
                        let sum = sums.entry(value.run).or_default();
                        *sum = sum.checked_add(u128::from(value.spent)).ok_or("response spent sum overflow")?;
                    }
                }
            }
            Event::RunCompleted(value) => {
                if runs.insert(value.run, value.spent).is_some() {
                    return Err("duplicate run.completed".into());
                }
            }
            Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Budget(_)
            | Event::SessionStarted(_)
            | Event::SessionEnded(_)
            | Event::RunStarted(_)
            | Event::ConversationOpened(_)
            | Event::ConversationClosed(_)
            | Event::ResponseStarted(_)
            | Event::TextDelta(_)
            | Event::ToolStarted(_)
            | Event::JobEnded(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::Notice(_) => {}
        }
    }
    let equal = runs.iter().all(|(run, spent)| sums.get(run).copied().unwrap_or(0) == u128::from(*spent))
        && sums.keys().all(|run| runs.contains_key(run));
    let excessive = runs.iter().any(|(run, spent)| sums.get(run).copied().unwrap_or(0) > u128::from(*spent));
    Ok((equal, excessive))
}

fn losses(sessions: &[Session]) -> BTreeMap<String, Measure<u64>> {
    let events = sessions.iter().try_fold(0_u64, |sum, session| sum.checked_add(session.ended.as_ref()?.loss.events));
    let channel =
        sessions.iter().try_fold(0_u64, |sum, session| sum.checked_add(session.ended.as_ref()?.loss.channel?));
    [("events", events), ("channel", channel)]
        .into_iter()
        .map(|(sink, count)| {
            let measure = if sessions.is_empty() {
                Measure::unavailable("no event session was observed")
            } else {
                count.map_or_else(
                    || Measure::unavailable("sink loss is absent or overflows its reported unit"),
                    |value| Measure::Observed { value },
                )
            };
            (sink.into(), measure)
        })
        .collect()
}

fn exit_agrees(observer: &Observer, exit: Exit, forced: Forced) -> CheckVerdict {
    let classification = observer.classify(exit, forced);
    if forced != Forced::No && classification.end != crate::End::Completed {
        return unavailable("exit-agrees", "forced unfinished run has no authoritative exit agreement");
    }
    if forced == Forced::No && observer.sessions.last().is_none_or(|session| session.ended.is_none()) {
        return unavailable("exit-agrees", "session.ended did not report an authoritative exit");
    }
    observed("exit-agrees", !matches!(classification.end, crate::End::HarnessError { .. }), classification.warnings)
}

fn limit_failure(event: &Event) -> bool {
    let failure = match event {
        Event::ResponseCompleted(value) => value.failure.as_ref(),
        Event::ConversationClosed(value) => value.failure.as_ref(),
        Event::RunCompleted(value) => value.failure.as_ref().and_then(|failure| failure.completion.as_ref()),
        Event::MessageReceived(_)
        | Event::MessageRead(_)
        | Event::MessageUnread(_)
        | Event::Budget(_)
        | Event::SessionStarted(_)
        | Event::SessionEnded(_)
        | Event::RunStarted(_)
        | Event::ConversationOpened(_)
        | Event::ResponseStarted(_)
        | Event::TextDelta(_)
        | Event::ToolStarted(_)
        | Event::JobEnded(_)
        | Event::ToolCompleted(_)
        | Event::CheckStarted(_)
        | Event::CheckCompleted(_)
        | Event::Notice(_) => None,
    };
    failure.is_some_and(|failure| failure.class == smith_events::FailureClass::Limit)
}

pub(super) fn own_peak_rss_bytes(observer: &Observer) -> Measure<u64> {
    let mut peak = None;
    for session in &observer.sessions {
        let Some(ended) = &session.ended else {
            return Measure::unavailable("session.ended not reported");
        };
        let Some(rss) = &ended.peak_rss_bytes else {
            return Measure::unavailable("session.ended reported peak_rss_bytes null: resource usage unread");
        };
        peak = Some(peak.map_or(rss.self_bytes, |value: u64| value.max(rss.self_bytes)));
    }
    peak.map_or_else(|| Measure::unavailable("session.ended not reported"), |value| Measure::Observed { value })
}

fn unread_resources(observer: &Observer, health: &mut Health) {
    let unread = observer
        .sessions
        .iter()
        .any(|session| session.ended.as_ref().is_some_and(|ended| ended.peak_rss_bytes.is_none()));
    if unread {
        let reason =
            "session.ended reported peak_rss_bytes null: resource usage unread; not yet: 04.7 effective memory bound";
        let peak = health.headroom.get_mut("peak-rss").expect("initialized peak RSS comparison");
        peak.fraction_used = Measure::unavailable(reason);
        let check = health
            .passive_checks
            .iter_mut()
            .find(|check| check.check == "headroom/peak-rss")
            .expect("initialized peak RSS judgment");
        check.passed = Measure::unavailable(reason);
    }
}

pub(super) fn health(observer: &Observer, exit: Exit, forced: Forced, teardown_bound_ms: u64) -> Health {
    let mut checks = vec![observed("stream-whole", stream_whole(observer), Vec::new())];
    let loss_per_sink = losses(&observer.sessions);
    let mut problems = Vec::new();
    let mut pair_violation = false;
    let mut spend_violation = false;
    let complete = !observer.sessions.is_empty()
        && observer.sessions.iter().all(|session| super::usage::incomplete(session).is_none());
    let mut nothing_lost = true;
    let mut inventory_missing = Vec::new();
    if observer.unknown {
        inventory_missing.push("unknown record types prevent a complete per-type emitted/loss comparison".into());
    }
    let mut teardown_violation = false;
    let mut spends = true;
    let mut no_limit = true;
    for (index, session) in observer.sessions.iter().enumerate() {
        match super::loss::compare(session) {
            Measure::Observed { value } => nothing_lost &= value,
            Measure::Unavailable { reason } => inventory_missing.push(reason),
            Measure::LowerBound { .. } => unreachable!("inventory comparison is exact or unavailable"),
        }
        let pairing = super::pairs::check(session);
        pair_violation |= !pairing.violations.is_empty();
        problems.extend(
            pairing.violations.into_iter().chain(pairing.missing).map(|issue| format!("session-{index}: {issue}")),
        );
        no_limit &= !session.records.iter().any(|record| limit_failure(&record.event));
        match spend_sums(session) {
            Ok((equal, excessive)) => {
                spends &= equal;
                spend_violation |= excessive;
            }
            Err(error) => {
                spends = false;
                spend_violation = true;
                problems.push(error);
            }
        }
        if let Some(ended) = &session.ended {
            nothing_lost &= session.started.delivery != Delivery::Complete || ended.loss.events == 0;
            teardown_violation |= ended.teardown_ms.is_some_and(|value| value > teardown_bound_ms);
        }
    }
    checks.push(if !nothing_lost {
        observed("nothing-lost", false, Vec::new())
    } else if !inventory_missing.is_empty() {
        unavailable("nothing-lost", &inventory_missing.join("; "))
    } else if matches!(loss_per_sink["events"], Measure::Observed { .. }) {
        observed("nothing-lost", nothing_lost && stream_whole(observer), Vec::new())
    } else {
        unavailable("nothing-lost", "authoritative session loss is unavailable")
    });
    checks.push(if forced != Forced::No || teardown_violation {
        observed("teardown", false, Vec::new())
    } else {
        unavailable("teardown", "not yet: 06-teardown session.ended aborted metadata")
    });
    checks.push(exit_agrees(observer, exit, forced));
    checks.push(if pair_violation || complete {
        observed("paired", problems.is_empty(), problems)
    } else {
        unavailable("paired", "event loss or incomplete stream prevents proving all pairs")
    });
    checks.push(if spend_violation || complete {
        observed("spend-sums", spends, Vec::new())
    } else {
        unavailable("spend-sums", "event loss or incomplete stream prevents proving spend sums")
    });
    checks.push(if !no_limit || complete {
        observed("no-limit-failure", no_limit, Vec::new())
    } else {
        unavailable("no-limit-failure", "event loss prevents proving absence of limit failures")
    });
    checks.push(if complete {
        let issues: Vec<_> =
            observer.sessions.iter().flat_map(|session| super::messages::terminals(session, false)).collect();
        observed("message-terminals", issues.is_empty(), issues)
    } else {
        unavailable("message-terminals", "event loss or incomplete stream prevents proving all message terminals")
    });
    let mut health = Health { loss_per_sink, headroom: BTreeMap::new(), passive_checks: checks };
    crate::passive::unavailable_headroom(
        &mut health,
        "not yet: 04.7 effective startup projection and endpoint/model identity join",
    );
    unread_resources(observer, &mut health);
    super::headroom::apply(observer, &mut health);
    health
}
