# Why smith is slower than Codex on shell-cwd (parity-87a8d1e8-20261015T170125Z)

2026-10-16. Read from the last closing run's attempts, smith 49-53 and Codex 54-58
(`~/.local/state/smith-bench/runs/parity-87a8d1e8-20261015T170125Z/`), all 10 passed.
The tally scripts were the analyst's, in `~/src/rust/tmp/design/`, and are
not kept; each section says what it counted.

## The gap, by mean seconds (smith 678 s, Codex 502 s; medians 654 s and 493 s)

| | smith | Codex | Δ |
|---|---|---|---|
| main-conversation requests | 525 s | 451 s | +74 |
| compaction | 74 s | 0 | +74 |
| inside tool calls (job + shell vs exec) | 79 s | 53 s | +26 |

A request costs the same on both: about 3 s plus 16 ms per output token (about 62 tokens/s).
Pooled median seconds per request: smith 6.24, Codex 6.71. The excess is the number of
requests: medians 65 against 46.

## Causes, ranked

| # | Cause | s | Change that removes it | Tokens |
|---|---|---|---|---|
| 1 | A cargo start or a job poll in a turn of its own: +14.6 requests | +90 | a shell call batched after the edit, in the same response; job polls batched with reads (both are already allowed) | about -1.8M cached, -2K output |
| 2 | A compaction smith did not need, in 14 of 15 smith attempts over the three runs | +74, plus re-reads | the window check's arithmetic (below) | fresh -43 to -93K, output -3 to -5K, cached +2.1 to +4.6M: about neutral |
| 3 | One file per edit request: all 129 edit responses held one edit of one file; Codex made 8-13 patches of 1.6-3.1 files | +62 | `edit` over several files (a patch), or the role's batching topic naming edits | about -2.0M cached |
| 4 | Blocking on cargo: 30 s job waits (`job_wait` default), 1 s start yields, attempt 52's 11 blocking cargo shells, `CARGO_BUILD_JOBS=2` in 4 of 5 attempts | +26 | polls beside other work (Codex polls at 1 s inside its next read); a job's end told between completions (parity `later.md`) | about neutral |
| - | Offsets: smith's parallel reads (-8 requests); Codex's separate git and log turns | -51, -28 | keep them | - |

## The compaction trigger

- It fired at real prompts of 120-172K tokens, 44-63% of the 272,000 window, against the
  4/5 threshold's 217,600 (`crates/smith-agent-service/src/profile/quantities.rs:193`).
- The bound is the last prompt, plus the last output, plus the appended tool bytes counted one
  token per byte, plus the whole 32,000-token output reservation
  (`crates/smith-domain-session/src/window.rs:88-98`, `compaction.rs:66-70`). Attempt 49:
  152,588 + 224 + 34,469 bytes + 32,000 = 219,281 > 217,600.
- No main request produced more than 2.4K output tokens. Uncompacted, smith's peak prompt
  would have been 177-220K: within 272K.
- Codex peaked at 149-179K, with no compaction (window 475,000). **No Codex request went past
  272K input tokens.**
- Each compaction took 57-105 s (a summary of 2.9-4.7K tokens at about 50 tokens/s), and was
  followed by re-reads of 13-22 paths (111-271 KB).

## Fresh tokens

smith's fresh input is 238-355K against Codex's 145-192K. It is new content, not cache misses
(prefix misses 8-18K an attempt, 66K once after a compaction): smith's tool output is 700-900 KB
an attempt (reads 565-770 KB: whole files or 10-19 KB windows), against Codex's 426-515K
characters (`sed -n` ranges, 3-6 files per command). Ranged reads would cut 60-90K fresh tokens and
keep prompts further from the trigger.

## The price basis

The presets price a Codex model at its long-context rates whenever its window passes 272,000
(shell.md 5.2, `crates/smith-local-shell/src/presets.rs` `list_prices`). Codex reports 475,000 for
`gpt-6.1-sol`, so every Codex token is priced at 2x fresh and cached and 1.5x output; smith, at
272,000, at base rates. Priced on the base rates for both (no Codex request passed 272K):

| Run | FC as reported | FC at base rates | shell-cwd median spend, smith / Codex at base | RC at base |
|---|---|---|---|---|
| baseline 744e0820 | 0.486 | 0.86 | $1.458 / $1.187 | 1.23 |
| first closing b7b78fb1 | 0.598 | 1.06 | $1.527 / $1.317 | 1.16 |
| last closing 87a8d1e8 | 0.532 | 0.94 | $1.452 / $0.987 | 1.47 |

Point estimates only, without intervals.

## Raising the window instead

- Memory: history scales at about 645 B a token of window (parity `findings/probes.md`); 475K
  needs about 130 MB more, and the standard profile's margin under its 2 GiB `memory` is 28,783
  bytes (parity retrospective, section 9).
- Price: past 272,000, the presets price smith's every token at the long-context rates, as they
  do Codex's now.

## The memory ceiling

- `memory` is 2 GiB in the standard profile, "decided as at most 2 GiB" (shell.md 6.3). It is
  the most the process may use (protocol/limits.md 5), not what it uses.
- At startup smith adds up a static worst case: every part at its derived limits, with the LLM
  pool full and every conversation open, each holding a full window and `jobs` jobs at their kept
  output's ceiling. It refuses to start when the sum passes `memory`
  (`crates/smith-local-shell/src/local_host/memory.rs`).
- The sum is now 28,783 bytes under 2,147,483,648, after the parity run's jobs. Before the run
  it was 208 MB under (parity `findings/probes.md`, F12).
- Measured: on `shell-cwd` smith's own process peaked at 36-43 MB (cargo and the other children
  peaked at 427-456 MB, outside smith's account).
- The last breakdown by part (reliability era, 6 conversations, 2.04 GB): llm 699 MB, local
  domain 685 MB, store 427 MB, delivery 75 MB, front 65 MB, machine 41 MB, the rest under 31 MB
  each. One conversation costs about 153 MB.
- `conversations` is 5, "main and four children" (shell.md 6.3, provisional), but `sub_agents`
  is off by default: a shipped run opens at most main and a compaction at once. The other three,
  about 460 MB, are priced for children the default never opens.
- The ways to make room: count the worst case more tightly (10.4's correction removed 416,580
  bytes); derive `conversations` from what the settings enable; lower `conversations` or `jobs`;
  or raise `memory`, a decided value, so the user's call. The first two keep the 2 GiB promise.


## Memory: what is measured and what is not

- How the code holds memory, by skein's `docs/foundation/programming-model.md` 6.1: slabs and
  queues allocate their capacity up front, at startup; bytes are `Box<[u8]>` allocated at their
  final length when needed, owned and moved, and freed with their owner. A table may allocate as
  it fills.
- Measured: 36-43 MB peak resident for smith itself on `shell-cwd`, against a 2 GiB worst case,
  so most of the worst case is bytes allocated on demand.
- Unmeasured: whether memory comes back once its use ends (a compacted history, a collected
  job's output, a closed conversation), and how much of the 36-43 MB is capacity reserved up
  front.
- What would show it: the simulated world's counting allocator (programming-model 6.3)
  asserting that the live heap returns near its baseline after a compaction, a job's collection
  and a conversation's close; and peak resident memory (already in `session.ended`) read in the
  benchmark beside time and cost.
