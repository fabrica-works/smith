# Findings for the parity plan

2026-10-14. The evidence the parity plan's decisions rest on (skein's
development.md, sections 3.4 and 4.1). Seven themes were written by
read-only sub-agents from the 2026-10-14 milestone
(`docs/development/benchmarks/smith-codex-2026-10-14.md`), its committed
summaries, the raw attempts under `~/.local/state/smith-bench/runs/`,
the reliability plan's record and the code at smith `689c15d0`; the
eighth is the designer's own live check. No spike was run: each entry
gives its symptom, root cause, the clean fix and its tests.

| File | Theme | Entries | What it decides |
|---|---|---|---|
| `measure.md` | what "on par" can mean in smith-bench's terms | M1-M10 | the criteria, margins, attempts and the offline verdict (decisions.md, section 1) |
| `harness.md` | the benchmark tasks' and the harness's own defects; the reliability run's rulings not yet in the design | T1-T14 | the tasks' next versions, the pins as shipped, one price catalog, calibration in task files (decisions.md, sections 2 and 3) |
| `compaction.md` | compaction answered with a call or with the wrong state | K1-K8 | `ToolChoice::None`, the summary's framing and instruction, `history` version 2 (decisions.md, section 4) |
| `time.md` | where smith's wall time goes against Codex's, and how it uses its tools | W1-W8 | jobs, the time record (decisions.md, section 5) |
| `tokens.md` | cache reuse, fresh and reasoning tokens | R1-R7 | caching as a diagnostic, not a criterion (decisions.md, 1.3) |
| `prompts.md` | the system text and tool descriptions, against Codex's | P1-P8 | the task role and the descriptions' facts (decisions.md, section 6) |
| `probes.md` | smith's failures and the probes that did not calibrate | F1-F13 | which probes this plan calibrates, which wait (decisions.md, section 7) |
| `live.md` | the designer's two live attempts and the providers' list prices | L1-L4 | the working tier (decisions.md, 2.2), a startup hang (7.3) |

**Where the findings disagree, and what was decided:**
- **The price catalog.** `measure.md` (Q8) and `tokens.md` (Q2) keep
  `benchmarks/agents/prices.toml`; `harness.md` (T6) takes smith's
  presets. Decided: the presets, the providers' list prices (decisions.md,
  1.4).
- **The working tier.** `measure.md` costs its suite on `gpt-6-astra`;
  the plan runs on `gpt-6.1-sol` (decisions.md, 2.2), so its dollar
  figures scale by about a fifth; its attempt counts and powers stand.
- **The `cache-reuse` probe.** `probes.md` (F5) classes its failure as
  smith's; `tokens.md` (R4) shows that the small tier's cache never
  passes the first 1,536 tokens, so the probe cannot pass as written.
  Decided: a version 2 later, not in this plan (decisions.md, 7.2).
- **Children.** `time.md` (W3) proposes asynchronous children; the plan
  compares smith as it ships, with `sub_agents` off, and leaves them for
  later (decisions.md, 5.3).

The tally scripts the sub-agents used lived in `/tmp` and are not kept;
each file's method section says enough to compute its numbers again.
