//! Random memory driver limits and charters.

use super::{
    ChangeSpec, Charter, Directory, Duration, Endpoint, Grants, Limits, Llm, OutcomeSpec, TIGHT, TextSpec, Token,
    Tools, VerdictRule, Workspace, bytes, run,
};

/// Small limits, so that every bound is met often: few runs and sessions,
/// few bytes each.
pub const LIMITS: Limits = {
    let mut limits = Limits {
        accounts: 4,
        endpoints: 3,
        configured_model_bytes: 4096,
        decoded_call_bytes: 4096,
        skew: Duration::ZERO,
        run: run::Limits {
            runs: 2,
            conversations: 4,
            run_bytes: 2048,
            brief_sections: 4,
            host_part: 1 << 20,
            room: 0,
            compaction_threshold: [4, 5],
            schema_costs: run::SchemaCosts::NONE,
            run_conversations: 3,
            calls: 4,
            answer_bytes: 128,
            guide_bytes: 128,
            outcome_bytes: 512,
            check_tail: 64,
            facts: 32,
            ..TIGHT.run
        },
        session: smith_domain::session::Limits {
            sessions: 4,
            history_messages: 12,
            history_bytes: 1_048_576,
            completion_bytes: 4096,
            answer_bytes: 4096,
            completion_blocks: 16,
            failure_bytes: 512,
            delegated_result_bytes: 131_072,
            parallel_tools: 2,
            tools: smith_domain::tools::Limits {
                shell_output: 128,
                kits: 4,
                calls: 2,
                file_bytes: 512,
                tool_payload: 512,
                read_bytes: 256,
                list_entries: 8,
                list_bytes: 4096,
                shell_head: 64,
                shell_tail: 64,
                search_hits: 4,
                search_bytes: 128,
                facts: 32,
                ..TIGHT.session.tools
            },
            ..TIGHT.session
        },
    };
    limits.session.facts = smith_domain::session::max_facts(&limits.session);
    limits
};

/// A charter with unchanged aggregate context filler distributed across role,
/// maximum Section cells, titles and bodies, granting everything and wanting a
/// change that passes its checks, a verdict, report or declared failure.
pub(super) fn workspace() -> Workspace {
    let repository = Directory {
        host: Box::from(b"/host".as_slice()),
        name: (*b"temper").into(),
        root: Token::new(1),
        writable: true,
        git: true,
        conflicts: Box::new([]),
    };
    Workspace { directories: Box::new([repository]) }
}

pub(super) fn charter(context_bytes: u64) -> Charter {
    let all = Tools { inspect: true, modify: true, shell: true };
    let rule = VerdictRule {
        name: (*b"request-changes").into(),
        text_max: 1024,
        fields: Box::new([]),
        items: smith_domain::run::outcome::ItemSpec {
            min: 1,
            max: 4,
            kinds: {
                let required: Box<[Box<[u8]>]> = Box::new([(*b"path").into()]);
                let kinds: Box<[Box<[u8]>]> = Box::new([(*b"nit").into()]);
                kinds
                    .into_vec()
                    .into_iter()
                    .map(|kind| smith_domain::run::outcome::ItemRule {
                        kind,
                        fields: required
                            .iter()
                            .map(|name| smith_domain::run::outcome::FieldRule { name: name.clone(), max: 1024 })
                            .collect(),
                    })
                    .collect()
            },
        },
    };
    let count = usize::try_from(LIMITS.run.brief_sections).expect("bounded receiving section count");
    let payload = context_bytes
        - u64::try_from(std::mem::size_of::<run::Section>() * count).expect("bounded owning Section array fits u64");
    let instructions = bytes(payload / 4);
    let mut remainder = payload - u64::try_from(instructions.len()).expect("bounded instructions payload fits u64");
    let mut sections = Vec::with_capacity(count);
    for section in 0..count {
        let remaining = u64::try_from(count - section).expect("bounded Section suffix count fits u64");
        let title = bytes(remainder / remaining / 3);
        let text = bytes(remainder / remaining - u64::try_from(title.len()).expect("bounded title payload fits u64"));
        remainder -= u64::try_from(title.len() + text.len()).expect("bounded Section payloads fit u64");
        sections.push(run::Section { title, text });
    }
    assert_eq!(remainder, 0, "unchanged aggregate filler owns every byte");
    Charter {
        mode: run::Mode::Finite,
        reserve: run::charter::Reserve { turns: 0, time: TIGHT.run.check_timeout },
        instructions,
        brief: run::Brief { sections: sections.into_boxed_slice() },

        grants: Grants { wait: true, deliver: None, tools: all, agents: true, host_tools: Box::new([]) },
        outcome: OutcomeSpec {
            change: Some(ChangeSpec {
                checks_must_pass: true,
                fields: Box::new([
                    smith_domain::run::outcome::FieldRule { name: b"title".as_slice().into(), max: 1024 },
                    smith_domain::run::outcome::FieldRule { name: b"body".as_slice().into(), max: 1024 },
                ]),
            }),
            verdicts: Box::new([rule]),
            report: Some(TextSpec { max: 512, fields: Box::new([]) }),
            failure: Some(TextSpec { max: 512, fields: Box::new([]) }),
        },
        budget: run::Budget { turns: 12, ..TIGHT.run.budget },
        llm: Llm {
            effort: None,
            prices: run::Prices { input: 1, cached: 1, cache_write: None, output: 1, unit: 1_000_000 },
            account: Some(0),
            endpoint: Endpoint(0),
            model: (*b"m").into(),
            window: 65_536,
            output: 256,
            dialect: 1,
        },
        models: Box::new([]),
        conventions: Some(smith_domain::run::Conventions {
            guide: b"AGENTS.md".as_slice().into(),
            checks: b".temper/pre-pr".as_slice().into(),
        }),
        resume: false,
        waiting: skein_lib::Duration::from_secs(30),
    }
}
