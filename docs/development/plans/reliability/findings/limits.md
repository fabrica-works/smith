# Findings: limits, sizing and timeouts of the LLM path (theme S)

Refs: smith at the spike tip `bench/smith-codex-2026-10-08` (`bf96e94`) unless noted; skein at `main` `e8bc018` (the skein spike branch only touches llm-connection shutdown, not limits or decoders). "My count" or "my arithmetic" marks numbers I derived from artifacts or formulas without running code.

## Summary

- **What went wrong.** smith's only configuration, the `standard` profile, had test-sized LLM limits on main: 8 KiB requests, 2 KiB tool arguments, 1,024 JSON tokens per provider event, 16 "parts", 2 KiB reasoning items and a 60 s absolute call cap. Real Codex traffic exceeded all of them. Every overflow killed the session: `Limit` is never retried, and `TimedOut` was retried into the same failure.
- **What the spike did.** In five rounds (`6136dc4`, `8e83931`, `ee512a1`, `cfb7408`, `d75190b`) it raised constants in one hand-written function (`standard_limits`) until each symptom stopped:
  - requests 8 KiB → 1 MiB;
  - tool arguments 2 → 32 KiB;
  - event tokens 1,024 → 4,096;
  - parts 16 → 256;
  - SSE event and document 8 → 512 KiB;
  - reasoning items 2 → 8 KiB;
  - call timeout 60 → 300 s, with head and idle tied to it.
- **Relationships are implicit, and two are already broken.**
  - 256 completion blocks against the transcript's 16 blocks per message.
  - 2 connections per endpoint against 4 parallel sub-agents.
- **Only the OpenAI path was fixed.** Anthropic keeps a 4,096-token wire output cap and an 8 KiB thinking cap.
- **The next cliffs are near.**
  - The event-token cap grows with history; the best root conversation was about 10% under it.
  - `parts` caps history length.
  - The 8 KiB reasoning cap sits at 1.6× the largest observed item.
  - The ~360 GB worst-case memory figure, against a configured 1 TiB, protects nothing.
- **The clean shape:**
  1. skein decodes events selectively and as a stream; an oversized call becomes a per-call outcome.
  2. A few declared quantities per model or endpoint (context window, max output, max tool payload, max reasoning item, concurrency) derive every byte cap with checked arithmetic, and startup checks each relationship.
  3. The session knows the request cost of its transcript and manages context before the cap.
  4. Deadlines measure progress (head, idle); the absolute bound is the session's remaining time.
  5. Memory is accounted as aggregate budgets with admission to a shared pool, so the bound is a real number.

---

### S1. Where each limit lives, and the relationships nobody checks

- **Origin:** not in spike (structural finding). It frames S2–S15.
- **Symptom:** five commits edit the same block, `crates/smith/src/config.rs:321-445` (`standard_limits`), each after a live failure.
- **Root cause:** one function sets about 30 numbers belonging to eight layers. Only four relationships are computed:
  - `completion = completion_worst_case(&client, decoded_call_bytes)` (config.rs:343);
  - `completion_blocks = parts` (:367);
  - `max_completion_blocks = parts` (:407);
  - `head = idle = call_timeout` (:416-417).

  The rest are implicit:

  | Limit (main → tip) | Lives in | Bounds | Must cover / derive from | Checked? |
  |---|---|---|---|---|
  | `http.request` 4096 | skein-http `client::Limits` | request head written | bearer (≤ `grant_value_bytes` 2048) + identity + configured headers | no; overflow is `Error::Limit` at prepare (skein-llm client.rs:914-945) |
  | `http.head` 4096, `headers` 32→64 | skein-http | response head | provider header set | no |
  | `http.send` 31, `read` 256, `sse.chunk` 128 | skein-http | upload piece / read demand | TLS record size (S12) | `chunk ≤ read` only |
  | `sse.line`/`event` 4/8 KiB → 512 KiB | skein-http sse | one whole buffered event | ≥ framing + largest document | no (event = document, so the largest admissible document is about 35 B short) |
  | `document_bytes` 8 KiB → 512 KiB | skein-llm `DocumentLimits` (= `openai::Limits`, shared by both dialects) | wire bytes per event; writer cap | escape × (answer + echoed instructions, tools, usage) | no |
  | `tokens` 1024 → 4096 | skein-llm | owned tokens per event, schema, replay | grows with history (S2) | no |
  | `string_bytes` 4 → 64 KiB | skein-llm | any received string; any sent text | ≥ system text, rendered results, arguments, answer (S10) | one unit test |
  | `request_bytes` 8 KiB → 1 MiB | skein-llm | encoded request | context window × bytes per token (S3) | no |
  | `parts` 16 → 256 | skein-llm | tools declared; messages; **total history blocks**; output items per response; content indexes; JSON array lengths | four quantities in one number (S3) | no |
  | `input_bytes` 2 → 32 KiB | skein-llm | one call's raw arguments | tool payload × escape, ≤ answer (S4) | no |
  | `opaque_bytes` 2 → 8 KiB | skein-llm | each replay envelope, reasoning item, Anthropic thinking | per model and effort (S6, S7) | no |
  | `answer_bytes` 8 → 64 KiB | skein-llm | output per completion | max output tokens × bytes per token | no |
  | `decoded_call_bytes` 4 → 64 KiB | smith-domain, protocol-llm `Receiving` | decoded ops | ≥ parts × `size_of::<Decoded>()` (smith-domain/src/limits.rs:79-83) | yes |
  | `completion_blocks` 16 → 256 | smith-domain-session | blocks per completion | ≤ transcript `message_blocks` 16 (S8) | **no, and broken** |
  | `tool_bytes` 32 KiB, `result_bytes` 32 → 256 KiB | smith-protocol-llm | declarations; supplied results | `result_bytes` is dead in production (S3) | no |
  | `session.messages` 64 → 256, `session_bytes` 32 MiB | smith-domain-session | transcript held | ≫ what can be sent (S3) | no |
  | `max_tokens` 4096 (run, session) | crates/smith/src/limits.rs:45,72 | Anthropic wire cap; reservation | the model's output cap (S7) | admission only |
  | `connections` 6, `per_endpoint` 2 | skein-llm-connection | pool | ≥ concurrent conversations per endpoint (S9) | no |
  | connect/handshake 10 s; head/idle 60/30 → 300 s; whole = `call_timeout` 60 → 300 s | llm-connection `Deadlines`; session | per call | progress per provider; session's remaining time (S5) | no |
  | transcript-v2 `message_blocks` 16, `turn_messages` 64 | smith-transcript schema | durable turns | ≥ completion blocks (S8) | no |

- **Spike change:** values only, plus the reasonable derivations above.
- **Assessment:** redo properly. These are constants raised until the symptom went away. Each value is defensible alone, but together they encode undocumented assumptions: escape factors, an echo budget, `parts` meaning four things, the transcript schema, rendering, and pool concurrency.
- **Clean fix:** S14. Declare a few quantities and derive the rest in the crate that owns each mechanism. Check every relationship above at startup and name the violated one.
- **Contracts:**
  - protocol/agent.md §4.
  - protocol/llm.md §8 says "Set per call by the domain: the request's bytes …". That has drifted: request bytes are a startup constant of the shared client.
  - skein llm.md "Bounds and wire format".
- **Layer:** skein (client limit derivation), smith binary (declarations), smith-agent-service (startup checks).
- **Tests:** derivation property tests (S14); drop constant-pinning tests (S15).
- **Benchmark:** per-entry probes.
- **Questions:** see S14.

### S2. Provider events: the whole-document token cap and echoed metadata (report item 1)

- **Origin:**
  - `8e83931`: `tokens` 1024 → 4096; fixture `crates/smith/tests/fixtures/provider-completed.json`; test `standard_profile_accepts_completed_events_with_metadata_and_tool_schemas`.
  - `6136dc4`: SSE line 4 → 16 KiB, event and document 8 → 32 KiB; test `…_echo_tool_schemas`.
  - `ee512a1`: SSE and document to 512 KiB.
  - Reliability report, cause 1.
- **Symptom:** a valid ~12 KB `response.completed` had 1,036 tokens against a cap of 1,024. My count of the fixture: 1,036 tokens, of which:
  - `usage.attribution.items` is 542 (29 entries at about 18.7 each);
  - the `tools` echo is 304;
  - `instructions` is a 2,416-byte string.

  smith reads none of them.
- **Root cause (verified):**
  1. skein buffers each SSE event whole (skein-http/src/sse.rs module doc: "An event's data goes up whole").
  2. `Decoder::event` calls `Json::from_bytes(&message.data, limits)` (skein-llm/src/dialect.rs:58). This materializes every token into an owned `List<Token>` under `tokens`, every string under `string_bytes`, and the wire under `document_bytes` (openai/json.rs:23-75).
  3. Only after that does `decode_event` read `type`, `response.status`, `usage` and `error`.

  The failure is `Limit` with `Response` evidence. It is never retried (smith-domain-session/src/session.rs:1752-1762) and recurs on every attempt, so the run dies. The client also labels it "malformed provider event" (skein-llm/src/client.rs:649).
- **Next cliff (verified on recordings, extrapolated):** the raw SSE in `smith-benchmark-2026-10-08/provider/*/00N.sse` (gpt-6-luna) shows one `attribution` entry per input item, plus one.
  - The event grows by about 17.5 tokens per input item:
    - codex-smoke-002: 632 → 667 → 702 tokens for 5 → 7 → 9 inputs;
    - smith-smoke: 531 → 566 tokens for 1 → 3 inputs.
  - The base with 8 tools is about 500 tokens, so 4,096 is crossed at about 190–205 input items.
  - The best successful root (`iteration7-07_shell_cwd-smith-1`) ended with 65 completions, 74 calls, 74 results, 24 reasoning items and 2 texts: about 175 items, an estimated 3.6–3.7k tokens (about 10% headroom).
  - With the 256-turn budget, a longer root will hit this deterministically.
  - The `tools` echo grows with each host tool and later MCP tool, and the `instructions` echo with charter size.
- **Spike change:** raised `tokens`, `document_bytes`, `sse.line`/`event` and `parts`. Added the real-structure fixture.
- **Assessment:** redo properly.
  - The cliff moved by about 4×. The caps restrict data smith discards.
  - The fixture idea is good, but the test pins one size and lives in a smith config unit test.
  - testing-strategy.md §1 says a failure is fixed in the lowest tier that shows it, and this one is skein's.
- **Clean fix (skein):** selective, streamed event decoding.
  - A path-filtered collector over skein-json's incremental tokenizer retains only what the dialect needs (for Codex: `type`, `output_index`, `item`, `delta`, `response.{status,usage.<known>,error,incomplete_details}`).
  - It skips other values with a bounded depth stack (programming-model.md §8) and a running byte count.
  - Strings that are not retained are scanned, not stored.
  - `tokens` then bounds retained tokens, which is small. `document_bytes` becomes an abuse cap on skipped bytes and can be generous at no memory cost.
  - Feed `data:` lines to the tokenizer as they arrive, so `sse.event` stops being a memory term.
  - Cheaper alternative: keep the buffers and make `Collector` skip-and-count. That fixes tokens, but bytes stay sized by echoes.
  - Also make failures name the cap that fired (client.rs:649, :662, :687).
- **Contracts:**
  - skein llm.md "Bounds and wire format" ("JSON is decoded from bounded SSE event buffers").
  - skein json.md: add a selective collector.
  - skein http.md, SSE section: streamed data.
  - smith protocol/llm.md §8: receiving caps bound retained values.
  - The spike edited none of these.
- **Layer:** skein-json, skein-http sse, skein-llm.
- **Tests:**
  - skein protocol world with tiny limits (`tokens` 16, document 256): large unknown and echoed fields still decode, and memory stays independent of echo size.
  - Fuzzy: random extensions never change the decoded output.
  - Move the fixture to skein as a golden event.
- **Benchmark:** probe "Read each of these 150 small files one call at a time and report each line count" (effort low).
  - Assert no `Limit` with `Response` evidence.
  - Record each completed event's bytes and tokens against input items through a recording proxy.
- **Questions:** should the selective collector be a general skein-json facility or LLM-specific? I recommend general.

### S3. The request envelope and history caps (report item 2; benchmark row "256 KiB")

- **Origin:**
  - `6136dc4`: request 8 → 64 KiB, string 4 → 32 KiB, parts 16 → 64, decoded 4 → 16 KiB.
  - `8e83931`: request 256 KiB, parts 256, opaque 8 KiB, decoded 64 KiB, `session.messages` 256, `result_bytes` 256 KiB.
  - `cfb7408`: request 1 MiB, and the receiving_limits.rs tests.
  - Report item 2; benchmark row; `request-size-diagnostic.json`; `request-root-replay.rs`.
- **Symptom:**
  - On main, requests failed "after several tool calls".
  - At 256 KiB, the repository root was refused before sending: 57 messages, 118 blocks and 266,534 B after 28 turns (`validation/request-root-replay.log`).
  - The trace shows three `Limit/Unsent` in `iteration3-07_shell_cwd-smith-1`.
- **Root cause:**
  1. The cap has no relation to any context window.
  2. The session grows its transcript under much looser limits (`session_bytes` 32 MiB, `messages` 256) than the encoder will send, with no "context nearly full" signal. Context management is still "Later" in domain/session.md §8.
  3. An overflow becomes `Error::Limit`, then `Failure::Limit`, which is not retried. The session and the run end, so conversation length is a hard cliff.
  4. `parts` also caps **total history blocks** (skein-llm/src/translate.rs:73; anthropic/request.rs:63), messages (translate.rs:50), tools and output items.
- **Next cliffs (my arithmetic):** roots grow by 2.8–4.2 input items and about 9.5 KB per turn. For one conversation, in order:
  1. the event-token cap, at about 190–205 items (S2), around 45–70 turns;
  2. `parts` 256, around 60–90 turns;
  3. 1 MiB, around 110 turns;
  4. `messages` 256, around 128 turns.

  All are below the new 256-turn ceiling.
- **Dead limit:** `result_bytes` is reached only through `prepare` and `prepare_for_contract` (prompt.rs:39, :198), which only tests use. Production calls `prepare_component` (prompt.rs:52-70, from component.rs:270) with `&[]`, so the 8× raise only inflates the worst case.
- **Spike change:** values, plus tests:
  - config.rs: a history of 48 call/result pairs;
  - receiving_limits.rs: 80 × 4 KiB reads through the adapter at the old and new caps;
  - an exact encoded boundary by binary search, with refusal one byte over and an allocator meter.
- **Assessment:** keep with rework.
  - A finite cap enforced before sending is right: "refuse at the entrance" (programming-model.md §7).
  - The number is arbitrary.
  - A session must never die because its own history grew.
  - The boundary tests check skein's encoder from smith, and `assert_eq!(…, 1_048_576)` is a change detector.
- **Clean fix:**
  - Declare `context_window_tokens` per model and derive `request_bytes = window × B` with checked arithmetic and a memory-policy cap.
  - The protocol exposes a pure per-block request-cost function. The session keeps a running sum plus the reported input tokens.
  - Above a high-water mark (about 80%), the session elides old tool output, then summarises (domain/session.md §8). Codex compacts against a per-model limit (`model_auto_compact_token_limit`, `research/codex-source-0.160.0/core/src/session/session.rs:1340`).
  - Add a typed "context full" refusal, distinct from `Limit`, that triggers context management instead of failure. The provider's `ContextTooLong` maps to the same path.
  - Split `parts` into `max_tools`, `max_history_items`, `max_output_items` and `max_content_index`.
- **Contracts:**
  - domain/session.md §8 must be decided.
  - domain/session.md §3 (fitting history) and §4 (failure classes).
  - protocol/llm.md §8.
  - skein llm.md Bounds.
  - The spike edited none of these.
- **Layer:** skein-llm, smith-protocol-llm, smith-domain-session, binary.
- **Tests:**
  - Domain world: growth past the mark is elided and continues (tiny limits).
  - Protocol world: an envelope refusal is typed and never sent.
  - Fuzzy: random output sizes never end a session with `Limit/Unsent`.
  - Drop the constant pins.
- **Benchmark:** probe "Read these 40 large files and summarise each", sized past the envelope.
  - Assert completion with elision in the trace.
  - Record request bytes and input tokens per turn.
- **Questions:**
  1. Where does the window come from? I recommend skein defaults with settings overrides.
  2. Elide before summarising? I recommend yes.

### S4. Tool arguments: 2 KiB → 32 KiB (`ee512a1`; benchmark row "2 KiB tool arguments")

- **Origin:** `ee512a1`:
  - `input_bytes` 2 → 32 KiB, `answer_bytes` 8 → 64 KiB, `string_bytes` → 64 KiB, document and SSE → 512 KiB;
  - `crates/smith/tests/receiving_limits.rs`, `end_to_end.rs`, and the `tests/support/mod.rs` `run_with_provider` seam.
- **Symptom:**
  - The line-decoder attempts aborted with `Model(Completion { failure: Limit, evidence: Response })`.
  - My scan of all smith traces: 73 of 1,662 calls (4.4%) exceeded 2 KiB.
  - The largest after the fix were 14,810 B (`shell`) and 12,560 B (`write`); none exceeded 16 KiB.
- **Root cause (verified):**
  - skein's decoder already marks one oversized call `too_large` with empty input (skein-llm/src/openai/response.rs:364), a field meant for per-call handling.
  - `translate::part` turns `too_large` into `Error::Limit` (translate.rs:331; Anthropic dialect.rs:188), failing the whole completion. skein llm.md: "A local size limit is a failure".
  - Implicit couplings:
    1. The full arguments come again in `response.output_item.done` and `response.completed`. The SSE and document caps were hand-sized to hold them (comment at config.rs:322-324: 64 KiB × 6 escape = 384 KiB < 512 KiB).
    2. The tools allow 64 KiB writes (`tools.file_bytes`, limits.rs:91; machine `file_bytes` 65,536), so a 60 KiB file can be read but not rewritten with one `write`.
    3. Anthropic's 4096-token wire cap is lower still (S7).
- **Spike change:** the values above. Tests:
  - an exact boundary, refusal one byte over, replay, the old-limit control and NUL-escape memory under skein's counting allocator;
  - a TLS-binary end-to-end large write with the referee.
- **Assessment:** keep with rework.
  - 32 KiB is 2.2× the observed maximum, but hand-picked, and one oversized call still kills the session.
  - The end-to-end binary test is the right tier.
  - `receive()` hand-rolls a 100,000-step client pump that duplicates skein-llm-world (AGENTS.md: reuse skein's test kit).
  - `assert_eq!(bytes, 32_768)` is a change detector.
- **Clean fix:**
  1. **In skein, oversize is a per-call outcome.**
     - The call arrives with arguments elided and an `oversized { bytes }` marker, charged to the answer by length only. The completion completes.
     - smith decodes it as `Decoded::Invalid { problem: TooLarge }`, which already exists (completion.rs:253-266; protocol/llm.md §4 "input too large").
     - The model is told "arguments of N bytes exceed M; use edit or smaller writes".
     - This needs S2's streaming so done and completed events never hold the arguments in memory.
  2. **Derive `max_tool_argument` from one declared "max tool payload".** The write tool's file cap and its schema `maxLength` follow from it, and the description states it. Assert `input ≤ answer ≤ string`.
- **Contracts:**
  - skein llm.md Vocabulary: "A local size limit is a failure…" becomes per-call oversize as a block outcome.
  - smith protocol/llm.md §4.
  - domain/tools.md §3 (advertise the cap).
  - The spike edited none of these.
- **Layer:** skein-llm, smith-protocol-llm, binary.
- **Tests:**
  - skein: an oversized call next to a normal one completes with one oversized block.
  - smith protocol world: rendered as a fixable error.
  - Domain world: recovery with edits.
  - Keep the binary regression.
- **Benchmark:** probe "Create `big.txt` with the numbers 1..12000, one per line, in a single `write`".
  - Assert no `Limit`, an error result naming the cap, and success after splitting.
- **Questions:** one declared tool payload for read, write and arguments? I recommend yes.

### S5. Completion deadlines: 60 → 300 s (`d75190b`; benchmark row "60 s whole-response timeout")

- **Origin:** `d75190b`:
  - `crates/smith/src/limits.rs:76-79`: `call_timeout` 300 s;
  - config.rs:416-417: head and idle = `call_timeout` (were 60 s and 30 s);
  - a new domain/session.md §4 bullet, local-host.md and workflow.md;
  - `crates/smith/tests/completion_deadlines.rs`.

  Also `completion-deadline-analysis.json`.
- **Symptom:**
  - In `iteration2-05_lease_queue-smith-1` (trace lines 143-158), a child made four attempts, each `TimedOut/Response` at 60.000 s, then ended `Failed { TimedOut }`: 240 s spent on one completion.
  - After the fix, a 66.3 s child response was observed.
- **Root cause:**
  - `whole` (protocol-llm component.rs:308-314, from `Request::Complete.timeout` = `limits.call_timeout`, session.rs:1681) is absolute, including active streaming.
  - The session treats `TimedOut` as transient with a zero backoff floor (session.rs:1754) and `retries: 3`. A deterministic slow attempt is repeated four times.
- **Spike change:** whole 300 s; head and idle also 300 s.
- **Assessment:** redo properly.
  - Raising the absolute cap only moves the cliff. Native Codex has no absolute deadline and a 300 s stream idle timeout (analysis JSON, `native_facts`).
  - With head = idle = whole, `head` (armed at the first sent byte) and `idle` (armed at the response, re-armed on each event) never expire before `whole` (skein-llm-connection/src/deadlines.rs). A dead server is detected only after 5 minutes, and a healthy stream is still cut at 5 minutes.
  - With retries, one completion can take about 20 minutes, the benchmark's whole task timeout.
  - The domain contract now carries profile numbers.
  - The tests use virtual time, which is good, but enshrine the absolute cap ("the whole-call cap wins even after activity rearms idleness").
  - `completion_deadlines.rs` hand-rolls a component pump that belongs in the `tests/protocol-llm` world.
- **Clean fix (streaming-aware; programming-model.md §7 "Progress deadlines"):**
  - **connect and handshake:** keep at 10 s.
  - **head:** short and per provider, e.g. 60 s, with upload progress counted (S12).
  - **idle (progress):** the largest gap between complete SSE events, per provider and model: Codex 300 s, Anthropic shorter (it streams `ping`). Activity already counts SSE messages (llm-connection call.rs:319-322).
  - **whole:** the session's remaining time (min of session and run expiry), already passed per call. Optionally a declared per-model attempt ceiling.
  - **Retry by phase:** head and idle timeouts are transient; a whole expiry after progress is not retried blindly. This needs llm-connection to expose `Due::{Head,Idle,Whole}`, which exists internally but all map to `TimedOut`.
- **Contracts:**
  - domain/session.md §4: replace the spike's numbers with this policy. The spike's text does not hold: profile numbers in a domain contract, and an absolute cap during streaming.
  - protocol/llm.md §6.
  - skein llm-connection.md §6: the expired phase in the failure.
- **Layer:** skein-llm-connection, smith-domain-session, binary.
- **Tests:**
  - Virtual-time protocol world: a steady stream past any fixed cap completes; a stall fails `TimedOut{Idle}` and is retried; expiry of the session's remaining time ends the session once with no retry.
  - Session world: the retry classification.
- **Benchmark:** probe a long-reasoning prompt at high or xhigh effort.
  - Assert that a response over 300 s completes within budget.
  - Record head latency and the largest inter-event gap per provider and model to set head and idle from data.
- **Questions:** should there be a per-attempt ceiling? I recommend none by default, with an optional per-model declaration.

### S6. Reasoning items: 2 → 8 KiB, and the next cliff

- **Origin:** `8e83931` (`opaque_bytes` 2048 → 8192); report item 2.
- **Symptom:** my scan of saved turns in all smith runs found 272 `encrypted_content` values: median 1,484 B, max 5,112 B (`iteration7-07_shell_cwd-smith-1`, turn 64).
  - main's 2 KiB was below a large share of them.
  - 8 KiB is 1.6× the largest at high effort.
- **Root cause:**
  - One cap bounds every replay envelope: reasoning items (openai/response.rs:380) and the small synthesized metadata (translate.rs `replay`, `min(document, opaque)`).
  - Reasoning size is per model and per effort.
  - Overflow is a completion-wide `Limit`, never retried.
- **Spike change:** 8 KiB.
- **Assessment:** keep with rework. It fits observed traffic, but it is a per-model property, the failure is fatal, and xhigh effort is likely to cross it.
- **Clean fix:**
  - Separate a small, fixed metadata cap from the reasoning cap.
  - Derive the reasoning cap from the declared output or reasoning tokens × the encryption and base64 expansion, or declare it per model.
  - Optional: on overflow, keep the completion and omit the reasoning item from later requests, recorded as a fact. Codex with `store:false` accepts that; continuity of reasoning suffers, correctness does not.
- **Contracts:** skein llm.md Vocabulary; smith domain/session.md §3 ("verbatim") if dropping is adopted.
- **Layer:** skein-llm.
- **Tests:** skein world with tiny caps: at the cap and one byte over. With a drop policy: the completion continues and the next request omits the item.
- **Benchmark:** xhigh-effort probe asserting no `Limit`; record reasoning item sizes.
- **Questions:** fail or drop? I recommend a derived cap that fails as a typed error, with dropping as an explicit opt-in.

### S7. The Anthropic path: what the spike did not fix

- **Origin:** not in spike. `fc939dc` only lowered the local-host.md example `max_tokens` from 8192 to the admitted 4096.
- **Symptom:** none observed. The benchmark had no Anthropic runs, and the live stories run `claude-haiku-4-5` without effort (`tests/support/live.rs`).
- **Root cause (verified in code, not run):** Anthropic shares `DocumentLimits`, but its cliffs are elsewhere:
  1. **Wire output cap.**
     - skein sends `max_tokens` = `prompt.max_output_tokens` (types.rs:99 → anthropic/request.rs:232).
     - smith sets it from session `max_tokens` ≤ 4096 (limits.rs:45,72), and admission refuses more, which is why `fc939dc` had to change the doc.
     - With adaptive thinking, thinking and output share those 4096 tokens.
     - A 32 KiB `write` (about 8–10k tokens) is truncated: `MaxTokens`, the session yields Truncated, and the run nudges or fails `Model(Truncated)` (smith-domain-run/src/run.rs:1755).
     - The effective argument cap is about 12–16 KB, the same failure `ee512a1` fixed for Codex. `output-budget-analysis.json` shows 4096 is inert on Codex, so it was never tested as a wire cap.
  2. **A truncated `tool_use`** must parse as an object on replay (anthropic/request.rs:337). Every later request is refused `Invalid`, which is not retried, so the session dies.
  3. **Thinking** text and signature accumulate in `opaque_bytes`-capacity lists (anthropic/response.rs:277-278), and the block must fit `min(opaque, document)` (:397). Thinking over about 8 KiB means `Limit`. This is likely at high effort; unverified live.
  4. **`answer_bytes`** counts thinking + signature + output.

  Not affected: S2's echo cliff.
- **Spike change:** none.
- **Assessment:** redo properly. All the spike's fixes are OpenAI-path.
- **Clean fix:**
  - Declare per-model max output, send it on the wire, and derive `answer_bytes` and the thinking cap from it.
  - Use per-dialect limits instead of one shared struct.
  - A truncated `tool_use` must never become an un-replayable block. skein marks it; smith replaces it in history with a text note and an error result.
- **Contracts:**
  - skein llm.md "Bounds and wire format" ("`max_tokens` (4096 by default)").
  - skein llm.md Vocabulary (replay refuses malformed input).
  - smith protocol/agent.md §4.
  - domain/run.md §9.
- **Layer:** skein-llm (anthropic), smith config and limits.
- **Tests:**
  - skein Anthropic world with tiny caps: thinking over the cap; a truncated `tool_use` followed by a continuation.
  - smith domain world: `MaxTokens` with a partial call continues.
- **Benchmark:** Anthropic probes:
  1. "Write a 40 KB file in one `write`": no Truncated failure.
  2. A high-effort task: no `Limit`; record thinking sizes and stop reasons.
- **Questions:** repair or refuse truncated calls? I recommend a typed unreplayable marker, with smith deciding.

### S8. Completion blocks above the transcript's 16 blocks per message (latent; introduced by the spike)

- **Origin:** `6136dc4` (`completion_blocks = parts`, 16 → 64); `8e83931` (→ 256).
- **Symptom:** not observed. The largest number of tool calls in one completion across all smith traces is 8. Grouped by owner and timestamp (my method), the distribution is 1:764, 2:11, 3:40, 4:63, 5:57, 6:27, 7:7, 8:1. Codex requests `parallel_tool_calls: true`.
- **Root cause (verified by reading):**
  - `crates/smith-transcript/schema/transcript-v2.schema:58` has `record Message { role: Role blocks: list 16 Block }`.
  - It implicitly matched the old 16.
  - A completion with 17 or more blocks, or its results message, is accepted by the session but fails when the turn is told: `encode_turn` errors, `send_turn` errors, then `service.mark_failed(Failure::Send)` (smith-agent-service/src/service.rs:1408-1412). The whole agent fails.
  - No startup check relates the two numbers.
- **Spike change:** none intended.
- **Assessment:** redo. The receiving cap must be ≤ the transcript bound, and an oversized completion refused at entrance.
- **Clean fix:**
  - `completion_blocks = min(provider output items, transcript message_blocks)`, asserted at startup.
  - Raise the bound in a new schema version only if parallel batches need it.
  - Optionally state the parallel-call limit in tool guidance.
- **Contracts:** protocol/transcript.md §7; domain/session.md §3; protocol/agent.md §4.
- **Layer:** binary, agent-service startup, smith-transcript.
- **Tests:** an agent-process world test with production limits (a 17-call completion is refused as a typed failure or told successfully); a startup consistency test.
- **Benchmark:** probe "Read these 20 files in parallel in one step"; assert no `Send` failure.
- **Questions:** schema bump or cap at 16? I recommend capping until parallel batches are designed.

### S9. Connection pool: two calls per endpoint, and the third is a fatal `Limit` (latent)

- **Origin:** not in spike (config.rs:398-399, unchanged).
- **Symptom:** not observed. The benchmark's delegations were serialized because they asked for write grants.
- **Root cause (verified by reading):**
  - skein-llm-connection refuses `Start` when an endpoint has `per_endpoint` connections and none is idle (component.rs `start` → `Refused { Pool }`).
  - smith maps `Pool` to `Error::Limit` (smith-protocol-llm/src/component.rs:342), then `Failure::Limit`, never retried (session.rs:1759).
  - smith runs up to 4 parallel inspect-only sub-agents, and up to 6 conversations. A third concurrent call to one endpoint ends its session.
- **Assessment:** a latent bug. A pool cap should cause waiting, not death.
- **Clean fix:**
  - (a) Queue pending calls, bounded by conversations, while `whole` (the remaining session time) runs.
  - (c) Derive `per_endpoint ≥` the concurrent conversations per endpoint (HTTP/1.1: one call per connection), checked at startup.
  - (b) Mapping `Pool` to a transient failure is worse: busy retries spend the retry budget.

  I recommend (a) and (c) together.
- **Contracts:** skein llm-connection.md §7; smith protocol/llm.md §6; domain/session.md §4.
- **Layer:** smith-protocol-llm, or skein-llm-connection.
- **Tests:** protocol world: three concurrent calls on one endpoint with `per_endpoint` 2 all complete. Run world: four parallel inspect-only sub-agents.
- **Benchmark:** probe "Use four sub-agents in parallel, each reading one directory, then summarise"; assert no child `Limit/Unsent`.
- **Questions:** none.

### S10. Rendered results, system text and host replies against one string cap (implicit)

- **Origin:** `6136dc4` (`string_bytes` 4 → 32 KiB; test `standard_profile_accepts_full_workspace_read_results`); `ee512a1` (→ 64 KiB).
- **Symptom:** on main, a 4 KiB read rendered to more than 4 KiB (the test renders 4,096 × 0xff to more than 16,384 B), and the request was refused.
- **Root cause:**
  - All sent text is checked against `string_bytes` (translate.rs:129; anthropic `text()`).
  - `render_outcome(outcome, string_bytes)` returns `Limit` when over (smith-protocol-llm/src/render.rs:16-21). The request is refused and the session dies.
  - `string_bytes` must cover each tool's worst rendering (another theme's `read_bytes` and shell limits), the system text (charter up to `run_bytes` 64 KiB plus guides), host replies (64 KiB plus "Error: ") and delegated results. None of this is checked.
- **Spike change:** values and one test.
- **Assessment:** keep the value, rework the model.
- **Clean fix:**
  - A pure `render_worst_case(tools::Limits)` and a startup assertion `string_bytes ≥ max(render, system, host reply + prefix, delegated)`.
  - Better: truncate tool output with an explicit "[N bytes omitted]" marker instead of refusing the request.
- **Contracts:** protocol/llm.md §5 and §10's referee.
- **Layer:** smith-protocol-llm, binary.
- **Tests:** a property test (random tool limits fit the string cap); a host reply at its cap.
- **Benchmark:** none needed.
- **Questions:** truncate or refuse? I recommend truncating with a marker.

### S11. The worst-case memory figure ("checked memory allowance was raised"; `8e83931`'s environment term)

- **Origin:**
  - The reliability report: "The checked memory allowance was raised…".
  - local-host.md and README: 354 → 359 → 360 GB.
  - `8e83931`: `crates/smith-protocol-machine/src/limits.rs:79-83` (`environment … checked_mul(4)`).
  - The receiving_limits.rs startup test.
- **Symptom:**
  - The bound was about 354 GB on main, 358,993,965,054 B at `ee512a1` and 359,734,783,998 B at `cfb7408` (workflow.md).
  - Eager startup is 4,233,560 B.
  - `memory_bytes` is 1 TiB in tests and the smoke harness (`smith_smoke.py:37`), and 1e12 in the README.
  - The user's own settings are not readable here, so the raise is unverified.
- **How it is computed:**
  - `service::worst_case` (smith-agent-service/src/service.rs:336) sums domain, channel, `llm::component_worst_case`, machine, io, files and queues.
  - The LLM part is `connection::worst_case + (per-call bound + contract) × connections (6)` (component.rs:103-117).
  - The per-call bound (smith-protocol-llm/src/limits.rs:72-104) multiplies independent maxima. My arithmetic at tip values:
    - `parts × document_bytes × 4` (:100) = 512 MiB;
    - `parts × tokens × 24 B × 4` (:99) = 96 MiB;
    - `parts × string × 2` (:101) = 32 MiB;
    - `client::worst_case` = 97.6 MiB (the spike measured 102,358,592 B; 57.9 MiB at 256 KiB);
    - `completion × 2` ≈ 4.4 MiB.
  - That is about 745 MiB per call, × 6 ≈ 4.4 GiB, plus pool ≈ 0.6 GiB: **about 5 GiB for the LLM path**, against about 30 MB on main.
  - The spike's own numbers give exactly **942 bytes of bound per byte of `request_bytes`**: (359,734,783,998 − 358,993,965,054) / (1,048,576 − 262,144).
  - Tokens cost 24 B each, so a 1 MiB request has a worst case of 1 M tokens.
  - The remaining ~350 GB comes from transcript and channel codec ceilings (nested per-field maxima: the transcript-v2 `Turn` is 64 messages × 16 blocks × MiB-scale fields, × `start_transcript` 64). That is outside this theme and estimated.
- **Effect:**
  - The bound is about 85,000× the eager heap.
  - No host has 360 GB, so `memory_bytes` is set to a token value, and programming-model.md §6.3's check ("refuses to start when the sum exceeds the configured memory") is vacuous.
  - "Not eager RAM" is true, and is exactly why the check catches nothing. The +0.7 GB for the 1 MiB envelope went unnoticed.
- **The environment ×4 term:** a coarse coexistence factor (defaults, per-command variables, merged list, encoded block) instead of named terms. It is conservative.
  - The `shell` check `capacity > limits.env_bytes` compares a count with bytes. The units are mismatched but conservative.
  - Keep with rework: name the terms.
- **Spike change:** raised values, updated docs, and added a test asserting bound ≤ 1 TiB and eager < 64 MiB.
- **Assessment:** redo the accounting model. The problem predates the spike, which made the LLM share about 150× larger and normalized it in the docs.
- **Clean fix:**
  - **Aggregate budgets.** All request transit is ≤ k × `request_bytes`, not `parts × document`. With S2, receiving retains about answer + one small event.
  - **Compact tokens** (offsets into one buffer) in skein-json and skein-llm `Json`, removing the 48× term.
  - **An LLM memory pool with admission.** Each `Start` reserves its measured request plus its receiving reservation, and calls wait when it is full (programming-model.md §7). The worst case is the pool size, independent of connections and envelope.
  - The same treatment for the codec ceilings (other theme).
  - **A target:** ≤ 1–2 GiB at 6 conversations and 1 MiB envelopes, so `memory_bytes` can be real memory.
- **Contracts:** skein programming-model.md §6.3 (fine as written); skein llm.md Bounds; llm-connection.md §7; smith protocol/llm.md §8; protocol/agent.md §4; README; local-host.md.
- **Layer:** skein-json, skein-llm, skein-llm-connection, smith-protocol-llm, smith-agent-service.
- **Tests:** a fuzzy memory driver with skein's counting allocator against the aggregate bound; assert the standard declaration is ≤ the target.
- **Benchmark:** record peak RSS against the bound; probe with 4 parallel inspect-only sub-agents with large histories.
- **Questions:**
  1. The target? I recommend ≤ 2 GiB.
  2. Accept waiting as backpressure? I recommend yes.

### S12. Uploads in 31-byte pieces, reads of 128–256 bytes (latent; worse at 1 MiB)

- **Origin:** not in spike (config.rs:326-327: `send: 31`, `read: 256`, `chunk: 128`). The spike made requests up to 128× larger.
- **Symptom:** not measured.
- **Root cause (verified by reading):**
  - The client uploads `http.send` bytes per grant (skein-llm/src/client.rs:481-497).
  - skein-tls seals each send through rustls `traffic.encrypt` (skein-tls/src/client.rs:803-823; session.rs:151-174).
  - A 1 MiB body is about 33.8k pieces, loop round trips and TLS records, roughly 0.7 MiB of record overhead (inferred).
  - The 266 KB root request is about 8,600 records per turn.
  - A 512 KiB event is read in 128-byte scans.
- **Assessment:** a latent performance cliff; test minima carried into production.
- **Clean fix:** derive `send` from the TLS plaintext record size (16 KiB), and `read` and `chunk` from TLS `read`; possibly coalesce in skein-tls.
- **Contracts:** none.
- **Layer:** binary config; maybe skein-tls.
- **Tests:** a serial measurement of CPU time to upload 1 MiB through the real loop, outside the budgets.
- **Benchmark:** compare time to first byte against request size from the proxy's `headers_seconds`, with 31 B and 16 KiB pieces.
- **Questions:** none.

### S13. HTTP head limits (`6136dc4`: headers 32 → 64)

- **Origin:** `6136dc4` (`headers` 64, test with 40 headers).
- **Symptom:** provider heads with many fields. `HeadTooLong` or `TooManyHeaders` is a fatal `Limit` with `Response` evidence (client.rs:561).
- **Root cause:**
  - The response head is still 4096 B, about 64 B per header. Real header sizes are unverified; the proxy did not record them.
  - The request head of 4096 must hold the bearer (≤ 2048), identity headers and configured headers. That is checked only at prepare (fatal `Limit`), not at startup.
- **Assessment:** keep the raise, rework the derivation.
- **Clean fix:**
  - A response head of 16–32 KiB.
  - Request head = grant + measured configured headers + identity, checked at startup.
- **Contracts:** protocol/agent.md §4.
- **Layer:** binary.
- **Tests:** a startup refusal; the existing many-headers test.
- **Benchmark:** record head sizes in the proxy.
- **Questions:** none.

### S14. A declared-quantity limits model (design direction)

- **Origin:** not in spike. `fc939dc` shows the gap: the docs said `max_tokens` 8192, the hidden profile admits 4096.
- **Symptom:** every new model, provider or task size hits a different hidden constant (S2–S13).
- **Proposal:**
  - **Declared per model** (skein-shipped defaults, settings overrides):
    - `context_window_tokens`;
    - `max_output_tokens` (wire cap where supported, and the reservation);
    - `max_reasoning_item_bytes` (or derived);
    - head and idle timeouts.
  - **Declared per deployment:**
    - maximum concurrent conversations;
    - maximum tool payload;
    - tool render caps (other theme);
    - the LLM memory pool;
    - `memory_bytes` as real memory.
  - **Derived in skein** (`client::Limits::derive(Declared) -> Option<Limits>`, checked arithmetic, documented factors):
    - `request_bytes`, `answer_bytes`, `input_bytes` (≤ answer) and `string_bytes`;
    - the reasoning cap;
    - retained tokens and skip caps;
    - separate `max_tools`, `max_history_items` and `max_output_items`.
  - **Derived and checked in smith:**
    - the session high-water mark (S3);
    - `completion_blocks ≤` the transcript bound (S8);
    - `string_bytes ≥` the render worst case (S10);
    - pool ≥ concurrency (S9);
    - deadline = remaining session time (S5);
    - the worst case from the pool (S11);
    - run and session `max_tokens ≤` the declared model output (instead of a fixed 4096).
  - **Startup:** any violated relationship refuses the configuration and names the relationship.
  - **Configuration:**
    - `models[]` gains `context_window` and `max_output_tokens` (optional for known models);
    - endpoints gain `connections`, `head_ms` and `idle_ms`;
    - no raw byte limits, except an expert override for tests.
  - **Trade-off:**
    - One hard-coded profile is simple and testable, but meets a new cliff with every model, and its bound decays as values grow.
    - Declared quantities need a defaults table and property tests, but scale across providers, hosts and task sizes.
    - Middle path: profiles as named bundles of declared values (`small`, `standard`, `large`) sharing one derivation.
- **Contracts:** protocol/agent.md §4; domain/run.md §9; protocol/llm.md §8; skein llm.md Bounds; llm-connection.md §7.
- **Layer:** skein-llm (derivation, defaults), smith binary, smith-agent-service, smith-domain-session.
- **Tests:** property tests that `derive` either refuses or satisfies every relationship; worlds keep tiny declared values (testing-strategy.md §3); probes per model.
- **Benchmark:** the S2–S9 probe matrix per configured model.
- **Questions:**
  1. Where do model defaults live? I recommend skein-llm with settings overrides.
  2. Expose raw byte limits? I recommend no.
  3. Keep the profile name? I recommend yes, as a bundle of declared values.

### S15. The spike's limit tests and fixture: what to keep

- **Origin:**
  - `6136dc4` and `8e83931`: five config.rs unit tests, plus `provider-completed.json`.
  - `ee512a1` and `cfb7408`: `receiving_limits.rs` (5 tests), `end_to_end.rs` (1), the `run_with_provider` seam.
  - `d75190b`: `completion_deadlines.rs` (3).
- **Assessment:**
  - **Keep:**
    - the real-structure fixture, moved to skein as a golden event;
    - the TLS-binary large-write end-to-end test;
    - the `run_with_provider` seam;
    - the counting-allocator checks (they reuse skein's `Counting`).
  - **Rework:**
    - exact-boundary tests belong in skein's tier with tiny limits;
    - the hand-rolled client and component pumps duplicate skein-llm-world and `tests/protocol-llm` (AGENTS.md);
    - the deadline tests should assert S5's policy.
  - **Drop:** the constant pins (`32_768`, `1_048_576`, `parts == 256`, `opaque == 8192`, 300 s). Replace them with derivation properties.
  - **Budgets:** the spike measured 8 receiving tests in 1.538 s and the deadline stories in 0.048 s, serially. Both are within 15 s and 60 s, and the skein-tier replacements will be cheaper.
- **Contracts:** none.
- **Layer:** skein and smith tests.
- **Benchmark:** none needed.
- **Questions:** none.

---

## Cross-theme dependencies

- **Turn budget and child quotas:** longer conversations hit S2 (about 190–205 items), S3 (`parts` 256, 1 MiB) and the channel `turns: 128` window (config.rs:389) first. Raising turns alone turns "turns exhausted" into "Limit".
- **Tool output sizes (`read_bytes`, `shell_head`/`shell_tail`, `output_bytes`):** they must fit `string_bytes` after rendering (S10) and drive per-turn request growth (S3). Raising them alone recreates main's refusal.
- **Shell environment merge (`8e83931`):** owns S11's environment ×4 term and the count-versus-bytes check.
- **Sub-agent parallelism:** S9's pool cliff; asynchronous children multiply calls per endpoint.
- **Diagnostics:** `Limit` failures carry generic detail (skein-llm client.rs:649, 662, 687). They should name the cap (S2), and timeouts their phase (S5).
- **Context management (session.md §8):** it is S3's clean fix.
- **Transcript schema:** S8 (16 blocks per message) and the codec ceilings behind most of the 360 GB (S11).
- **skein shutdown and pool-close spike (`416cffd`, `f5fca44`):** S9's pending queue must settle at owner close with exactly one terminal per queued call.