# Session 05: the working tier's model, one effort bound, a start that fails

2026-10-14. The working tier is `gpt-6.1-sol` at high effort
(decisions.md 2.2), which smith's presets do not declare; an effort's
bound is written in six places; and `smith exec` in a repository with no
commit prints "Repository head unavailable" and waits until it is killed
(findings L1, L2, L3). This session adds the model to the presets and the
working tier, keeps one effort bound, and ends a failed start with exit 2.

## Read first

- smith `docs/design/shell.md`, sections 4.3, 5.2 ("Tiered list
  prices"), 6.1 and 10 (its world list).
- smith `docs/design/protocol/limits.md`, section 2.4; `docs/design/benchmarks.md`,
  section 7.
- `findings/live.md` L1 to L4; `decisions.md` 2.2 and 7.3.
- skein `programming-model.md`, sections 3 and 10; `testing-strategy.md`,
  sections 2.1 and 2.9.

## What the design asks

- **`gpt-6.1-sol`** on `codex`: window 272,000, output 32,000, payload
  64 KiB, head 60 s, idle 300 s; its notional prices its provider's list
  prices, tiered as 5.2 says, the numbers in code; no efforts declared
  (shell.md 6.1). A tier's model is one the presets declare
  (benchmarks.md 7).
- **A model's effort is a name of at most 32 bytes,** the charter
  schema's (limits.md 2.4).
- **A start that fails before its run opens ends the command** with its
  code and last line: a writable repository whose head the start cannot
  read is exit 2, `smith: run refused: repository <directory>: its head
  cannot be read` (shell.md 4.3).

## Choices this plan makes

- `gpt-6.1-sol`'s rates, per million tokens: $2 fresh, $0.10 cache read,
  $2.50 cache write, $10 output; past 272,000 tokens of window, $4, $0.20,
  $5 and $15 (live.md L3; the cache-write rate at 1.25 times fresh, as the
  other Codex presets; decisions.md, 10.4).
- `benchmarks/agents/models.toml`'s `working.codex` becomes
  `gpt-6.1-sol` at `high` in 05.1, so that the presets declare the tier's
  model in the same landing.

## Rules that bind it

- **The smoke** (README, rule 8) on each landing.
- **Formats move once** (rule 12): this session bumps nothing. A new
  model adds effective limits' names by the existing per-model scheme,
  which is no new version of the vocabulary; the built-in golden is the
  settings' own test file.
- **Users outside Files** (rule 3).

## Increments

### 05.1 `gpt-6.1-sol` and the working tier
Scope: shell.md 6.1 and 5.2; benchmarks.md 7 (decisions.md 2.2).
Needs: nothing.
Files: `crates/smith-local-shell/src/presets.rs` (`MODELS`, `:7-13`; the
  Codex block, `:50-63`; `prices`, `:109-121`; its doc, `:106-107`),
  `crates/smith-local-shell/src/settings/built-in.golden`,
  `crates/smith-local-shell/src/settings/tests.rs` (`:93-101`, `:139-144`,
  and the memory tests at `:448-586` where a sixth model changes their
  sums), `benchmarks/agents/models.toml`, `benchmarks/tests/smith_adapter.rs`
  (the counts of rendered models and allowances, `:686-687`, `:765-766`,
  `:853-854`: 2 becomes 3).
Shape: `MODELS` gains `codex/gpt-6.1-sol`, declared as luna's block with
  its own name; `prices` gains its base and long-context arms (above);
  the built-in golden gains its keys (quoted, since the name holds a dot).
Test shape: `gpt_six_one_sol_is_declared_with_its_list_prices` (a table
  test over both tiers of its rates, literals), beside the tier test at
  `settings/tests.rs:139`. Under 20 ms. The golden test
  `the_built_in_layer_matches_its_golden` takes the new golden.
Proof: unit, those tests. Not the process layer: `smith check` needs an
  account in its settings, which only the harness gives; the model's first
  live use is 06.2's and 12.1's, and findings L1 is its closest evidence
  (it answered on the borrowed login at high effort).
Don't: declare efforts for the model.
Size: 60 lines.
Removes: `gpt-6-astra` as the working tier.

### 05.2 One effort bound
Scope: limits.md 2.4.
Needs: nothing.
Files: `crates/smith-domain-run/src/charter.rs` (`:420`),
  `crates/smith-local-shell/src/settings/schema.rs` (`:24`, `:119`),
  `crates/smith-agent-shell/src/config.rs` (`:815`), the transcript's
  schema bound (`crates/smith-transcript/schema/transcript-v3.schema:88`,
  read only), `tests/codecs/tests/`.
Shape: each code literal 32 for an effort becomes
  `smith_charter::CEILINGS.llm_effort_some`; the transcript's schema keeps
  its 32, and a test holds it equal to the charter's.
Test shape: `the_transcripts_effort_bound_equals_the_charters` (in
  `tests/codecs/tests/transcript.rs`), and the existing 33-byte refusal
  (`settings/tests.rs:641-646`) unchanged. Under 20 ms.
Proof: unit, those tests.
Don't: change a schema.
Size: 20 lines.
Removes: four literals.

### 05.3 A failed start ends `smith exec`
Scope: shell.md 4.3 (findings L2; decisions.md 7.3).
Needs: nothing.
Files: `crates/smith-local-domain/src/boundary.rs` (`Failure::HeadUnavailable`,
  `:274`), `crates/smith-local-domain/src/domain.rs` (`head_result`,
  `:881-908`; `finish_failed_start`, `:833-837`; `closed`, `:987-991`),
  `crates/smith-local-domain/src/tests.rs`, `crates/smith-local-protocol/src/terminal.rs`
  (`:167`, its test at `:414`), `tests/end-to-end/end_to_end.rs`,
  `tests/end-to-end/support/` (an uncommitted repository fixture), and
  `crates/smith-host-domain/src/channel.rs` (`Directory`, read for its
  name).
Shape: `Failure::HeadUnavailable { directory: Box<[u8]> }`, the
  directory's name as the start names it. In headless mode a failed start
  sets the phase done and asks `Request::Exit { status: ExitStatus::Refused
  }`, which the front end maps to 2; the line is "run refused: repository
  <directory>: its head cannot be read", which the host prefixes with
  "smith: ". Interactive mode keeps its notice and its prompt.
Test shape: `crates/smith-local-domain/src/tests.rs`:
  `a_headless_start_whose_head_cannot_be_read_exits_refused` (step test,
  under 20 ms); `tests/end-to-end/end_to_end.rs`, beside
  `local_host_io_failure_keeps_exit_1` (`:440`):
  `exec_in_a_repository_without_a_commit_exits_two_with_its_line`
  (asserts exit 2, the last line, and no provider request; under 5 s).
Proof: end to end, that test (the process layer: the shipped binary
  against the fake provider).
Don't: create a commit in the user's repository, or run without a head.
Size: 80 lines.
Removes: the silent wait.

## Done when

- `heavy cargo nextest run -p smith -E 'test(exec_in_a_repository_without_a_commit_exits_two_with_its_line)'`
  passes, once.
- `rg -n 'gpt-6.1-sol' crates/smith-local-shell/src/presets.rs benchmarks/agents/models.toml`
  prints a line from each.
