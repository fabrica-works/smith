//! Copied process limits: temper `25ac2ad`, domain/run.md, section 14.
//! The world supplies immutable caps; none is inferred from domain state.

use skein_lib::Duration;
use smith_domain::{Limits, run, session, tools};

/// Fixture window leaving one 4096-token output above its four-fifths threshold.
pub const MODEL_WINDOW: u32 = 20_480;

/// The most a charter may ask for in the calm world, which every session may
/// take.
///
/// Scripted-world contract: domain/run.md, sections 13 and 14; testing-strategy.md, section 7.
pub const BUDGET: run::Budget = run::Budget { turns: 64, spend: 10_000, time: Duration::from_secs(3600) };

const CEILING: session::Budget = session::Budget {
    turns: BUDGET.turns,
    input: 1 << 24,
    output: 1 << 24,
    cache_read: 1 << 26,
    cache_write: 1 << 24,
    time: BUDGET.time,
};

/// An agent process's limits in the calm world: room for the one run it
/// carries, with room for main and several child conversations.
///
/// Scripted-world contract: domain/run.md, sections 13 and 14; testing-strategy.md, section 7.
pub const LIMITS: Limits = Limits {
    accounts: 4,
    endpoints: 3,
    configured_model_bytes: 4096,
    decoded_call_bytes: 4096,
    skew: Duration::ZERO,
    run: run::Limits {
        runs: 1,
        conversations: 6,
        run_bytes: 1 << 16,
        brief_sections: 4,
        host_part: u64::MAX,
        room: 0,
        compaction_threshold: [4, 5],
        compaction_material: 0,
        schema_costs: run::SchemaCosts::NONE,
        directories: 2,
        directory_name_bytes: 256,
        host_path_bytes: 4096,
        conflicts: 64,
        conflict_path_bytes: 4096,
        host_tools: 2,
        host_input_bytes: 65_536,
        host_reply_bytes: 65_536,
        answered_calls: 16,
        answered_bytes: 4096,
        host_timeout: Duration::from_secs(60),
        host_timeout_max: Duration::from_secs(60),
        host_backoff: Duration::from_millis(50),
        verdicts: 2,
        calls: 16,
        budget: BUDGET,

        models: 2,
        run_conversations: 6,
        answer_bytes: 1024,
        nudges: 1,
        guide_bytes: 1024,
        io_timeout: Duration::from_secs(5),
        outcome_bytes: 4096,
        delivery_timeout: Duration::from_secs(60),
        check_timeout: Duration::from_secs(60),
        wrap_up: Duration::from_secs(60),
        check_tail: 512,
        facts: 1024,
        messages: 8,
        message_bytes: 4096,
        offer_messages: 8,
        offer_bytes: 32_782,
        waiting: skein_lib::Duration::from_secs(300),
    },
    session: session::Limits {
        threshold: [4, 5],
        token_bytes: skein_llm::TOKEN_BYTES,
        sessions: 6,
        spend: 10_000,
        protocol_allowance: 4096,
        history_blocks: u32::MAX,
        history_messages: 64,
        history_bytes: 33_554_432,
        completion_bytes: 4096,
        answer_bytes: 4096,
        completion_blocks: 16,
        failure_bytes: 512,
        delegated_result_bytes: 4_194_304,
        budget: CEILING,
        retries: 3,
        backoff_base: Duration::from_millis(200),
        backoff_max: Duration::from_secs(5),

        facts: 1024,
        parallel_tools: 4,
        tools: tools::Limits {
            shell_output: 512,
            kits: 6,
            calls: 4,
            repos: 2,
            path_bytes: 256,
            host_path_bytes: 4096,
            known_files: 16,
            file_bytes: 1 << 16,
            tool_payload: 1 << 16,
            read_bytes: 4096,
            list_entries: 64,
            list_bytes: 4096,
            match_lines: 8,
            file_timeout: Duration::from_secs(30),
            jobs: 4,
            job_poll: Duration::from_secs(30),
            job_wait: Duration::from_secs(30),
            shell_timeout: Duration::from_secs(30),
            tool_deadline: Duration::from_secs(120),
            shell_head: 256,
            shell_tail: 256,
            search_hits: 16,
            search_bytes: 1024,
            search_timeout: Duration::from_secs(30),
            facts: 1024,
        },
    },
};

/// An agent process's limits in random worlds: room for fewer conversations,
/// sessions and calls than its run may ask for, so that some are refused.
///
/// Scripted-world contract: domain/run.md, sections 13 and 14; testing-strategy.md, section 7.
pub const TIGHT: Limits = Limits {
    accounts: LIMITS.accounts,
    endpoints: LIMITS.endpoints,
    configured_model_bytes: LIMITS.configured_model_bytes,
    decoded_call_bytes: LIMITS.decoded_call_bytes,
    skew: LIMITS.skew,
    run: run::Limits {
        conversations: 4,
        calls: 6,
        run_conversations: 4,
        answer_bytes: 256,
        guide_bytes: 256,
        check_tail: 128,
        ..LIMITS.run
    },
    session: session::Limits {
        threshold: [4, 5],
        token_bytes: skein_llm::TOKEN_BYTES,
        sessions: 4,
        spend: 10_000,
        protocol_allowance: 4096,
        history_blocks: u32::MAX,
        history_messages: 32,
        history_bytes: 33_554_432,
        completion_bytes: 4096,
        answer_bytes: 4096,
        completion_blocks: 16,
        failure_bytes: 512,
        delegated_result_bytes: 4_194_304,
        retries: 2,

        parallel_tools: 3,
        tools: tools::Limits { kits: 4, calls: 3, ..LIMITS.session.tools },
        ..LIMITS.session
    },
};
