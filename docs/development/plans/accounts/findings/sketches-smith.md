# smith interface sketches under clippy

The "Interface sketches" blocks of `smith/01-grants.md`, `02-notices.md` and `03-records.md` were applied cumulatively to smith `origin/main` c0c89ddc63e16c028d8d239bbdfaac2ed77067b8 in a scratch worktree on 2026-10-19, with `todo!()` bodies (each under `#[expect(clippy::todo)]`), a local stand-in (commented "Sketch stand-in") for each skein type the pin lacks (`Credential`/`Scheme`, `Report`, the new `Failure` arms; the pin is unchanged), and callers fixed only where a sketch change broke them (147 files, +1285 -763). After each session `heavy --tail 40 cargo clippy -p <changed crates> --all-targets -- -D warnings` ran. Each block shows the first run on the sketch as written (`# literal`) and the clean run after the fixes named under it; the clean lines are re-runs on the final tree, since the per-session output was not kept. The plan files changed while this ran (01's sketch is now comment-only, 03's events lines moved to `05-events.md`, `GrantError` to `04-values.md`): the verdicts say which text was tested; line numbers are today's, with the old ones in brackets.

### 01 grants
```
$ heavy --tail 40 cargo clippy -p smith-agent-service -p smith-channel-world -p smith-host-protocol -p smith-hosts-world -p smith-local-service -p smith-protocol-channel -p smith-protocol-llm --all-targets -- -D warnings
# literal: error[E0616]: field `phase` of struct `component::Component` is private (notices.rs:17; also bodies, machine, events)
# literal: error[E0624]: method `drain` is private (notices.rs:33; 12 errors in all)
# literal: error[E0277]: the trait bound `grants::GrantError: Hash` is not satisfied (protocol-llm component.rs:50)
# literal: error[E0004]: non-exhaustive patterns: `Err(grants::GrantError::Scheme)` not covered (component.rs:373)
    Checking smith-local-service v0.0.0 (.../sm-sketch/crates/smith-local-service)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.22s
heavy: task 53 (build) exit 0, waited 65s, ran 9s, log /home/free/src/rust/tmp/logs/20261019T105224-53.log
```
Verdict: compiles after three fixes to the text first tested (the full `GrantValue`, `Values`, `GrantError` sketch); today's comment-only sketch keeps only the first.
- 01-grants.md:280 [old :339] and the Files list (:85, :138): `notices.rs` as `src/notices.rs` is a sibling of `component.rs`, so it cannot touch `Component`'s private `phase`, `bodies`, `machine`, `events` or `drain`. Fix: name `crates/smith-protocol-channel/src/component/notices.rs`, a child module declared `mod notices;` in `component.rs` (a child sees its parent's private items; no visibility change).
- 04-values.md:232 [old 01-grants.md:355]: the tested `GrantError` derive lacked `Hash`, which `protocol-llm`'s `Failure` (derives `Hash`, holds `Grants(GrantError)`) needs. Fixed in the text: no derive line any more, the existing derives stay.
- 04-values.md:88 [old 01-grants.md:260]: `GrantError::Scheme` is added to an exhaustive `match self.grants.read(..)` in `component.rs` that no step names. Fix: add to 04.1's Files text "`component.rs`'s match on `grants.read` gains a `Scheme` arm".

### 02 notices
```
$ heavy --tail 40 cargo clippy -p smith-agent-service -p smith-agent-world -p smith-channel-world -p smith-domain -p smith-domain-run -p smith-domain-session -p smith-host-domain -p smith-host-protocol -p smith-host-world -p smith-inline-agent -p smith-local-domain -p smith-local-protocol -p smith-local-service -p smith-local-shell -p smith-local-world -p smith-protocol-channel -p smith-protocol-llm -p smith-protocol-llm-world -p smith-run-world -p smith-session-world --all-targets -- -D warnings
# literal: error[E0204]: the trait `Copy` cannot be implemented for this type (domain-run facts.rs:37; host-domain channel.rs:435)
# literal: error[E0382]: use of moved value: `end` (domain-run run.rs:1334; 3 in all)
# literal: error[E0277]: the trait bound `ModelFault: Hash` is not satisfied (host-domain channel.rs:444)
# literal: error: use of a disallowed macro `core::default::Default` (domain-session llm.rs:539; domain-run boundary.rs:904)
    Checking smith-agent-service v0.0.0 (.../sm-sketch/crates/smith-agent-service)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 14.74s
heavy: task 52 (build) exit 0, waited 1s, ran 15s, log /home/free/src/rust/tmp/logs/20261019T105237-52.log
```
Verdict: the text first tested needed three fixes; of today's text only the `Default` one remains.
- 02-notices.md:343: `derive(Default)` on `Report` is a `disallowed_macros` entry (programming-model 10.2; the same finding as skein 01's `Report`), once per `Report` (llm and run mirror). Fix: drop `Default`; add `Report::none()` building three `Box::default()`.
- 02-notices.md:352-353 [first text had no such line]: dropping `Copy` from `Fault` cascades, which the first text left unsaid: `End`, `Failure`, `FactKind`, `Fact`, `Answered` (facts.rs:37) and host-domain's `RunFailure` (channel.rs:435, holds `ModelFault`). Today's :352 covers End, Failure and the facts.rs kinds (that route, `EndKind`/`FailureKind` as `Copy`, I did not build: I made the facts types `Clone`); it does not name `RunFailure`. Fix: add "`RunFailure` is Clone only" to :352.
- 02-notices.md:361 [first text: `ModelFault` derived `Clone, PartialEq, Eq, Debug`]: lacked `Hash`, which `RunFailure` needs. Fixed in today's text.
- Not sketch faults, for the implementer: `match_same_arms` forces `Invalid | Refused | Unsupported` into one arm (smith-agent-service answers.rs:275 and responses.rs:894, local-protocol terminal/failure.rs:286, local-service stream.rs:227); and `manual_map` plus the `Option::map` ban means every `Option<Duration>` conversion (reset to `Wall`, to ms) needs a `?`/`let ... else` helper, not `.map`.

### 03 records
```
$ heavy --tail 40 cargo clippy -p smith-agent-service -p smith-agent-shell -p smith-agent-world -p smith-bench -p smith-channel-world -p smith-codecs-world -p smith-domain -p smith-domain-run -p smith-domain-session -p smith-events -p smith-host-domain -p smith-host-protocol -p smith-host-world -p smith-hosts-world -p smith-inline-agent -p smith-local-domain -p smith-local-process-world -p smith-local-protocol -p smith-local-service -p smith-local-shell -p smith-local-world -p smith-protocol-channel -p smith-protocol-llm -p smith-protocol-llm-world -p smith-run-world -p smith-session-world --all-targets -- -D warnings
# literal: error: large size difference between variants (smith-events records.rs:1089, `Event`: 736 vs 496 bytes)
# literal: error: this function has too many lines (101/100) (smith-domain route.rs:749, `complete`)
# literal: error: large size difference between variants (tests/agent/src/messages_referee.rs:16, `Seen`)
    Checking smith-domain v0.0.0 (.../sm-sketch/crates/smith-domain)
    Checking smith-events v0.0.0 (.../sm-sketch/crates/smith-events)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 00s
heavy: task 48 (build) exit 0, waited 0s, ran 61s, log /home/free/src/rust/tmp/logs/20261019T104404-48.log
```
Verdict: compiles after three lint fixes; the tested text included the events lines now in `05-events.md`. `cargo clippy --workspace --all-targets -- -D warnings` also exits 0 on the final tree (01+02+03).
- 05-events.md:183-185: `AnswerFailure` grows by 128 bytes, so `Event::RunCompleted` is 240 bytes over `ResponseCompleted` (limit 200): `large_enum_variant` on `smith_events::Event`. Fix: say `Event` takes `#[expect(clippy::large_enum_variant, reason = "a record is built once and written directly")]` (precedent: smith-host-domain boundary.rs:11); boxing `RunCompleted.failure` also works but touches every reader and writer.
- 03-records.md:308 ("Kind not Copy"): `Turn` loses `Copy` with `Kind`, so the referee worlds' `Seen::Turn` trips `large_enum_variant` (tests/agent/src/messages_referee.rs:57, tests/protocol-llm/src/messages_referee.rs:48). Fix: add "`Turn` is not Copy; the referee worlds box it" to :308. The sketch's other costs are covered: each `Kind::First`/`Window` pattern needs `..` and each constructor `account: None, backend: Backend::Direct`; each `turn.kind` copy becomes `.clone()`.
- 03-records.md:306 (03.1): `Option<GrantName>` nests `route::complete` one level and puts it at 101 lines (limit 100). Fix: say the unsent-unauthorized path becomes a helper fn.
- Note, no compile effect: `record::Backend` (03-records.md:311, `Program { name }`) and `smith_events::Backend` (05-events.md:175, `ClaudeCode`/`Unknown`) share a name in smith-agent-service; the mapping of a program's name to `ClaudeCode` or `Unknown` is unstated.
