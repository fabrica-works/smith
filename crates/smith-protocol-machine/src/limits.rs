//! Bounded machine ownership (programming-model.md, section 6.3;
//! protocol/agent.md, section 2).

use alloc::boxed::Box;
use core::mem::size_of;
use skein_json::tokenizer;
use skein_lib::stream::Down;
use skein_lib::{Duration, List, Map, Queue, Stack, Token};

use crate::component::{Pending, Root};
use crate::process::Process;

/// File translation capacity, checked before effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Maximum simultaneous file requests.
    pub operations: u32,
    /// Maximum admitted workspace roots.
    pub roots: u32,
    /// Maximum relative path bytes.
    pub path_bytes: u32,
    /// Maximum content bytes for load or store.
    pub file_bytes: u32,
    /// Maximum entries retained from a tree listing.
    pub entries: u32,
    /// Maximum retained entry cells and names.
    pub entry_bytes: u64,
    /// Maximum simultaneous child processes.
    pub processes: u32,
    /// Upper bound for either command output window.
    pub output_bytes: u32,
    /// Upper bound for retained search hits.
    pub search_hits: u32,
    /// Upper bound for retained search path and text bytes.
    pub search_bytes: u32,
    /// Maximum one rg JSON or NUL-delimited path record accepted for parsing.
    pub search_line_bytes: u32,
    /// Maximum configured command environment bytes.
    pub env_bytes: u32,
    /// Grace between a polite signal and a kill.
    pub group_stop: Duration,
}

/// The most records one step can emit into each queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaxOut {
    /// Domain terminals.
    pub to_domain: u32,
    /// IO requests.
    pub below: u32,
}

/// A terminal per process and room for its stop followed by close.
#[must_use]
pub const fn max_out(limits: &Limits) -> MaxOut {
    let stopping = limits.processes.checked_mul(2).expect("process emission count fits");
    let below = if stopping > 3 { stopping } else { 3 };
    MaxOut { to_domain: if limits.processes == 0 { 1 } else { limits.processes }, below }
}

/// Heap retained by the component, plus its largest transient translation.
/// Caller-owned request buffers handed below are accounted by their owner.
#[must_use]
pub fn worst_case(limits: &Limits) -> Option<u64> {
    // Installing a new workspace temporarily coexists with the old root list.
    let roots = List::<Root>::worst_case(limits.roots)?.checked_mul(2)?;
    let operations = Map::<Token, Pending>::worst_case(limits.operations)?;
    let guides = Map::<Token, crate::guide::Guide>::worst_case(limits.operations)?
        .checked_add(u64::from(limits.file_bytes).checked_mul(u64::from(limits.operations))?)?;
    let listing = List::<Option<smith_domain::tools::Entry>>::worst_case(limits.entries)?
        .checked_mul(2)?
        .checked_add(limits.entry_bytes)?
        .checked_add(u64::from(limits.file_bytes).checked_mul(2)?)?
        .checked_add(u64::from(limits.path_bytes).checked_mul(2)?)?
        .checked_add(512)?
        .checked_add(u64::from(limits.search_line_bytes).checked_mul(5)?)?
        .checked_mul(u64::from(limits.processes))?;
    let processes = Map::<Token, Process>::worst_case(limits.processes)?;
    let pipes = Map::<Token, Token>::worst_case(limits.processes.checked_mul(2)?)?;
    let window = limits.output_bytes.max(limits.search_bytes);
    let output = u64::from(window).checked_mul(2)?.checked_mul(u64::from(limits.processes))?;
    let search = u64::from(limits.search_bytes)
        .checked_add(u64::from(limits.search_line_bytes))?
        .checked_add(
            u64::from(limits.search_hits).checked_mul(u64::try_from(size_of::<smith_domain::tools::Hit>()).ok()?)?,
        )?
        .checked_mul(u64::from(limits.processes))?;
    let variables = u64::from(limits.env_bytes / 2).checked_mul(u64::try_from(size_of::<Box<[u8]>>()).ok()?)?;
    let encoded_environment = variables.checked_add(u64::from(limits.env_bytes))?;
    // Startup name validation and one spawn copy are disjoint transients.
    let environment = encoded_environment
        .checked_add(encoded_environment.max(skein_lib::Set::<&[u8]>::worst_case(limits.env_bytes / 2)?))?;
    let tokenizer_limits = tokenizer::Limits {
        depth: 8,
        string: limits.search_line_bytes.max(1),
        number: 20,
        chunk: 64,
        length: limits.search_line_bytes.max(1),
    };
    // One JSON line parses at a time. Its token payloads and a sorting swap
    // can overlap the retained line and hit buffers at their largest size.
    let parse = tokenizer::worst_case(&tokenizer_limits)?
        .checked_add(Queue::<tokenizer::Event>::worst_case(1)?)?
        .checked_add(Queue::<Down>::worst_case(1)?)?
        .checked_add(Stack::<u8>::worst_case(8)?)?
        .checked_add(u64::from(limits.search_line_bytes).checked_mul(2)?)?
        .checked_add(u64::from(limits.search_bytes).checked_mul(2)?)?;
    let scratch = List::<Token>::worst_case(limits.processes)?
        .checked_add(u64::from(window).checked_mul(2)?)?
        .checked_add(parse)?;
    roots
        .checked_add(operations)?
        .checked_add(guides)?
        .checked_add(processes)?
        .checked_add(pipes)?
        .checked_add(output)?
        // lib.md, section 7: each output pipe can deliver its maximum.
        .checked_add(u64::from(crate::process::OUTPUT_READ).checked_mul(2)?)?
        .checked_add(search)?
        .checked_add(listing)?
        .checked_add(environment)?
        .checked_add(scratch)
}

/// Numeric declarations needed by this layer's transitional derivation.
/// The service supplies only owned numbers and this domain's own child limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Derivation {
    pub operations: u32,
    pub roots: u32,
    pub path_bytes: u32,
    pub file_bytes: u32,
    pub entries: u32,
    pub entry_bytes: u64,
    pub shell_head: u32,
    pub shell_tail: u32,
    pub search_hits: u32,
    pub search_bytes: u32,
    pub environment_bytes: u32,
    pub group_stop: Duration,
}

/// The first field this layer cannot derive, returned to its parent before effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// A JSON search line, derived from retained bytes and escaping.
    SearchLineBytes,
}

impl Field {
    /// The field name in the effective limits, without its layer.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SearchLineBytes => "search_line_bytes",
        }
    }
}

/// Derive this layer from declarations, refusing checked arithmetic overflow.
/// Contract: protocol/limits.md, sections 2 and 3.
pub fn derive(inputs: &Derivation) -> Result<Limits, Field> {
    Ok(Limits {
        operations: inputs.operations,
        roots: inputs.roots,
        path_bytes: inputs.path_bytes,
        file_bytes: inputs.file_bytes,
        entries: inputs.entries,
        entry_bytes: inputs.entry_bytes,
        processes: inputs.operations.min(8),
        output_bytes: inputs.shell_head.max(inputs.shell_tail).max(inputs.search_bytes),
        search_hits: inputs.search_hits,
        search_bytes: inputs.search_bytes,
        // JSON escapes one text/path byte into at most six bytes. The fixed
        // envelope includes match offsets, line number and routing keys.
        search_line_bytes: inputs
            .search_bytes
            .checked_add(inputs.path_bytes)
            .ok_or(Field::SearchLineBytes)?
            .checked_mul(6)
            .ok_or(Field::SearchLineBytes)?
            .checked_add(256)
            .ok_or(Field::SearchLineBytes)?,
        env_bytes: inputs.environment_bytes,
        group_stop: inputs.group_stop,
    })
}
