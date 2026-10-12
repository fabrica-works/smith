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
