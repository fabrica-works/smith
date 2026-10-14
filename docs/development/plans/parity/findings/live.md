# Findings: the designer's live checks

2026-10-14. The two live attempts the designer ran for the working-tier
decision, and the list prices it rests on. Written by the parity plan's
designer (skein's development.md, section 4.1).

## 1. In one page

- **smith runs `gpt-6.1-sol` at high effort on the borrowed Codex login.**
  One `smith exec` on the archived arm `8a9747fa` answered a shell task
  correctly in 2 turns and 8.3 s, for $0.003376 notional, with the login
  file's digest unchanged (L1).
- **`smith exec` hangs in a repository that has no commit.** In a fresh
  `git init` workspace it printed "Repository head unavailable", made no
  request, and sat silent until the 300 s outer timeout (L2).
- **The providers' list prices** (developers.openai.com, read
  2026-10-14): `gpt-6.1-sol` $2 input, $0.10 cached input, $10 output per
  million tokens; `gpt-6-astra` $10, $1 and $50; `gpt-6-luna` $0.10,
  $0.01 and $0.50; each with a long-context tier past 272,000 input
  tokens at twice the input and cache rates and 1.5 times the output
  rate. smith's presets (`crates/smith-local-shell/src/presets.rs:111-117`)
  match them for astra and luna; the harness's
  `benchmarks/agents/prices.toml` does not (L3).
- **The user's own Codex works at `gpt-6.1-sol`, high:** `~/.codex/config.toml`
  lines 1-2 (read only, written 2026-10-12), as the workspace's
  coordinator profile does (L4).

## 2. Entries

### L1 smith on `gpt-6.1-sol`

- **What ran:** `heavy --gate` task 3613 (log
  `~/src/rust/tmp/logs/20261014T080456-3613.log`), the archived arm
  `~/.local/state/smith-bench/arms/8a9747fa…/smith` (`8a9747fa`, binary
  SHA-256 `c2c3fe9e…`), with private `XDG_CONFIG_HOME`, `XDG_STATE_HOME`
  and `TMPDIR` under `~/src/rust/tmp/design/sol-check/`. Its user layer
  declared `model = "codex/gpt-6.1-sol"`, `effort = "high"`, the borrowed
  `codex` account (`file = "~/.codex/auth.json"`, read only), and the
  model's `window` 272,000, `output` 32,000 and list prices. Prompt: "Run
  the shell command: wc -c README.md. Then answer with the number of bytes
  it printed, and nothing else." in a one-commit repository.
- **What it showed:** `smith check --json exec` accepted the settings
  (credential `usable`). The run: `response.completed` twice, `stop`
  `tools`; usage 1,238 then 132 input tokens, 0 then 1,152 cached, 27 and
  25 output; one `shell` call `exited` 0, then `finish`; `run.completed`
  `accepted`, text "6", 2 turns, 3,376 micro-dollars, 8,316 ms. The login
  file's SHA-256 prefix was the same before and after.
- **For the design:** shell.md 6.1 needs a preset row for the model; the
  dialect needs nothing new.

### L2 `smith exec` in a repository without a commit

- **What ran:** the same command and settings, before the workspace's
  first commit (`git init` only), task 3613's earlier run (log
  `~/src/rust/tmp/logs/20261014T080418-3613.log`).
- **What it showed:** standard error: the trust roots, the anthropic
  warning, the chat name, then "smith: Repository head unavailable". The
  stream held `session.started`, a `not_contained` notice and, at the
  outer `timeout`'s signal after 299,998 ms, `session.ended` with
  `answered: false`; no `run.started`, no request.
- **Root cause:** not traced here. The product treats a workspace
  directory that is a repository as one with a head; a repository with
  none is neither refused nor run.
- **The clean fix:** a plain defect (smith's `docs/development/workflow.md`,
  section 5): a workspace repository without a commit either runs, with
  no head to deliver against, or is refused at startup with exit 2 naming
  it (shell.md, 4.3); never silence. Its test: an end-to-end story that
  starts `smith exec` in a repository with no commit and expects an exit
  within the test's bound.

### L3 List prices

- **Read:** developers.openai.com/api/docs/models/{gpt-6.1-sol,gpt-6-astra,gpt-6-luna},
  2026-10-14, as quoted in section 1.
- **For the design:** the common priced spend of benchmarks.md 8.1 is
  priced from smith's presets; `prices.toml` goes. At these prices a
  `parity` run's smith share is estimated at $7 to $10 on `gpt-6.1-sol`
  (the repository task about $1.2 an attempt, a fixture $0.03 to $0.08),
  from the 2026-10-14 runs' token medians; Codex's share is of the same
  order and is not smith's notional spend.

### L4 The user's working model

- **Read:** `~/.codex/config.toml`, lines 1-2: `model = "gpt-6.1-sol"`,
  `model_reasoning_effort = "high"`; file modified 2026-10-12T10:21Z.
  `~/.codex/models_cache.json` lists the model with the efforts low to
  ultra and a 500,000-token context window. skein's
  `scripts/dev/workspace/coordinator.config.toml` runs the coordinator on
  the same model and effort.
- **For the design:** benchmarks.md 7's working tier is "the user's
  working model and effort"; decisions.md, 2.2.
