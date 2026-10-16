//! Named-thread collection retains immutable response ids and bounded raw files.
//! It opens Codex's state database read-only and follows only this root's edges;
//! unrelated sessions and credentials are never read. A missing parent remains
//! an unavailable child scope (benchmarks.md, sections 6, 8.2, 10 and 14).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Deserialize;

use crate::{Convention, ScopeKind, TokenLedger, TokenRecord, Usage};

use super::ParseError;

mod cache;
mod time;

const MAX_THREADS: usize = 4096;
const MAX_BYTES: usize = 64 * 1024 * 1024;

/// One immutable provider response supplied by a named thread's rollout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseUsage {
    pub response: String,
    pub model: String,
    pub usage: Usage,
}

/// A child named by this attempt and its own persisted parent evidence.
#[derive(Debug)]
pub struct ChildThread {
    pub thread: String,
    pub link: Link,
    pub responses: Vec<ResponseUsage>,
}

/// Whether the child's native metadata identifies its parent.
#[derive(Debug, PartialEq, Eq)]
pub enum Link {
    /// Native metadata links this child to a named parent in the attempt.
    Linked { parent: String },
    /// The child is known, but no trustworthy parent is persisted.
    Unlinked,
}

/// The bounded raw copy of one selected thread, for the guarded archive.
#[derive(Debug)]
pub struct RolloutFile {
    pub thread: String,
    pub bytes: Vec<u8>,
}

/// Root and child response evidence, never cumulative turn totals.
#[derive(Debug)]
pub struct Rollouts {
    pub responses: Vec<ResponseUsage>,
    pub children: Vec<ChildThread>,
    pub files: Vec<RolloutFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_field_names)] // These are the pinned persisted field names.
struct NativeUsage {
    input_tokens: u64,
    cached_input_tokens: u64,
    cache_write_input_tokens: Option<u64>,
    output_tokens: u64,
    reasoning_output_tokens: Option<u64>,
    total_tokens: u64,
}

impl NativeUsage {
    fn usage(self) -> Result<Usage, String> {
        if self.input_tokens.checked_add(self.output_tokens) != Some(self.total_tokens) {
            return Err("total_tokens differs from input plus output".into());
        }
        let usage = Usage::Codex {
            input: self.input_tokens,
            cached_input: self.cached_input_tokens,
            cache_write: self.cache_write_input_tokens,
            output: self.output_tokens,
            reasoning: self.reasoning_output_tokens,
        };
        usage.normalise()?;
        Ok(usage)
    }
}

fn bad(record: &str, path: &str, reason: impl Into<String>) -> ParseError {
    ParseError { record: record.into(), path: path.into(), reason: reason.into() }
}

fn required_string<'a>(value: &'a serde_json::Value, name: &str) -> Result<&'a str, ParseError> {
    value
        .get(name)
        .and_then(serde_json::Value::as_str)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| bad("rollout", name, "expected a nonempty string"))
}

fn metadata(
    payload: &serde_json::Value,
    thread: &str,
    parents: &mut BTreeMap<String, Option<String>>,
    ancestors: &mut BTreeSet<String>,
) -> Result<(), ParseError> {
    let id = required_string(payload, "id")?;
    if id != thread && !ancestors.contains(id) {
        return Err(bad("session_meta", "payload.id", "rollout metadata is neither its owner nor a declared ancestor"));
    }
    let parent = match payload
        .pointer("/source/subagent/thread_spawn/parent_thread_id")
        .or_else(|| payload.get("parent_thread_id"))
    {
        Some(serde_json::Value::String(id)) if !id.is_empty() => Some(id.clone()),
        Some(serde_json::Value::Null) | None => None,
        Some(_) => return Err(bad("session_meta", "payload.parent_thread_id", "invalid parent id")),
    };
    if parent.as_deref() == Some(id) || (id != thread && parent.as_deref() == Some(thread)) {
        return Err(bad("session_meta", "payload.parent_thread_id", "cyclic parent metadata"));
    }
    if let Some(previous) = parents.get(id)
        && previous != &parent
    {
        return Err(bad("session_meta", "payload.parent_thread_id", "conflicting repeated metadata"));
    }
    if let Some(parent) = &parent {
        ancestors.insert(parent.clone());
    }
    parents.insert(id.to_owned(), parent);
    if parents.len() > MAX_THREADS || ancestors.len() > MAX_THREADS {
        return Err(bad("session_meta", "payload", "metadata lineage exceeds the thread bound"));
    }
    Ok(())
}

/// Decode the pinned rollout's relevant fields; unknown kinds are skipped.
/// Known usage shapes are strict and conflicting duplicate response ids refuse.
pub fn parse_rollout(bytes: &[u8], thread: &str) -> Result<(Link, Vec<ResponseUsage>), ParseError> {
    let text = std::str::from_utf8(bytes).map_err(|error| bad("rollout", "UTF-8", error.to_string()))?;
    let mut parents = BTreeMap::new();
    let mut ancestors = BTreeSet::new();
    let mut responses = BTreeMap::new();
    let mut order = Vec::new();
    let mut model = String::new();
    for (offset, line) in text.lines().enumerate() {
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| bad("rollout", &format!("line {}", offset + 1), error.to_string()))?;
        let kind = required_string(&value, "type")?;
        match kind {
            "session_meta" => {
                let payload = value.get("payload").ok_or_else(|| bad(kind, "payload", "missing metadata"))?;
                metadata(payload, thread, &mut parents, &mut ancestors)?;
            }
            "turn_context" => {
                let payload = value.get("payload").ok_or_else(|| bad(kind, "payload", "missing context"))?;
                model = required_string(payload, "model")?.into();
            }
            "token_usage_record" => {
                let payload = value.get("payload").ok_or_else(|| bad(kind, "payload", "missing usage"))?;
                if required_string(payload, "thread_id")? != thread {
                    return Err(bad(kind, "payload.thread_id", "response belongs to another thread"));
                }
                let response = required_string(payload, "response_id")?.to_owned();
                let native: NativeUsage = serde_json::from_value(
                    payload.get("usage").cloned().ok_or_else(|| bad(kind, "payload.usage", "missing usage"))?,
                )
                .map_err(|error| bad(kind, "payload.usage", error.to_string()))?;
                let usage = native.usage().map_err(|reason| bad(kind, "payload.usage", reason))?;
                if !responses.contains_key(&response) {
                    order.push(response.clone());
                }
                if let Some(previous) = responses.insert(response.clone(), (model.clone(), usage.clone()))
                    && previous != (model.clone(), usage)
                {
                    return Err(bad(kind, "payload.response_id", "conflicting repeated usage"));
                }
            }
            _ => {}
        }
    }
    if !parents.contains_key(thread) {
        return Err(bad("session_meta", "payload", "named rollout has no metadata"));
    }
    Ok((
        parents
            .get(thread)
            .expect("owner metadata checked")
            .clone()
            .map_or(Link::Unlinked, |parent| Link::Linked { parent }),
        order
            .into_iter()
            .map(|response| {
                let (model, usage) = responses.remove(&response).expect("observed response");
                ResponseUsage { response, model, usage }
            })
            .collect(),
    ))
}

fn read_file(home: &Path, file: &Path, remaining: &mut usize) -> Result<Vec<u8>, ParseError> {
    let resolved = file.canonicalize().map_err(|error| bad("rollout", "file", error.to_string()))?;
    let home = home.canonicalize().map_err(|error| bad("rollout", "home", error.to_string()))?;
    if !resolved.starts_with(home.join("sessions")) && !resolved.starts_with(home.join("archived_sessions")) {
        return Err(bad("rollout", "file", "named rollout is outside Codex's session directories"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(resolved)
        .map_err(|error| bad("rollout", "file", error.to_string()))?
        .take(u64::try_from(*remaining).expect("bounded bytes") + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| bad("rollout", "file", error.to_string()))?;
    *remaining =
        remaining.checked_sub(bytes.len()).ok_or_else(|| bad("rollout", "bytes", "attempt rollout bound exceeded"))?;
    Ok(bytes)
}

/// Read only the named root and descendants from the pinned Codex state store.
pub fn collect_rollouts(home: &Path, thread: &str) -> Result<Rollouts, ParseError> {
    let mut result = Rollouts { responses: Vec::new(), children: Vec::new(), files: Vec::new() };
    collect_named(home, thread, &[], &mut result)?;
    Ok(result)
}

pub(super) fn collect_named(
    home: &Path,
    thread: &str,
    known_children: &[String],
    result: &mut Rollouts,
) -> Result<(), ParseError> {
    if thread.is_empty() {
        return Err(bad("rollout", "thread", "root id is empty"));
    }
    let database = home.join("state_5.sqlite");
    let mut ids = BTreeSet::from([thread.to_owned()]);
    ids.extend(known_children.iter().cloned());
    if ids.len() > MAX_THREADS {
        return Err(bad("rollout", "children", "thread bound exceeded"));
    }
    if !database.exists() {
        for child in ids.iter().filter(|id| id.as_str() != thread) {
            result.children.push(ChildThread { thread: child.clone(), link: Link::Unlinked, responses: Vec::new() });
        }
        return Ok(());
    }
    let connection =
        Connection::open_with_flags(database, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
            .map_err(|error| bad("state", "open", error.to_string()))?;
    let mut edges = connection
        .prepare("SELECT child_thread_id FROM thread_spawn_edges WHERE parent_thread_id=?1 ORDER BY child_thread_id")
        .map_err(|error| bad("state", "thread_spawn_edges", error.to_string()))?;
    let mut pending: Vec<_> = ids.iter().cloned().collect();
    let mut offset = 0;
    while offset < pending.len() {
        let rows = edges
            .query_map([&pending[offset]], |row| row.get::<_, String>(0))
            .map_err(|error| bad("state", "children", error.to_string()))?;
        for child in rows {
            let child = child.map_err(|error| bad("state", "child_thread_id", error.to_string()))?;
            if child.is_empty() {
                return Err(bad("state", "child_thread_id", "empty child id"));
            }
            if ids.insert(child.clone()) {
                if ids.len() > MAX_THREADS {
                    return Err(bad("state", "children", "thread bound exceeded"));
                }
                pending.push(child);
            }
        }
        offset += 1;
    }
    let mut remaining = MAX_BYTES;
    for id in &ids {
        let file: Option<Option<String>> = connection
            .query_row("SELECT rollout_path FROM threads WHERE id=?1", [id], |row| row.get(0))
            .optional()
            .map_err(|error| bad("state", "threads.rollout_path", error.to_string()))?;
        let (mut link, responses) = match file.flatten() {
            Some(file) => {
                let bytes = read_file(home, &PathBuf::from(file), &mut remaining)?;
                result.files.push(RolloutFile { thread: id.clone(), bytes });
                parse_rollout(&result.files.last().expect("raw file retained").bytes, id)
                    .map_err(|error| bad("rollout", "file", format!("thread {id}: {error}")))?
            }
            None => (Link::Unlinked, Vec::new()),
        };
        if let Link::Linked { parent } = &link
            && (!ids.contains(parent) || parent == id)
        {
            link = Link::Unlinked;
        }
        if id == thread {
            result.responses = responses;
        } else {
            result.children.push(ChildThread { thread: id.clone(), link, responses });
        }
    }
    Ok(())
}

impl Rollouts {
    /// Replace root turn aggregates with immutable per-response scope evidence.
    pub fn token_records(&self, root: &str) -> Result<Vec<TokenRecord>, String> {
        self.token_ledger(root)?.records()
    }

    /// Retain response records with each rollout's resolved model.
    pub fn token_ledger(&self, root: &str) -> Result<TokenLedger, String> {
        let mut ledger = TokenLedger::default();
        ledger.declare(root, ScopeKind::Root, Convention::Codex, "named root has no per-response usage")?;
        for response in &self.responses {
            ledger.record_model(root, &response.response, &response.model, response.usage.clone())?;
        }
        for child in &self.children {
            match &child.link {
                Link::Linked { parent } => {
                    ledger.declare(
                        &child.thread,
                        ScopeKind::Child { parent: parent.clone() },
                        Convention::Codex,
                        "named child has no per-response usage",
                    )?;
                    for response in &child.responses {
                        ledger.record_model(
                            &child.thread,
                            &response.response,
                            &response.model,
                            response.usage.clone(),
                        )?;
                    }
                }
                Link::Unlinked => {
                    let missing = format!("child {} has no persisted parent link", child.thread);
                    ledger.declare(&child.thread, ScopeKind::UnlinkedChild, Convention::Codex, &missing)?;
                }
            }
        }
        Ok(ledger)
    }
}

impl Rollouts {
    /// Resolve the root's first native model and declared context window.
    pub fn resolve_model(&self, thread: &str, identity: &mut crate::ModelIdentity) -> Result<(), ParseError> {
        identity.resolved_model = crate::Measure::unavailable("root rollout turn_context model not reported");
        identity.resolved_window =
            crate::Measure::unavailable("root rollout token_count model_context_window not reported");
        let Some(file) = self.files.iter().find(|file| file.thread == thread) else { return Ok(()) };
        let text = std::str::from_utf8(&file.bytes).map_err(|error| bad("rollout", "UTF-8", error.to_string()))?;
        let mut model_seen = false;
        let mut window_seen = false;
        for line in text.lines() {
            let value: serde_json::Value =
                serde_json::from_str(line).map_err(|error| bad("rollout", "JSON", error.to_string()))?;
            if value["type"] == "turn_context" && !model_seen {
                model_seen = true;
                if let Some(model) = value["payload"]["model"].as_str().filter(|model| !model.is_empty()) {
                    identity.resolved_model = crate::Measure::Observed { value: model.into() };
                }
            }
            if value["type"] == "event_msg" && value["payload"]["type"] == "token_count" && !window_seen {
                window_seen = true;
                if let Some(window) =
                    value["payload"]["info"]["model_context_window"].as_u64().filter(|window| *window > 0)
                {
                    identity.resolved_window = crate::Measure::Observed { value: window };
                }
            }
        }
        Ok(())
    }
}

impl Rollouts {
    fn reaches_root(&self, child: &ChildThread, root: &str) -> bool {
        let mut current = child;
        for _ in 0..self.children.len() {
            match &current.link {
                Link::Linked { parent } if parent == root => return true,
                Link::Linked { parent } => {
                    let Some(next) = self.children.iter().find(|child| child.thread == *parent) else { return false };
                    current = next;
                }
                Link::Unlinked => return false,
            }
        }
        false
    }

    /// Count responses per linked thread from changing native token-count totals.
    pub fn responses_per_thread(&self, root: &str) -> Result<BTreeMap<String, crate::Measure<u64>>, ParseError> {
        let mut counts = BTreeMap::new();
        for thread in std::iter::once(root).chain(
            self.children.iter().filter(|child| self.reaches_root(child, root)).map(|child| child.thread.as_str()),
        ) {
            let mut totals = BTreeSet::new();
            if let Some(file) = self.files.iter().find(|file| file.thread == thread) {
                let text =
                    std::str::from_utf8(&file.bytes).map_err(|error| bad("rollout", "UTF-8", error.to_string()))?;
                for line in text.lines() {
                    let value: serde_json::Value =
                        serde_json::from_str(line).map_err(|error| bad("rollout", "JSON", error.to_string()))?;
                    if value["type"] == "event_msg" && value["payload"]["type"] == "token_count" {
                        let usage = value["payload"]["info"]["total_token_usage"].clone();
                        if !usage.is_null() {
                            let native: NativeUsage = serde_json::from_value(usage.clone())
                                .map_err(|error| bad("token_count", "total_token_usage", error.to_string()))?;
                            native.usage().map_err(|reason| bad("token_count", "total_token_usage", reason))?;
                            totals.insert(usage.to_string());
                        }
                    }
                }
            }
            let count = if totals.is_empty() {
                crate::Measure::unavailable("thread has no token_count usage records")
            } else {
                crate::Measure::Observed { value: u64::try_from(totals.len()).expect("bounded rollout records") }
            };
            counts.insert(thread.into(), count);
        }
        Ok(counts)
    }

    /// Fill linked conversation counts and per-thread responses; shares stay unknown.
    pub fn counts(&self, root: &str, counts: &mut crate::Counts) -> Result<(), ParseError> {
        if !self.files.iter().any(|file| file.thread == root) {
            return Ok(());
        }
        let children = u64::try_from(self.children.iter().filter(|child| self.reaches_root(child, root)).count())
            .expect("bounded threads");
        counts.children = crate::Measure::Observed { value: children };
        counts.conversations = crate::Measure::Observed { value: children.checked_add(1).expect("bounded threads") };
        let mut responses = Some(0_u64);
        for count in self.responses_per_thread(root)?.values() {
            responses = match count {
                crate::Measure::Observed { value } => responses.and_then(|sum| sum.checked_add(*value)),
                crate::Measure::LowerBound { .. } | crate::Measure::Unavailable { .. } => None,
            };
        }
        counts.responses = responses.map_or_else(
            || crate::Measure::unavailable("linked thread token_count records missing"),
            |value| crate::Measure::Observed { value },
        );
        Ok(())
    }
}
