//! Rendering properties with deterministic input and the ordinary Rust test kit.
#![allow(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "testing-strategy.md, section 4: test harnesses are ordinary Rust"
)]

use skein_lib::{Duration, Rng};
use smith_domain::tools;

use crate::{CUT_MARKER_BYTES, render_outcome, render_worst_case};

fn tools() -> tools::Limits {
    tools::Limits {
        kits: 1,
        calls: 1,
        repos: 1,
        path_bytes: 32,
        host_path_bytes: 4096,
        known_files: 1,
        file_bytes: 1024,
        tool_payload: 1024,
        read_bytes: 64,
        list_entries: 8,
        list_bytes: 512,
        match_lines: 8,
        file_timeout: Duration::from_secs(30),
        shell_timeout: Duration::from_secs(30),
        shell_timeout_max: Duration::from_secs(120),
        shell_head: 64,
        shell_tail: 64,
        search_hits: 8,
        search_bytes: 128,
        search_timeout: Duration::from_secs(30),
        facts: 32,
    }
}

#[test]
fn random_outcomes_at_random_caps_fit_their_derived_render_bound() {
    let mut rng = Rng::new(941);
    for case in 0_usize..256 {
        let mut caps = tools();
        caps.read_bytes = u32::try_from(rng.next_u64() % 1024).unwrap();
        caps.shell_head = u32::try_from(rng.next_u64() % 1024).unwrap();
        caps.shell_tail = u32::try_from(rng.next_u64() % 1024).unwrap();
        caps.search_hits = u32::try_from(rng.next_u64() % 32).unwrap();
        caps.search_bytes = u32::try_from(rng.next_u64() % 1024).unwrap();
        let bound = render_worst_case(&caps, 1024, 2048).unwrap();
        let entries = (0..caps.list_entries)
            .map(|_| tools::Entry { name: vec![0xff; 8].into(), kind: tools::Kind::Directory })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let hits = (0..caps.search_hits)
            .map(|_| tools::Hit { path: Box::new([]), line: u32::MAX, text: Box::new([]) })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let cases = [
            tools::Outcome::Read {
                content: vec![b'\n'; usize::try_from(caps.read_bytes).unwrap()].into(),
                skipped: u32::MAX,
                lines: u32::MAX,
                total: u32::MAX,
                cut: true,
            },
            tools::Outcome::Listed { entries, more: u64::MAX },
            tools::Outcome::Found { hits, more: u64::MAX, timed_out: true },
            tools::Outcome::EditFailed {
                edit: 1,
                why: tools::Mismatch::Ambiguous {
                    count: u32::MAX,
                    lines: vec![u32::MAX; usize::try_from(caps.match_lines).unwrap()].into(),
                },
            },
            tools::Outcome::Exited {
                exit: tools::Exit::Code { code: u8::MAX },
                head: vec![0xff; usize::try_from(caps.shell_head).unwrap()].into(),
                tail: vec![0; usize::try_from(caps.shell_tail).unwrap()].into(),
                dropped: u64::MAX,
            },
        ];
        for outcome in cases {
            let (full, _) = render_outcome(&outcome, &tools::Namespace::One, bound).unwrap();
            if case % 8 == 0 {
                assert_eq!(
                    render_outcome(&outcome, &tools::Namespace::One, bound).unwrap().0,
                    full,
                    "same outcome renders identical bytes"
                );
                let cap = CUT_MARKER_BYTES + u32::try_from(rng.next_u64() % 128).unwrap();
                assert_eq!(
                    render_outcome(&outcome, &tools::Namespace::One, cap),
                    render_outcome(&outcome, &tools::Namespace::One, cap),
                    "cut rendering is deterministic too"
                );
            }
            assert!(full.len() <= usize::try_from(bound).unwrap());
            assert!(!full.ends_with(b" bytes omitted]"));
        }
    }
}

#[test]
fn a_cut_marker_counts_the_exact_bytes_left_out_of_the_rendering() {
    let outcome = tools::Outcome::Read { content: vec![b'x'; 512].into(), skipped: 0, lines: 1, total: 1, cut: false };
    let (full, error) = render_outcome(&outcome, &tools::Namespace::One, 4096).unwrap();
    for cap in CUT_MARKER_BYTES..128 {
        let (cut, cut_error) = render_outcome(&outcome, &tools::Namespace::One, cap).unwrap();
        let marker = cut.windows(2).position(|bytes| bytes == b"\n[").unwrap();
        let count =
            core::str::from_utf8(&cut[marker + 2..]).unwrap().split(' ').next().unwrap().parse::<usize>().unwrap();
        assert_eq!(count, full.len() - marker);
        assert_eq!(&cut[..marker], &full[..marker]);
        assert_eq!(cut_error, error);
        assert!(cut.len() <= usize::try_from(cap).unwrap());
    }
}

#[test]
fn cuts_preserve_every_utf8_character_boundary() {
    let original = "é界🦀".repeat(30);
    for cap in CUT_MARKER_BYTES..128 {
        let cut = crate::render::cut_text(original.as_bytes().into(), cap);
        let text = core::str::from_utf8(&cut).unwrap();
        let (prefix, marker) = text.split_once("\n[").unwrap();
        let omitted = marker.split(' ').next().unwrap().parse::<usize>().unwrap();
        assert!(original.starts_with(prefix));
        assert_eq!(omitted, original.len() - prefix.len());
        assert!(cut.len() <= usize::try_from(cap).unwrap());
    }
}

#[test]
fn rendering_derivation_refuses_arithmetic_overflow() {
    let mut caps = tools();
    caps.list_bytes = u64::MAX;
    assert_eq!(render_worst_case(&caps, 1, 1), None);
}

#[test]
fn valid_utf8_survives_and_only_invalid_bytes_and_controls_are_escaped() {
    let cases = [
        ("é界🦀 –\n\t".as_bytes(), "exit code 0\né界🦀 –\n\t".as_bytes()),
        (
            "\u{7ff}\u{800}\u{d7ff}\u{e000}\u{ffff}\u{10000}\u{10ffff}".as_bytes(),
            "exit code 0\n\u{7ff}\u{800}\u{d7ff}\u{e000}\u{ffff}\u{10000}\u{10ffff}".as_bytes(),
        ),
        (b"\xc0\xaf\xf4\x90\x80\x80".as_slice(), b"exit code 0\n\\xc0\\xaf\\xf4\\x90\\x80\\x80".as_slice()),
        (b"a\x00\r\x1b\x7fb".as_slice(), b"exit code 0\na\\x00\\x0d\\x1b\\x7fb".as_slice()),
        ("a\u{85}b".as_bytes(), b"exit code 0\na\\xc2\\x85b".as_slice()),
        (b"a\xff\xc3\xa9\xe2\x82".as_slice(), "exit code 0\na\\xffé\\xe2\\x82".as_bytes()),
        (b"\xf0\x80\x80\x80\xed\xa0\x80".as_slice(), b"exit code 0\n\\xf0\\x80\\x80\\x80\\xed\\xa0\\x80".as_slice()),
    ];
    for (input, expected) in cases {
        let outcome = tools::Outcome::Exited {
            exit: tools::Exit::Code { code: 0 },
            head: input.into(),
            tail: Box::new([]),
            dropped: 0,
        };
        let first = render_outcome(&outcome, &tools::Namespace::One, 4096).unwrap();
        assert_eq!(first.0.as_ref(), expected);
        assert!(!first.1);
        assert_eq!(first, render_outcome(&outcome, &tools::Namespace::One, 4096).unwrap());
    }
}

#[test]
fn rendered_utf8_cuts_remain_valid_and_count_exact_omitted_bytes() {
    let outcome = tools::Outcome::Exited {
        exit: tools::Exit::Code { code: 0 },
        head: "é界🦀".repeat(30).into_bytes().into(),
        tail: Box::new([]),
        dropped: 0,
    };
    let (full, _) = render_outcome(&outcome, &tools::Namespace::One, 4096).unwrap();
    for cap in CUT_MARKER_BYTES..128 {
        let (cut, error) = render_outcome(&outcome, &tools::Namespace::One, cap).unwrap();
        let text = core::str::from_utf8(&cut).unwrap();
        let (prefix, marker) = text.split_once("\n[").unwrap();
        let omitted = marker.split(' ').next().unwrap().parse::<usize>().unwrap();
        assert_eq!(omitted, full.len() - prefix.len());
        assert_eq!(prefix.as_bytes(), &full[..prefix.len()]);
        assert!(cut.len() <= usize::try_from(cap).unwrap());
        assert!(!error);
    }
}

fn described_prompt() -> smith_domain::llm::Prompt {
    use smith_domain::llm;
    llm::Prompt {
        choice: llm::ToolChoice::Auto,
        namespace: tools::Namespace::One,
        asker: llm::Asker::Main,
        affinity: llm::Affinity { key: [0; 16], thread: 0 },
        endpoint: llm::Endpoint(1),
        model: b"test".as_slice().into(),
        system: Box::new([]),
        tools: tools::Grants { inspect: true, modify: true, shell: true },
        served: Box::new([]),
        messages: Box::new([]),
        max_tokens: 16,
    }
}

#[test]
fn golden_descriptions_state_batching_and_every_effective_parameter() {
    let stated = crate::Stated {
        host_part: u64::MAX,
        path_bytes: 37,
        read_window: 73,
        tool_payload: 257,
        file_bytes: 257,
        call_input_bytes: 401,
        list_entries: 11,
        search_hits: 13,
        search_bytes: 127,
        shell_timeout: Duration::from_secs(12),
        tool_deadline: Duration::from_secs(43),
        shell_output: 151,
        output_ceiling: 151,
    };
    let golden = [
        (
            "Read lines; cuts are marked. Whole-file limit 257 bytes. Batch independent reads, lists and searches side by side.",
            3_usize,
        ),
        (
            "List matching files and their directory tree; depth defaults to whole, direct children 1. Glob selects files. Max 11 entries; omitted files and directories counted once. List / for directory names. Batch independent reads, lists and searches side by side.",
            3,
        ),
        (
            "Search files; max 13 matches, 127 bytes; cuts are marked. Batch independent reads, lists and searches side by side.",
            3,
        ),
        (
            "Write a file; read its current version first. Whole-file limit 257 bytes. For a larger file, write its first part within the byte bound, then extend it with edits; put as many edits in one call as fit the bound. Batch writes, edits and commands; run serially in call order.",
            2,
        ),
        (
            "Apply ordered edits atomically; no earlier read. Each old must match once, or at least once with all. Whole-file limit 257 bytes. For a larger file, write its first part within the byte bound, then extend it with edits; put as many edits in one call as fit the bound. Batch writes, edits and commands; run serially in call order.",
            5,
        ),
        (
            "Run a command; files may change. Max 151 output bytes; cuts are marked. Batch writes, edits and commands; run serially in call order.",
            2,
        ),
    ];
    let schemas = crate::schemas(&described_prompt(), &stated);
    assert_eq!(schemas.len(), golden.len());
    for (schema, (words, parameters)) in schemas.iter().zip(golden) {
        assert_eq!(schema.description.as_ref(), words.as_bytes());
        let text = core::str::from_utf8(&schema.schema).unwrap();
        assert_eq!(
            text.matches("\"description\"").count(),
            parameters,
            "{}",
            core::str::from_utf8(&schema.name).unwrap()
        );
        assert!(!text.contains("runs alone"));
        match schema.name.as_ref() {
            b"read" => assert!(
                text.contains("max 37 bytes") && text.contains("max 73 bytes") && text.contains("counting from 1")
            ),
            b"list" => assert!(text.contains("max 37 bytes")),
            b"search" => assert!(text.contains("max 37 bytes") && text.matches("max 401 bytes").count() == 2),
            b"write" => assert!(text.contains("max 257 bytes") && text.contains("max 401 bytes")),
            b"edit" => assert_eq!(text.matches("max 257 bytes").count(), 2),
            b"shell" => assert!(
                text.contains("Seconds")
                    && text.contains("12000 ms")
                    && text.contains("43000 ms")
                    && text.contains("max 401 bytes")
            ),
            _ => panic!("only the six workspace tools"),
        }
    }
    assert_eq!(schemas, crate::schemas(&described_prompt(), &stated));
}

#[test]
fn stated_quantities_copy_effective_limits_and_refuse_capture_overflow() {
    let limits = tools();
    let stated = crate::Stated::from_tools(&limits, 401).unwrap();
    assert_eq!(stated.path_bytes, 32);
    assert_eq!(stated.tool_payload, 1024);
    assert_eq!(stated.read_window, 64);
    assert_eq!(stated.search_hits, 8);
    assert_eq!(stated.search_bytes, 128);
    assert_eq!(stated.output_ceiling, 128);
    assert_eq!(stated.shell_output, 128);
    assert_eq!(stated.shell_timeout, Duration::from_secs(30));
    assert_eq!(stated.tool_deadline, Duration::from_secs(120));
    let excessive = tools::Limits { shell_head: u32::MAX, shell_tail: 1, ..limits };
    assert!(crate::Stated::from_tools(&excessive, 401).is_none());
}

#[test]
fn path_descriptions_follow_the_actual_conversation_namespace() {
    use smith_domain::tools::{Name, Namespace};
    for (namespace, wanted, absent) in [
        (Namespace::One, "Relative to the working directory, e.g. src/lib.rs", "/name/..."),
        (
            Namespace::Several {
                working: Name::new(b"private-name".as_slice().into()).expect("valid namespace fixture"),
            },
            "Relative to the working directory or /name/...",
            "private-name",
        ),
    ] {
        let mut prompt = described_prompt();
        prompt.namespace = namespace;
        for schema in crate::schemas(
            &prompt,
            &crate::Stated {
                host_part: u64::MAX,
                path_bytes: 256,
                read_window: 1024,
                tool_payload: 4096,
                file_bytes: 4096,
                call_input_bytes: 2048,
                list_entries: 16,
                search_hits: 8,
                search_bytes: 256,
                shell_timeout: Duration::from_secs(60),
                tool_deadline: Duration::from_secs(600),
                shell_output: 192,
                output_ceiling: 192,
            },
        ) {
            if schema.name.as_ref() != b"shell" {
                let text = core::str::from_utf8(&schema.schema).expect("valid namespace fixture");
                assert!(
                    text.contains(wanted),
                    "{}",
                    core::str::from_utf8(&schema.name).expect("valid namespace fixture")
                );
                assert!(!text.contains(absent));
            }
        }
    }
}

#[test]
fn resolved_failure_paths_render_in_the_fixed_namespace_live_and_on_replay() {
    let working = tools::Resolved::Working { path: "src/café.rs".as_bytes().into() };
    let other = tools::Resolved::Other {
        directory: tools::Name::new(b"docs".as_slice().into()).expect("name"),
        path: b"guide.md".as_slice().into(),
    };
    let outside = tools::Resolved::Outside { path: b"elsewhere/file".as_slice().into() };
    let several = tools::Namespace::Several { working: tools::Name::new(b"work".as_slice().into()).expect("name") };
    let table: [(tools::Outcome, tools::Namespace, &[u8]); 5] = [
        (
            tools::Outcome::NotFound { path: working.clone() },
            tools::Namespace::One,
            "path not found: src/café.rs".as_bytes(),
        ),
        (tools::Outcome::NotFound { path: working }, several, "path not found: /work/src/café.rs".as_bytes()),
        (tools::Outcome::ReadOnly { path: other }, tools::Namespace::One, b"path is read-only: /docs/guide.md"),
        (
            tools::Outcome::Outside { path: outside },
            tools::Namespace::One,
            b"path is outside the workspace: /elsewhere/file",
        ),
        (
            tools::Outcome::NotDirectory { path: tools::Resolved::Working { path: Box::new([]) } },
            tools::Namespace::One,
            b"path is not a directory: .",
        ),
    ];
    for (outcome, namespace, expected) in table {
        let rendered = render_outcome(&outcome, &namespace, 4096).expect("bounded path feedback");
        assert_eq!(rendered, (expected.into(), true));
        assert_eq!(rendered, render_outcome(&outcome.clone(), &namespace, 4096).expect("replay"));
    }
}

#[test]
fn oversized_call_feedback_states_size_bound_and_batched_recovery_within_its_cap() {
    use smith_domain::llm::Problem;
    let golden = b"input too large: 1024 bytes, bound 256 bytes; make a smaller write or an edit. For a larger file, write its first part within the bound, then extend it with edits; put as many edits in one call as fit the bound";
    let exact = u32::try_from(golden.len()).expect("fixed feedback length");
    assert_eq!(
        crate::failure::problem(Problem::Oversize { bytes: 1024, bound: 256 }, exact).expect("exact room").as_ref(),
        golden.as_slice()
    );
    assert_eq!(
        crate::failure::problem(Problem::Oversize { bytes: 1024, bound: 256 }, exact - 1),
        Err(crate::Error::Limit)
    );
    assert_eq!(
        crate::failure::problem(Problem::CutOff { bytes: 17 }, 4096).expect("cut note").as_ref(),
        b"input cut off after 17 bytes; make a smaller call and try again".as_slice()
    );
}

#[test]
fn fixed_own_schema_measurement_matches_the_complete_fixed_inventory() {
    let stated = crate::Stated::from_tools(&tools(), 2048).expect("bounded quantities");
    let mut prompt = described_prompt();
    prompt.namespace = tools::Namespace::One;
    prompt.served = Box::new([smith_domain::llm::Served::Wait, smith_domain::llm::Served::SubAgent]);
    let inventory = crate::schemas(&prompt, &stated);
    let rendered =
        inventory.iter().map(|tool| tool.name.len() + tool.description.len() + tool.schema.len()).sum::<usize>();
    assert_eq!(crate::fixed_schema_bytes(&stated), u64::try_from(rendered).ok());
    assert_eq!(inventory.len(), 8);
}

#[test]
fn drawn_contract_schema_costs_equal_every_rendered_byte() {
    use smith_domain::run::outcome::{ChangeSpec, FieldRule, ItemRule, ItemSpec, OutcomeSpec, TextSpec, VerdictRule};
    let costs = crate::schema_costs().expect("startup renderer costs");
    let mut rng = Rng::new(46_106);
    let words = [b"plain".as_slice(), b"\\\"", b"\n\t\x01", "é😀".as_bytes()];
    for _ in 0..512 {
        let mut fields = || {
            (0..rng.next_u64() % 5)
                .map(|_| FieldRule {
                    name: words[usize::try_from(rng.next_u64() % 4).expect("word index")].into(),
                    max: u32::try_from(rng.next_u64() & u64::from(u32::MAX)).expect("u32 cap"),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice()
        };
        let change = ChangeSpec { checks_must_pass: false, fields: fields() };
        let report = TextSpec { max: u32::MAX, fields: fields() };
        let failure = TextSpec { max: 0, fields: fields() };
        let kind_fields = fields();
        let verdict_fields = fields();
        let kinds = (0..rng.next_u64() % 4)
            .map(|index| ItemRule {
                kind: words[usize::try_from(index).expect("word index")].into(),
                fields: kind_fields.clone(),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let verdicts = (0..rng.next_u64() % 4)
            .map(|index| VerdictRule {
                name: words[usize::try_from(index).expect("word index")].into(),
                text_max: u32::MAX,
                fields: verdict_fields.clone(),
                items: ItemSpec { min: 0, max: 23, kinds: kinds.clone() },
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let outcome = OutcomeSpec {
            change: (rng.next_u64() % 2 == 0).then_some(change.clone()),
            report: (rng.next_u64() % 2 == 0).then_some(report),
            failure: (rng.next_u64() % 2 == 0).then_some(failure),
            verdicts,
        };
        assert_eq!(
            costs.finish(&outcome),
            Some(
                u64::try_from(crate::finish_schema(&outcome, u32::MAX).expect("rendered finish").len())
                    .expect("length")
            )
        );
        assert_eq!(
            costs.deliver(&change),
            Some(
                u64::try_from(crate::deliver_schema(&change, u32::MAX).expect("rendered deliver").len())
                    .expect("length")
            )
        );
    }
}
