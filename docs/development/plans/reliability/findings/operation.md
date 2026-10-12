# Findings: shutdown, traces and failure diagnostics (theme O)

Refs: smith `main` = `30259d8`, spike tip = `bench/smith-codex-2026-10-08` (`bf96e94`); skein `main` = `e8bc018`, skein spike = `bench/llm-shutdown-2026-10-08` (`416cffd`, `f5fca44`). Line numbers are at the spike tip unless marked `main:`.

## Summary

- **What went wrong.** Every successful hosted run ended with the agent being killed. The LLM pool in skein had no way for its owner to close it, so its idle connections lived for their 15-second keep time. The agent correctly waited for them and ignored SIGTERM after its answer, so the host killed it at grace (5 s) plus kill_after (2 s). Nobody saw this happen: the host drops the exit status, `Gone { Stopped }` says nothing, and every world ran with a tiny idle keep that hid the wait. The trace had three separate ways of losing records: facts left in the domain queue after the answer were dropped without being counted; the writer thread was never waited for (and then the process was killed); and only one prompt slot was captured per pass. On failure, the local host printed a fixed "Run failed". It threw away the typed run failure, the typed agent fault, the exit status and the agent's stderr tail. The session also dropped the provider's failure detail.
- **What the spike did.** It added `Component::close` to skein's pool and had smith's cleanup call it. It gave the trace thread a fixed 50 ms drain at exit. It added a local, trace-only drain of facts after the answer, with phase guards. It split the loss counters, and added an operator `Diagnostic` side queue in the local service that prints Rust `Debug` on stderr.
- **What the clean version looks like.** Make it a skein rule: every component that keeps entities beyond a request has an owner close with a `Closed` terminal, a process ends when everything it owns has settled under io's deadlines, and worlds assert that teardown never waits on a keep time. Facts leave the domain at one point and go to two independent sinks, each with an explicit delivery policy. The trace becomes a specified, typed record stream that ends with a summary record, written through io and not by a detached thread. Typed failures, the exit status and the bounded detail flow up to the local *domain*, which decides what the person sees. No `Debug` text and no side channel.

---

### O1. The 7-second teardown: the LLM pool had no owner close

- **Origin:** smith `a2c2e53`; skein `416cffd`. Report row "Accepted tasks lingered about seven seconds"; `smith-benchmark-2026-10-08/shutdown-fix-notes.txt`.
- **Symptom:** `runs/baseline-01_backoff-smith-1/runner.stdout`: `completion_observed_s` 23.454 against `wall_clock_s` 30.456. The 7.0016 s gap equals the host's `grace` 5 s plus `kill_after` 2 s (`crates/smith/src/local_shell.rs:216-217`). Report line 89 says the same. The trace ends at the `finish` call, with no terminal fact.
- **Root cause (verified in code):**
  1. skein `crates/skein-llm-connection/src/component.rs` (main) had no owner entry point that closes idle connections. They close only after `idle_keep` (15 s, `crates/smith/src/config.rs:400`) or when the pool needs the slot. `Cancel` of a completed call does nothing, because the pool forgets the call's token at completion. So the owner had no handle on its idle sockets.
  2. smith `cleanup` (`main:crates/smith-agent-service/src/service.rs:876`) closed the channel streams, the signal fd and the roots, but not the pool. `done()` correctly requires `owners.is_empty()`, so the agent waited for the idle keep. The waiting itself was correct lifecycle behaviour (programming-model.md §5.2).
  3. After the answer, the host enters `Exiting` with `grace` (`crates/smith-host-domain/src/domain.rs:948-970`). At the Grace alarm it sends Terminate, then Kill after `kill_after` (`:376-387`, `:1182-1186`).
  4. The agent ignores SIGTERM after answering. `io::Event::Shutdown` becomes a domain `Cancel` (`service.rs:925`), and a cancel of an answered run does nothing (`smith-domain-run` test `a_cancel_of_a_run_that_has_answered_changes_nothing`). So the agent was SIGKILLed on every successful run. This is an inference from the exact 7.0016 s interval and the code path; no run recorded the signal (O5).
  5. The same gap held in the in-process placement. `Effects::close` (`main:service.rs:312-316`) claims to close "reusable lower entities" but only sets two flags. `finish_exit` (`smith-local-service/src/service.rs:466-477`) waits on `effects.closed()`, so in-process exit waited the full 15 s idle keep, with no kill to cut it short. This comes from reading the code; it was not measured.
- **Spike change:** skein:
  - a `closing` flag, and `Refusal::Closed` at admission;
  - `fire` aborts or closes one binding per call;
  - `has_work` stays true until every connection is `Closing`;
  - `next_deadline` returns `None` while closing (`f5fca44`).

  smith: `smith-protocol-llm::Component::close` drops `next` demands and forwards the close (`component.rs:151-158`). `cleanup` calls it (`service.rs:888`). The lock is pinned to the unpublished skein `f5fca44`. Teardown fell to about 5 ms.
- **Assessment:** **keep with rework.** The diagnosis is right, and the change is at the right layer: only the pool can reach its idle bindings, so the close has to be in skein. It is principled: no timer was lowered and nothing was force-exited. But it fixes one owner instead of the rule (O3). The mechanism has gaps (O2). smith closes at the wrong moment and still ignores SIGTERM (O4). The failure was invisible (O5) and masked by tests (O6).
- **Clean fix:** O2 for the skein mechanism, O3 for the general rule, O4 for smith's sequence. Long term, every pooled resource (more providers, MCP servers over HTTP, OAuth or DNS clients, several endpoints per run with sub-agents) needs the same close.
- **Contracts:**
  - skein `llm-connection.md` §1, §4, §5 and §8;
  - smith `protocol/agent.md` §6 (the spike's enumeration should become the general text in O4);
  - `protocol/llm.md` §1 (the spike's "Owner shutdown" holds apart from the `Invalid` mapping in O2).
- **Layer:** skein `skein-llm-connection`; smith `smith-protocol-llm`, `smith-agent-service`.
- **Tests:** The spike added:
  - a skein unit test: closed admission, one binding per fire, settlement;
  - a protocol world for both dialects, closing after completion and while idle;
  - a simulated world in which the peer stays live until the client settles, plaintext and TLS, against memory bounds;
  - smith component tests;
  - an agent-process story asserting `settled_at < 1 s`.

  The skein tests are well-shaped. The smith story ties two constants together; use O6's invariant instead.
- **Benchmark:** Measured (7.002 s down to about 5 ms), but only by timing the harness. Probe: "Call finish with report 'ok'", run without and with sub-agents so that two pooled connections exist. Measure from the host's `Answered` to process exit and assert under 1 s. Once O5 lands, also assert `exit: Code(0), forced: none`.
- **Questions:** see O2 and O3.

### O2. The skein close mechanism: semantics, terminal, deadlines, refusal class

- **Origin:** skein `416cffd`, `f5fca44`; smith `a2c2e53`.
- **Symptom:** none observed. These are design defects in the fix itself.
- **Root cause / analysis:**
  1. **No closed terminal.** `close()` sets a flag outside the `down` vocabulary, and nothing comes back up when the pool has settled. smith infers settlement from its io owner table. This breaks "Close goes down, closed comes up" (programming-model.md §1) at the component level.
  2. **Abort-only.** `fire` cancels every connection not already closing. A service that wants in-flight calls to finish cannot get "drain", and the pool, not the owner, decides to cancel the owner's calls. "Policy above, mechanism below" (programming-model.md §4) argues for: refuse new calls, close idle connections now, close each busy connection when its call ends. An owner that wants abort cancels its own calls.
  3. **Deadlines hidden wholesale.** `f5fca44` makes `next_deadline` return `None` for the whole pool while closing. This is harmless under abort-only, because `terminal()` clears each connection's table. The real hazard is a stale deadline on a connection already `Closing`: `fire` `continue`s past it and the loop spins. The principled form is §5.4: deadlines are an exhaustive function of each connection's phase, and `Closing` implies none. Under drain semantics the component-wide `None` would be wrong.
  4. **Wrong failure class.** `Refusal::Closed | Endpoint => Error::Invalid` (`smith-protocol-llm/src/component.rs:341`) makes a call refused by a closing pool look like an invalid request. A `Complete` in the pass after channel end would record `CompletionFailed { Invalid }`, which misclassifies the run's last failure.
  5. `size_of::<Component>()` added to `worst_case` is unrelated; it belongs in its own commit.
- **Assessment:** **keep with rework.** Keep:
  - `Refusal::Closed`;
  - close being permanent and idempotent;
  - one binding per `fire`;
  - physical bindings retained until io settles;
  - the live-peer simulated world.

  Rework: add a `Closed` terminal; choose drain semantics; derive deadlines per phase; keep a distinct refusal class above.
- **Clean fix:**
  - `Request::Close` plus `Event::Closed` (or a `closed()` predicate the owner's `done` composes; I prefer the event).
  - Drain semantics: idle connections close at once; busy ones close when their call ends.
  - Phase-derived deadlines; remove the `if self.closing { return None }` override.
  - smith closes only after the domain is quiescent (O4), so `Refusal::Closed` is `unreachable!` in smith.
- **Contracts:** skein `llm-connection.md`:
  - §1: the component's own active, closing, closed lifecycle;
  - §4: the spike's refusal text holds;
  - §5: the drain rule and the `Closed` event replace "Active calls end once as cancelled";
  - §6: a closing connection has no call deadline, and io bounds it;
  - §8: add close racing a call terminal, and a peer that never closes.
- **Layer:** skein `skein-llm-connection`; smith `smith-protocol-llm`.
- **Tests:**
  - Protocol world: close in every connection phase crossed with a racing call terminal. Assert one terminal per call, one `Closed`, and no spin (`has_work` false and no past-due deadline).
  - Simulated world: a peer that ignores the half-close, settled by io's close deadline.
  - Put the cross product in the fuzzy sweep.
- **Benchmark:** none beyond O1.
- **Questions:**
  - (a) Should owner close drain or abort? I recommend drain, with the owner cancelling.
  - (b) An `Event::Closed` or a `closed()` predicate? I recommend the event.

### O3. The missing general rule in skein: every owner settles what it owns, and a process ends when everything has settled

- **Origin:** not in spike (latent).
- **Symptom:** O1. One owner kept entities alive on its own timer, and no rule or check said the process owner had to be able to end them.
- **Root cause:** programming-model.md §5.2 states the per-entity lifecycle ("The owner closes what it owns"; "Shutdown is an event"). It never says:
  - that a **component** keeping entities beyond requests (a pool, a cache, a listener, a keepalive) is itself an entity, with an owner close and a closed terminal;
  - how a **process ends**: shell.md has §6 "Startup" but no "The end" (when the loop exits, that only io's deadlines bound it, and that a supervisor's grace and kill are a backstop and not a mechanism).

  notes.md has no open question on this either. So `llm-connection.md` had idle expiry as the only way to close.
- **Other owners (checked):**
  - skein: `skein-channel` has `Finish`/`Close`/`Closed` (`boundary.rs:30-111`): fine. io deliberately has no "close all". HTTP, SSE and the LLM client are per-connection machines. `skein-llm-connection` is the only pool today. `skein-oauth`'s machine was not verified beyond its `next_deadline`.
  - smith: the agent's `cleanup` is a hand-written close list. `done()` is a 25-term conjunction (`service.rs:1876-1906`). The in-process `Effects::close` claimed a close it did not do.
  - Future cases: MCP-server HTTP pools, an HTTP keep-alive client, a DNS client (io.md §4), browser and fake-checkout.
- **Spike change:** none at this level.
- **Assessment:** **redo properly.** Without the rule, the next pool repeats O1.
- **Clean fix:** Text for programming-model.md §5.2:
  - "**Components follow the lifecycle too.** A step machine that keeps entities beyond the requests that made them takes a close from its owner: it refuses new work at its entrance, closes what it keeps through the ordinary lifecycle, and reports closed once everything below has settled. Its keep and idle times are policy for a live service, never the way it ends."
  - "**The end.** After its last word, a service closes every component and entity it owns, and ends when each has reported closed and io is empty. Only io's close deadlines bound this. A termination signal while closing turns the remaining graceful closes into aborts."

  Add shell.md §"The end". Add a testing-strategy.md §6 invariant: "a world that ends by itself ends without any deadline firing after the last word other than io's close and retry deadlines". A cheaper approximation: teardown stories set every keep and idle time beyond the world's horizon.
- **Contracts:** skein `programming-model.md` §5.2, `shell.md` (new section), `testing-strategy.md` §6, `llm-connection.md` §1. smith `protocol/README.md` §5, `protocol/agent.md` §6.
- **Layer:** skein foundation and components; smith service composition.
- **Tests:** implement the invariant once in skein's world harness (record which deadline table fired). smith worlds inherit it. This is skein-first mechanism, per AGENTS.md.
- **Benchmark:** O1's probe.
- **Questions:** Should the invariant live in skein's world harness (my recommendation) or per service?

### O4. smith's end of life: when to close, what SIGTERM means after the answer, and the teardown bookkeeping

- **Origin:** smith `a2c2e53`; latent sequencing issues.
- **Symptom:** O1. Also, SIGTERM after the answer had no effect.
- **Root cause / analysis:**
  1. **Close at channel end, not at the domain's end.** `cleanup` runs only once `channel_ended` (`service.rs:883`), which needs the host to disconnect after the last word (`smith-host-domain/src/domain.rs:654-661`, `:1263`). The domain finished with the pool when it answered. Closing at the answer overlaps the LLM sockets' settlement with the channel's end handshake, and avoids O2's `Invalid` misfire. On a host hang-up mid-run, the pool's aborts race the domain's own cancels.
  2. **SIGTERM after the answer is a no-op.** agent.md §6 says only "a termination signal is the domain's cancel".
  3. **Unrelated constants.** Host `grace` 5 s (`local_shell.rs:216`) equals the agent's `io.close_timeout` 5 s (`config.rs:361`). One graceful close against a silent peer uses the whole grace, and since SIGTERM does nothing, that becomes a kill at 7 s. The local host writes the agent's configuration, so it knows both values, but nothing relates them.
  4. **Bookkeeping by list.** `cleanup`, `done()` and `work_pending()` (`service.rs:1838-1906`) each enumerate queues and flags. The spike's fact changes added clauses to all three. This is the shape that let the pool be forgotten.
- **Spike change:** `service.llm.close()` in `cleanup` behind `cleanup_started`. It also covers the in-process `Effects`.
- **Assessment:** **keep with rework.** The call is correct and needed. The moment, the signal semantics and the bookkeeping should be redone.
- **Clean fix:**
  - A teardown phase enum `Live → Answered → Closing → Closed`, matched exhaustively in one place:
    - when the answer is sent, close each owned component (LLM pool, machine, roots, trace sink per O7) and finish the channel;
    - when the channel ends, close its streams;
    - done = every component closed and io empty.

    `work_pending` and `done` derive from the phase plus component predicates.
  - io `Shutdown` while `Answered`/`Closing` switches outstanding graceful closes to `Abort`. While `Live` it stays the domain's cancel.
  - The local host refuses (or derives) a grace below the agent's teardown bound (io close timeout plus a margin). Other hosts get the same documented relation.
- **Contracts:**
  - `protocol/agent.md` §6, replacing the spike's enumeration: "after its answer the agent closes everything it owns and finishes its channel. It exits once each has settled. Only io's close deadlines bound this. A termination signal after the answer aborts what is still closing."
  - `domain/host.md` §4: a host's exit grace exceeds the agent's teardown bound.
  - `protocol/hosts.md` §5.2: the local host derives its grace.
- **Layer:** `smith-agent-service`; `smith` local shell limits.
- **Tests:**
  - Agent-process world: answer, then exit with no host signal (O3 invariant); SIGTERM after the answer with a peer that ignores the half-close, exiting by abort before io's close deadline; host hang-up mid-call gives one terminal per call, `Cancelled` and not `Invalid`.
  - Local-process world: the same in both placements.
- **Benchmark:** O1's probe, plus one interrupt after the answer: the exit must follow at once.
- **Questions:** Should the pool close at the answer (my recommendation) or at channel end?

### O5. Supervision cannot see a forced exit: exit status dropped, `Gone { Stopped }` carries no evidence

- **Origin:** not in spike (latent). It is why O1 went unnoticed.
- **Symptom:** the baseline's stderr shows only "local host started", although the agent was terminated and then killed on every run.
- **Root cause (verified):**
  - `smith-host-protocol/src/process.rs:311` matches `IoEvent::Exited { owner, .. }` and drops `kernel::Exit` (`Code(u8)`/`Signal(u32)`).
  - The host's `Event::Exited { owner }` (`boundary.rs:81`) has no status.
  - `Gone { end: End::Stopped, detail }` (`domain.rs:1373`) does not record whether the host signalled the process. `End::Stopped` cannot tell "exited 0" from "killed after grace" or "crashed with 101".
  - On main, the local service ignored `Gone.detail` entirely.
- **Spike change:** none for exit evidence.
- **Assessment:** **redo properly.**
- **Clean fix:** Carry `Exit` through smith-host-protocol. Use `End::Stopped { exit, forced: No | Terminated | Killed }`, or add a `Fact::Forced`. When a run answered and was then forced, the local domain shows a warning ("agent did not exit within its grace; terminated"). It is not a run failure. Temper gets the same evidence.
- **Contracts:** `domain/host.md` §4 (how the process ended); `protocol/hosts.md` §3 and §5.1.
- **Layer:** `smith-host-protocol`, `smith-host-domain`, `smith-local-domain`.
- **Tests:** host world: an agent that exits 0, exits non-zero, ignores Terminate, and exits on Terminate after its answer. Assert the evidence and one observation each.
- **Benchmark:** O1's probe asserts `forced: No`.
- **Questions:** Should a forced exit after an accepted answer change the local exit status? I recommend a warning only.

### O6. The tests hid the linger: worlds ran with tiny keep times and peers that hang up

- **Origin:** not in spike (latent); the spike's tests partly address it.
- **Symptom:** all worlds passed while every shipped run was killed.
- **Root cause (verified):**
  - `tests/local-process/src/process.rs:56` sets `idle_keep = Duration::from_millis(5)` (added in `868006d`). The real-loop tier reuses `process::lower_configuration()`.
  - The agent-process world uses 1 s (`tests/agent-process/src/lib.rs:73`), and the simulator jumps its clock to the deadline.
  - skein's simulated LLM world's `Judge` made the fake peer `shutdown()` once the calls passed (`tests/llm-connection/src/simulated.rs`), so the pool ended on the peer's EOF.
  - smith `testing.md` §6 checks "nothing is left running" but not that the agent ended by itself.
- **Spike change:** skein's judge keeps the peer live until the client is empty (`close_owner`), which is right. smith adds `settled_at() < 1 s`. The 5 ms override stays.
- **Assessment:** **keep with rework.** The skein judge change is exactly right. The smith assertion bounds the whole run, not the teardown, and would pass with any idle keep below the run's duration.
- **Clean fix:** the O3 invariant, plus a configuration rule: teardown-relevant keep and idle times stay at shipped values or above the world's horizon. Remove the 5 ms override. Fakes never hang up to help a client settle unless the scenario is about peer hang-up.
- **Contracts:** smith `docs/design/testing.md` §6: add "the agent exited on its own after its answer; its host sent no signal". skein `testing-strategy.md` §6.
- **Layer:** skein world harness; smith worlds.
- **Tests:** as stated. Cost is negligible, because simulated time is free.
- **Benchmark:** none beyond O1.
- **Questions:** none.

### O7. Trace writer: detached thread, unsettled at exit; the spike's 50 ms drain

- **Origin:** smith `a54ba26`.
- **Symptom:** records accepted by the writer could be missing after a normal exit. On main, `agent_library` polled the file for up to 1 s after `drop(agent)` (`main:crates/smith/tests/agent_library.rs:170-186`). In the baseline, the SIGKILL from O1 also killed whatever the writer held.
- **Root cause (verified):** `crates/smith/src/trace.rs` (main) spawns a `std::thread` draining `sync_channel::<String>(32)` into a `File`. On exit, `finish` read the counters and returned without waiting. More basically, the shell writes a file outside the ring, against programming-model.md §1 ("One thread, one loop") and §2.1 ("Anything that can be a ring operation is one"). The reason is that io has no append file stream (io.md §5: "File streams come later").
- **Spike change:** `finish()` drops the sender and waits up to `SHUTDOWN_GRACE` = 50 ms (`trace.rs:23`, `:185-191`). A stalled writer stays detached and its pending records count as dropped. agent.md §5 now spells out 50 ms, 32 lines and 400,000 bytes.
- **Assessment:** **redo properly**, keeping the spike as a stop-gap.
  - 50 ms is a hidden policy constant in the shell.
  - The trace's settlement sits outside the lifecycle every other descriptor follows.
  - The unit tests use real threads, `/tmp` files and wall-clock asserts, against AGENTS.md's "deterministic clocks".
  - The `agent_library` test now blocks real time inside a simulated `iterate`.
  - The contract text puts implementation sizes into a design document.
- **Clean fix:**
  - **Preferred:** the trace is an io stream. The shell opens the file `O_APPEND` beneath a root and hands it to io as an adopted write stream (`Io::adopt_write_pipe` exists for stdout). The protocol layer encodes records with `Writer::new(len)` under io's output cap. Shutdown is `Finish` then `Close`, bounded by io's close deadline, and `done()` waits for it like any descriptor. There is no thread and no 50 ms constant. Two things are unverified: whether the pipe stream works unchanged on a regular file, and what a stalled filesystem needs (a write deadline per io.md §5). That is skein io work to do first.
  - **Alternative:** keep a shell thread, written down as a §2.1 exception, with a configured drain deadline, a join, and the outcome in the trace's end record.
- **Contracts:** smith `protocol/agent.md` §5 (delivery rule per O10 instead of the numbers); skein `io.md` §5 (append stream), `programming-model.md` §2.1, `notes.md` "The trace queue".
- **Layer:** skein io; smith shell moves into the agent protocol layer as a sink.
- **Tests:** agent-process world with a simulated file stream. The referee checks every accepted record and the end record, deterministically. The real loop covers the real file. Drop the wall-clock unit tests.
- **Benchmark:** any run; assert the final record is the end record with zero loss.
- **Questions:** Should the trace move to io (my recommendation, with skein append streams first) or stay a documented shell thread?

### O8. Terminal facts were dropped without being counted after the answer

- **Origin:** smith `136b9c4`. Report row "Terminal facts disappeared after answering"; `trace-terminal-capture-review.json`.
- **Symptom:** traces lacked `Answered`; the baseline trace ends at `finish`. The harness detects the end of a run from trace facts (`terminal.py`), so it missed a failed run: `runs/iteration1-02_line_decoder-smith-1` took 305 s with `completion_observed_s: None` (report line 91).
- **Root cause (verified):** `domain_down` drained facts only `if admitted && !answer_sent && !channel_ended` (`main:service.rs:1331`). Requests are handled first, so the `Answer` sets `answer_sent`, and the same step's facts (`Answered`, final `Used`) were never popped. `done()` ignored the fact queue. They were **not counted**, which breaks "dropped and counted" (agent.md §5; channel.md §7). This is a bug, not a design choice.
- **Spike change:** an `else if answer_sent || channel_ended` local-only drain (`service.rs:1366-1376`). `done()` requires `!domain_facts_pending` (`:1878`), with a matching `work_pending` clause (`:1845`) and `Domain::has_facts`. The loss counters are split (`agent_shell.rs:91-117`). The test asserts one `Answered … Failed(Budget(Turns))` by matching a `Debug` string.
- **Assessment:** **keep the invariant, redo the mechanism.** Keep "the process does not end until every emitted fact has reached its sinks or been counted". The mechanism adds a third phase predicate in three functions, and the test matches `Debug` text.
- **Clean fix:** O9's single drain makes this case disappear. Keep `done()` waiting for the fact queue to empty.
- **Contracts:** `protocol/agent.md` §5: "every fact reaches each sink or is counted, up to the process's end". The spike's "remain eligible for local capture" sentence becomes redundant.
- **Layer:** `smith-agent-service`.
- **Tests:** agent-process world. The referee checks trace = emitted − counted loss (via the O10 summary record). Cover every answer kind (accepted, failed, refused, parked, cancelled), each with its terminal fact exactly once, using typed decoding.
- **Benchmark:** assert exactly one terminal fact, equal to the host's answer.
- **Questions:** none.

### O9. Fact routing: the trace sits downstream of channel room; three phase booleans in three functions

- **Origin:** smith `6a1dfb2` (refining `136b9c4`); latent coupling on main.
- **Symptom:** the first `136b9c4` version drained pre-admission facts to the trace. That took them away from the channel's later projection and caused a busy loop, as the review's peer-guard refinement describes. Post-spike task-07 runs still lose 32 to 70 channel facts per run (`runs/iteration{5,6}-07_shell_cwd-smith-1`, `iteration7-07_shell_cwd-smith-3` stderr).
- **Root cause:** `trace_fact` is called only inside the channel projection loop (`service.rs:1338-1365`), which stops when channel queues lack room. While the host is slow, facts wait in the domain queue, which then drops (`domain.lost`) for **both** sinks. So trace completeness depends on the host's backpressure. This also contradicts channel.md §7, which says facts that don't fit are dropped and never held back. Phase lives in `admitted`, `answer_sent` and `channel_ended`, combined differently in `domain_down`, `work_pending` and `done`.
- **Spike change:** gated the local drain and the `work_pending` clause on `answer_sent || channel_ended`. Added white-box tests that set private fields.
- **Assessment:** **redo properly.** Strict-subset review of `service.rs` (about 35 production lines; the rest of the roughly 330 added lines are `mod tests`):
  - Compliant: `match` on `Option`; a bounded `for`; no own-enum `if let`; no closures or tuples; saturating counters.
  - Against §5.4 in spirit: phase as booleans instead of an enum (`#[expect(clippy::struct_excessive_bools)]` already flags it), repeated in three functions.
  - The tests (`cancelled_preparation`, `assert_local_terminal_capture`, `pre_admission_backpressure_…`, `retained_active_facts_alone_…`, `settlement_waits_…`) hand-step the root domain with a 60-line charter fixture. They then set `answer_sent`, `admitted = None` or `limits.queue = 1` to fake states that may be unreachable. This is world material done as unit tests on private predicates.
- **Clean fix:** one drain, two independent sinks.
  - **Drain:** `domain_down` pops up to the queue bound every pass, regardless of channel phase or room.
  - **Trace sink:** receives everything, under its own policy (O10).
  - **Channel sink:** projects only while in its projecting phase (after `Admitted` is sent, before the `Answer`) and only with reserve room; the rest is counted "not projected". The channel component already owns that phase (channel.md §5), so the service needs no fact booleans. Pre-admission facts are not projected.
  - **Bookkeeping:** `work_pending` keeps only "domain has facts and a sink can take them"; `done` keeps "domain facts empty".
  - **Optional:** one ordered request-and-fact queue, only if a host needs exact interleaving.
- **Contracts:** `protocol/agent.md` §5 (one drain, two sinks, each counting its loss); `protocol/channel.md` §7 (projected only while admitted and before the answer). `domain/run.md` §11 is unchanged.
- **Layer:** `smith-agent-service`, `smith-protocol-channel`.
- **Tests:** agent-process world with a scripted host that delays acknowledgements during a fact-heavy run with sub-agents. Assert: complete trace (or loss counted at the trace sink only), channel loss counted, the scheduler parks, and the answer unchanged ("facts change nothing"). The fuzzy suite sweeps window sizes. Delete the private-field unit tests.
- **Benchmark:** "Read these 20 files and report their first lines" with sub-agents. Assert trace loss 0 and channel loss reported.
- **Questions:** Hold facts until the channel can project them, or drop and count at the channel (my recommendation, per channel.md §7)?

### O10. The fact stream's delivery guarantee, and what the trace *is*

- **Origin:** not decided in the spike. skein `notes.md` "The trace queue" is still open: "drop and count, or hold the step".
- **Symptom:** before the spike, five chained buffers each dropped on their own: child domain queues, the root fact queue, the channel reserve, `trace_facts` and the writer channel. One summed stderr line ("40 observations were dropped") couldn't say where. The harness parses `format!("{fact:?}")` (`trace.rs:111`) using regexes over `Debug` (`terminal.py`), and the stderr counters by regex (`smith_smoke.py` `observation_loss`). The trace has become an interface with no contract (agent.md §10, "Traces kept: their format": open).
- **What the trace is (verified):** a shell-crate JSONL capture (`crates/smith/src/trace.rs`). Records are:
  - `fact` (a `Debug` string);
  - `prompt` (hex of the whole encoded prompt);
  - `call` (hex id, name and input);
  - `tool` (result size);
  - `completion` (UTF-8 text);
  - `usage` (a `Debug` string).

  Each is stamped `at_ns` at drain time, not emission time. Channel facts are also stamped at projection (`service.rs:1356`), although channel.md §8 says "when, as a duration since the run's start". The writer is a thread (O7). There is no end record; the counts go to stderr only.
- **Assessment:** a design gap in `protocol/agent.md` §5 and in skein's foundation. Silently losing terminal facts was a bug (O8). Lossy liveness facts are fine. A trace that cannot be trusted to be complete, or to say that it is not, is a gap.
- **Clean fix (recommendation):**
  1. **Inside the process, facts are not lost.** Resolve skein's open question as "hold the step": the root fact queue is a step output with room reserved like `MAX_OUT`. Five cascading losses become none before the sinks.
  2. **Each sink has an explicit policy and counts its own loss.**
     - Channel: drop and count (liveness); it never holds the run back.
     - Trace: a configured mode. *complete* (default) backpressures through the stream's output cap; disk latency is milliseconds against seconds-long turns. *best effort* drops and counts. A filesystem stall past a deadline becomes *abandoned* and is recorded.
  3. **The trace says what it is.**
     - A header record: schema version, capture policy, mode, agent version, the limits that matter.
     - A summary record at the end: per-kind counts emitted, per-sink loss, the end reason, the typed answer.
     - A reader can then prove completeness without stderr.
  4. **Typed, versioned records** sharing the channel fact vocabulary (channel.md §8), plus content kinds. Timestamps are taken at emission, relative to the run's start. No `Debug` text anywhere.
  5. Keep the spike's separate counters, but in the summary record.

  Alternative: stay lossy and add only the summary record and schema. It is cheaper, but evaluators then discard incomplete traces.
- **Contracts:**
  - skein: `programming-model.md` §3 and `notes.md` (resolve "The trace queue").
  - smith: `domain/run.md` §11; `protocol/agent.md` §5 (modes, header and summary; drop the spike's 50 ms, 32-line and 400,000-byte numbers); `protocol/channel.md` §8 (emission timestamps).
  - Possibly a new `protocol/trace.md`, closing agent.md §10.
- **Layer:** skein foundation; smith domain (timestamps, room-first facts), agent protocol layer (sinks, encoding), shell (opening the file).
- **Tests:** keep "facts change nothing". Add "complete mode loses nothing": a slow trace stream gives summary loss 0 and an unchanged answer. Add codec round-trip and size-bound tests.
- **Benchmark:** `./benchmarks` reads the typed trace (or the host's answer) and never parses `Debug`. Probe: assert header, summary, zero loss, one terminal equal to the host's answer, and usage totals matching.
- **Questions:**
  - (a) Should facts be lossless inside the process (recommended)?
  - (b) Should the trace default to complete mode (recommended)?
  - (c) Should there be a separate `protocol/trace.md`?

### O11. Prompt capture loses prompts: one slot per pass, and a 64 KiB cap that grows quadratically

- **Origin:** not in spike (latent). The spike only counts the loss and rejects oversized prompts before formatting (`trace.rs:114-118`).
- **Symptom:** `iteration6-07_shell_cwd-smith-1` traced 104 prompts against 128 usage records ("trace prompts: 24"). `iteration7-07…-smith-3` traced 160 against 175 (16 lost).
- **Root cause (verified):**
  - `trace_prompt: Option<…>` (`service.rs:118`) holds one prompt. A second `Complete` in the same pass, which sub-agents cause often, is counted lost (`:1534-1536`). The shell drains one per iteration (`agent_shell.rs:164`).
  - `capture_prompt` refuses prompts over `TRACE_PROMPT_BYTES` = 65,536 (`:19`, `:686`).
  - Each record is the whole conversation, so trace size grows quadratically. Past 64 KiB, no prompt is recorded at all.
- **Spike change:** counting only.
- **Assessment:** **redo properly.**
- **Clean fix:** a bounded prompt queue sized by the calls startable per pass (the pool's connection count). Record each turn's new messages, or a transcript-position reference, rather than the whole prompt. Take the cap from configuration, with a typed "truncated" record instead of a drop.
- **Contracts:** `protocol/agent.md` §5 ("everything" defined per turn); `transcript.md` if positions are referenced.
- **Layer:** `smith-agent-service`, trace sink.
- **Tests:** agent-process world with three sessions starting calls in one pass; one prompt record per completion.
- **Benchmark:** with sub-agents, the count of prompt records equals the count of usage records.
- **Questions:** Deltas (my recommendation, so a trace stands alone) or transcript references?

### O12. Trace record storage: `Box<str>` shrink and the checked reserve

- **Origin:** smith `136b9c4` (part). Report row "burst trace storage retained excess capacity".
- **Symptom:** queued `String`s kept spare capacity, so the reserve under-counted.
- **Root cause:** `serde_json` building `String`s with capacity above their length.
- **Spike change:** `into_boxed_str()` (`trace.rs:101`). A checked-const `MEMORY_RESERVE` that includes `FORMAT_BYTES` = 2 MiB (`:26-38`). That figure is an estimate, not a computed bound; the review says it is "not a measured whole-process RSS ceiling".
- **Assessment:** **drop in the clean version.** Sized encoding in the loop (`Writer::new(len)` into the io stream) has no growth and no scratch. The bound then follows from the record limits and the output cap, through each container's `worst_case`.
- **Contracts:** drop the byte figures from agent.md §5.
- **Layer:** smith protocol layer.
- **Tests:** a memory-tier check of the trace sink at its worst case, against the counting allocator.
- **Benchmark:** none.
- **Questions:** none.

### O13. Hosted-agent failure diagnostics: typed faults erased at the local boundary; the spike added a side channel

- **Origin:** smith `ecb7562`. Report row "Failure terminals were hard to diagnose".
- **Symptom:** `runs/diagnostics-02_line_decoder-smith-1`: stdout "Run failed", stderr only "local host started". After the spike (`diagnostics2-02…/agent/stderr.txt`): "agent run failed: Model(Completion { failure: Limit, evidence: Response })", then "agent ended (Stopped): smith: agent started; …; 3 observations were dropped".
- **Root cause (verified):** the host domain hands up typed faults and the tail, as `domain/host.md` §4 requires ("the agent's failures, typed …, each with bounded detail for operators"). The local service then erased them:
  - `Faulted { .. }` became `ExternalEvent::Failed`;
  - `Gone { .. }` became `ExternalEvent::Gone`;
  - a failed `Answered` became `ExternalFinal::Failed`.

  All three are untyped (`smith-local-domain/src/boundary.rs:23-67`). The local domain shows the fixed "Run failed" (`domain.rs:949-956`).
- **Spike change:** a `Diagnostic` enum and queue in the local *service* (protocol layer): `Faulted`, `Gone { end, detail }`. A `diagnostic_failure: bool` decides whether a `Stopped` tail is shown (`service.rs:139`, `:669-760`). Routing pauses while the queue is full (`:932`). The shell `eprintln!`s `{fault:?}` and `from_utf8_lossy(detail)` (`local_shell.rs:129-140`).
- **Assessment:** **redo properly.**
  - It got right: operator detail stays out of turns and prompts, it is bounded, it pauses rather than drops, and it is in the worst case.
  - It got wrong:
    - It is a side channel around the domain, and the protocol layer decides what to show (`diagnostic_failure`), against programming-model.md §4.
    - `Debug` is the operator vocabulary.
    - It covers only the child placement: in-process `Final::InProcess` gets nothing.
    - The person, who is the operator for the local host, still sees the fixed "Run failed".
- **Clean fix:** typed local boundary:
  - `ExternalEvent::Failed { fault }`;
  - `Gone { end, exit, forced, detail }` (O5);
  - `ExternalFinal::Failed { failure }` (O14).

  The local domain decides what to show and when, and both placements converge there. `smith-local-protocol` renders a closed vocabulary to the terminal, followed by the bounded detail. These lines never enter saved turns or prompts. stderr keeps a matching line for unattended use.
- **Contracts:**
  - `domain/README.md` §1, extended to the operator. "A failure is never replaced by a fixed message" already applies, and "Run failed" breaks it.
  - `domain/host.md` §8 (what the person sees on failure).
  - `protocol/hosts.md` §5.1: rewrite the spike's paragraph to say the local domain shows typed failures and detail, instead of "the shell drains the diagnostic queue".
- **Layer:** `smith-local-domain` (decision), `smith-local-service` (typed translation only), `smith-local-protocol` (rendering).
- **Tests:** local-process world for each `host::Fault`, each `End` and a failed answer, in both placements. Assert the shown text, and that turns and prompts never contain it (moved from the spike's unit tests to the referee).
- **Benchmark:** an unspawnable binary path, or an agent config over its memory ceiling; assert one typed line and the detail.
- **Questions:**
  - (a) Show diagnostics on the terminal next to "Run failed" (recommended) or on stderr only?
  - (b) Should the exit status differ by failure class?

### O14. Typed run failures, and the fixed "Run failed"

- **Origin:** smith `e69bfc3`.
- **Symptom:** same as O13.
- **Root cause:** `protocol::answer_to_local` reduces `RunResult::Failed { failure }` to `ExternalFinal::Failed`. Even the in-process path maps Model, Budget, Policy and Stale to the same "Run failed" (`smith-local-domain/src/domain.rs:950-956`).
- **Spike change:** `Diagnostic::RunFailed { failure }` from `route_host`, printed as `{failure:?}`.
- **Assessment:** **redo properly** (as O13): the right data in the wrong place and format.
- **Clean fix:** `ExternalFinal::Failed { failure }`. Final text "Run failed: <class in words>" for both placements, for example "the turn budget (64) ran out" or "the model's response exceeded a receiving limit (tool input 2,048 bytes)". The detail comes from O16. A total match over `run::Failure` means a new kind breaks the build until it has words.
- **Contracts:** `domain/host.md` §8; `protocol/hosts.md` §5.1; `domain/run.md` §10 (the host should show the typed failure, not only act on it).
- **Layer:** `smith-local-domain`, `smith-local-service`, `smith-local-protocol`.
- **Tests:** local world, one scenario per `run::Failure` variant in both placements.
- **Benchmark:** a budget of 1 turn and a two-turn task; assert the budget wording and that the trace summary agrees.
- **Questions:** none beyond O13.

### O15. Spawn and admission diagnostics

- **Origin:** smith `8466afb`.
- **Symptom:** `End::Busy`, `Invalid(_)` and `Unspawned` printed nothing without a prior fault flag.
- **Root cause:** O13's `diagnostic_failure` flag. Entrance ends are their own failure.
- **Spike change:** an exhaustive `match end`; an empty detail still prints the end (`local_shell.rs:133-138`).
- **Assessment:** **keep the rule, redo the place** (the local domain). `Stopped` needs O5's evidence to know when the tail matters.
- **Clean fix:** part of O13 and O5.
- **Contracts:** the spike's hosts.md §5.1 sentence holds as a rule.
- **Layer:** `smith-local-domain`.
- **Tests:** local world, one per `End` variant.
- **Benchmark:** as O13.
- **Questions:** none.

### O16. The failure detail the operator needs is dropped below the operator

- **Origin:** not in spike (latent).
- **Symptom:** `diagnostics2-02` ends with `Model(Completion { failure: Limit, evidence: Response })`. Finding *which* limit (the 2 KiB decoded tool-argument cap) took more work (report line 91). The agent's own stderr says nothing about the failure.
- **Root cause (verified):**
  - The session drops the detail: `smith-domain-session/src/session.rs:612` `drop(detail)`, by design per `smith-protocol-llm/src/failure.rs:12-14` ("the policy consumes/drops it after observation"). Nothing observes it. The fact is content-free, and `domain::Content` has no failure kind.
  - skein: `DecodeError::TooLarge` is classed `Limit` but given the text "malformed provider event" (`crates/skein-llm/src/client.rs:644-649`). Non-JSON HTTP errors say "provider HTTP error" without the status (`:762-766`).
  - The agent logs to stderr only "when it ends without an answer" (agent.md §6; `agent_shell.rs:114` uses `{:?}`). A failed answer is an answer, so nothing is written.
- **Spike change:** none.
- **Assessment:** **redo properly.** README §1 is honoured towards the LLM and broken towards the operator.
- **Clean fix:**
  - skein: typed `Failure::Limit { which, bound }` (completion bytes, parts, decoded call input, SSE line or event, head). The HTTP status goes into the failure; provider text stays, clipped.
  - smith session: keep the last failure detail (bounded). Emit `Content::Failure { owner, detail }` to the trace under capture policy. Carry the typed limit into `run::Failure::Model(Completion { … })`, so O14 can name it.
  - Agent stderr: one line per *failed answer* too, with the typed failure and the last detail, bounded. Hosts already keep the tail, so every placement gets it.
  - Feeding a tool-input `Limit` back to the LLM instead of failing the run belongs to the limits theme.
- **Contracts:**
  - skein `llm.md`, `llm-connection.md` §4 (the limit kind is part of the class);
  - smith `domain/session.md` (detail kept for the operator, never the LLM's history by default);
  - `domain/run.md` §10–11;
  - `protocol/agent.md` §6 (stderr line on a failed answer).
- **Layer:** skein `skein-llm`; smith `smith-domain-session`, `smith-domain`, `smith` shell.
- **Tests:**
  - skein client tests: each limit gives its kind; a non-JSON HTTP error carries its status.
  - smith session world: the detail reaches trace content and never a prompt.
  - Agent-process world: stderr's last line for a failed answer.
- **Benchmark:** (1) an invalid model name gives a provider 4xx; assert the class and the provider's message are shown. (2) A tiny completion byte cap; assert the shown text names the limit and its bound.
- **Questions:** Is provider failure text safe on stderr by default? It could echo request content. I recommend stderr clipped to `detail_bytes`, and the trace only under `calls`/`everything`.

---

## Cross-theme dependencies

- **Limits and deadlines:** O16's typed `Limit` kinds and the "feed back vs. fail" choice change what the limit fixes report. O4's grace vs. close-timeout relation joins the other derived host limits.
- **Sub-agents and turn budget:** concurrent sessions cause O11's prompt loss and more pooled connections at teardown (O1). Budget failures are the commonest case O14 must word.
- **Startup and waiting (empty brief):** pre-admission facts and the `Admitted` ordering (O9) depend on how admission is sequenced.
- **Channel and host protocol:** O5 (`End`/`Exit` evidence) and O13 (typed local boundary) change host-domain and local-domain vocabularies. The channel's fact reserve (`fact_reserve_frames: 1`, `fact_reserve_bytes: 128` in `config.rs`) explains the 32–70 channel losses per run.
- **Benchmark harness:** `terminal.py` and `smith_smoke.py` parse `Debug` facts and a stderr regex. O10's typed trace with header and summary replaces both. Completion should come from the host's answer, not from trace facts.
- **skein dependency and publishing:** the spike pins an unpublished skein `f5fca44`. O2 and O3 change that skein API before any smith consumer lands. O7 needs skein io append streams first.