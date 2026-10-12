# Reliability and performance: findings

Copied 2026-10-12 from the first run's `findings.md`, unchanged below
this note. Here, `findings/<theme>.md` is this directory's
`<theme>.md`; `QUESTIONS.md` is now the plan's `decisions.md`, numbered
the same; `findings/scripts/` (the analyses, in Python) stays in the
first run's directory, `~/src/rust/plans/reliability-plan/findings/`, on
debian-16gb-hel1-1.

2026-10-09. Step 1 of the reliability plan: what the two spikes found, why
it happened, which spike changes are sound, and what the clean version
needs. Nothing here is decided. Step 2 turns section 9 into `QUESTIONS.md`.
Step 3 then writes the design documents, and step 4 the plan sessions.

Inputs:

- spike 1, real-world use: smith `fix/interactive-reliability-2026-10-08`
  (`6136dc4`, `8e83931`), with
  `docs/development/smith-reliability-report-2026-10-09.txt`;
- spike 2, benchmark against Codex: smith `bench/smith-codex-2026-10-08`
  (19 commits on spike 1), with
  `docs/development/benchmarks/smith-codex-2026-10-08.md` and the
  artifacts in `~/src/rust/smith-benchmark-2026-10-08/`. The other
  `bench/*-2026-10-0x` branches are its topic branches, already folded in;
- skein `bench/llm-shutdown-2026-10-08` (`416cffd`, `f5fca44`), which
  smith's spike lock pins;
- two directions from the user (section 8): smith's crates are libraries
  only, with binaries in `./src` as glue; the standalone agent becomes a
  designed single-process product for benchmarks and daily use.

Refs: smith `main` is `30259d8`, the spike tip `bf96e94`; skein `main` is
`e8bc018`. The evidence is in eight theme files under `findings/`, one per
analyst, each entry with symptom, root cause, spike change, assessment,
clean fix, contracts, layer, tests, benchmark probe and questions. They
are cited here by entry ID:

| Prefix | File | Theme | Entries |
|---|---|---|---:|
| L | `findings/lifecycle.md` | messages, startup, activations | 15 |
| S | `findings/limits.md` | limits, sizing and timeouts of the LLM path | 15 |
| C | `findings/context.md` | context growth, prompt caching, tokens | 10 |
| D | `findings/delegation.md` | sub-agents, delegation, run budgets | 12 |
| O | `findings/operation.md` | shutdown, traces, failure diagnostics | 16 |
| T | `findings/tools.md` | workspace tools, the machine, local setup | 12 |
| P | `findings/packaging.md` | crate layout and the standalone agent | 18 |
| B | `findings/benchmarks.md` | the spike's harness and method | 14 + probes |

`findings/scripts/` holds the trace-analysis scripts behind the caching
numbers (C1, C9).

## 1. In one page

- **The spikes found real defects and fixed most of them by raising
  constants.** About half the changes are sound mechanisms. The rest are
  limits raised until the symptom went away: tool arguments 2 → 32 KiB,
  request 8 KiB → 1 MiB, event tokens 1,024 → 4,096, completion deadline
  60 → 300 s, turns 64 → 256. Each moved a cliff, and the next cliffs
  are close: a long root conversation dies at about 190 history items or
  60–90 turns, well under the new 256-turn ceiling (S2, S3).
- **The 256-turn ceiling did not cause the repository-task successes.**
  Both successes used fewer than 128 turns, and the last failure ran out
  of time with 81 turns unused. The real causes:
  - a child with no share takes the whole remaining budget;
  - the main agent has no reserve to finish with;
  - no model is ever told its budget or time (D1, D3, D4).
- **Most of the uncached-input gap is cache routing, not context size.**
  Without an affinity key, 72% of Smith's repository requests read no
  cache, although the prefix is byte-stable. With perfect reuse Smith
  would have used about 0.18 M fresh tokens, under Codex's 0.28 M (C1).
  Claude requests are never cached: skein's Anthropic dialect sends no
  `cache_control` (C5).
- **Nothing manages context.** History is append-only and does not know
  the model's window. It ends at one of five fixed cliffs (C7). The
  planned in-place elision would break caching. Compaction should start a
  new window that reuses the cached prefix (C8).
- **Every successful hosted run ended in SIGKILL, and nobody saw it.**
  skein's LLM pool had no owner close. The host drops exit statuses. The
  test worlds used 5 ms keep times, which hid the wait (O1, O5, O6). The
  missing piece is a general skein rule: a component that keeps
  resources has an owner close and a closed terminal, and a process ends
  only when everything it owns has settled (O3).
- **Observability has no contract.** The trace is Rust `Debug` text, and
  it loses records three different ways. The local host reduces typed
  failures to a fixed "Run failed" and exits 0. The benchmark harness and
  smith's own live tests regex-parse `Debug` strings. A versioned event
  vocabulary is the foundation for diagnostics, benchmarks and the
  product (O10, P12, P13, B13).
- **A message has no terminal.** "Offered is not proof of read" is the
  right principle, but only the local host implements it. The clean
  contract gives every message exactly one terminal: read, refused, or
  unread at the answer (L5). Startup should open the run only when it
  has work, and a start can carry its triggering messages (L1).
- **The standalone agent was never designed.** `smith local` is a
  supervisor that spawns `smith agent` per activation. Half the spike's
  fixes exist only because of that path. The in-process mode cannot
  serve benchmarks today: it writes no trace, hides failures, and has no
  stop ladder (P6–P9). The product should be one single-process binary
  built on smith's inline agent, the twin of `smith-host-domain` from
  jig's one-host draft (P18).
- **The extra tool calls are mostly an accounting artifact; the real cost
  is round trips.** The causes are small output windows (4 KiB reads,
  6 KiB shell), one snippet per edit, a path convention that models get
  wrong in a third of runs, and descriptions that discourage batching
  (T3–T6).
- **The deterministic tiers could not have caught these bugs.** Every
  world person types one line. Worlds run with test-only keep times.
  Referees contradict the new behaviour. The shipped agent adapter is not
  the one worlds run, and spike tests pin constants (L11, O6, P4, S15).
- **The spike's benchmark harness mostly worked around missing
  observability.** About four cheap probes plus checks on every run's
  trace would have found every defect in roughly 2 minutes and under
  100k tokens. The spike spent about 2,000 agent-seconds and 6 M tokens.
  With three runs per side, no comparison can reach statistical
  significance (B1, B4, §4).

## 2. The spike, change by change

| Commit | What | Verdict | Entries |
|---|---|---|---|
| `6136dc4` | limits raised; waiting guidance | limits: redo as derived model; guidance: keep with rework | S1–S3, S10, S13, L13 |
| `8e83931` | message lifecycle, queued opening, `AWAIT` | lifecycle: keep behaviour, redo mechanism; `AWAIT`: superseded | L1, L5–L9, L14 |
| `8e83931` | shell environment merge | keep behaviour, redo ownership | T1, S11 |
| `8e83931` | `list .` mapping | redo (one path translation) | T2, T3 |
| `8e83931` | live interactive story | keep with rework | L12 |
| `3fae871` | local chats may grant sub-agents | keep | D9 |
| `140c351` | empty brief awaits first task | keep with rework | L1–L3 |
| `9340755` | memory fixture past `Awaiting` | keep | L2 |
| `a2c2e53` + skein `416cffd`, `f5fca44` | close LLM pool at shutdown | keep with rework | O1, O2, O4 |
| `a54ba26` | trace drain with 50 ms grace | redo (trace through io) | O7, P2 |
| `ecb7562`, `e69bfc3`, `8466afb` | operator diagnostics side queue | redo (typed, in the local domain, both placements) | O13–O15, P7 |
| `ee512a1` | tool arguments 2 → 32 KiB | keep with rework (per-call oversize) | S4 |
| `ad8a793`, `fc939dc` | local-host docs | rewrite with the product | L1, S7, T12 |
| `d75190b` | completion deadline 60 → 300 s | redo (progress deadlines) | S5 |
| `cfb7408` | request envelope 256 KiB → 1 MiB | keep the bound, derive it, add context management | S3, C7, C8 |
| `136b9c4`, `6a1dfb2` | terminal facts, admission guards, boxed records | keep invariant, redo mechanism | O8, O9, O12 |
| `5848e59`, `44e5120` | turns 64 → 128 → 256 | redo (budget model) | D1–D3 |
| `44e5120` | `sub_agent.max_turns` | keep with rework (`max_seconds`, child told its share, final turn) | D5, D6 |
| `49057ae` | benchmark report; README cache recipe | report: keep with erratum; recipe: replace with domain affinity | D12, C1 |

Dated validation logs that the spikes appended to
`docs/development/workflow.md` do not carry over (D, cross-theme notes).

## 3. Cross-cutting findings

### 3.1 One event vocabulary underneath everything

Five themes ask for the same thing:

- a typed, versioned stream of records, the same for `--json` output and
  the trace file;
- a header record and an end-of-run summary record with loss counts;
- emission timestamps;
- call ids on tool records;
- per-response usage, including cache writes and reasoning tokens where
  the provider reports them;
- effective child shares on sub-agent facts.

Sources: O10, O11, O16, P12, P13, D11, B13. Today's consumers are the
benchmark harness, the live tests and operators, and they parse `Debug`
text and stderr sentences.

**Conflict to resolve:**

- O10 wants facts lossless inside the process, with the trace in a
  "complete" mode by default.
- P13 wants authoritative records never dropped, and detail records
  best-effort with exact counts.

Both agree on per-sink counting and a summary record. The open choice is
the default for detail records (section 9).

### 3.2 Limits: from constants to declared quantities

The constants live in one hand-written function covering eight layers.
Most relationships between them are implicit, and two are already broken:

- 256 completion blocks against a transcript that stores 16 per message
  (S8);
- two connections per endpoint, so a third concurrent call is a fatal
  `Limit` (S9).

The direction S14 proposes:

- a few declared values per model (context window, max output, reasoning
  item size, head and idle timeouts);
- a few per deployment (concurrency, max tool payload, render caps, the
  LLM memory pool);
- everything else derived with checked arithmetic, and every relationship
  checked at startup, with refusals that name the relationship.

Related: T5 and T7 (tool and machine sizes in lockstep), S11 (the ~360 GB
worst-case memory bound protects nothing), P3 and B12 (`smith profile
--json` replaces the benchmark's binary registry), T12 (`smith check`).

**Conflict to resolve:** where model defaults live.

- C7 says the charter carries the window.
- S14 says skein ships defaults that settings override.

These compose: skein ships model defaults, settings override them, and
the host puts the effective values in the charter.

### 3.3 Context: affinity, a stable prefix, window-aware admission, compaction

The order of value:

1. affinity identity owned by the domain (C1);
2. the stable-prefix invariant written down and tested (C3);
3. Anthropic cache breakpoints, and cache writes decoded and priced (C5,
   C6);
4. admission that knows the window, and compaction (C7, C8).

**Conflict to resolve:**

- S3 recommends "elide old tool output first, then summarise", which is
  `session.md` §8 as written.
- C8 shows elision breaks prefix caching and Anthropic's preserved
  thinking, and recommends compaction as a new window instead.

C8's argument is stronger; section 9 lists it as a question.

Caching changes tokens and cost, not wall time. On the repository task,
latency barely depends on cached versus uncached input (C9), so the
997 s against 413 s gap lies elsewhere: delegation (D7) and round trips
(T4).

### 3.4 Budgets the model can plan with

The proposed budget model:

- **Roles:** time and spend are the planning bounds; turns are a loop
  guard (D1, D2).
- **A finishing reserve** for the main agent that children cannot spend,
  with a wind-down that gives the main a final turn (D3).
- **Visibility:**
  - the budget is stated in cache-stable system text;
  - every sub-agent result carries a usage line;
  - a child is told its share and gets a wrap-up turn with no tools (D4,
    D6).
- **`sub_agent` arguments:** `max_seconds` beside `max_turns` (D5).
- **Scheduling, in order:** guidance and parallel read-only children now;
  a run-level write lease next; asynchronous children and scoped writes
  in a later document (D7).

Two skein mechanisms are needed: `tool_choice: none` in skein-llm (D3,
D6), and process write containment (D7c, T9).

### 3.5 Messages and starting work

Three themes meet here:

- L1 has a start carry its triggering messages.
- L5 gives every message exactly one terminal.
- P14 has headless `smith exec` put the prompt in the start.

These are one mechanism. With it, the startup race disappears in both
interactive and headless use, and "Waiting for a message" shows only when
every relayed line has been read (L3). What happens to an unread line
after cancel or failure is host policy, and it must be written down (L6,
Q8). The `wait` tool costs an extra completion per exchange; deferring
its result removes that (L13).

### 3.6 Ownership and the end of a process (skein)

O3 proposes text for skein's programming model:

- components that keep entities take an owner close and report closed;
- a service ends when everything it owns has settled, bounded only by
  io's deadlines;
- the world harness asserts that no keep or idle deadline fires after the
  last word.

On the smith side:

- one exhaustive teardown phase, `Live → Answered → Closing → Closed`;
- the LLM pool closes at the answer;
- SIGTERM after the answer aborts what is still closing;
- the host grace exceeds the agent's teardown bound (O4);
- exit evidence reaches the host (O5).

### 3.7 The product: library crates, glue binaries, one process

The user's two directions, worked through in theme P:

- **Crates:**
  - split `crates/smith` into `smith-agent-shell` and `smith-local-shell`;
  - move the limits profile into the service crates;
  - make the root package `smith`, with `./src` binaries that are glue;
  - move the step `clippy.toml` to `crates/`, and put a shell one at the
    root with skein echo's bans (P1, P11).
- **Single process, on smith's inline agent** (P18). Today's dual-path
  in-process local domain would need its instrumentation, failures and
  stop ladder invented a second time (P6–P8).
- **Spawning agents** stays a tested library capability for temper and an
  optional isolated mode. Worlds then host the shipped
  `smith-agent-shell::Agent` instead of a test-only copy (P4, P10).
- **Front ends:**
  - headless `smith exec` with documented exit codes and `--json` (P14);
  - an interactive chat that renders the event stream (P15).
- **Ergonomics:**
  - provider presets and layered settings with discovery;
  - `smith check` and `smith config show`;
  - credentials (P16, P17, T11, T12):
    - borrow an existing CLI login, read-only and access-token-only;
    - `localhost` redirects in skein-oauth;
    - environment tokens for CI.

### 3.8 The tool surface

- **Round trips:**
  - a tree listing;
  - multi-hunk edits, with the format per dialect decided by
    measurement;
  - output budgets the model chooses within checked ceilings;
  - 32 KiB read windows and whole guides;
  - honest descriptions that invite batching (T4–T6).
- **Correctness:**
  - one path translation (T2);
  - one path namespace that tools and shell share, with failures naming
    the resolved path (T3);
  - one owner for the shell environment (T1);
  - stated timeout units (T7);
  - UTF-8 passed through when rendering (T8).
- **Containment:** process groups in skein-io now, contained trees later.
  Ambient build configuration (cache wrappers, read-only artifacts) needs
  a declared build profile (T9, T10).

### 3.9 Why the tests missed it, and what to change

- **Worlds:**
  - a person script that types on observed events, with fuzzy timing and
    EOF anywhere;
  - a "no line lost" liveness rule;
  - referees fixed to match the contract (L11).
- **Values:** shipped keep and idle times, not test-only ones (O6).
- **Harness invariant:** the O3 invariant, in skein's world harness.
- **Adapters:** one agent adapter shared by the binary and the worlds
  (P4).
- **Spike tests:**
  - boundary tests move to the lowest tier that shows the failure, mostly
    skein;
  - constant pins are replaced by derivation properties;
  - hand-rolled component pumps give way to skein's worlds (S15).

### 3.10 Latent bugs to fix whatever the design

| Bug | Entry |
|---|---|
| A completion with more than 16 blocks fails the whole agent when its turn is saved | S8 |
| A third concurrent call to one endpoint ends its session | S9 |
| A host can panic the agent with a message near the size bound | L15 |
| A non-empty brief can silently drop the person's first line | L4 |
| A follow-up's state save racing the answer hits `unreachable!` (fixed in spike 1, to keep) | L7 |
| Host drops the agent's exit status; forced exits are invisible | O5 |
| The session drops provider failure detail; skein labels a `Limit` "malformed provider event" | O16, S2 |
| In-process mode ignores the `trace` setting and exits 0 on failure | P6, P7 |
| Prompt capture keeps one prompt per pass and nothing over 64 KiB | O11 |
| The `sub_agent` schema offers `agents`, which the run drops | D10 |
| Shell timeout has no unit and is silently clamped to 60 s | T7 |
| Non-ASCII output is escaped, so a copied snippet never matches an edit | T8 |
| Anthropic sends `max_tokens` 4096 shared with thinking; a truncated `tool_use` makes the history unreplayable | S7 |
| Host watchdog `no_progress` equals `call_timeout`; facts are best-effort | P9 |
| A timed-out shell's grandchildren outlive it | T9 |

## 4. skein work

Each lands in skein, with its design text, before its smith consumer
(AGENTS.md). Grouped by crate.

- **Foundation:**
  - `programming-model.md` §5.2: components close; the end of a process
    (O3).
  - `shell.md`: "The end" (O3).
  - `testing-strategy.md` §6: the teardown invariant (O3, O6).
  - `notes.md`: resolve "the trace queue" (O10).
- **skein-llm-connection:**
  - owner close with drain semantics, a `Closed` event and per-phase
    deadlines (O2);
  - the expired phase in `TimedOut` (S5);
  - waiting for a connection instead of refusing, or sizing checked at
    startup (S9).
- **skein-llm, caching:**
  - neutral affinity with per-call headers and reserved header names
    (C1);
  - Anthropic cache breakpoints (C5);
  - cache-write and reasoning tokens in `Usage`, with "not reported"
    distinct from zero (C6, P13).
- **skein-llm, limits and failures:**
  - `tool_choice: none` (D3, D6);
  - oversize as a per-call outcome (S4);
  - typed `Limit` kinds and the HTTP status in failures (O16);
  - per-dialect limits derived from declared quantities, with model
    defaults (S7, S14);
  - a reasoning cap separate from replay metadata (S6).
- **skein-llm, later and only if measured:**
  - Codex routing state (C2);
  - `reasoning.context` (C4).
- **skein-json, skein-http:**
  - selective, streamed event decoding (S2);
  - compact tokens (S11).
- **skein-io, skein-shell:**
  - an append file stream for the trace (O7, P2);
  - durable whole-file replace and a private secret store (P2, P17);
  - process groups (T9);
  - `Host` and `drive` moved out of `skein-world` (P5);
  - trust-root loading (P).
- **skein-oauth:**
  - `localhost` redirects (T11);
  - access-only token records (P17);
  - the OAuth io driver its callers now write themselves (P).
- **Later:** contained trees with per-tree limits and write scopes (T9,
  T10, D7c). Process-tree accounting for the benchmark harness, if it is
  generic (B §3).

## 5. Design documents affected

- **smith, domain:**
  - `domain/README.md`: §1 (failures visible to operators), §4 (crate
    tree).
  - `domain/run.md`:
    - §3 (start, opening, prompts);
    - §5.3 (children, shares, lease);
    - §6 (messages, waiting, terminals);
    - §8.1, §9 (budget roles, reserve);
    - §10, §11, §14, §15.
  - `domain/session.md`:
    - §2–§4 (affinity, append-only, failures, progress deadlines);
    - §5, §6 (context dimension, cache prices);
    - §7;
    - §8 rewritten (compaction).
  - `domain/tools.md`: §2–§5 (namespace, tools, budgets, environment).
  - `domain/host.md`:
    - §2, §4 (message terminals, exit evidence, grace);
    - §5 (unread policy);
    - §7 (credential sources);
    - §8, §9 (the product, one process via the inline agent);
    - §10.
- **smith, protocol:**
  - `protocol/README.md`: §4, §5.
  - `protocol/agent.md`:
    - §3 (environment, view);
    - §4 (profile as declared quantities);
    - §5 (events);
    - §6 (end of life).
  - `protocol/channel.md`: §3, §5, §7, §8.
  - `protocol/charter.md`: window, prices, reserve.
  - `protocol/hosts.md`: §3, §5.1–§5.6.
  - `protocol/llm.md`: §2 (affinity, prefix), §3 (tool surface), §4–§6,
    §8, §11.
  - `protocol/transcript.md`: affinity, compaction turn, usage fields.
  - New `protocol/events.md`: the event vocabulary. It closes
    `agent.md` §10's open "format of traces kept".
- **smith, other:**
  - `testing.md`: §2, §5–§7.
  - New `docs/design/benchmarks.md`.
  - `docs/development/local-host.md`: rewritten for the product.
  - `workflow.md`: the spike's dated logs removed; a benchmark rule added.
- **skein:**
  - the foundation documents in section 4;
  - `llm.md`, `llm-connection.md`, `json.md`, `http.md`, `io.md`,
    `shell.md`, `oauth.md`, `fake-llm.md` (the Anthropic caching
    disposition).
- **jig:** the one-host draft, adopted on smith's side (P18).

## 6. Order matters

Some findings gate others:

1. **Events first.**
   - The event vocabulary and the end-of-run summary (3.1) come before
     benchmark probes can assert anything typed.
   - `smith profile --json` and `smith check` come before the harness can
     drop its registry (B12, P3).
2. **Baseline needs a stopgap.** The baseline on `main` must be taken with
   today's binary, so the first harness needs a temporary adapter that
   reads today's trace. It should be isolated in one module and deleted
   once events land. The alternative is to take "before" numbers from the
   spike's archived runs (section 9).
3. **skein before smith.** skein's ownership rule and LLM changes come
   before the smith changes that consume them. Several smith fixes span
   skein, smith protocol and domain, such as affinity and the tool-call
   oversize outcome.
4. **The inline agent before the product binary.** The inline agent
   (P18) precedes it. Until then, benchmarks use the spawned binary.

## 7. Benchmarks: what step 1 established

**Principles** (B, C10):

- Tests stay tests. Probes show the real-world effect.
- A probe forces the behaviour, so the model does not choose whether to
  exercise it, and it is calibrated: it fails on the pre-fix binary and
  passes 10 of 10 times after.
- Fixtures are for trends and comparison. Run them interleaved A/B in one
  session with at least 5 runs per side; three runs can never reach
  significance.
- The repository task is rare.
- Never at the merge gate. A smoke run is recorded when a change touches
  the runtime, limits, providers or the host.
- Isolated homes, pinned configs and dedicated sign-ins for every agent.
  A missing measurement is "unavailable", never a fallback from another
  scope (B9).
- Probes run on the smallest tier: `gpt-6-luna` at low effort, and
  `claude-haiku-5-5` at low effort. Claude Code takes `--effort` and
  `--max-budget-usd` in `-p` mode (checked 2026-10-09).

**Probe catalogue.** These are candidates, deduplicated across themes.
"Passive" checks run on every attempt.

| Probe | Guards | Shape | Cost (estimate) |
|---|---|---|---|
| first-request | L1, L3 | empty brief, one-line task, both placements: the first prompt holds the task, no `wait` before it | seconds |
| smoke-tools | T1, T2, O1, O8 | list `.`, read, `printenv`, write, finish: file contents, typed terminal, zero loss | 10 s, ~10k tokens |
| follow-up | L5–L7, L12 | greeting, then a request sent on `finish`: two answers, task once in the carried prompt | ~30 s |
| large-write | S4, S7 | one ~10–40 KB `write` call, on both providers: no `Limit`, no truncation | 15–40 s |
| history | S2, S3, C7, C8 | many reads past the envelope or a small window: completes, compaction recorded | 1–2 min; weekly |
| cache-reuse | C1–C6 | "pointer chasing" through 12 files: reuse ≥ 0.85, full misses ≤ 1 per 10 requests; variants with children, a user turn, Anthropic, and an idle gap | 20–60 s each; idle gap nightly |
| budget | D3–D6 | starvation, wrap-up, child quota, and budget visibility: the main still finishes, a capped child answers | 5–40 s each |
| parallel children | D7, S8, S9 | four read-only children, and a 20-file parallel read: no `Limit`, no `Send` failure, overlap observed | 30–60 s |
| failures | O13–O16, P7 | invalid model; tiny completion cap; one-turn budget: typed failure in words, nonzero headless exit | seconds |
| paths | T3 | two directories: zero `NotFound`/`Outside` | seconds |
| tool outputs | T5–T8 | 12 KB test output, a long doc quote, `sleep 70`, a UTF-8 edit | seconds each |
| process tree | T9 | `sleep 600 & wait` with a 5 s timeout: nothing survives | seconds |
| credentials | T11, P17 | a borrowed login: one completion, the login file unchanged | seconds |
| slow completion | S5, S6 | high-effort reasoning: no deadline or reasoning-size failure; head latency and inter-event gaps recorded | minutes; occasional |
| passive | O1, O5, O8, B P8 | every run: exit after answer < 0.5 s, not forced; one terminal equal to the host's answer; headroom against each limit | free |

**Comparisons:**

- fixtures 01 and 02 by default;
- 03 occasionally;
- the repository task 07 as a milestone, frozen with vendored pins.

New repository tasks need reference solutions. Per-attempt AI review is
replaced by sampled audits (B).

## 8. The user's directions, recorded

1. smith's crates are libraries only and produce no binary. Binaries live
   in `./src` and are short glue: smith is a kit consumed as libraries,
   and its binaries are for showcase and internal use. Worked out in P1,
   P11.
2. The smith-based agent used for benchmarks, and for daily use, is
   designed for it: single-process, with the instrumentation benchmarks
   need and the ergonomics a user wants. The current binary is "more an
   accidental demo than something that was designed". Worked out in
   P6–P18. The analysis confirms the binary spawns agent processes and
   recommends one process on the inline agent, keeping spawning as a
   library capability.
3. A `./benchmarks` directory in smith shows each fix works and stays
   working, against Codex first and Claude Code later. It is cheap by
   default, and structured for future needs. Worked out in B §6.

## 9. Questions for step 2

These are deduplicated. Each recommendation is the analyst's, sometimes
reconciled across themes. `QUESTIONS.md` will present them for decision.
Implementation choices are not listed.

### Cross-theme

| # | Question | Recommendation | From |
|---|---|---|---|
| X1 | Context: compaction as a new window, dropping in-place elision? | yes | C8 vs S3 |
| X2 | Where model defaults (window, output, timeouts) live | skein-llm defaults, settings override, effective values in the charter | C7, S14 |
| X3 | Default policy for detail records | lossless in the process; trace complete by default; channel drops and counts | O10 vs P13 |
| X4 | Benchmark baseline before events land | a temporary adapter for today's trace, isolated and deleted later | §6 |
| X5 | Order: inline agent and product binary before or after behaviour fixes | foundations first (events, skein rules, limits model, harness), then product, then behaviour themes | §6, P Q8 |

### Messages and startup

| # | Question | Recommendation | From |
|---|---|---|---|
| L-Q1 | A start carries its triggering messages, which headless `exec` also uses | yes | L1, P14 |
| L-Q2 | Empty brief without wait authority | open on the instructions | L1 |
| L-Q3 | Parking while awaiting the first message | charter waiting time, once Q1 lands | L1 |
| L-Q4, Q5 | Opening composition; messages per yield | brief, then all queued messages, FIFO and bounded | L4 |
| L-Q6 | A `finish` crossing a queued message | refused for wait-granted charters | L5 |
| L-Q7 | The host kit reports unread names at the answer | yes | L5 |
| L-Q8 | Unread lines after cancel or failure | show and drop; never auto-start after an interrupt | L6 |
| L-Q9 | Messages between completions, mid-work | design now, implement later | L10 |
| L-Q10 | The `wait` mechanism | deferred result, one request per exchange | L13 |
| L-Q11 | Time spent awaiting counts against wall time | yes, documented | L2 |

### Limits

| # | Question | Recommendation | From |
|---|---|---|---|
| S-Q1 | Limits as declared quantities, with raw byte limits not exposed; profiles as named bundles | yes | S14 |
| S-Q2 | Selective JSON collector, general or LLM-only | general, in skein-json | S2 |
| S-Q3 | One declared tool payload for read, write and arguments | yes | S4 |
| S-Q4 | Per-attempt completion ceiling | none by default; head and idle progress deadlines; whole = remaining session time | S5 |
| S-Q5 | Oversized reasoning item | typed failure; dropping it is opt-in | S6 |
| S-Q6 | Truncated Anthropic `tool_use` | skein marks it; smith replaces it with a note and an error result | S7 |
| S-Q7 | Completion blocks above the transcript bound | cap at 16 until parallel batches are designed | S8 |
| S-Q8 | Oversized rendered tool output | truncate with an explicit marker | S10 |
| S-Q9 | Memory target with admission to a shared pool, so the bound is real | ≤ 2 GiB; waiting as backpressure | S11 |

### Context and caching

| # | Question | Recommendation | From |
|---|---|---|---|
| C-Q1 | Affinity granularity | a run-tree key, with a per-conversation thread id | C1 |
| C-Q2 | Who supplies the identity | the host, in the start, with a minted fallback | C1 |
| C-Q3 | Remove the static endpoint `cache_key` and the README recipe | yes | C1 |
| C-Q4 | Private ChatGPT routing extras; WebSocket transport | only if measured; WebSocket on skein's roadmap | C2 |
| C-Q5 | Anthropic cache TTL and markers | 5 minutes; explicit markers | C5 |
| C-Q6 | Usage "not reported" | `Option` in `Usage` | C6 |
| C-Q7 | Compaction threshold and what a new window keeps | about 80% of the window; task, summary and pending messages | C7, C8 |

### Budgets and delegation

| # | Question | Recommendation | From |
|---|---|---|---|
| D-Q1 | Turns stay in the charter as a generous loop guard only | yes | D1 |
| D-Q2 | Spend made real, with notional prices for subscriptions | yes | D2 |
| D-Q3 | Finishing reserve as a charter field with a default; final turn offers only `finish` | yes | D3 |
| D-Q4 | `max_seconds` beside `max_turns` | yes | D5 |
| D-Q5 | Write-lease vocabulary now; parent read-only overlap as the first asynchronous step | yes, the overlap in a later document | D7 |
| D-Q6 | Local `sub_agents` default | `false` until the reserve lands | D9 |
| D-Q7 | Benchmark report erratum for the 256-turn attribution | yes | D12 |

### Shutdown, traces and diagnostics

| # | Question | Recommendation | From |
|---|---|---|---|
| O-Q1 | Pool close: drain or abort; event or predicate | drain, with the owner cancelling; a `Closed` event | O2 |
| O-Q2 | Teardown invariant | in skein's world harness | O3 |
| O-Q3 | When the pool closes | at the answer | O4 |
| O-Q4 | A forced exit after an accepted answer | warning only | O5 |
| O-Q5 | Trace sink | through io, after skein append streams | O7 |
| O-Q6 | Trace prompts | per-turn deltas | O11 |
| O-Q7 | Showing diagnostics | on the terminal in words; exit status per failure class | O13 |
| O-Q8 | Provider failure text on stderr | clipped; full text in the trace only under content capture | O16 |

### Tools and local setup

| # | Question | Recommendation | From |
|---|---|---|---|
| T-Q1 | A charter-level environment overlay | not now | T1 |
| T-Q2 | One directory hides its name; host absolute paths as aliases | yes, yes | T3 |
| T-Q3 | `edit` needs only a unique match, not a prior read | yes; `write` keeps the rule | T4 |
| T-Q4 | Edit format | per dialect, same typed call, decided by measurement | T4 |
| T-Q5 | Model-selectable output budgets | yes, within profile ceilings | T5 |
| T-Q6 | Read window | 32 KiB with paging | T6 |
| T-Q7 | A background-process tool | yes, after process groups | T7 |
| T-Q8 | Process groups before contained trees; per-tree resource limits | yes | T9, T10 |
| T-Q9 | Reading another tool's login file, read-only and access-token-only | yes, opt-in per account | T11, P17 |
| T-Q10 | `localhost` redirects in skein-oauth | yes | T11 |
| T-Q11 | Subscription models unpriced; a named core inherited environment | yes | T12 |

### Packaging and the product

| # | Question | Recommendation | From |
|---|---|---|---|
| P-Q1 | Root package `smith`; step `clippy.toml` moves to `crates/` | yes | P11 |
| P-Q2 | Ship a `smith-agent` binary | keep it until temper decides how its worker runs agents | P11 |
| P-Q3 | Spawning as a product option | library capability now; an `isolate` option later | P10 |
| P-Q4 | The event vocabulary is a public versioned contract; the trace file is the same stream | yes | P13 |
| P-Q5 | Headless exit codes; edits in place without a commit | yes; `--deliver` keeps the change contract | P14 |
| P-Q6 | Settings format and discovery | TOML for people, JSON accepted, `.smith/` discovery | P16 |
| P-Q7 | Build the product on smith's inline agent, answering the one-host draft's open questions | yes | P18 |

### Benchmarks

| # | Question | Recommendation | From |
|---|---|---|---|
| B-Q1 | Harness language | a Rust workspace crate under `benchmarks/` | B §6 |
| B-Q2 | "Vanilla" Codex and Claude Code | defaults plus a pinned config in the repo, in isolated homes | B |
| B-Q3 | Probe models | luna/low and claude-haiku-5-5/low; comparisons at the user's working model | B §5 |
| B-Q4 | Where results live | raw outside the tree; summaries, baselines and per-task variation committed | B |
| B-Q5 | Gate policy | never the merge gate; smoke runs recorded for runtime changes; every fix names its probe | B §4 |
| B-Q6 | The repository task | kept, frozen, rare | B2 |
| B-Q7 | Hidden graders inside the repo | yes, with isolated run locations and seeds that exclude `benchmarks/` | B §7 |
| B-Q8 | Per-attempt AI review | dropped; sampled audits instead | B |
