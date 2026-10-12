# Findings: context growth, prompt caching and token efficiency (theme C)

## Summary

On the repository task, Smith used 12.4 times more fresh input than Codex even though its contexts were smaller. Per-request usage in the raw traces shows why. Without an affinity key, 72% of Smith's cache-eligible requests read zero cached tokens (245 of 341 across the three scored attempts). Nearly every hit that does occur covers a whole earlier request's prompt. That pattern means the prefix is byte-stable and the requests are landing on random cache shards: it is a routing problem, not a prefix problem. With perfect prefix reuse, Smith's median repository attempt would have used about 0.18 M fresh tokens, not 3.5 M. That is less than Codex's 0.26 to 0.28 M.

The spike changed no code here. It ran a 3+3 experiment: a fresh UUID used as both the body `cache_key` and the `session-id` header raised the cached fraction from 25% to 72%. It also documented a static per-endpoint key in `crates/smith/README.md`.

I also found latent problems:
- The Anthropic dialect sends no `cache_control`, so Claude prompts are never cached. Temper did cache them; skein dropped it.
- Cache-write tokens are hard-coded to zero and have no price.
- History is append-only and has no idea of the model's context window. It ends at one of five terminal cliffs: 256 blocks, 256 messages, 1 MiB per request, 32 MiB per session, or the provider's context-too-long error.
- The in-place elision planned in `domain/session.md` §8 would itself break caching, and would break Anthropic's preserved thinking.

The clean fix has six parts:
1. A conversation affinity identity owned by the domain, stable across resume, carried in every prompt, and rendered by skein's dialect. For Codex that is `prompt_cache_key` plus the `session-id` header. This needs per-call headers in skein.
2. A stable-prefix (append-only) invariant written into the contracts and checked by wire-level tests.
3. Fixed, deterministic breakpoints in the Anthropic dialect.
4. Cache writes decoded and priced.
5. Admission that knows the context window, and compaction done as a new window that reuses the cached prefix.
6. Cheap "pointer-chasing" probes that measure cache reuse on every request.

Refs: smith code is cited at `bench/smith-codex-2026-10-08` (spike tip; none of the cited request-path lines changed in the spike). skein code is cited at `main` `e8bc018`; the skein spike branch does not touch `skein-llm`. Codex code is cited from `smith-benchmark-2026-10-08/research/codex-source-0.160.0/` (Codex rust-v0.160.0), plus extra files in `/tmp/smith-benchmark-codex-research/source/`. My analysis scripts are in `findings/scripts/` (`usage_series.py`, `hit_shape.py`, `ideal.py`, `codex_series.py`, `latency.py`).

---

### C1. No affinity identity by default: the root cause of the fresh-input gap

- **Origin:** Report "Remaining gaps" (the cache-affinity experiment). The only spike change is `49057ae`'s `crates/smith/README.md`. The underlying cause is latent on `main`.
- **Symptom:**
  - The report's token table gives repository-task median fresh input as 3,515,963 for Smith and 284,378 for Codex. Cached input is 868,224 for Smith and 7,262,592 for Codex.
  - Per-request series (`scripts/usage_series.py` over `runs/iteration7-07_shell_cwd-smith-1/agent/trace.jsonl`): the root ran 65 requests and its context grew monotonically from 1.3 K to 76 K tokens. Requests were 2 to 4 s apart, yet most of them read 0 cached tokens. The rare hits were large: request 32 cached 37,632 of 41,231, request 59 cached 68,736 of 70,841, and child request 26 cached 40,832 of 41,563.
  - Across all Smith runs (`scripts/hit_shape.py`), the scored attempts without a key had zero-cache rates of 70 to 100% on requests above 1,024 tokens. On the repository task: 83 of 118, 44 of 67 and 118 of 156.
  - Of every hit in every run, all but 3 equal some earlier request's full prompt length to within 256 tokens. The 3 exceptions are cross-run hits between repetitions with an identical early prefix: `iteration2-02_line_decoder-smith-2/3` both read 2,560 at request 6.
  - Codex on the same task (`scripts/codex_series.py` over `iteration7-07_shell_cwd-codex-1/agent/result.json`): 97 responses across 3 threads, none with zero cache. 97.6% of root input was cached, and fresh input per request was about 250 to 2,500 tokens, which is the newly appended suffix.
  - Ideal versus actual (`scripts/ideal.py`). "Ideal" is the fresh input each conversation would use if every request reused its predecessor's prompt; that equals the sum of final context sizes. Smith repository attempts: 180,665 versus 3,515,963 (19.5x); 108,517 versus 1,786,796; 268,215 versus 3,528,161.
  - The experiment (`cache-experiment-results.json`) raised the median cached fraction from 0.25 to 0.72. Its per-request series show 90 to 97% reuse after warm-up, with 3 or 4 full misses per run of about 14 requests. Treatment run 3 used 20,092 fresh against an ideal of 8,608 (2.3x); control run 3 used 34,027 against 6,696 (5.1x).
- **Root cause:**
  - Smith sends neither an affinity key nor a session header unless the operator configures one:
    - `crates/smith-protocol-llm/src/prompt.rs:133-142` sets `cache_key: None`.
    - `component.rs:284-285` and `endpoints.rs:125-137` copy only a static per-endpoint `cache_key`.
    - skein writes `prompt_cache_key` only when it is `Some` (`skein-llm/src/openai/request.rs:132-135`).
    - skein's automatic Codex headers are only `chatgpt-account-id` (`skein-llm/src/client.rs:993-1001`). Extra headers can only be static per endpoint (`client.rs:886-990`); there are no per-call headers.
  - Codex sets a key on every request, `Some(self.prompt_cache_key(..))` at `core/src/client.rs:976`, and the key is the session id (`client.rs:575-587`). The root sends the `session-id` header with that value, with the comment "ChatGPT derives cache affinity from the Responses session-id header" (`client.rs:589-597`). It also sends a `thread-id` header (`codex-api/src/requests/headers.rs:5-13`).
  - The session id is the root thread id. It is restored on resume, and spawned children inherit it through the controller identity (`core/src/session/session.rs:880-916`; the comment at line 909 says "session_id is equal to the root thread's ID"). The `agent_control` source is not in the snapshot, so I have not verified the child inheritance beyond that comment.
  - The scored Smith settings carry no `cache_key` and no headers (checked `runs/iteration7-07_shell_cwd-smith-1/agent/settings.json`). The experiment's treatment settings carry both.
  - Note that the public-API guidance quoted in `cache-affinity-analysis.json`, "GPT-5.6+ routing is automatic, key optional", does not transfer to the ChatGPT subscription route. The measured effect is large.
- **Spike change:** None in code. `49057ae` adds a README recipe to `crates/smith/README.md`: put one opaque UUID in the endpoint's `cache_key` and in a generic extra header `session-id`, generated "once for the chat", preserved across turns and resumes. The report calls this an optional configuration.
- **Assessment:** Redo properly.
  - The recipe works for one process and one run.
  - It is a constant in static configuration. Every run and chat that shares a settings file shares one key forever, and the README asks users to manage a per-chat identity by hand.
  - It routes through a generic header mechanism, so nothing stops it from colliding with a dialect-owned header later.
  - It is not resume-aware (the transcript does not record it) and not testable as a domain property.
  - "Optional" is the wrong default. The measured effect is a 2 to 20 times change in fresh tokens, and nothing in the request needs to change.
- **Clean fix:** Make affinity a domain-level identity that is stable for a conversation's life. Each dialect renders it.
  1. **Identity source.** The host supplies a bounded, opaque run identity in the start. It is stable across activations and resumes; temper and the local host already own run and chat identity. When the host supplies none, the run domain mints one from the Env RNG at admission. That is deterministic in simulation and random in production.
     - Options considered:
       - (a) host-supplied, with minted as the fallback. Recommended.
       - (b) minted per activation. Simplest, but a resume starts with a cold key.
       - (c) a hash of the conversation's first turn. Stable with no new state, but identical runs, such as benchmark repetitions, share a key.
     - Never derive it from credentials or paths; `cache-affinity-analysis.json` also warns about this.
  2. **Granularity.** Mirror native Codex for the Codex dialect: the body `prompt_cache_key` and the `session-id` header carry the run (tree) identity, and a per-conversation `thread-id` carries the conversation ordinal.
     - The alternative is a per-conversation key, which is the analysis file's `smith:v1:<startup>:<owner>` proposal. It isolates conversations but diverges from what the backend is tuned for.
     - Either is a fixed-size value, for example `[u8; 16]` plus a `u32`. It needs no allocation and fits the strict subset.
  3. **Domain.** The run passes the identity into each session `Spec`. Children derive theirs from the run identity and their ordinal. The session puts it unchanged into every `Prompt`. It never changes within a conversation.
  4. **Transcript.** It is recorded once per transcript, so a resume reuses it (as Codex does at `session.rs:880-916`).
  5. **skein-llm.** Replace `Prompt.cache_key: Option<Box<[u8]>>` with a neutral `affinity` that the dialect renders:
     - Codex: body `prompt_cache_key`, plus `session-id` and `thread-id` as per-call headers. `client.rs` `headers()` needs a per-call header source derived from the prompt, and `session-id` and `thread-id` join `reserved()` (`client.rs:1048-1066`) so endpoint configuration cannot duplicate them.
     - Anthropic: accept it with no wire effect. Today `anthropic/request.rs:35-37` rejects `Some(cache_key)` as `Unsupported`, which would break any default.
     - Future OpenAI API-key dialect: `prompt_cache_key`, and optionally `prompt_cache_retention` (unverified for current models).
  6. **Remove** the static endpoint `cache_key` option and the README recipe. At most, keep a diagnostic override.
- **Contracts:**
  - `domain/run.md` §3.2: the start carries the run's identity. §5.3: children derive theirs.
  - `domain/session.md` §2 (a conversation's identity, fixed for its life) and §3 (kept in its transcript).
  - `protocol/transcript.md` §3 and §6: the identity is in the transcript, with a version bump.
  - `protocol/llm.md` §2: the "endpoint's options … a cache key" bullet becomes "the conversation's affinity, rendered by the dialect".
  - `protocol/agent.md` §4: drop "a cache key".
  - skein `docs/design/llm.md`: the "Vocabulary and ownership" paragraph and the Codex/Anthropic request paragraph (around line 140), which today says Anthropic's "cache-affinity key is unsupported".
  - The spike edited none of these.
- **Layer:**
  - skein-llm first, per AGENTS.md ("a missing shared mechanism belongs in skein before its smith consumer"): neutral affinity, dialect rendering, per-call headers, reserved names.
  - Then `smith-domain-run` (mint and derive), `smith-domain-session` (carry), `smith-transcript` (persist), `smith-protocol-llm` (map; drop `EndpointOptions.cache_key`), and `crates/smith/src/config.rs` (drop the field).
- **Tests:**
  - The spike added none.
  - skein focused tests:
    - The Codex encoding carries the key in the body and in both headers. No affinity means neither.
    - An endpoint header named `session-id` is refused.
    - Anthropic accepts affinity with byte-identical output.
    - skein's fake peer records per-request headers, so a fake-LLM world can assert that one conversation keeps the same affinity.
  - smith session world (referee invariant): every `Complete` for an owner carries the same affinity; distinct conversations carry distinct ones; a resumed session carries the transcript's.
  - Protocol world: the affinity reaches the wire for each dialect.
  - Memory driver: the fixed affinity bytes are in the worst case.
  - All of these run in milliseconds, well inside the 15 s and 60 s budgets.
- **Benchmark:** Measured by the experiment, only for the combined treatment, with one conversation and 3+3 runs. Future probe: P1 in C10.
- **Questions:**
  1. Run-tree key with a per-conversation `thread-id` (recommended), or a key per conversation?
  2. Should the host supply the run identity in the start (recommended, with a minted fallback), or should the agent always mint it?
  3. Remove the static endpoint `cache_key` and the README recipe (recommended), or keep them as a diagnostic override?

### C2. Residual misses after affinity: sticky turn routing and incremental transport

- **Origin:** Not in the spike. This is what remains after C1.
- **Symptom:** With key and header, each treatment run still had 3 or 4 requests in about 14 with zero cache. Examples: `diagnostics-cache-session-03_planner-smith-1` request 7 (0 of 5,024) and `-smith-2` request 8 (0 of 6,411). Treatment fresh input was still 2.3 times the ideal. Codex had 0 full misses in 97 responses on a much larger task.
- **Root cause (partly unverified):** Codex adds three routing mechanisms that Smith lacks.
  - The `x-codex-turn-state` sticky-routing token. The server returns it on the first response of a turn, and the client replays it for every request in that turn (`core/src/client.rs:163`, `:284-305`, `:2316-2336`).
  - Responses over WebSocket. Once a connection holds the previous response, Codex sends only the new input items with `previous_response_id` (`client.rs:14-18`, `:1380-1446`). The default OpenAI provider has `supports_websockets: true` (`/tmp/smith-benchmark-codex-research/source-0.160.0/model-provider-info/src/lib.rs:555`).
  - The `thread-id` header.

  Which of these closes the gap is unknown. I could not determine whether the scored Codex runs used WebSocket, because the native runs did not capture the wire.
- **Spike change:** None.
- **Assessment:** Do not build yet; measure first.
  - The turn-state token is a private ChatGPT-backend contract. It belongs in the Codex dialect if anywhere. Because it is per-call state returned by the response, the dialect would have to hand an opaque routing token back to the caller to replay. That is a conversation-level counterpart of `Replay`.
  - WebSocket incremental transport is a large skein change: a WebSocket client, connection affinity per conversation, and a full-resend fallback on reconnect. It would also decouple request size from history size, which matters to the request-envelope theme.
- **Clean fix:** Run probe P1 with three arms first: key plus `session-id`; plus `thread-id`; plus turn-state. If the turn-state arm closes the gap, add an opaque "routing state" that the dialect returns with the completion and the session carries until the next user message. That mirrors Codex's per-turn scope (`client.rs:288-290`: never replayed across turns). Treat the WebSocket transport as a separate long-term skein design.
- **Contracts:** skein `llm.md`, if routing state is adopted. `domain/session.md` §3, so the routing state is not part of the transcript, since it is per turn.
- **Layer:** skein-llm (dialect, client), plus a thin session carry.
- **Tests:** If adopted: the dialect returns the token from a scripted response header and replays it within a turn but never after a new user message; the fake peer asserts this.
- **Benchmark:** P1 arms; the metric is the full-miss count.
- **Questions:** Adopt private ChatGPT routing extras such as turn-state (recommendation: only if P1 shows a material gain)? Put WebSocket incremental transport on skein's roadmap (recommendation: yes, as a later item tied to the request envelope)?

### C3. The request prefix is stable in practice, but nothing guarantees it

- **Origin:** Not in the spike. The prefix analysis is `cache-prefix-analysis.json` and `cache-prefix-measurements.json`.
- **Symptom:**
  - All 123 adjacent captured prompt projections keep their predecessor as an exact byte prefix, with an unchanged system hash.
  - The projections omit tool definitions and rendered owned results, so they are only partial evidence. The hit-shape analysis in C1 closes that gap: hits of 90 to 98% covering a full earlier request can only happen if the wire bytes, including encrypted reasoning items and rendered tool output, were identical.
  - The candidate causes listed in the brief are all absent today: timestamps, budget text, tool-order changes, reasoning dropped or re-encoded, history rewritten.
- **Root cause (why it holds):**
  - The system text is computed once at run admission and child open, and contains no clock or budget values (`crates/smith-domain-run/src/prompt.rs:27-53`; the spike only added static text).
  - Tool schemas are fixed literals in a fixed order (`crates/smith-protocol-llm/src/tools.rs:18-95`).
  - The transcript is an append-only `List<Message>` (`crates/smith-domain-session/src/session.rs:156-186`), copied whole at emission (`session.rs:1666-1688`).
  - Rendering is a pure function of typed outcomes (`render.rs:16-29`).
  - Reasoning and replay go back verbatim (`skein-llm/src/translate.rs:270-285`; `openai/request.rs:143-145`).
  - The JSON writer emits fixed key order (`openai/request.rs:99-142`).
  - `max_output_tokens` varies per request (`session.rs:1667-1668`), but Codex omits it and Anthropic does not put it in the cache prefix.

  None of this is stated as a contract. `protocol/llm.md` §2 says only that "caching follows from the order".
- **Risks:**
  - The planned context management, "old tool output is elided first" (`domain/session.md` §8), rewrites history. See C8.
  - Per-turn reminders injected into earlier messages.
  - Any future dynamic system text, such as remaining time or budget, which the report's "clearer remaining-time guidance for delegation" invites.
  - Non-deterministic serialization in new code.
- **Spike change:** None.
- **Assessment:** Write the existing behaviour into the contracts and guard it with tests.
- **Clean fix:**
  - **Invariant:** within a conversation, the model, endpoint options, affinity, system text and offered tools are fixed. History is append-only. The prompt for completion n+1 equals the prompt for n, followed by completion n's blocks and what was appended after them. Dynamic guidance (remaining time or turns) goes in appended messages, never in the system text or tool descriptions. The only exception is an explicit window boundary (C8).
  - Translation and rendering are byte-deterministic.
  - Optionally, order the system text from stable to volatile (shared guides and mechanics before the child brief, which `render_child_prefix` puts first at `prompt.rs:83-89`). That only pays off if siblings' shared prefix exceeds the provider minimum (1,024 tokens on OpenAI, as the cached amounts show), so it is low priority today.
- **Contracts:** `domain/session.md` §3 (append-only and fixed per conversation); `protocol/llm.md` §2 (determinism and the "dynamic guidance is appended" rule) and §5; `domain/run.md` §3.3 (the system text is immutable for a session's life). skein `llm.md`: dialect encoding is deterministic.
- **Layer:** Contract text in smith; a determinism note in skein.
- **Tests:**
  - Focused protocol-world test: drive four or more completions through the real component, skein client and scripted peer. Capture the raw request bodies and assert, for each adjacent pair: byte-identical `instructions` and `tools`; and the input items of request k are a byte prefix of request k+1's, followed by k's replayed outputs.
  - Session-world referee invariant: each owner's prompt k+1 extends prompt k.
  - The existing `tests/protocol-llm/tests/brief.rs:376-420` (`native_accounting`) already checks decoded-history prefixes against a fake peer that models prefix caching. Extend that rather than adding a new harness.
  - These are deterministic, run in milliseconds, and are the regression net for C1. The live probe then only has to catch routing.
- **Benchmark:** Covered by the deterministic tests. P1 shows the effect live.
- **Questions:** None.

### C4. Reasoning items across user messages (Codex's `reasoning.context`)

- **Origin:** Not in the spike. Latent.
- **Symptom:** Not measured. The benchmark tasks have one user turn, and nudges are rare.
- **Root cause (inferred from Codex source):** Smith sends `reasoning: {effort}` with no `context` field (`openai/request.rs:125-131`). Codex's comment says that when `context` is omitted "Responses uses the default, which is currently `current_turn`" (`core/src/client.rs:874-880`). Codex sends `all_turns` only with Responses Lite. The smoke capture `provider/codex-smoke-002/001.request.json` shows `"reasoning": {"effort": "low", "context": "all_turns"}`. If `current_turn` drops reasoning items that come before the latest user message, then every new user message would change the rendered prompt from the first earlier reasoning item onward. That would be one large cache miss per user turn. User messages arrive in interactive chats (host messages after `wait`), with run nudges (`crates/smith-domain-run/src/run.rs:586-587`), and on continuations. Anthropic's older models behave the same way; newer Claude models keep earlier thinking (per Anthropic's prompt-caching guidance). I have not verified this server behaviour.
- **Spike change:** None.
- **Assessment:** Probe before changing anything.
- **Clean fix:** If P4 confirms the miss, make the reasoning context a dialect capability chosen per model or route, never a domain policy. Send `all_turns` where the backend accepts it outside Responses Lite. The domain contract does not change: history keeps every reasoning block.
- **Contracts:** skein `llm.md` (Codex request fields).
- **Layer:** skein-llm (Codex dialect).
- **Tests:** Dialect encoding of the option. Replay tests already cover verbatim reasoning.
- **Benchmark:** P4 (C10).
- **Questions:** None until P4 has run.

### C5. The Anthropic path has no prompt caching at all

- **Origin:** Not in the spike. It is a regression from temper.
- **Symptom:** `skein-llm/src/anthropic/request.rs:227-277` writes `system` as a plain string, `tools` and `messages`, and never `cache_control`. Anthropic caches only when it is asked to, through block-level `cache_control` breakpoints or top-level automatic caching. So every Claude request is billed and prefilled cold. The usage decoder already maps `cache_read_input_tokens` and `cache_creation_input_tokens` (`anthropic/response.rs:562-563`), so this would be visible as zeros.
  - Temper's legacy dialect placed markers on the last system block, the last tool, and the last block of each of the last two user messages, with a 1 h TTL (`temper` `6621b6b8^:crates/temper-legacy-llm-anthropic/src/request.rs:107-200`; `identity.rs:11`).
  - skein's migration note marks this "deliberately unsupported … no equivalent cache optimization is claimed" (`skein/docs/design/fake-llm.md:220`).
  - `protocol/llm.md` §2 and §11 still promise "the dialect's cache markers go where skein's client puts them", which is false today.
- **Root cause:** Dropped in the move to skein's neutral `Prompt`, which had no caching vocabulary.
- **Spike change:** None.
- **Assessment:** Redo properly in skein. This is a prerequisite for running Smith on Claude at reasonable cost.
- **Clean fix:** Place breakpoints deterministically inside the Anthropic dialect. They are a pure function of the prompt's shape, so the domain needs no involvement.
  - One breakpoint on the last system block. Render `system` as a block array; Anthropic renders tools then system, so this caches both.
  - One tail breakpoint on the last eligible block of the last message.
  - Never more than 4 breakpoints.
  - Anthropic's guidance recommends exactly this pair for agent loops.
  - Smith's turns add about 3 or 4 lookback positions each (a run of consecutive `tool_use` blocks counts as one position, as does a run of `tool_result` blocks), so the 20-block lookback is safe. Add an intermediate marker only if a turn could add more than about 15 positions.
  - TTL: default 5 minutes.
    - Synchronous delegation leaves the parent idle while a child runs. In the benchmark the root waited 224 s (`iteration7-...-smith-1` root, requests 10 to 11).
    - A child longer than 5 minutes expires the parent's tail entry.
    - Options: always 5 minutes and accept the miss (recommended until P3 measures it); a 1 h TTL only on the system breakpoint (cheap); or a keep-alive. The TTL must be chosen when an entry is written, before the response shows that a child was spawned.
  - Minimum cacheable length is 512 to 4,096 tokens depending on the model. Smith's static prefix (about 600 to 1,300 tokens) caches only on newer models; the tail breakpoint carries the value.
  - The dialect must accept the neutral affinity from C1 and do nothing with it.
  - Preserved thinking on current Claude models requires append-only history: edits invalidate later thinking blocks, and accounts created on or after 2026-08-31 get a 400. That reinforces C3 and C8.
  - Whether top-level automatic caching is accepted on the OAuth/Claude-Code route is unverified, so prefer explicit markers.
  - Mid-conversation system messages exist on new models and are the cache-safe channel for dynamic guidance. That is a later dialect capability.
- **Contracts:**
  - skein `docs/design/llm.md`: the Anthropic request paragraph gets marker placement and TTL.
  - skein `docs/design/fake-llm.md:220`: reverse the disposition.
  - `protocol/llm.md` §2 and §11 become true and specific.
  - `domain/session.md` §6 links to C6.
- **Layer:** skein-llm (Anthropic dialect), plus the fake peer's accounting.
- **Tests:**
  - skein request-encoding tests: markers at exactly these positions, never on thinking blocks, at most 4, positions stable across a growing conversation.
  - The fake Anthropic peer models cache reads up to the last marker, so smith worlds see nonzero `cache_read`.
  - Extend smith's `native_accounting`-style test to Anthropic.
- **Benchmark:** P5 (C10): the same pointer-chasing probe on a small Claude model.
- **Questions:** Default TTL (recommendation: 5 minutes everywhere until P3 data exists)? Use top-level automatic caching where available (recommendation: no, keep explicit markers)?

### C6. Cache accounting: writes are hard-coded to zero and have no price

- **Origin:** Not in the spike. The gap is recorded in `cache-affinity-analysis.json` ("Smith provider cache writes are not decoded") and in the report ("Cache writes are unavailable because Smith's decoder supplies zero").
- **Symptom:**
  - `skein-llm/src/openai/response.rs:524-538` sets `cache_write_tokens: 0` unconditionally. The ChatGPT responses carry `input_tokens_details.cache_write_tokens`: see `provider/smith-smoke/001.metrics.json` (value 0 there; native rollouts also report 0).
  - Smith's price model covers input, cached and output only. `domain/session.md` §6 says "New input and cache writes use the input rate", and the charter `Prices` have `input`, `cached`, `output` and `unit`. Anthropic bills writes at 1.25 times input (5 min TTL) or 2 times (1 h TTL). So once C5 lands, spend and reservations under-count on Claude.
  - Zero and "not reported" cannot be told apart.
- **Root cause:** The decoder was written before write counts existed; the price model is a two-rate simplification.
- **Spike change:** None.
- **Assessment:** Redo properly. Small, but it underpins every cache measurement and budget.
- **Clean fix:**
  - skein decodes `cache_write_tokens` when present and records "not reported" distinctly: either `Option` in `Usage`, or a per-dialect capability flag.
  - The charter `Prices` gain a cache-write rate, defaulting to the input rate for compatibility, with checked rational arithmetic as §6 already requires.
  - The session's `cache_write` budget dimension then means something.
- **Contracts:** `domain/session.md` §6; `protocol/charter.md` (prices); `protocol/transcript.md` §2 (usage fields); skein `llm.md` (`Usage`).
- **Layer:** skein-llm (decoder), `smith-charter`, `smith-domain-session` (pricing).
- **Tests:** Decoder focused tests (present, absent, zero); a pricing test with a write rate at the `u64` boundary; and the existing `native_accounting` with `cache_write: true`.
- **Benchmark:** Probes report the write count when the provider gives one.
- **Questions:** Model "not reported" as `Option<u64>` in `Usage` (recommended) or as a dialect capability?

### C7. History growth: append-only, blind to the window, and ending at one of several cliffs

- **Origin:** Not in the spike. The spike's response was the 1 MiB envelope (`cfb7408`, owned by another analyst).
- **Symptom:**
  - Every completion appends one assistant message holding its reasoning, text and tool calls. Reasoning is an opaque encrypted item, at most 8 KiB raw (`crates/smith/src/config.rs:336`); the largest observed was about 4.6 KB. Tool calls keep their full input, including whole-file `write` contents up to 32 KiB.
  - Every batch appends one user message of rendered results. The domain already bounds these at the source:
    - `read`: 4 KiB windows (`crates/smith/src/limits.rs:90`).
    - `shell`: 2 KiB head plus 4 KiB tail (`config.rs:369-370`).
    - `search`: 16 hits.
    - `list`: 64 entries or 4 KiB.
  - Nothing is ever dropped, summarized or elided. Measured final contexts: root about 76 K tokens; children up to about 63 K; Codex root 125 K.
  - As history approaches the model's window, the session does not know it. It ends at whichever cliff comes first, and every cliff is terminal:
    - (a) More than 256 blocks across the prompt fails with `Limit`. Checked in `crates/smith-protocol-llm/src/prompt.rs:328-338`, `skein-llm/src/translate.rs:70-75`, and `openai/request.rs:50`; `parts` is set at `config.rs:334`. A read-heavy child reached 169 blocks in 22 turns (`iteration3-07_shell_cwd-smith-1`, owner 4294967296), so this cliff arrives well before 1 MiB in tool-heavy conversations.
    - (b) 256 messages ends the session as `TranscriptFull` (`config.rs:368`; `boundary.rs:338-339`).
    - (c) More than 1 MiB of request bytes fails with `Limit`.
    - (d) 32 MiB of session bytes (`limits.rs:66`).
    - (e) The provider's `ContextTooLong`, which is never retried (`session.rs:1752-1762`) and fails the session.
  - None of these is expressed in tokens or tied to the model. 1 MiB of JSON is roughly the size of a GPT-class 272 K window, but far below a 1 M Claude window.
- **Root cause:** `domain/session.md` §8 defers context management ("Later"). The limits are independent byte and count constants.
- **Spike change:** It raised the envelope from 256 KiB to 1 MiB, which moves cliff (c).
- **Assessment:** The per-result bounds at the source are sound and should stay. They are Smith's version of Codex's tool-output truncation policy (`TruncationPolicyConfig::bytes(10_000)` fallback, `/tmp/smith-benchmark-codex-research/source/models-manager/src/model_info.rs:127`). The cliffs need a token-aware policy in front of them. Raising constants until the symptom disappears only moves the cliff.
- **Clean fix: admission that knows the context window.**
  - Each model entry declares its usable input window in tokens. The host chooses models, so the charter carries it; the agent configuration enforces a ceiling. Codex uses a 272 K fallback at 95% usable (`model_info.rs:129-133`).
  - The session tracks the last prompt's exact size from usage (input + cache read + cache write) and an upper bound on what it has appended since. Bytes are a safe upper bound on tokens, and it is integer arithmetic.
  - Before each completion it checks: last prompt + last output + appended bound + the output reservation ≤ window × threshold. If not, it compacts (C8) instead of calling.
  - `ContextTooLong` becomes "compact once, then fail if it recurs".
  - Derive the block, message and byte limits from the window with headroom, so they are no longer independent cliffs, and make the 256-block `parts` limit a per-request admission that compaction relieves rather than a hard end.
  - Child conversations stay the primary isolation tool. A child's reading never enters the parent; only its final message does (`domain/session.md` §7).
- **Contracts:**
  - `domain/session.md` §6 adds "context" as a dimension, separate from cumulative token budgets; §8 is replaced by C8.
  - `protocol/charter.md`: the model's window.
  - `domain/run.md` §14 (limits).
  - `protocol/llm.md` §8: limits derived from the window.
- **Layer:** `smith-domain-session` (admission); charter and config; the protocol only reports usage, as today.
- **Tests:**
  - Session world: scripted fake usage pushes a conversation across the threshold. It must compact, never send a request above the bound, and fail cleanly on a repeated `ContextTooLong`.
  - Fuzzy: random usage and append sizes never produce a request whose bound exceeds the window.
  - Memory driver: compaction reclaims old transcript bytes.
  - Keep worlds small enough for the 60 s fuzzy budget by using small windows, such as 8 K tokens.
- **Benchmark:** P6 (C10): a long-horizon probe with a deliberately small window.
- **Questions:** Should the window come from the charter (host) (recommended) or from an agent-side model table? What compaction threshold (recommendation: about 80% of the usable window, mirroring Codex's configurable limit; `config_toml.rs:176-189`)?

### C8. Compaction policy: start a new window, do not elide in place

- **Origin:** Not in the spike. The contract placeholder is `domain/session.md` §8 and §11 and `protocol/transcript.md` §11 ("Summaries").
- **Symptom:** No policy exists. The planned order in §8 is "old tool output is elided first; a summarising session, requested from the opener like a sub-agent, comes after".
- **Root cause / analysis:**
  - Eliding old tool output in place is a history edit. Each elision misses the cache from the first edited block onward, which re-bills the whole tail fresh. On Claude models with preserved thinking, it also invalidates every later thinking block (a 400 for newer accounts).
  - A summarizing session opened by the opener has a different system and tools prefix, so it prefills the whole history uncached. Anthropic's agent guidance says "fork operations must reuse the parent's exact prefix".
  - Codex compacts in-thread into a new "window" (`window_id`, `window_number`, `context_window_id`; `core/src/session/session.rs:665-725`; strategy "Memento", `responses_metadata.rs:116-145`). It does this locally or remotely through `/responses`, triggered by `model_auto_compact_token_limit`, an optional post-turn percentage threshold, and a custom `compact_prompt` (`config_toml.rs:176-189`, `:272-273`).
  - The compaction implementation itself is not in the snapshot, so the details of summary content and kept messages are unverified.
  - Native Codex did not compact in the benchmark (`native07-usage-reconciliation.json`: `compaction_records: 0`), so compaction played no part in the measured gap.
- **Spike change:** None.
- **Assessment:** Replace §8's ordering with a design that keeps the cache intact.
- **Clean fix: compaction as a window boundary inside the session.**
  1. When C7's admission triggers, the session issues a compaction completion. It uses the same model, system, tools and full history, with one appended user message holding the run's summary instruction. Because only the end is new, the request reads the cached prefix.
  2. The answer becomes the new window's opening: the original task message, then the summary, then any host messages still pending.
  3. The window boundary is told to the opener as a turn of its own kind, so transcripts and resume carry only the current window.
  4. The old transcript is reclaimed.
  5. The cache resets once per window, which amortizes the cost.

  Supporting rules:
  - The run supplies the summary instruction text at open, since the run owns prompts (`domain/run.md` §3.3). The session owns when it fires.
  - A compaction counts as a turn and as spend.
  - Provider-native compaction can become a later dialect optimization where its opaque compaction items replay verbatim through `Replay`: Anthropic's server-side compaction (beta) and context editing, and OpenAI's remote compaction as Codex uses it.
  - In-place elision is dropped. If it is kept at all, it should only happen at a window boundary.
  - Source-side result bounds (C7) and delegating to children remain the first lines of defence.
- **Contracts:**
  - `domain/session.md` §8 is rewritten and §11's open question closed. §3 gains the window-boundary exception to the append-only rule (C3).
  - `protocol/transcript.md` §2 and §11: a compaction turn kind; resume from the last window.
  - `domain/run.md` §3.3: the summary instruction; §9: compaction spend.
- **Layer:** `smith-domain-session` (state, a new `Compacting` transition), `smith-domain-run` (instruction text), `smith-transcript` (turn kind). skein only for later provider-native options.
- **Tests:**
  - Session world stories: compact at the threshold; compact after `ContextTooLong`; resume from a compacted transcript; tool calls in a compaction answer treated as not run; budget crossed during compaction.
  - Referee: the prompt stays append-only within each window; one window boundary per compaction.
  - Memory driver: transcript bytes drop after the boundary.
- **Benchmark:** P6 (C10). Measure task success, the number of compactions, and cache reuse on the compaction request and after it.
- **Questions:**
  1. Approve "compaction as a new window inside the session" and drop elision (recommended)?
  2. Keep the pending host messages and the last k results in the new window? Recommendation: the task, the summary and pending host messages only, and measure.

### C9. What fixing caching will and will not buy

- **Origin:** Not in the spike. Analysis.
- **Symptom / evidence:**
  - **Latency.** I fitted latency against fresh, cached and output tokens over 433 repository completions (`scripts/latency.py`, a rough least-squares fit; some prompt records are missing). The fit gives about 0.04 s per 1 K input tokens whether cached or not, and about 20 s per 1 K output tokens. The experiment's wall time moved only from 59.8 s to 57.7 s. At these context sizes, caching is a token, cost and rate-limit issue. It does not explain the 997 s versus 413 s wall-time gap.
  - **Small prompts.** Smith's static prompt is tiny (262 instruction and 304 tool tokens per the attribution in `provider/smith-smoke/001.metrics.json`). Root openings are about 1.3 K tokens and child openings about 0.7 K. The provider caches nothing below 1,024 tokens and caches in 128-token steps (observed cached amounts: 1024, 1280, 2304 …). So the first 2 to 4 requests of every conversation are always uncached. Codex's first request had 12 K cached from its shared base prompt, even under a fresh key. This costs only a few K tokens per conversation; do not pad prompts.
  - **Turn count.** Turns multiply re-sent context. Smith used about 1.3 times the responses and 2 times the tool calls of Codex. Small read windows (4 KiB) force more turns. With caching fixed, each extra turn costs only its appended tokens plus the latency of a round trip.
- **Assessment:** Prioritize C1 and C3 first, then C5 and C6, then C7 and C8. Leave turn-count reduction to the tools and delegation themes.
- **Clean fix / Contracts / Layer / Tests:** None beyond the other entries.
- **Benchmark:** P1 reports latency next to cache reuse, so a latency claim is never made from token data.
- **Questions:** None.

### C10. Cheap probes for a future `./benchmarks`

- **Origin:** Not in the spike. The spike's experiment used a full coding task (planner), whose plans and turn counts vary between runs, and that confounded its results.
- **Design principle:** Make the task shape deterministic, so that the request count and the per-turn additions are fixed by construction and plan variance disappears. Then measure reuse per request rather than in aggregate.
  - **"Pointer chasing":** the workspace holds about 12 files of about 2 KB each. Each file names the next one and contains filler text. The brief says: read the first file, follow the pointers one read at a time, then call `finish` with the final file's token. Use a small, cheap model (`gpt-6-luna`, low effort; or `claude-haiku-5-5`).
  - **Metrics from the existing trace** (per-completion usage records with an owner; do not rely on prompt records, which are dropped above 64 KiB):
    - Reuse R = Σ cache_read_i / Σ total_{i−1}, over requests whose predecessor is at least 1,152 tokens.
    - Full misses M = the number of requests with cache_read_i < 0.5 × total_{i−1}.
    - Excess fresh = Σ fresh_i − Σ max(total_i − total_{i−1}, 0).
    - Wall time and output tokens are reported next to these.
  - **Stability:**
    - 3 repetitions; use the median R.
    - Pass if median R ≥ 0.85 and M ≤ 1 per 10 eligible requests.
    - Compare against a trailing baseline and alert on a drop of more than 0.15.
    - From time to time, run an "affinity off" control arm. It should give R around 0.2 to 0.4 (the no-key runs above), which proves the probe still discriminates.
    - The deterministic tests in C3 catch prefix regressions with no network. This probe catches routing and provider changes.
- **Probes:**
  - **P1 (affinity, C1/C2):** pointer chasing, one conversation. Arms: key + `session-id`; + `thread-id`; + turn-state (C2). About 14 requests per run.
  - **P2 (children):** the parent delegates two sequential pointer-chasing children. Each conversation must reach R ≥ 0.85, and each must keep a distinct `thread-id` (if adopted) under the shared run key.
  - **P3 (idle gap):** the parent waits for a host message delivered after 4 and then 8 minutes, through the local host's `wait`. Measure the first request after the gap. This sets the TTL decision in C5 and quantifies cache expiry during delegation.
  - **P4 (user-turn boundary, C4):** at medium effort, so reasoning items exist: yield, continue with a host message, then do 5 more pointer reads. Measure reuse on the first request after the user message. Repeat with `reasoning.context` set to `all_turns` if the backend accepts it.
  - **P5 (Anthropic, C5/C6):** P1 on a small Claude model. Assert that `cache_read` covers the previous prompt and that `cache_write` is about the size of each turn's addition.
  - **P6 (compaction, C7/C8):** a charter window of about 32 K tokens with about 80 K of total reading. Assert success, compactions ≥ 1, and that no request exceeds the window. Report R on the compaction request and in the window after it.
- **Cost:** P1, P2, P4 and P5 run in seconds to a minute each on small models. P3 is gated by wall time, so run it nightly.
- **Questions:** Should these probes live in `./benchmarks` and run on demand, with only P1 running before every merge that touches `skein-llm` or `smith-protocol-llm`? Recommendation: yes.

---

## Cross-theme dependencies

- **Request envelope (1 MiB, `cfb7408`):** C7's window-aware admission and C8's compaction change what the envelope must hold. The 256-block `parts` limit is a separate cliff that arrives first in tool-heavy conversations. WebSocket incremental transport (C2) would decouple request size from history size.
- **Delegation, sub-agents and turn budgets (`44e5120`, `5848e59`):** synchronous delegation leaves parents idle long enough to risk cache expiry (C5 TTL, P3). Children are the main context-isolation tool (C7). Compaction completions consume turns and spend (C8). The child system text puts the brief first (C3 ordering note).
- **Tools theme:** per-result bounds (4 KiB read windows, shell head and tail) set both context growth and turn count (C7, C9).
- **Interactive and waiting (spike 1, `6136dc4`/`8e83931`):** host messages and nudges are new user messages. They are the trigger for C4's possible per-turn miss, and the idle times between them exceed cache retention.
- **Trace and diagnostics:** the probes depend on per-completion usage records keeping their owner and timing. C6 changes the `Usage` shape that the trace and transcript record.
- **skein ownership:** neutral affinity, per-call headers, reserved header names (C1), Anthropic breakpoints (C5) and cache-write decoding (C6) all land in `skein-llm` before smith consumes them. They should be coordinated with the skein shutdown branch's owner of `skein-llm-connection`.