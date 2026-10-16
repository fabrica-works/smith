# The plan's commands, run once at the base

2026-10-16, smith `4ddb9b67` (`origin/main`; the plan's files untracked
beside it). Each command run once from `~/src/rust/smith`, through `heavy`
where it compiles or tests, nothing live.

## 1. The size count (`code-organization.md`, section 8)

Over `7e8f97d0~1...7e8f97d0` in place of `origin/main...HEAD`. Exit 0.

```
git diff --numstat 7e8f97d0~1...7e8f97d0 -- ':(glob)crates/*/src/**/*.rs' ':(glob)src/**/*.rs' ':(glob)benchmarks/src/**/*.rs' ':(exclude,glob)**/generated/**' ':(exclude,glob)**/tests.rs' ':(exclude,glob)**/tests/**' | awk '{ s += $1 } END { print s + 0 }'
2
```

Without the `awk`: `2	2	crates/smith-local-shell/src/role.rs` (the
commit's only file). The exclusions, checked on `1366a36f`, which touches
`crates/smith-domain-session/src/tests.rs` and `tests/session/tests/jobs.rs`:
only `jobs.rs` and `session.rs` are listed, and the count is 1.

## 2. README rule 7's grep

```
git diff 7e8f97d0~1 7e8f97d0 -- . ':!docs' | grep -inE '^\+.*\b(actually|genuine|immutable|canonical|verified|qualified|robust|properly|python3)\b|^\+.*#\[(allow|expect)\('
```

Prints nothing, exit 1 (grep's: no match). The expression parses; on the
input `+    #[allow(dead_code)]`, `+ properly done`, `- robust` it prints
the first two lines and exits 0.

## 3. README rule 13's trailers

```
git -C ~/src/rust/smith fetch -q origin && git -C ~/src/rust/smith log origin/main --grep='^Plan: wall-clock$' --format='%(trailers:key=Baseline-Summary,valueonly)%(trailers:key=Change-Summary,valueonly)' | grep .
```

Prints nothing, exit 1 (grep's), as expected at the base. `origin/main` is
`4ddb9b67`. The trailers on `main` are `Plan: parity` and `Step:
smith-12.4`, so `^Plan: wall-clock$` and `^Step: smith-07.1$` match the
form the run writes.

## 4. README "Done when" items 2, 4, 5 and 6

**Item 2** (the newest `parity` summary's verdict). The shell part picks
`benchmarks/summaries/parity-87a8d1e8-20261015T170125Z.json`, `sha` =
`87a8d1e8f430a7d32e559c284a0d5218aae1db4c`, and `test -z` holds (no
wall-clock commit). `heavy cargo run -q -p smith-bench -- parity <run>
--commit <sha>`: exit 1.

```
O  statistic=29 interval=[0.8806, 1] margin=26 holds=true
FT statistic=1.0898 interval=[1.0208, 1.1659] margin=1.2 holds=true
FC statistic=0.5318 interval=[0.4849, 0.5840] margin=1.2 holds=true
RT statistic=1.3263 interval=[1.0311, 1.7558] margin=1.2 holds=false
RC statistic=0.7782 interval=[0.5489, 0.9710] margin=1.5 holds=true
```

**Item 4** (each change's report): the loop has no `Change-Summary` to
read; prints nothing, exit 0.

**Item 5** (the formats): exit 0. It prints

```
crates/smith-events/src/lib.rs:34:pub const VERSION: u64 = 2
crates/smith-transcript/src/lib.rs:31:FORMAT_VERSION: u64 = 4
crates/smith-charter/src/lib.rs:26:FORMAT_VERSION: u64 = 2
benchmarks/src/result.rs:14:RESULT_VERSION: u32 = 3
benchmarks/src/summary.rs:10:SUMMARY_VERSION: u32 = 3
benchmarks/src/summary.rs:13:BASELINE_VERSION: u32 = 1
```

The events' 2, the transcript's 4, the charter's 2, the result's 3, the
summary's 3 and the baseline's 1: the expression catches each constant,
and `rg -n 'const [A-Z_]*VERSION'` over those crates finds no other. The
transcript's and the charter's are `impl` constants (`Turn::FORMAT_VERSION`,
indented), so the match starts at `FORMAT_VERSION`.

**Item 6** (the gates' times): exit 0. `scripts/check.sh passed: 2261
tests / 4.926 s; 42 tests / 6.094 s; 25 s in all`: focused 4.9 s of 15,
fuzzy 6.1 s of 60.

## 5. The sessions' "Done when", at the base

Test packages, from `cargo metadata --no-deps`: `tests/session` is
`smith-session-world`, `tests/protocol-llm` is `smith-protocol-llm-world`,
`tests/agent` is `smith-agent-world`; the harness is `smith-bench`. Every
name the session files use exists.

Failing at the base, as expected (the code is not written): 01's
`"repository/shell-cwd" = 10` (exit 1; line 11 reads `= 5`) and `29\.0`
(prints `parity.rs:179`); 02's `parity-change.toml` (absent, exit 1); 03's
`VERSION: u64 = 3` (exit 1) and `threshold_bytes|fn reached` (prints 6
lines); 04's `pub struct Guard` (exit 1); 05's `--test memory_returns`
(exit 101, `no test target named memory_returns in smith-agent-world`);
06's `Baseline-Summary` count (prints 0); 07's `Step: smith-07.1` count
(prints 0); 08's `FORMAT_VERSION: u64 = 5` and `every replacement one
file`, 09's `Hasten`, 10's `a window from where a search found them`
(each exit 1). Each file each names exists.

Passing at the base already, so they decide nothing alone:
- 02's `heavy cargo nextest run -p smith-bench -E 'test(/change/) | test(/projection/)'`:
  exit 0, 6 landed tests match (`..._changes_a_protected_path_...`,
  `..._digests_changed`, and others).
- 04's `heavy cargo nextest run -p smith-protocol-llm-world -p smith-session-world -E 'test(/line/)'`:
  exit 0, 6 landed tests match (`deadline`, `lines`). The default profile
  skips `fuzzy_*` binaries (`.config/nextest.toml`), so 04.2's
  `drawn_ratios_and_rendered_sizes_send_nothing_past_the_line`, in
  `tests/session/tests/fuzzy_simulation.rs`, is not run by it; `--profile
  fuzzy` runs it.
- 06's `ls benchmarks/baselines/parity-smith.json benchmarks/baselines/parity-codex.json`:
  both exist (version 1, landed by `9d1119b3`, the parity plan's).

## 6. smith's `check` (03.1, 03.2 and 03's "Done when")

`heavy cargo run -q -- check`: exit 101, `cargo run could not determine
which binary to run` (the default members hold `smith`, `smith-agent`,
`smith-bench`, `test-peer`). `heavy cargo run -q -p smith -- check`: exit
2, `smith: built-in settings: model: required value is absent` (no user
layer on this machine). With the model alone, `accounts.codex.source` is
absent (exit 2); with `source = "borrow"` and no file, it reads the
workspace directory as the login (exit 3).

The command that exits 0, reading the Codex login read only and calling
nothing:

```
heavy cargo run -q -p smith -- check -m codex/gpt-6.1-sol -c 'accounts.codex.source="borrow"' -c "accounts.codex.file=\"$HOME/.codex/auth.json\""
```

Exit 0 (one warning: the `anthropic` account not signed in). With a login
file that does not exist it exits 3, the credentials only, and prints the
same limits. It prints `run.room = 153600 tokens` and
`session.history_bytes = 6331914 bytes`; no `run.compaction_material`;
`formats {"channel":2,"charter":2,"chat_store":1,"check":1,"events":2,"transcript":4}`.
heavy shows the last lines only: the whole report is in its log.

## 7. The memory margins (03.2)

`heavy cargo nextest run -p smith-local-shell -p smith-agent-shell -E 'test(/fit/)' --no-capture`:
exit 0. The filter matches 5 tests, the two named among them
(`settings::tests::the_default_inline_product_parts_fit_two_gibibytes`,
`config::tests::product_values_pass_every_startup_check_and_fit_complete_memory`),
besides `drawn_declarations_are_named_or_fit_complete_process_memory`,
`native_validated_dense_prompts_fit_the_actual_joint_receiving_ceiling`
and `a_terminal_status_fits_its_observed_width`.

```
PRODUCT_TOTAL 2027041481/2147483648
PRODUCT_COMPLETE service=1664604060 startup_header=22774 shell=398808 complete=1665025642 memory=2147483648
PRODUCT_PARTIAL ... history=6331914 history_items=49470 completion=2750730
```

Margins under 2,147,483,648: `PRODUCT_TOTAL` 120,442,167 bytes (114.9
MiB, 94.4% used); `PRODUCT_COMPLETE` 482,458,006 bytes (460.1 MiB, 77.5%
used). The larger is `PRODUCT_TOTAL`. The smallest multiple of 64 MiB at
or above it today is 2,080,374,784 (31 × 64 MiB), below the shipped 2 GiB:
03.2's rule raises `memory` only once 03.2 adds more than 53,333,303 bytes
to `PRODUCT_TOTAL`; below that, it would lower it.

## 8. `smith-bench`'s check (01's "Done when", 02.1, 02.3)

`heavy cargo run -q -p smith-bench -- check`: exit 0, `checked 19 tasks,
9 suites, 3 configuration pins, 15 summaries, 2 baselines`.

## 9. The transcript generator (08.1)

From `docs/development/plans/parity/smith/11-offered.md`:

```
rev=$(grep -A2 'name = "skein-codegen"' Cargo.lock | grep -o '#[0-9a-f]*' | tr -d '#' | head -1)
heavy cargo run -q --manifest-path ~/.cargo/git/checkouts/skein-336d5ae5eecfa0f9/${rev:0:7}/testing/skein-codegen/Cargo.toml -- crates/smith-transcript/schema/transcript-v5.schema crates/smith-transcript/src/generated
```

`rev` = `e30fe586ff941471e2c05264353abc8eb63dc635` (source
`git+https://github.com/forgetic/skein.git`); `ls` finds
`~/.cargo/git/checkouts/skein-336d5ae5eecfa0f9/e30fe58/testing/skein-codegen/Cargo.toml`.
Not run. At the base: `schema/transcript-v4.schema`, `src/generated/v4.rs`,
`golden/v4/`; 08.1 writes `transcript-v5.schema` first, as above.

## 10. The users of what the Shapes change

| Search | Files |
|---|---:|
| `rg -l 'Purpose::Ordinary' crates tests benchmarks` | 20 (24 uses): 6 in `crates/` (`smith-agent-service` `responses.rs`, `tests.rs`; `smith-domain-session` `session.rs`, `tests.rs`; `smith-protocol-llm` `prompt.rs`, `tests.rs`), 11 in `tests/protocol-llm/`, 2 in `tests/session/`, `tests/agent/tests/host_tools.rs` |
| `rg -l 'threshold_bytes' crates tests benchmarks` | 5: `smith-agent-service` `profile.rs`, `profile/validate.rs`; `smith-domain-session` `limits.rs`, `tests.rs`; `tests/protocol-llm/tests/history_window.rs` |
| `rg -l 'NotRun::' crates tests` | 18 |
| `rg -l 'JobAct::' crates tests` | 12 |
| `rg -l '"version":2' benchmarks/tests/recorded tests` | 0: the event stream's key is `"v"` |
| `rg -l '"v":2' benchmarks/tests/recorded tests` | 7: six recorded `*.jsonl` and `tests/codecs/tests/events.rs` |
| `rg -l 'TASK_ROLE' crates tests` | 4: `smith-local-shell` `role.rs`, `settings/resolve.rs`, `settings/resolve/local.rs`, `settings/tests.rs` |

The events' version 3 also meets: `crates/smith-events/golden/v2/` (216
goldens), read by 18 files (`rg -l 'smith-events/golden/v2|\.\./golden/v2'`,
13 of them in `benchmarks/`); and `crates/smith-events/golden/unsupported-v3.jsonl`
(`{"v":3,...}`), which `tests/codecs/tests/events.rs`'s
`a_frozen_line_of_another_version_is_refused_by_name` expects refused as
`Error::Version(3)`. `Purpose` on `main` derives `Clone, Copy, PartialEq,
Eq, Hash, Debug` (`llm.rs:241`) and is re-exported by `smith-domain/src/llm.rs:20-23`.

The memory tests: `crates/smith-local-shell/src/settings/tests.rs:498`
(`the_default_inline_product_parts_fit_two_gibibytes`) and
`crates/smith-agent-shell/src/config.rs:1430`
(`product_values_pass_every_startup_check_and_fit_complete_memory`).

## 11. The two type sketches under clippy

A scratch crate (`cargo new --lib`, edition 2024, rustc 1.95.0 as smith's),
the sketches verbatim with stubs for `skein_llm::Prompt`, `Failure`,
`Measure<T>`, `Criterion`, `Projection`, `Stop`, and one caller of
`guard::measure` so that it is not dead. `heavy cargo clippy -- -D warnings`:

- **As written:** both compile, exit 0, no lint.
- **`Purpose` keeping its derive on `main`:** 6 errors, E0204 (`Copy`
  cannot be implemented) and E0277/E0369 (`Guard` lacks `Clone`,
  `PartialEq`, `Eq`, `Hash`, `Debug`). `Guard` needs `#[derive(Clone,
  Copy, PartialEq, Eq, Hash, Debug)]`.
- **Under each crate's lints** (`missing_debug_implementations = "deny"`
  in smith's workspace and in `benchmarks/Cargo.toml`, with
  `clippy::pedantic`): 6 errors, `type does not implement Debug`, on
  `Purpose`, `Guard`, `ChangeSection`, `Ratio`, `Outcome`, `Beside`.
- **With the derives** (`Guard` as above; `#[derive(Clone, Debug,
  PartialEq)]` on the four harness types): exit 0, no lint, pedantic
  included. A summary section also needs `Serialize` and `Deserialize`, as
  `parity::Parity` has; not compiled (no `serde` in the scratch crate).

On `main`, `benchmarks/src/parity.rs:23`'s `Criterion` is a struct (`name`,
`statistic`, `interval`, `margin`, `holds`), not a criterion's name: the
sketch's `Ratio { criterion: Criterion, statistic, lower, upper }` would
carry a second statistic and interval. The scratch stub is a name enum
(FT, FC, RT, RC). `benchmarks/src/result.rs:487` has an `Outcome` struct
beside the sketch's `parity::change::Outcome`.

## 12. The review fixes' commands, run once

2026-10-16, by the planner, after the three reviews (development.md 5.4:
"The fixes of the reviews above get the same run").

- README "Done when" item 8 at the base: prints nothing, exit 0.
- Item 9's three commands at the base: each prints nothing. The window
  of 272,000 is declared in `crates/smith-local-shell/src/presets.rs`
  (`window = 272000`, `:52`, `:59`, `:66`), which item 9 reads beside
  `quantities.rs`.
- 11's baselines' `jq`, on `parity-87a8d1e8-20261015T170125Z.json`:
  smith `d4bad0b05a22dfd5c12e72245484f4624b78b3aeaef5cd11da22ea001626ed35`,
  Codex `ecced90914ddd0e0ddf5331bf6e2190fc8ee6543aa73c9da25b28b5ed07d8afa`,
  the digests `9d1119b3`'s body names.
- README rule 13's counts, on that run's first two `shell-cwd` attempts:
  `codex-standard-12eb3e81 54 [unavailable compactions, 51 responses,
  command_execution 57 (lower bound)]`, `smith-standard-136f46cd 49 [1
  compaction, 72 responses, edit 24, job 14, ...]`.
