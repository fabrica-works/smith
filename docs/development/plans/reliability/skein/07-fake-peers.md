# Session 07: the fake peers answer what they accept

Provisional, 2026-10-12, added during the run (decisions.md, 13.12). One
step, skein-07.s1, not in the plan as written: smith 05.12's process sweep
found it, and smith 05.12 waits for it.

### 07.s1 A connection the fake peers will not take is rejected
Scope: the fake LLM and OAuth peers answer a socket that io announced, when stopping or at their connection limit, with `Reject`, never `Abort` (skein io.md, 3.1: "an announced socket before its answer, is the layer above's bug, asserted"; 3.3: "Opening, Announced | `Close`, `Abort`, `Stream` | impossible").
Needs: nothing.
Files: `testing/skein-fake-peers/src/llm.rs:156`, `src/oauth.rs:171`; `testing/skein-fake-peers/tests/{llm.rs,oauth.rs}`.
Shape: `io::Request::Reject { socket }` in place of `io::Request::Abort { entity: socket }` at both lines; nothing else changes.
Test shape: `testing/skein-fake-peers/tests/llm.rs`, beside `fake_peer_binds_loopback_and_shutdown_settles_the_actual_listener` (:62), reusing `limits()` (:10) and `make` (:33): `a_connection_past_the_peers_limit_is_rejected_and_the_peer_settles` (two clients at `connections: 1`, or one client after `shutdown()`, whichever the test reaches in fewer lines); the same in `tests/oauth.rs` for the issuer. Each fails on the parent with io's panic "an announced socket is bound or rejected, not closed". At most 40 lines each, under 0.5 s.
Proof: unit, the two tests above, once: `heavy cargo nextest run -p skein-fake-peers`. smith 05.12's process sweep shows it in smith once it repins.
Don't: change skein-io or its assertion; change any other answer of the peers; add a peer option.
Size: at most 10 lines of production code.

### 07.s2 skein-world's tree supervision skips a PID that names no process
Added during the run (decisions.md, 13.67). Scope: `testing/skein-world/src/tree.rs`'s
supervision opens a pidfd for each discovered member and skips `ESRCH`
(3) only; `pidfd_open` returns `EINVAL` (22) for a PID that names a
thread, or one gone and reused as a thread, and the world fails with
"opening supervised tree member N: Other(22)". It did twice in this run
(smith 04.s1's gate, smith 04.s2's live Proof). `EINVAL` is skipped as
`ESRCH` is; nothing else changes.
Needs: nothing.
Files: `testing/skein-world/src/tree.rs` (:249); its tests.
Shape: one match arm.
Test shape: a unit test, if the module's tests can hand `open_pidfd` a
thread's id (`gettid` of a spawned thread), `a_thread_id_found_in_the_tree_is_skipped`;
otherwise none, the commit body saying why. Under 20 ms.
Proof: that test once, or the gate.
Don't: skip any other error.
Size: at most 10 production lines.

### 07.s3 A reasoning summary's parts are not output items
Added during the run (decisions.md, 13.70), from the designer's answer
of 2026-10-12T19:30:20Z (skein bdbe1fd0, llm.md sections 2, 4.2 and 5).
Scope: the Codex decoder refuses a delta whose part index reaches
`output_items` (`crates/skein-llm/src/openai/response.rs:473-477`); for a
reasoning delta that index is `summary_index` (:380-381), so a
high-effort summary of ten or more parts fails as `Limit`
(smith 13.6's `probes-slow-completion-d104cecb-20261012T192137Z/01`).
llm.md now: "A part's index within an item ... is not an output item".
The sub-index check goes; the kind check and the bytes against the
answer stay.
Needs: nothing.
Files: `crates/skein-llm/src/openai/response.rs`; its tests.
Shape: no new type.
Test shape: llm.md section 5's story: a Codex reasoning summary of more
parts than output items, and a message of more content parts, complete,
their deltas forwarded with their indexes, under limits whose output
items are two; it fails on the parent. Under 20 ms.
Proof: that test, once; smith's `probes/slow-completion` once after
smith-10.10's repin takes it.
Don't: change `output_items` or its formula, the item checks at :329 and
:498, or the Anthropic decoder's block index check.
Size: at most 20 production lines.

### 07.s4 A run of consecutive reasoning items is one block
Added during the run (decisions.md, 13.78), from the designer's answer of
2026-10-12T20:13:31Z (skein d03b84f2, llm.md sections 2, 4.2 and 5).
Scope: both decoders keep a run of consecutive reasoning items (Codex's
reasoning segments, Anthropic's back-to-back thinking) as one block, its
items in order, each within the reasoning limit, together within the
completion's share (output items x reasoning) and one block's retained
tokens; the encoder replays them byte for byte as consecutive input
items. Codex's decoder takes a provider item index past the blocks it
holds without an `opened` entry per item (`response.rs:325-330` requires
`index == self.opened.len()` today).
Needs: nothing.
Files: `crates/skein-llm/src/openai/{response.rs,request.rs}`,
`crates/skein-llm/src/anthropic/` (its decoder and encoder); their tests.
Shape: no new type beyond the block's ordered items.
Test shape: llm.md section 5's "Reasoning runs" story, both dialects,
and a run past the share failing as `Limit`, naming it; it fails on the
parent. Under 20 ms each.
Proof: those tests, once.
Don't: change `output_items` or its formula, or add a count; change the
reasoning limit per item, its opt-in drop, or the receiving reservation.
Size: README rule 9's.
