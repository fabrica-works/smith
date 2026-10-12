//! Native legacy observations keep only trace-visible scopes and response usage.
//! Completion ordinals pair content with its native terminal; aggregate Used
//! facts never charge tokens again. Legacy loss and provider response identity
//! are not observable (benchmarks.md, sections 6.2 and 8.2).

use std::collections::BTreeMap;

use crate::agent::{HarnessFault, Seen};
use crate::{Convention, Counts, Measure, ScopeKind, Spend, SpendBasis, TokenLedger, TokenRecord, Usage};

use super::legacy::{Event, Observation, Observer as LegacyObserver, Value};
use super::{Face, Prepared};

mod checks;

mod health;

/// The format selected by the verified native face; the formats never mix.
#[derive(Debug)]
pub enum Observer {
    /// The frozen Debug vocabulary, retained until historical back-fill ends.
    Legacy(LegacyObserver),
    /// The codec's typed incremental process stream.
    Events(super::events::Observer),
}

impl Observer {
    /// Reconcile the selected stream with the actual settled exit.
    #[must_use]
    pub fn classify(&self, exit: crate::Exit, forced: crate::Forced) -> crate::Classification {
        match self {
            Self::Legacy(observer) => observer.classify(exit, forced),
            Self::Events(observer) => observer.classify(exit, forced),
        }
    }

    /// Preserve legacy loss as unknown; typed sessions report their own loss.
    #[must_use]
    pub fn loss(&self) -> Measure<u64> {
        match self {
            Self::Legacy(observer) => observer.loss(),
            Self::Events(observer) => observer
                .health(crate::Exit::Code(0), crate::Forced::No, 0)
                .loss_per_sink
                .remove("events")
                .unwrap_or_else(|| Measure::unavailable("event loss was not reported")),
        }
    }
}

/// An observer paired with the private invocation that selected its face.
#[derive(Debug)]
pub struct Observing {
    pub prepared: Prepared,
    pub observer: Observer,
    records: Vec<Observation>,
}

fn lower(value: u64) -> Measure<u64> {
    Measure::LowerBound { value, missing: vec!["legacy trace loss is unobservable".into()] }
}

fn optional(value: &Value, field: &str) -> Result<Option<u64>, String> {
    let Value::Struct { fields, .. } = value else {
        return Err("usage requires named fields".into());
    };
    match fields.get(field) {
        Some(Value::Number(number)) => Ok(Some(*number)),
        Some(Value::Name(name)) if name == "None" => Ok(None),
        Some(Value::Tuple { name, values }) if name == "Some" && values.len() == 1 => match values[0] {
            Value::Number(number) => Ok(Some(number)),
            Value::Name(_)
            | Value::String(_)
            | Value::Bytes(_)
            | Value::Sequence(_)
            | Value::Tuple { .. }
            | Value::Struct { .. } => Err(format!("invalid usage.{field}")),
        },
        None if field == "reasoning_tokens" => Ok(None),
        _ => Err(format!("invalid usage.{field}")),
    }
}

fn usage(value: &Value) -> Result<Usage, String> {
    Ok(Usage::SmithEvents {
        input: optional(value, "input_tokens")?,
        cache_read: optional(value, "cache_read_tokens")?,
        cache_write: optional(value, "cache_write_tokens")?,
        output: optional(value, "output_tokens")?,
        reasoning: optional(value, "reasoning_tokens")?,
    })
}

impl Observing {
    /// Retain an outside observer with no knowledge of the running domains.
    pub fn new(prepared: Prepared) -> Result<Self, HarnessFault> {
        let observer = match prepared.face {
            Face::Legacy(_) => Observer::Legacy(LegacyObserver::new(prepared.registry.facts)),
            Face::Events(_) => {
                let receiving = prepared
                    .registry
                    .receiving
                    .ok_or_else(|| HarnessFault { what: "event face lacks its immutable receiving bounds".into() })?;
                let limits = receiving.limits().map_err(|what| HarnessFault { what })?;
                Observer::Events(super::events::Observer::with_ceiling(&limits, receiving.encoded_line)?)
            }
        };
        Ok(Self { prepared, observer, records: Vec::new() })
    }

    /// Consume one complete native line and report only observed script cues.
    pub fn feed_line(&mut self, line: &str) -> Result<Vec<Seen>, HarnessFault> {
        let observer = match &mut self.observer {
            Observer::Events(observer) => return observer.feed(format!("{line}\n").as_bytes()),
            Observer::Legacy(observer) => observer,
        };
        let answered = observer.accepted().is_some() || observer.answer().is_some();
        let observation = observer.observe_line(line).map_err(|error| HarnessFault { what: error.to_string() })?;
        let mut seen = Vec::new();
        match &observation {
            Some(Observation::Fact { event: Event::CompletionAnswered { .. }, .. }) => seen.push(Seen::Response),
            Some(Observation::Fact { event: Event::ConversationOpened { child: true, .. }, .. }) => {
                seen.push(Seen::Child);
            }
            Some(Observation::Call { name, .. }) => {
                let tool = std::str::from_utf8(name).map_err(|error| HarnessFault { what: error.to_string() })?;
                seen.push(Seen::ToolCall { tool: tool.into() });
            }
            Some(_) | None => {}
        }
        if !answered && (observer.accepted().is_some() || observer.answer().is_some()) {
            seen.push(Seen::Answer);
        }
        if let Some(observation) = observation {
            self.records.push(observation);
        }
        Ok(seen)
    }

    /// Preserve each selected face's passive evidence and name its missing fields.
    #[must_use]
    pub fn health(&self, exit: crate::Exit, forced: crate::Forced) -> Option<crate::Health> {
        match &self.observer {
            Observer::Legacy(observer) => Some(health::legacy(observer, exit, forced)),
            Observer::Events(observer) => {
                Some(observer.health(exit, forced, self.prepared.teardown.as_nanos() / 1_000_000))
            }
        }
    }

    /// Author checks keep unavailable evidence where the selected format omits it.
    #[must_use]
    pub fn checks(&self, checks: &[crate::EventCheck], task: &str) -> Vec<crate::CheckVerdict> {
        checks
            .iter()
            .map(|check| match &self.observer {
                // The supported LocalTrace invocation renders LocalReserved's
                // micro-dollar rates. Other invocations refuse before here.
                Observer::Events(observer) => observer.check_with_currency(check, task, 1_000_000),
                Observer::Legacy(_) => checks::evaluate(&self.records, check),
            })
            .collect()
    }

    /// File-dependent predicates run after the process tree settles.
    #[must_use]
    pub fn checks_in_workspace(
        &self,
        checks: &[crate::EventCheck],
        task: &str,
        workspace: &std::path::Path,
    ) -> Vec<crate::CheckVerdict> {
        checks
            .iter()
            .map(|check| match (check, &self.observer) {
                (crate::EventCheck::WrittenFile { path, content_bytes_at_least }, Observer::Events(observer)) => {
                    observer.written_file(path, *content_bytes_at_least, workspace)
                }
                (
                    crate::EventCheck::OversizedWrite { path, content_bytes_above, protocol_input_bytes_above },
                    Observer::Events(observer),
                ) => observer.oversized_write(path, *content_bytes_above, *protocol_input_bytes_above, workspace),
                (crate::EventCheck::FilePieces { path, bytes_above, file_bytes }, Observer::Events(observer)) => {
                    let protocol_bound = checks.iter().find_map(|check| match check {
                        crate::EventCheck::OversizedWrite { protocol_input_bytes_above, .. } => {
                            *protocol_input_bytes_above
                        }
                        crate::EventCheck::WrittenFile { .. }
                        | crate::EventCheck::FilePieces { .. }
                        | crate::EventCheck::ToolCall { .. }
                        | crate::EventCheck::UsageSums
                        | crate::EventCheck::NoVerdict { .. }
                        | crate::EventCheck::Completions { .. }
                        | crate::EventCheck::FirstRequestHoldsTask
                        | crate::EventCheck::NothingBeforeTask
                        | crate::EventCheck::HandledOnce
                        | crate::EventCheck::CompletionPerExchange
                        | crate::EventCheck::NoStop { .. }
                        | crate::EventCheck::Blocks { .. }
                        | crate::EventCheck::ConversationOpened { .. }
                        | crate::EventCheck::PromptWithinWindow
                        | crate::EventCheck::CacheReuse { .. }
                        | crate::EventCheck::MainAfterChild
                        | crate::EventCheck::FinalTurn
                        | crate::EventCheck::ChildShare { .. }
                        | crate::EventCheck::BudgetVisible
                        | crate::EventCheck::SpendWithin { .. }
                        | crate::EventCheck::CallsInOneResponse { .. }
                        | crate::EventCheck::ChildrenOverlap { .. }
                        | crate::EventCheck::NoFailure { .. }
                        | crate::EventCheck::RunCompleted { .. }
                        | crate::EventCheck::ProviderFailure { .. }
                        | crate::EventCheck::NoRequest
                        | crate::EventCheck::ToolPath { .. }
                        | crate::EventCheck::ToolResultBytes { .. }
                        | crate::EventCheck::ToolTimeout { .. }
                        | crate::EventCheck::SlowCompletion { .. }
                        | crate::EventCheck::BudgetTold
                        | crate::EventCheck::BudgetReport
                        | crate::EventCheck::CallsPerResponse { .. }
                        | crate::EventCheck::ChildrenInTurn { .. }
                        | crate::EventCheck::NoRun => None,
                    });
                    match protocol_bound {
                        Some(bound) => {
                            observer.file_pieces_for_protocol_probe(path, *bytes_above, *file_bytes, bound, workspace)
                        }
                        None => observer.file_pieces(path, *bytes_above, *file_bytes, workspace),
                    }
                }
                _ => self.checks(std::slice::from_ref(check), task).remove(0),
            })
            .collect()
    }

    /// Counts are lower bounds because the old writer cannot report loss.
    #[must_use]
    pub fn counts(&self) -> Counts {
        match &self.observer {
            Observer::Events(observer) => return observer.counts(),
            Observer::Legacy(_) => {}
        }
        let mut counts = Counts {
            responses: lower(0),
            provider_attempts: Measure::unavailable(
                "legacy request intents do not report actual provider attempt starts",
            ),
            retries: Measure::unavailable("legacy retry intent does not prove another provider attempt started"),
            completions: lower(0),
            tool_calls: BTreeMap::new(),
            tool_failures: BTreeMap::new(),
            children: lower(0),
            conversations: lower(0),
            compactions: lower(0),
            message_terminals: BTreeMap::new(),
        };
        for record in &self.records {
            let destination = match record {
                Observation::Fact { event: Event::CompletionAnswered { .. }, .. } => {
                    if let Measure::LowerBound { value, .. } = &mut counts.responses {
                        *value += 1;
                    }
                    Some(&mut counts.completions)
                }
                Observation::Fact { event: Event::ConversationOpened { child, .. }, .. } => {
                    if *child && let Measure::LowerBound { value, .. } = &mut counts.children {
                        *value += 1;
                    }
                    Some(&mut counts.conversations)
                }
                Observation::Call { name, .. } => {
                    let tool = String::from_utf8_lossy(name).into_owned();
                    Some(counts.tool_calls.entry(tool).or_insert_with(|| lower(0)))
                }
                Observation::Fact { .. }
                | Observation::Usage { .. }
                | Observation::Tool { .. }
                | Observation::Prompt { .. }
                | Observation::Completion { .. } => None,
            };
            if let Some(Measure::LowerBound { value, .. }) = destination {
                *value += 1;
            }
        }
        counts.compactions = Measure::unavailable("legacy trace does not identify compaction scopes");
        counts
    }

    /// Delegation is exact on the event face; the frozen trace does not expose it.
    #[must_use]
    pub fn delegation(&self) -> crate::Delegation {
        match &self.observer {
            Observer::Events(observer) => observer.delegation(),
            Observer::Legacy(_) => crate::Delegation::unavailable("legacy trace does not report exact delegation"),
        }
    }

    /// Count content usage once for each trace-visible completion ordinal.
    pub fn tokens(&self) -> Result<Vec<TokenRecord>, String> {
        match &self.observer {
            Observer::Events(observer) => return observer.tokens().map_err(|error| error.what),
            Observer::Legacy(_) => {}
        }
        let mut ledger = TokenLedger::default();
        let mut scopes = BTreeMap::new();
        let mut ordinals = BTreeMap::<u64, u64>::new();
        for record in &self.records {
            match record {
                Observation::Fact { event: Event::ConversationOpened { conversation, child, parent, .. }, .. } => {
                    let role = if *child {
                        match parent {
                            Some(parent) => ScopeKind::Child { parent: format!("conversation-{parent}") },
                            None => ScopeKind::UnlinkedChild,
                        }
                    } else {
                        ScopeKind::Root
                    };
                    scopes.entry(*conversation).or_insert(role);
                }
                Observation::Fact { event: Event::SessionOpened { opener }, .. } => {
                    scopes.entry(*opener).or_insert(ScopeKind::UnlinkedChild);
                }
                Observation::Fact { .. }
                | Observation::Call { .. }
                | Observation::Tool { .. }
                | Observation::Prompt { .. }
                | Observation::Usage { .. }
                | Observation::Completion { .. } => {}
            }
        }
        for (owner, role) in &scopes {
            ledger.declare(
                &format!("conversation-{owner}"),
                role.clone(),
                Convention::SmithEvents,
                "legacy per-response content or parent link was not retained",
            )?;
        }
        for record in &self.records {
            match record {
                Observation::Fact { event: Event::CompletionAnswered { opener, .. }, .. } => {
                    let ordinal = ordinals.entry(*opener).or_default();
                    *ordinal = ordinal.checked_add(1).ok_or("response ordinal overflow")?;
                }
                Observation::Usage { owner, value, .. } => {
                    let Some(ordinal) = ordinals.get(owner).copied().filter(|ordinal| *ordinal > 0) else {
                        continue;
                    };
                    if scopes.get(owner) == Some(&ScopeKind::UnlinkedChild) {
                        continue;
                    }
                    ledger.record(
                        &format!("conversation-{owner}"),
                        &format!("legacy-{owner}-{ordinal}"),
                        usage(value)?,
                    )?;
                }
                Observation::Fact { .. }
                | Observation::Call { .. }
                | Observation::Tool { .. }
                | Observation::Prompt { .. }
                | Observation::Completion { .. } => {}
            }
        }
        if !scopes.values().any(|role| *role == ScopeKind::Root) {
            return Ok(Vec::new());
        }
        let mut records = ledger.records()?;
        for record in &mut records {
            for metric in [
                &mut record.usage.fresh,
                &mut record.usage.cache_read,
                &mut record.usage.cache_write,
                &mut record.usage.output,
                &mut record.usage.reasoning,
            ] {
                match metric {
                    Measure::Observed { value } => *metric = lower(*value),
                    Measure::LowerBound { missing, .. } => missing.push("legacy trace loss is unobservable".into()),
                    Measure::Unavailable { .. } => {}
                }
            }
        }
        Ok(records)
    }

    /// Price retained scoped usage with the arm's archived notional list rates.
    pub fn spend(&self, records: &[TokenRecord]) -> Result<Spend, String> {
        match &self.observer {
            Observer::Events(observer) => return super::events::reported_spend(observer, 1_000_000),
            Observer::Legacy(_) => {}
        }
        let Some(total) = records.iter().find(|record| record.role == ScopeKind::Total) else {
            return Ok(Spend {
                basis: SpendBasis::Priced,
                usd: Measure::unavailable("legacy root usage is unavailable"),
            });
        };
        let model = &self.prepared.rendered["models"][0];
        let mut amount = 0_u128;
        let mut observed = false;
        let mut missing = Vec::new();
        for (metric, field) in [
            (&total.usage.fresh, "input_price"),
            (&total.usage.cache_read, "cached_price"),
            (&total.usage.output, "output_price"),
        ] {
            let rate = model[field].as_u64().ok_or("Smith notional price is unavailable")?;
            let value = match metric {
                Measure::Observed { value } => Some(*value),
                Measure::LowerBound { value, missing: gaps } => {
                    missing.extend(gaps.iter().cloned());
                    Some(*value)
                }
                Measure::Unavailable { reason } => {
                    missing.push(reason.clone());
                    None
                }
            };
            if let Some(value) = value {
                observed = true;
                amount = amount.checked_add(u128::from(value) * u128::from(rate)).ok_or("notional spend overflow")?;
            }
        }
        let shape = match self.prepared.face {
            Face::Legacy(shape) => shape,
            Face::Events(_) => return Err("event face cannot price legacy usage".into()),
        };
        let currency = match shape {
            super::legacy::Shape::LocalJson | super::legacy::Shape::LocalDeclared => 1_u64,
            super::legacy::Shape::LocalPriced
            | super::legacy::Shape::LocalDelegating
            | super::legacy::Shape::LocalReserved => 1_000_000,
        };
        let denominator = model["price_unit"]
            .as_u64()
            .and_then(|unit| unit.checked_mul(currency))
            .filter(|unit| *unit > 0)
            .ok_or("notional price unit is unavailable")?;
        let value = amount.to_string().parse::<f64>().map_err(|error| error.to_string())?
            / denominator.to_string().parse::<f64>().map_err(|error| error.to_string())?;
        missing.sort();
        missing.dedup();
        let usd = if !observed {
            Measure::unavailable(missing.join("; "))
        } else if missing.is_empty() {
            Measure::Observed { value }
        } else {
            Measure::LowerBound { value, missing }
        };
        Ok(Spend { basis: SpendBasis::Priced, usd })
    }
}
