# Session 10: the lines the work needs

2026-10-16. Fresh tokens, in `findings/causes.md`: smith's fresh input
was 238,000 to 355,000 tokens an attempt against Codex's 145,000 to
192,000, and it was new content, not cache misses: smith read whole
files or 10 to 19 KB windows, 565 to 770 KB of reads an attempt, where
Codex read `sed -n` ranges. Ranged reads would cut 60,000 to 90,000
fresh tokens an attempt and keep prompts further from the threshold.
The design asks the role's topic 2 for the lines the work needs: a
window from where a search found them, a whole file only when the work
needs all of it (decisions.md, section 8). This step changes the role,
so it is proven by its `parity-change` run.

In the baseline, `parity-c10edbe5-20261016T184102Z`, fresh tokens were
59% of RC's excess: on `shell-cwd`, smith's cost $0.614 an attempt to
Codex's $0.367, of $1.552 to $1.131 in total (means; cached $0.680 to
$0.562, output $0.258 to $0.202). This step addresses RC, and RT
through prompts further from the threshold.

## Read first

- smith `docs/design/shell.md`, section 6.6 (topic 2, and "A change to
  them is proven").
- smith `docs/design/benchmarks.md`, sections 16.5 and 16.6.
- `decisions.md`, section 8; `findings/causes.md`, "Fresh tokens";
  `findings/planning-audit.md` A25.

## What the design asks

- **Topic 2:** finding files with `list` and `search` first, then reading
  the lines the work needs: a window from where a search found them, a
  whole file only when the work needs all of it (shell.md 6.6).
- **No new call field and no smaller default `read_window`**
  (decisions.md 8, "Not taken").

## Choices this plan makes

- The role's second paragraph becomes:

  > Find relevant files with `list` and `search` first. Then read the
  > lines the work needs: a window from where a search found them, and a
  > whole file only when the work needs all of it. Follow references and
  > existing conventions before making a change. Read further windows
  > when a result is cut.

## Rules that bind it

- **One change at a time** (README, rule 11), **the stop** (rule 12).
- **No words naming a task's files or commands** (`code-organization.md`,
  section 7).

## Increments

### 10.1 The role reads the lines the work needs
Scope: shell.md 6.6, topic 2 (findings A25; fresh tokens).
Needs: smith 09's polls [09.1] landed or dropped.
Files: `crates/smith-local-shell/src/role.rs` (`TASK_ROLE`, `:6-20`),
  `benchmarks/summaries/`.
Shape: `TASK_ROLE`'s second paragraph takes the words above; the role
  stays within 4,096 bytes, the cap `the_task_role_is_within_four_kibibytes`
  checks (1,818 bytes on main after 08.1, about 1,915 with 09.1's words,
  about 1,995 with this step's 80). 08.1 removed `TASK_ROLE_BOUND`:
  `role_bound` is the role's own length, so each byte this step adds
  counts in the host part, and
  `settings::tests::composition_adds_every_model_of_accounts_with_a_source`
  (`settings/tests.rs:308`) measures it in the gate (ruling 120).
Test shape: the existing `the_task_role_is_within_four_kibibytes`
  (`role.rs:27`) and `each_tool_the_task_role_names_is_one_of_smiths_own`
  (`:32`) hold; no new test.
Proof: live, through the stop: `heavy cargo run -p smith-bench -- build
  HEAD` and `build origin/main`, then `heavy --gate cargo run -p
  smith-bench -- suite parity-change --arm <HEAD's dir> --parent-arm
  <origin/main's dir> --baseline-run <the file README rule 13's first
  command prints> --landed <each file its second prints>`: 40 attempts,
  about 3 hours, run once; its exit decides (README, rule 12). Its
  summary is amended into the step's commit and named in
  `Change-Summary:`; the commit body gives its criteria, projection and
  stop lines, README rule 13's counts beside, with the report's fresh tokens per arm, and names the fresh
  tokens' cause and the criteria it addresses, RC and RT, with the
  baseline's figures above.
Don't: change `read`'s schema or `read_window`.
Size: 10 lines.
Removes: nothing.

## Questions to raise when reached

- None known.

## Done when

- `cd ~/src/rust/smith && rg -c 'a window from where a search found them' crates/smith-local-shell/src/role.rs` prints 1, or `awk -F'\t' '$1 == "smith-10.1" { print $4 }' ~/src/rust/tmp/smith/wall-clock/ledger.tsv` prints `dropped`.
