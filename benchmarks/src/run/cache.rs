//! Cache metadata comes from the selected adapter's settled observations.
//! No cache counts are inferred from terminal codes or cumulative token totals.

use crate::agent::Observing;
use crate::process::ProcessResult;
use crate::{CacheObservation, EventCheck, Measure, Task};

pub(super) fn observation(process: &ProcessResult, task: &Task) -> Option<Measure<CacheObservation>> {
    let (threshold, miss_limit) = task.event.iter().find_map(|check| match check {
        EventCheck::CacheReuse { median_at_least, full_misses_per_ten_at_most } => {
            Some((*median_at_least, *full_misses_per_ten_at_most))
        }
        EventCheck::CallCount { .. }
        | EventCheck::ToolCall { .. }
        | EventCheck::UsageSums
        | EventCheck::JobRunning
        | EventCheck::CompletionDuringJob
        | EventCheck::JobTerminal
        | EventCheck::NoVerdict { .. }
        | EventCheck::Completions { .. }
        | EventCheck::FirstRequestHoldsTask
        | EventCheck::NothingBeforeTask
        | EventCheck::HandledOnce
        | EventCheck::CompletionPerExchange
        | EventCheck::NoStop { .. }
        | EventCheck::Blocks { .. }
        | EventCheck::ConversationOpened { .. }
        | EventCheck::PromptWithinWindow
        | EventCheck::MainAfterChild
        | EventCheck::FinalTurn
        | EventCheck::CompactionChoice
        | EventCheck::CompactionCarried
        | EventCheck::ChildShare { .. }
        | EventCheck::BudgetVisible
        | EventCheck::SpendWithin { .. }
        | EventCheck::CallsInOneResponse { .. }
        | EventCheck::CallsPerResponse { .. }
        | EventCheck::ChildrenInTurn { .. }
        | EventCheck::ChildrenOverlap { .. }
        | EventCheck::NoFailure { .. }
        | EventCheck::RunCompleted { .. }
        | EventCheck::ToolPath { .. }
        | EventCheck::WrittenFile { .. }
        | EventCheck::OversizedWrite { .. }
        | EventCheck::FilePieces { .. }
        | EventCheck::ToolResultBytes { .. }
        | EventCheck::ToolTimeout { .. }
        | EventCheck::SlowCompletion { .. }
        | EventCheck::BudgetTold
        | EventCheck::BudgetReport
        | EventCheck::ProviderFailure { .. }
        | EventCheck::NoRun
        | EventCheck::NoRequest => None,
    })?;
    Some(match process.observing.as_deref() {
        Some(Observing::Smith(value)) => value.events().cache_reuse(threshold, miss_limit),
        Some(Observing::Codex(value)) => value.collected.as_ref().map_or_else(
            || Measure::unavailable("named Codex rollout collection is absent"),
            |rollouts| {
                value.observer.thread().map_or_else(
                    || Measure::unavailable("Codex root thread is absent"),
                    |root| rollouts.cache_reuse(root, threshold, miss_limit),
                )
            },
        ),
        None => Measure::unavailable("cache probe requires its selected adapter"),
    })
}
