# Session 06: the model's text

2026-10-14. In 6 of 15 fixture attempts smith's first call was `list`
with an empty path, refused as an invalid value; the shell's description
gives its timeout in seconds and milliseconds at once; `search` never says
it runs `rg` on a regular expression; and smith's model works with a
96-byte role, or none in the harness, against Codex's 21,429 bytes
(findings P1 to P4). This session lands after the baseline `parity` run
(README, rule 11): the empty path and the descriptions' facts first, then
smith's task role, proven together by one `compare` run.

## Read first

- smith `docs/design/protocol/llm.md`, sections 3 and 4.
- smith `docs/design/domain/tools.md`, sections 2 and 7 (its paths list).
- smith `docs/design/shell.md`, sections 5.2, 6.6 and 10 (its world list);
  `docs/design/domain/run.md`, sections 3.1 and 3.3;
  `docs/design/protocol/limits.md`, sections 3.9 and 4
  (`host-part-holds-the-role`); `docs/design/benchmarks.md`, section 4.2
  ("An improvement shown on fixtures").
- `findings/prompts.md` P1 to P4 and P8; `decisions.md` 6.1 and 6.2.
- skein `programming-model.md`, section 3; `testing-strategy.md`,
  sections 2.1 and 2.5.

## What the design asks

- **The empty path is the working directory:** `""` decodes to the same
  typed path as `.`, and is never refused as a bad value (llm.md 4).
- **Each description states, beside its limits** (llm.md 3): for `shell`,
  that `timeout_seconds` is in seconds, its default `shell_timeout` and
  its ceiling `tool_deadline`, each in seconds; for `search`, that it runs
  `rg` and that `pattern` is a regular expression; for each path field,
  that `.` and the empty path both name the working directory, how its
  byte bound counts (each name, `.` and `..`, one separator between each
  two, a repeated, leading or trailing separator as nothing), and that the
  path keeps the bound once resolved, from the workspace's root. With one
  directory, paths are relative and the directory's name is never shown
  (tools.md 2).
- **The task role** (shell.md 6.6): one text of at most 4 KiB, the same
  for every model and for chats and tasks, on eight topics in order,
  naming only smith's own tools, saying nothing the run's own sections
  say; its words are code in `smith-local-shell`; it is a run's
  instructions, first in the system text (run.md 3.1, 3.3), counted in
  the window's host part. Settings that set `instructions` replace it.
- **`host-part-holds-the-role`:** for each declared model, the host's part
  holds the task role at its bound beside a guide at `guide` for each of
  the start's directories (limits.md 4).

## Choices this plan makes

- The role's topic 5 says "running the tests the change needs" here; its
  second half, "and long commands as jobs", lands with the `job` tool in
  11.2, since the role names only tools smith offers (decisions.md, 10.5).
- The role writes each tool's name in backticks, and only tools' names:
  its test reads them so.
- `host-part-holds-the-role` runs when the shell gives the task role: the
  local shell passes the role's bound into the profile, and a profile
  composed without it (the agent process's own configuration, test
  profiles) skips the check, since only the shell gives that role
  (shell.md 6.6).
- The role's proof is one `compare` run of two smith arms, 06.1's parent
  and 06.2, so that it measures the descriptions and the role together
  (benchmarks.md 4.2), with the designer's line of 30 attempts
  (decisions.md, 10.5).

## Rules that bind it

- **The baseline comes first** (README, rule 11): 06.1 and 06.2 land
  after 12.1's summary is on `main`.
- **The live budget** (rule 10): 06.2's line is 30 attempts and $3.
- **Users outside Files** (rule 3), for the description texts that tests
  pin.

## Increments

### 06.1 The empty path, and the descriptions' facts
Scope: llm.md 3's "The facts each description states" (but `shell`'s
  yield and the `job` tool, 11.2's) and llm.md 4's empty path; tools.md
  2 and 7's paths (findings P3, P4).
Needs: the baseline `parity` run's summary on `main` [12.1]; smith 08's
  vocabulary [08.1], so that 06.2's parent arm writes the events the
  harness reads.
Files: `crates/smith-protocol-llm/src/decode.rs` (`path`, `:435-451`),
  `crates/smith-protocol-llm/src/descriptions.rs` (`words`, `:48-99`;
  `path`, `:101-110`; `properties`, `:124-181`), `crates/smith-protocol-llm/src/tests.rs`
  (`golden_descriptions_state_batching_and_every_effective_parameter`,
  `:294`; `path_descriptions_follow_the_actual_conversation_namespace`,
  `:388`; `fixed_own_schema_measurement_matches_the_complete_fixed_inventory`,
  `:488`), `tests/protocol-llm/tests/workspace.rs` (`:530`, `:615`),
  `tests/protocol-llm/tests/production_tools.rs` (`:40`, `:90`, `:330`),
  `tests/tools/tests/` (a path's bound).
Shape: `decode::path` returns `Path { absolute: false, parts:
  [Part::Current] }` for `""`, for every path field. The descriptions add
  the facts above, with the effective values written in; the shell's
  timeout reads in seconds only; the one-directory `list` description
  offers `.`, never `/`.
Test shape: `the_empty_path_decodes_as_the_working_directory_for_each_path_field`
  (a table test over `read`, `list`, `write`, `edit`, `search`; in
  `tests/protocol-llm/tests/production_tools.rs`),
  `each_description_states_its_facts` (a table test of literal
  substrings per tool), `a_path_at_its_bound_and_one_past_as_written_and_as_resolved`
  (a table test in the tools world, repeated separators counting
  nothing), each under 20 ms but the world's, under 0.5 s. The pinned
  description tests take the new text, their names kept.
Proof: world, those tests; the live effect is measured by 06.2's
  `compare` run.
Don't: change a limit, a schema's field or a tool's behaviour.
Size: 120 lines.
Removes: the empty path's refusal, the milliseconds in `timeout_seconds`'
  description.

### 06.2 The task role
Scope: shell.md 5.2 and 6.6; run.md 3.1; limits.md 4's
  `host-part-holds-the-role` (findings P1, P2, P8; decisions.md 6.1).
Needs: 06.1.
Files: `crates/smith-local-shell/src/role.rs` (new), `crates/smith-local-shell/src/lib.rs`,
  `crates/smith-local-shell/src/settings/resolve/local.rs` (`:143-151`),
  `crates/smith-local-shell/src/compose.rs` (passing the role's bound),
  `crates/smith-agent-service/src/profile/checks.rs` (`Check`,
  `:12-55`, names `:60-84`), `crates/smith-agent-service/src/profile/validate.rs`,
  `crates/smith-agent-service/src/profile/quantities.rs` (`Configuration`,
  `:145`), `crates/smith-agent-service/src/profile.rs` (after
  `run::derive`, `:~514`), `benchmarks/suites/role.toml` (new),
  `benchmarks/summaries/`.
Shape: `pub const TASK_ROLE: &str` in `role.rs`, at most 4,096 bytes, on
  shell.md 6.6's topics in order; `pub const TASK_ROLE_BOUND: u32 = 4096`.
  Without `instructions` in settings, the run's instructions are
  `TASK_ROLE`. `Configuration` gains `role_bound: Option<u32>`;
  `Check::HostPartHoldsTheRole` (named `host-part-holds-the-role`)
  refuses, for each declared model, a host part below the role's bound
  plus `run.directories` guides at `guide`, with their rendering.
Test shape: `crates/smith-local-shell/src/role.rs`'s tests:
  `the_task_role_is_within_four_kibibytes`,
  `each_tool_the_task_role_names_is_one_of_smiths_own`; the profile's
  tests: `a_host_part_below_the_role_and_its_guides_is_refused`,
  `a_profile_without_the_shells_role_skips_the_role_check`. Under 20 ms
  each.
Proof: live, once: build 06.1's parent and 06.2, write `suites/role.toml`
  (tasks `fixtures/01-backoff`, `fixtures/02-line-decoder` and
  `fixtures/03-planner`, interleaved, 5 per arm, no rerun, two `binary`
  arms naming those commits, `reference` the parent's arm, smith's
  `standard` pin, no Codex), then `heavy --gate cargo run -p smith-bench
  -- suite role`; commit its summary, and put its comparisons in the
  commit body. 30 attempts, $3, about 90 minutes. If the summary reports
  a difference (benchmarks.md 9: its interval "excludes 1, and the effect
  exceeds the task's minimum detectable effect", the comparison's
  `detected: true`) with the change slower or costlier than its parent on
  any fixture, stop on a question.
Don't: put in the role what the run's own sections say (tools, the
  checkout, sub-agents, finishing, the budget), or name a tool smith does
  not offer.
Size: 150 lines, the role's text included.
Removes: the 96-byte default role.

## The ready check's corrections

Ruling 137, against smith `main` at `c7d54533`. 06.1's Files include
these; where a line below and the text above differ, the line below holds.

**06.1**
- Lines: `decode.rs` `path` `:440-456` (its refusal `:452-454`);
  `descriptions.rs` `words` `:48-100`, `path` `:102-111`, `properties`
  `:125-183`; `tests.rs`'s three tests `:323`, `:417`, `:517`.
- The namespace reaches the list description: `words` and `description`
  (`descriptions.rs:32-38`) take it, passed by `owned`
  (`smith-protocol-llm/src/tools.rs:154-161`). Files gain both.
- `fixed_schema_bytes` (`tools.rs:71-86`) measures path fields with
  `Namespace::One` only, and feeds `endpoint_allowance`
  (`smith-agent-service/src/profile.rs:151-153`) and `own_part`
  (`profile/room.rs:34`): it measures the larger of the two namespaces.
  Files gain `tools.rs:71`.
- The tools world's decoder copy (`tests/tools/src/translate.rs:14-25`)
  maps `b""` to `[Part::Current]`, as the protocol does. Files gain it.
- World tests that cap a request at 16384 bytes (`workspace.rs:177`,
  `:475`, `:644`, `brief.rs:477`, `nesting.rs:34`, `root_wire.rs:125`,
  `budget.rs:262`): a cap passed only because the path descriptions grew
  becomes 32768; name each in the commit body. No other expected value
  changes.

**06.2** (from its block of 2026-10-15T04:46:38Z, against `main` at
`d9e9b6c5`; decisions.md has the ruling's whole text)
- The role check's count is the run's (limits.md 3.9: "the run counts it
  directly"): `crates/smith-domain-run/src/prompt.rs` gains one public
  count of the role and the guides, re-exported from
  `crates/smith-domain-run/src/lib.rs` beside `fixed_prompt_bytes`
  (`:84`); the profile calls it through `smith_domain::run`. Files gain
  both.
- The role's bound reaches the profile in
  `crates/smith-local-shell/src/settings/resolve.rs`, after `from_value`
  (`:83-86`), which both `smith check` (`check.rs:76`) and a run
  (`compose.rs:107-109`) call. Files gain it; `compose.rs` changes only
  where the bound must reach a later step there.
- 06.2's proof Needs `smith-03.s3` (the harness prepares binary arms;
  from its block of 2026-10-15T05:03:10Z). Its arms stay as built:
  before `894d132f`, after `a29a4ac5`.

## Questions to raise when reached

- None known.

## Done when

- `heavy cargo nextest run -p smith-local-shell -E 'test(task_role)'`
  passes, once.
- `ls benchmarks/summaries/role-*.json` lists one summary.
