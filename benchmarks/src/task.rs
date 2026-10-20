//! Strict task documents, retaining only the manifest and no runtime state.
//! `read_task` refuses unknown keys with a file and key path; task checks are
//! closed enums, following benchmarks.md, sections 3.2 and 5.2.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A task's judgment, supplied by its author to the runner.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// A forced behaviour whose checks must all pass.
    Probe,
    /// A small coding task judged by its external grader.
    Fixture,
    /// A frozen repository task judged by its external grader.
    Repository,
}

/// An agent-neutral outcome the task author asks the harness to inspect.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "check", rename_all = "kebab-case", deny_unknown_fields)]
pub enum OutcomeCheck {
    /// The named file must have exactly these UTF-8 bytes.
    FileContent { path: PathBuf, content: String },
    /// The named file must have this SHA-256 digest.
    FileDigest { path: PathBuf, sha256: String },
    /// The named regular file must contain at least this many bytes.
    FileSize { path: PathBuf, bytes_at_least: u64 },
    /// The named workspace path must be absent.
    Absent { path: PathBuf },
    /// The named file must match the given pattern.
    Pattern { path: PathBuf, pattern: String },
    /// The named seed file must retain its original bytes.
    Protected { path: PathBuf },
    /// The settled attempt's terminal classification must have this name.
    End { end: String },
    /// The settled process must return this code, rather than a signal.
    ExitCode { code: i32 },
    /// Standard error's last retained complete line must match the pattern.
    LastLine { pattern: String },
    /// The actual final result text must match the pattern.
    Answer { pattern: String },
    /// The paired smith attempts must accept and advance their shared sign-in generation.
    CredentialRefresh,
}

/// A typed check over smith records, supplied by a task author.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "check", rename_all = "kebab-case", deny_unknown_fields)]
pub enum EventCheck {
    /// A captured path call has its exact joined successful terminal.
    ToolPath {
        tool: String,
        path: String,
        #[serde(default)]
        aliases: Vec<String>,
        verdict: String,
    },
    /// One successful captured write supplies the exact settled file bytes.
    WrittenFile { path: PathBuf, content_bytes_at_least: u64 },
    /// Exactly one write is refused by the authored protocol or native payload bound.
    OversizedWrite {
        path: PathBuf,
        content_bytes_above: u64,
        /// Select protocol-only refusal against the actual stated call-input bound.
        protocol_input_bytes_above: Option<u64>,
    },
    /// An oversized write is repaired by a first write and captured extending edits.
    FilePieces {
        path: PathBuf,
        bytes_above: u64,
        /// The authored native `file_bytes` bound, also bounding replay scratch.
        file_bytes: u64,
    },
    /// Calls of one tool match the supplied cardinality and terminal metadata.
    ToolCall {
        tool: String,
        count: Option<Count>,
        input_bytes_at_least: Option<u64>,
        verdict: Option<String>,
        exit: Option<i64>,
        clamped: Option<bool>,
    },
    /// Tool starts match the count the task asks of the model.
    CallCount { tool: String, count: Count },
    /// An actual tool terminal retains at least this many result bytes.
    ToolResultBytes { tool: String, at_least: u64 },
    /// A shell request times out at its emitted bound, with its exact clamp and grace.
    ToolTimeout { requested_seconds: u32, deadline_ms: u64, clamped: bool, grace_ms: u64, command: Option<String> },
    /// A real provider call exceeds the exercise floor and retains its timing and opaque sizes.
    SlowCompletion { longer_than_ms: u64, head_ms: u64, idle_ms: u64, reasoning_bytes: u64 },
    /// Response usage sums match each authoritative run total.
    UsageSums,
    /// A shell call answers running with its conversation's job number.
    JobRunning,
    /// A request starts after a job's running answer and before its terminal.
    CompletionDuringJob,
    /// Every observed job has one end reported by a call or its closing session.
    JobTerminal,
    /// No tool ends with any of these verdicts.
    NoVerdict { verdicts: Vec<String> },
    /// Completions have this cardinality before the named boundary.
    Completions { count: Count, before: Before },
    /// The first captured request's new messages contain the task.
    FirstRequestHoldsTask,
    /// No response or wait precedes receipt of the task message.
    NothingBeforeTask,
    /// Every request has exactly one read or refused terminal.
    HandledOnce,
    /// Every person exchange produces one completion.
    CompletionPerExchange,
    /// No completed response has this stop classification.
    NoStop { stop: String },
    /// Response blocks have the supplied oversized and cut cardinalities.
    Blocks { oversized: Option<Count>, cut: Option<Count> },
    /// Conversations of this native kind have this cardinality.
    ConversationOpened { kind: String, count: Count },
    /// No measured prompt exceeds its conversation's declared window.
    PromptWithinWindow,
    /// Warmed responses satisfy the calibrated cache reuse thresholds.
    CacheReuse { median_at_least: f64, full_misses_per_ten_at_most: u32 },
    /// Main completes after its child returns within the run budget.
    MainAfterChild,
    /// A budget ending retains text and requests no tools in its final turn.
    FinalTurn,
    /// Every request in a compaction conversation disables tool calls.
    CompactionChoice,
    /// Compaction carries owed results and opens its parent with its summary.
    CompactionCarried,
    /// The child stays within its opening's declared completion share.
    ChildShare { turns_at_most: u32 },
    /// Prompt budget text agrees with snapshots and children avoid the reserve.
    BudgetVisible,
    /// The last child usage's told remainder matches its writing-step budget.
    BudgetTold,
    /// Main's report repeats the last remainder it was actually told.
    BudgetReport,
    /// Reported spend does not exceed this notional dollar ceiling.
    SpendWithin { ceiling_usd: f64 },
    /// One provider answer contains this many calls of the named tool.
    CallsInOneResponse { tool: String, at_least: u32 },
    /// Each captured response has at most this many calls of the named tool.
    CallsPerResponse { tool: String, at_most: u32 },
    /// Every child returns before its successor is opened.
    ChildrenInTurn { count: u32 },
    /// This many child conversations overlap on the emission clock.
    ChildrenOverlap { at_least: u32 },
    /// No response has this completion failure class.
    NoFailure { class: String },
    /// The authoritative run terminal has this status and optional typed failure identity.
    RunCompleted { status: String, class: Option<String>, which: Option<String>, reason: Option<String> },
    /// The run retains actual provider rejection status and bounded diagnostic content.
    ProviderFailure { status_at_least: u16, status_at_most: u16, detail_bytes_at_most: u32 },
    /// The process refuses before making a provider request.
    NoRequest,
    /// A configured startup refusal emits no run or provider event stream.
    NoRun,
}

impl EventCheck {
    /// The stable task and result name of this typed predicate.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::ToolPath { .. } => "tool-path",
            Self::WrittenFile { .. } => "written-file",
            Self::FilePieces { .. } => "file-pieces",
            Self::OversizedWrite { .. } => "oversized-write",
            Self::ToolCall { .. } => "tool-call",
            Self::CallCount { .. } => "call-count",
            Self::ToolResultBytes { .. } => "tool-result-bytes",
            Self::ToolTimeout { .. } => "tool-timeout",
            Self::SlowCompletion { .. } => "slow-completion",
            Self::UsageSums => "usage-sums",
            Self::JobRunning => "job-running",
            Self::CompletionDuringJob => "completion-during-job",
            Self::JobTerminal => "job-terminal",
            Self::NoVerdict { .. } => "no-verdict",
            Self::Completions { .. } => "completions",
            Self::FirstRequestHoldsTask => "first-request-holds-task",
            Self::NothingBeforeTask => "nothing-before-task",
            Self::HandledOnce => "handled-once",
            Self::CompletionPerExchange => "completion-per-exchange",
            Self::NoStop { .. } => "no-stop",
            Self::Blocks { .. } => "blocks",
            Self::ConversationOpened { .. } => "conversation-opened",
            Self::PromptWithinWindow => "prompt-within-window",
            Self::CacheReuse { .. } => "cache-reuse",
            Self::MainAfterChild => "main-after-child",
            Self::FinalTurn => "final-turn",
            Self::CompactionChoice => "compaction-choice",
            Self::CompactionCarried => "compaction-carried",
            Self::ChildShare { .. } => "child-share",
            Self::BudgetVisible => "budget-visible",
            Self::BudgetTold => "budget-told",
            Self::BudgetReport => "budget-report",
            Self::SpendWithin { .. } => "spend-within",
            Self::CallsInOneResponse { .. } => "calls-in-one-response",
            Self::CallsPerResponse { .. } => "calls-per-response",
            Self::ChildrenInTurn { .. } => "children-in-turn",
            Self::ChildrenOverlap { .. } => "children-overlap",
            Self::NoFailure { .. } => "no-failure",
            Self::RunCompleted { .. } => "run-completed",
            Self::ProviderFailure { .. } => "provider-failure",
            Self::NoRequest => "no-request",
            Self::NoRun => "no-run",
        }
    }
}

/// A task author's bound on one event cardinality.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Count {
    /// Exactly this cardinality.
    Exactly(u32),
    /// At most this cardinality.
    AtMost(u32),
    /// At least this cardinality.
    AtLeast(u32),
}

impl Count {
    /// Compare an observed cardinality without narrowing its unit.
    #[must_use]
    pub fn holds(self, observed: u64) -> bool {
        match self {
            Self::Exactly(value) => observed == u64::from(value),
            Self::AtMost(value) => observed <= u64::from(value),
            Self::AtLeast(value) => observed >= u64::from(value),
        }
    }
}

/// The boundary up to which a task counts completions.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Before {
    /// Every completion up to the authoritative answer.
    Answer,
    /// Those preceding the first finish tool.
    Finish,
}

/// A command the author gives the grader after the agent's tree has ended.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Grade {
    pub command: Vec<String>,
    pub deadline_seconds: u64,
    pub max_output_bytes: u64,
}

/// Limits supplied by the author to an agent as far as its interface permits.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    pub turns: Option<u64>,
    pub seconds: Option<u64>,
    /// A notional spend ceiling in US dollars.
    pub spend: Option<f64>,
}

/// Provisional task costs used until committed runs provide measurements.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Estimate {
    pub seconds: u64,
    pub tokens: u64,
}

/// The binary, attempts and frozen inputs written by calibrate (benchmarks.md, section 5.2).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Calibration {
    pub binary: String,
    pub binary_sha256: String,
    pub date: String,
    /// A committed path relative to the benchmarks directory.
    pub summary: String,
    pub attempts: u32,
    pub passes: Vec<String>,
    pub deviations: Vec<String>,
    /// The failing control's commit, or `none: <why>`.
    pub control: String,
    pub failed_before: Vec<String>,
    pub prompt_sha256: String,
    pub seed_sha256: String,
    pub checks_sha256: String,
}

/// A named prompt, settings or check change supplied by a task's author.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub calibration: Option<Calibration>,
    /// Replace the base checks and deciding names; absent keeps historical additions.
    #[serde(default)]
    pub replace_checks: bool,
    /// Result check names that decide this variant, added to the base task.
    #[serde(default)]
    pub deciding: Vec<String>,
    pub name: String,
    pub prompt: Option<String>,
    pub effort: Option<String>,
    pub person: Option<Vec<crate::agent::Cue>>,
    pub input_end: Option<crate::agent::Close>,
    #[serde(default)]
    pub smith: BTreeMap<String, toml::Value>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    #[serde(default)]
    pub outcome: Vec<OutcomeCheck>,
    #[serde(default)]
    pub event: Vec<EventCheck>,
    #[serde(default)]
    pub waives: Vec<crate::Passive>,
    pub budget: Option<Budget>,
}

/// A frozen task supplied by its author to offline checks and the runner.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Task {
    /// Result check names that decide this task.
    #[serde(default)]
    pub deciding: Vec<String>,
    pub id: String,
    pub version: u32,
    pub kind: Kind,
    pub title: Option<String>,
    #[serde(default)]
    pub guards: Vec<String>,
    #[serde(default)]
    pub behaviours: Vec<String>,
    pub tier: String,
    pub effort: Option<String>,
    pub prompt: String,
    #[serde(default)]
    pub person: Vec<crate::agent::Cue>,
    pub input_end: Option<crate::agent::Close>,
    pub deadline_seconds: u64,
    pub estimate: Option<Estimate>,
    pub seed_sha256: String,
    /// A seed-relative working directory; absent keeps the entire seed root.
    pub working_directory: Option<PathBuf>,
    #[serde(default)]
    pub budget: Budget,
    #[serde(default)]
    pub smith: BTreeMap<String, toml::Value>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    #[serde(default)]
    pub setup: Vec<Vec<String>>,
    #[serde(default)]
    pub protected: Vec<PathBuf>,
    #[serde(default)]
    pub outcome: Vec<OutcomeCheck>,
    #[serde(default)]
    pub event: Vec<EventCheck>,
    #[serde(default)]
    pub grade: Vec<Grade>,
    #[serde(default)]
    pub waives: Vec<crate::Passive>,
    pub repetitions: Option<u32>,
    #[serde(default)]
    pub variant: Vec<Variant>,
    pub calibration: Option<Calibration>,
}

impl Task {
    /// Digest effective checks and deciding names in their authored order.
    pub fn checks_sha256(&self) -> Result<String, String> {
        #[derive(Serialize)]
        struct Checks<'a> {
            outcome: &'a [OutcomeCheck],
            event: &'a [EventCheck],
            grade: &'a [Grade],
            deciding: &'a [String],
        }
        let checks =
            Checks { outcome: &self.outcome, event: &self.event, grade: &self.grade, deciding: &self.deciding };
        Ok(format!("{:x}", Sha256::digest(toml::to_string(&checks).map_err(|error| error.to_string())?)))
    }

    /// A block gates only the effective prompt, seed and checks it calibrated.
    pub fn calibrated(&self) -> Result<bool, String> {
        let Some(block) = &self.calibration else { return Ok(false) };
        Ok(block.prompt_sha256 == format!("{:x}", Sha256::digest(self.prompt.as_bytes()))
            && block.seed_sha256 == self.seed_sha256
            && block.checks_sha256 == self.checks_sha256()?)
    }

    /// Keep a Smith task's deliberate model override in its frozen identity.
    pub(crate) fn requested_model<'a>(&'a self, agent: crate::Agent, default: &'a str) -> Result<&'a str, String> {
        if agent == crate::Agent::Smith
            && let Some(value) = self.smith.get("model")
        {
            return value
                .as_str()
                .filter(|model| !model.is_empty())
                .ok_or_else(|| "Smith model override must be a nonempty string".into());
        }
        Ok(default)
    }

    /// Apply one named authored change, retaining additive checks unless replacement is explicit.
    pub fn with_variant(&self, name: &str) -> Result<Self, String> {
        let variant = self.variant.iter().find(|variant| variant.name == name).ok_or("unknown task variant")?;
        let mut task = self.clone();
        task.calibration.clone_from(&variant.calibration);
        if let Some(prompt) = &variant.prompt {
            task.prompt.clone_from(prompt);
        }
        if let Some(effort) = &variant.effort {
            task.effort = Some(effort.clone());
        }
        if let Some(person) = &variant.person {
            task.person.clone_from(person);
        }
        if let Some(close) = variant.input_end {
            task.input_end = Some(close);
        }
        if variant.replace_checks {
            task.deciding.clear();
            task.outcome.clear();
            task.event.clear();
        }
        task.deciding.extend(variant.deciding.clone());
        task.environment.extend(variant.environment.clone());
        task.outcome.extend(variant.outcome.clone());
        task.event.extend(variant.event.clone());
        task.waives.extend(variant.waives.clone());
        task.smith.extend(variant.smith.clone());
        if let Some(budget) = &variant.budget {
            task.budget.clone_from(budget);
        }
        Ok(task)
    }
}

/// A manifest or input refusal returned by the harness to its caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub file: PathBuf,
    pub key: String,
    pub reason: String,
}

impl Refusal {
    /// A caller's named refusal, retaining the document and exact key path.
    pub fn new(file: &Path, key: &str, reason: impl Into<String>) -> Self {
        Self { file: file.to_path_buf(), key: key.to_owned(), reason: reason.into() }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}: {}", self.file.display(), self.key, self.reason)
    }
}

impl std::error::Error for Refusal {}

/// Parse a task document and name the file and key path on refusal.
pub fn read_task(file: &Path) -> Result<Task, Refusal> {
    crate::formats::read_document(file)
}

#[cfg(test)]
pub(crate) fn parse_task(file: &Path, text: &str) -> Result<Task, Refusal> {
    crate::formats::parse_document(file, text)
}
