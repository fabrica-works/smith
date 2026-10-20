//! Effective numeric workspace-tool declarations supplied by the service.
//! This value owns no authority, paths or endpoint data and admits no call.
//! Contract: protocol/llm.md, section 3; protocol/limits.md, sections 2 and 3.

use skein_lib::Duration;
use smith_domain::tools;

/// The service's immutable effective quantities stated when a conversation opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Stated {
    /// Maximum rendered host part and raw sub-agent brief bytes.
    pub host_part: u64,
    pub path_bytes: u32,
    /// Default read window, in bytes.
    pub read_window: u32,
    /// Maximum content and replacement snippets in one piece, in bytes.
    pub tool_payload: u32,
    /// Largest file read or edited whole, in bytes.
    pub file_bytes: u32,
    /// Current complete provider-written call input, including JSON framing, in bytes.
    pub call_input_bytes: u32,
    pub list_entries: u32,
    pub search_hits: u32,
    pub search_bytes: u32,
    pub shell_timeout: Duration,
    pub tool_deadline: Duration,
    pub job_poll: Duration,
    pub job_wait: Duration,
    pub jobs: u32,
    /// Default kept command output, in bytes.
    pub shell_output: u32,
    /// Combined head and tail capture ceiling, in bytes.
    pub output_ceiling: u32,
}

impl Stated {
    /// Copy the service's effective numbers; no policy or capability is inferred.
    #[must_use]
    pub fn from_tools(limits: &tools::Limits, call_input_bytes: u32) -> Option<Stated> {
        let output = limits.shell_head.checked_add(limits.shell_tail)?;
        Some(Stated {
            host_part: u64::MAX,
            path_bytes: limits.path_bytes,
            read_window: limits.read_bytes,
            tool_payload: limits.tool_payload,
            file_bytes: limits.file_bytes,
            call_input_bytes,
            list_entries: limits.list_entries,
            search_hits: limits.search_hits,
            search_bytes: limits.search_bytes,
            shell_timeout: limits.shell_timeout,
            tool_deadline: limits.tool_deadline,
            job_poll: limits.job_poll,
            job_wait: limits.job_wait,
            jobs: limits.jobs,
            shell_output: limits.shell_output,
            output_ceiling: output,
        })
    }
}
