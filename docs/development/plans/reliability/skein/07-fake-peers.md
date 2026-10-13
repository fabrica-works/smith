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
Files: `crates/skein-llm/src/openai/{response.rs,request.rs}`, `crates/skein-llm/src/translate.rs` (13.79), new `crates/skein-llm/src/openai/response/reasoning.rs` (13.81),
`crates/skein-llm/src/anthropic/` (its decoder and encoder); their tests.
Shape: no new type beyond the block's ordered items.
Test shape: llm.md section 5's "Reasoning runs" story, both dialects,
and a run past the share failing as `Limit`, naming it; it fails on the
parent. Under 20 ms each.
Proof: those tests, once.
Don't: change `output_items` or its formula, or add a count; change the
reasoning limit per item, its opt-in drop, or the receiving reservation.
Size: README rule 9's.

### 07.s5 `directory_access`: whether a directory can be written
Added during the run (decisions.md, 13.88), from the designer's answer of
2026-10-12T21:31:06Z (skein 698b9a93, shell.md 6.1 and section 9).
Scope: a startup function in `skein-shell`, beside `effective_user`
(`ring.rs:1664`), through the ring adapter: `directory_access(path)`
answers as the kernel does for the process's effective user, with
`faccessat2(..., W_OK | X_OK, AT_EACCESS)`: writable, absent, not a
directory, or denied with the kernel's error; it creates nothing, opens
nothing for writing and changes nothing.
Needs: nothing.
Files: `crates/skein-shell/src/{lib.rs,ring.rs}`; `tests/ring`.
Shape: one function and its answer's enum.
Test shape: shell.md section 9's startup-read tests: a writable
directory, an absent path, a file, a directory without write permission,
and nothing created. Under 20 ms each.
Proof: those tests, once.
Don't: read mode bits or an owner in its place; open anything for
writing.
Size: README rule 9's.

### 07.s6 An inherited write pipe takes its owner's write deadline
Added during the run (decisions.md, 13.100), from the designer's answer
of 2026-10-12T23:49:46Z (skein ca600491, io.md 5.1). Scope:
`adopt_write_pipe` (`skein-io/src/layer.rs:164-171`) takes an optional
write deadline, as `adopt_append` has one; io arms, cancels and tells it
(`Failed`) the same way; without one, a write pipe waits on its reader,
as before.
Needs: nothing.
Files: `crates/skein-io/src/layer.rs` and the pipe's state; their tests.
Shape: an optional deadline parameter.
Test shape: a write pipe whose reader stops is given up past its stated
deadline, as an append stream is; one without a deadline still waits; it
fails on the parent. Under 20 ms each.
Proof: those tests, once.
Don't: change `adopt_append` or a child's pipes.
Size: README rule 9's.

### 07.s7 Replay values charged at their total
Added during the run (decisions.md, 13.104), from the designer's answer
of 2026-10-13T00:49:24Z (skein d77165ba, llm.md 4.2). Scope: a client's
replay storage is reserved today per output item at a reasoning item's
worst case, twice (`client.rs`, the `replay_records` term,
`List::<Compact>::worst_case(tokens) x output_items x 2`); replay values
are held at their own length and charged at the total a completion's
blocks keep, within the receiving reservation; the per-item fixed
reservation goes.
Needs: nothing.
Files: `crates/skein-llm/src/client.rs` and the replay storage; their
tests.
Shape: no new type.
Test shape: a client's worst case at the standard declaration falls by
the per-item term, and a completion whose replay values reach the
receiving share is still held; it fails on the parent. Under 20 ms.
Proof: those tests, once.
Don't: change `output_items`, the reasoning share or the receiving
reservation.
Size: README rule 9's.
