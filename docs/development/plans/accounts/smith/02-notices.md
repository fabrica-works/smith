# smith 02: notices

An account's failures get the design's classes end to end: the session
keeps skein's refused and unsupported, never retried, with the
provider's report; the run ends limited, refused or declined; the agent
raises the notices; the host types each fault, exits by it and tells
the person of a limited or refused account.

Needs at start: smith 01.2's channel version 4 and 01.3's skein classes
on `origin/main`; 02.2 also waits for smith 04.1's grant value union.

## Read first

- smith's design at `137a1cb4`: `protocol/channel.md` 4, 5;
  `protocol/limits.md` 2.4; `protocol/hosts.md` 5.1; `domain/session.md`
  4; `domain/run.md` 10, 11; `domain/host.md` 2, 7, 8; `shell.md` 4.3,
  7.5. skein's at `378fd8c`: `llm.md` 2.5, 4.6 to 4.8.
- skein `docs/foundation/programming-model.md` 2.1, 4.4, 6.2, 10.4;
  `testing-strategy.md` 2.1, 2.2, 8, 10.
- The README's Rules; code-organization.md (6's owners, 7's Don't, 8's
  sizes); decisions.md 3, 10, 13, 16 and design questions 6, 7, 14.

## What the design asks

- session.md 4: refused, exhausted and unsupported are never retried.
- channel.md 4, limits.md 2.4: the three notices; the reset is optional.
- run.md 10, 11: how the run ends; the reset measured at the answer.
- host.md 7, 8, hosts.md 5.1: a notice fails the run; typed, exhaustive.
- shell.md 4.3, 7.5: limited exits 1, refused 3, in 4.3's words.

## Choices

- The model's own refusal is `Declined`, keeping its wire tag 3 (its
  arm named `declined` in version 4's schema, smith 01.2) and the event
  vocabulary's word `refused`; the account's is `Refused { account, code }`; the
  reset is optional (decision 3; design question 7).
- The run's faults carry what the agent raises: limited, refused,
  declined and the completion's report. The program's two (a login
  lost, a program that could not run) are decoded by the host, raised
  by nothing until `accounts-sources` (decision 7).
- The event vocabulary and its words are smith 05.1's and 05.2's: until
  then the agent and the local service write version 3's nearest.
- Downstream defaults: a refused notice is handled as rejected was, a
  new fault as a model failure (decision 16). Codec and world tiers only
  (decision 10).

## Rules that bind it

- The channel is version 4 from smith 01.2; nothing here bumps it.
- Transitional code names its end in a comment: the increment, here or
  in smith 03, that removes it.
- A provider's reason is diagnostic: it goes in the refused notice and
  the failure's detail, never in a fact or a turn.
- Files over 800 lines grow by at most ~100 per increment
  (code-organization.md, 8): in production `session.rs` (3665),
  agent-service's `service.rs` (2968), `run.rs` (2584), local-domain's
  `domain.rs` (2115), `responses.rs` (1870), `sinks.rs` (1513),
  host-domain's `domain.rs` (1496), local-service's `process.rs`
  (1423), domain-run's `boundary.rs` (1086), `route.rs` (1076) and
  protocol-channel's `translate.rs` (873); the listed tests likewise.

## Increments

### 02.1 The session's and the run's account failures
Scope: session.md 4; run.md 10, 11; skein llm.md 2.5; design question 7.
Needs: smith 01.3's skein classes; smith 01.2's version 4.
Files: `crates/smith-domain/src/{llm,boundary,route,peer,translate}.rs`,
  `crates/smith-domain-run/src/{boundary,facts,run,agent,lib,tests}.rs`,
  `crates/smith-domain-session/src/{llm,session,compaction,tests}.rs`,
  `tests/protocol-llm/tests/{component_boundary,adapter_boundary}.rs`,
  `crates/smith-agent-service/src/{events,calls,children,sinks}.rs`,
  `crates/smith-protocol-llm/src/{boundary,component,failure}.rs`,
  `crates/smith-domain/src/{feedback,tests,translation_tests}.rs`,
  `crates/smith-agent-service/src/{service,answers,responses}.rs`,
  `crates/smith-inline-agent/src/{boundary,workspace}.rs`,
  `tests/{agent,protocol-llm,session}/src/translate.rs`,
  `crates/smith-inline-agent/src/{translate,tests}.rs`,
  `crates/smith-domain/src/tests/completion_retry.rs`,
  `tests/agent/tests/{feedback_table,compaction}.rs`,
  `tests/protocol-llm/tests/adapter_continuation.rs`,
  `tests/protocol-llm/tests/completion_deadlines.rs`,
  `tests/run/tests/{simulation,fuzzy_simulation}.rs`,
  `crates/smith-protocol-llm/src/{guard,tests}.rs`,
  `crates/smith-protocol-channel/src/translate.rs`,
  `tests/{agent,protocol-llm}/src/{wire,world}.rs`,
  `crates/smith-local-service/src/service.rs`,
  `crates/smith-host-protocol/src/tests.rs`, `tests/local/src/world.rs`,
  `tests/channel/tests/stories.rs`, `tests/host/src/inline.rs`,
  `tests/run/src/partner.rs`, `tests/run/src/child_final_referee.rs`
  (`judge`'s `FactKind::Ended { end: End::Budget(Exhausted::Turns), ..
  }` takes `EndKind`), `tests/agent/src/memory.rs` (its `Event::Failed`
  literal; ruling 122).
Shape: the session's `llm::Failure` gains `Refused` and `Unsupported`,
  and `Exhausted`'s `retry_after` is an `Option<Duration>`; `llm::Report
  { code, request, field }` (new) holds skein's report, each part empty
  when not given: `Report::none()` builds it from three
  `Box::default()`s, for `disallowed_macros` bans `Default`, and is
  `#[must_use]`, as `run::Report::none()` is. Neither new
  class is retried (`backoff` refuses them) and both end the session
  `Failed`. `failure.rs` maps skein's classes one to one, ending smith
  01.3's `failed()` mapping; the domain's `Event::Failed` gains `report`
  (the session's does not), and each literal of it takes one.
  `route.rs`'s `failed` keeps the last report on the peer, as it keeps
  `retry_at`, sets `retry_at` only for a reset said, and pushes
  `Request::Exhausted { account, retry_after: Option<Duration> }`, which
  agent-service sends to `send_limited` as it is, ending smith 01.2's
  zero-as-absent. The run: `CompletionFailure` gains `Refused` and
  `Unsupported`, and `Exhausted { retry_after: Option<Duration> }`;
  `run::Report` (new) mirrors the session's, `none()` too;
  `Fault::Refused` becomes `Declined`; `Fault::Exhausted` becomes
  `Limited { account, reset_at: Option<Time> }`; `Fault::Refused {
  account, code }` is new; `Completion` gains `report`. `translate::end`
  gives a session failed by `Exhausted` the run's `Limited`, by
  `Refused` the run's `Refused` with the report's code, and the rest
  `Completion` with the report. `Fault`, `End` (which holds one) and
  `run::Failure` lose `Copy` and keep `Clone`, for a code is bytes: each
  listed file that matches or copies one borrows or clones it, as
  `tests/run/src/partner.rs` and `tests/run/tests/simulation.rs` do
  (they use `CompletionFailure::Limit`). `facts.rs` stays `Copy` and
  content-free (run.md, 11): `FactKind::Ended` and `Answered::Failed`
  hold `EndKind` and `FailureKind` (new), the kind without code or
  report. protocol-channel's `translate.rs` writes version 4's limited
  (the reset measured at the answer), refused, declined, the two classes
  and `CompletionFault`'s code, request id and field. `feedback.rs`
  words `limited`, `account-refused` and, for the classes, `refused` and
  `unsupported`; `Declined` keeps `refused`. Until smith 05.1 and 05.2,
  `answers.rs` and `responses.rs` write events version 3's nearest:
  limited as exhausted, both refusals as refused, unsupported as
  invalid. Until 02.3, inline-agent maps `Limited` to the host's
  `Exhausted`, `Declined` and `Refused` to its `Refused`, the classes to
  `Invalid`. Downstream: `run::Failure` loses `Copy`; its use in
  `crates/temper-engine-smith/src/finish.rs` (a match by pattern) is
  expected to compile unchanged, which the repin of the repository
  downstream of jig checks (decision 16).
Test shape: unit tests each under 20 ms. In the session's `tests.rs`:
  `a_rate_limit_is_waited_out_and_lasting_failures_are_not_retried`
  gains rows refused and unsupported, one attempt each (`Harness`). In
  smith-domain the new tests go in `src/tests/completion_retry.rs`
  (`tests.rs` has 2400 lines) with `Harness::admit` and `step`;
  `no_grant_fails_locally_and_exhaustion_reports_the_account` takes
  `Limited`; the new ones are
  `a_limited_account_ends_the_run_with_its_reset`, rows a reset said,
  said zero, not said, and time passing (`env.now` moves 30 ms after the
  notice; `reset_at` stays the notice's time plus its reset),
  `a_refused_account_ends_the_run_with_its_code` and
  `an_unsupported_request_ends_the_run_naming_its_field`. In
  agent-service's `answers.rs`:
  `a_limited_accounts_reset_is_measured_at_the_answer` (was
  `an_exhausted_accounts_wait_is_measured_at_the_answer`) keeps its rows
  and adds time passing: the notice said 80 ms at 20 ms, the answer at
  60 ms says 40 ms, less than the notice's. In protocol-llm's
  `tests.rs`, smith 01.3's `skein_failures_map_to_the_sessions_classes`
  takes the session's own classes. World tests each under 0.1 s: agent
  world, `feedback_table.rs` (`Settings::calm`, `World::with_failure`):
  `every_model_fault_and_completion_failure_keeps_its_evidence` and
  `every_completion_failure_reaches_the_answer_with_its_fields` gain a
  row per new fault and class; `component_boundary.rs`, beside
  `every_shared_failure_class_keeps_each_evidence_and_detail`
  (`limits()`, `prompt()`, `credential`, `adapter::prepare`):
  `skeins_report_reaches_the_session_whole`, rows code, request id,
  field, none. Channel world, `stories.rs` (`CEILINGS`,
  `smith_charter::CEILINGS`), beside
  `a_model_failure_keeps_transport_evidence_and_cooldown`, which writes
  a failure at the answer by `answer_record` the same way, read by smith
  01.2's accessors:
  `a_limited_account_is_written_with_its_reset_at_the_answer` and
  `a_refused_account_is_written_with_its_code` (rows a code, an empty
  one). Reference test:
  ```rust
  #[test]
  fn a_limited_account_is_written_with_its_reset_at_the_answer() {
      let now = skein_lib::Time::from_nanos(20_000_000);
      for (reset_at, said) in [
          (Some(skein_lib::Time::from_nanos(100_000_000)), Some(skein_lib::Duration::from_nanos(80_000_000))),
          (Some(now), Some(skein_lib::Duration::ZERO)),
          (None, None),
      ] {
          let answer = smith_domain::run::Answer::Failed {
              failure: smith_domain::run::Failure::Model(smith_domain::run::Fault::Limited { account: 7, reset_at }),
              spent: smith_domain::run::Spend::ZERO,
              turns: 0,
          };
          let record = smith_protocol_channel::answer_record(now, answer, None, &CEILINGS, &smith_charter::CEILINGS)
              .expect("bounded failure");
          let smith_channel::RunResult::Failed(failed) = record.result() else { panic!("expected failure") };
          let smith_channel::RunFailure::Model(model) = failed.reason() else { panic!("expected model failure") };
          let smith_channel::ModelFault::Limited(limited) = model.value() else { panic!("expected limited") };
          assert_eq!((limited.account(), limited.reset()), (7, &said));
      }
  }
  ```
Proof: world,
  `every_completion_failure_reaches_the_answer_with_its_fields`:
  refused, unsupported and limited (with a reset and with none) reach
  the run's answer with code, request id, field and reset; under 0.1 s.
Don't: retry a refused or unsupported call; move a run to another
  account; put a report or a reason in a fact; derive a reset not said.
Size: about 400 lines.
Removes: `Fault::Exhausted`; the bare `Duration` of the session's and
  the run's `Exhausted { retry_after }`, now an `Option<Duration>`; smith 01.2's zero-as-absent
  and 01.3's `failed()` mappings.

### 02.2 The notices raised
Scope: channel.md 4; session.md 4; host.md 7; hosts.md 5.1.
Needs: 02.1's run faults; smith 01.2's notices on the wire; smith 04.1's
  grant value union.
Files: `crates/smith-domain/src/{boundary,route,tests}.rs`;
  `crates/smith-agent-service/src/{service,tests}.rs`;
  `crates/smith-host-domain/src/{channel,domain,parent,tests}.rs`;
  `crates/smith-inline-agent/src/{translate,tests}.rs`;
  `crates/smith-local-service/src/process.rs`;
  `crates/smith-local-domain/src/{domain,parent_tests}.rs`;
  `tests/agent/src/{world,memory}.rs`, `tests/hosts/src/lib.rs`;
  `tests/protocol-llm/src/world.rs`, `tests/host/src/{lib,inline}.rs`;
  `tests/host/tests/{memory,simulation}.rs`;
  `tests/protocol-llm/tests/root_memory.rs`;
  `tests/channel/tests/{stories,composed}.rs`.
Shape: the domain's `Request::Rejected` and `Exhausted` become
  `Unauthorized { grant }`, `Limited { account, reset }` (an
  `Option<Duration>`) and `Refused { account, code, reason }` (new):
  `route.rs` raises refused once per failed session, its reason from the
  failure's detail; unsupported raises none. agent-service sends each by
  smith 01.2's senders. host-domain's `Up` and `parent::Request` take
  the same three; `domain.rs` checks refused as it checks limited: an
  account lent to an admitted agent, else `rules`, including an account
  the host lent that no admitted agent holds. inline-agent's request
  mapping and local-service's `to_host_event` pass them through, ending
  smith 01.2's mapping. Until 02.4, local-domain shows limited and
  refused as `AccountFailure::Exhausted`; unauthorized asks for the
  credential as rejected did. No host moves a run. Downstream: jig's
  test service and worker match `smith::Up` and the parent requests; the
  repository downstream of jig matches them in its service, engine and
  worker, refused handled as rejected was (decision 16).
Test shape: unit tests each under 20 ms, in smith-domain's `tests.rs`:
  `grant_updates_do_not_change_a_call_in_flight_and_rejections_are_once_per_generation`
  keeps its name for unauthorized;
  `a_refused_account_raises_one_notice_with_its_reason`;
  `an_unsupported_request_raises_no_notice`. In host-domain's
  `tests.rs`: `a_notice_for_an_account_not_lent_is_refused_as_rules`,
  rows unauthorized, limited, refused, and an account lent but held by
  no admitted agent. Channel world, under 0.1 s: `stories.rs`'
  `account_notices_reach_the_host_without_values` already drives the
  agent's side to host-protocol's `OpenEvent` (smith 01.2), and keeps
  it; `composed.rs`, over the setup of
  `the_host_start_enters_the_agent_domain_and_its_answer_returns_to_the_host_domain`,
  gains `account_notices_reach_the_host_domain_without_values`, a row
  per notice, from the agent domain's raise to host-domain's `Up`
  (ruling 122).
Proof: world, `account_notices_reach_the_host_domain_without_values`: each of
  unauthorized, limited and refused crosses both real halves into the
  host domain with its account and no value; under 0.1 s.
Don't: send a refused notice for unsupported or for an LLM's own
  refusal; let a host retry on another account.
Size: about 290 lines.
Removes: `Rejected` and `Exhausted` in the domain's, the host's and
  the parent's requests; smith 01.2's `to_host_event` mapping.

### 02.3 The host's faults
Scope: run.md 10; host.md 8; hosts.md 5.1; shell.md 4.3; design
  question 7.
Needs: 02.1's run faults; smith 01.2's version 4.
Files: `crates/smith-host-domain/src/{channel,lib,tests}.rs`;
  `crates/smith-host-protocol/src/{answer,tests}.rs`;
  `crates/smith-inline-agent/src/{translate,tests}.rs`;
  `crates/smith-local-service/src/stream.rs`;
  `crates/smith-local-domain/src/{end,tests}.rs`;
  `crates/smith-local-protocol/src/{terminal.rs,terminal/failure.rs}`;
  `crates/smith-local-shell/src/local_host/words.rs`;
  `tests/host/tests/{simulation.rs,simulation/inline.rs}`;
  `tests/host/tests/fuzzy_simulation.rs`;
  `tests/channel/tests/stories.rs`.
Shape: `host::ModelFault` is `Completion { .., report }`, `Limited {
  account, reset: Option<Duration> }`, `Provider`, `Declined`, `Refused
  { account, code }`, `Unauthorized { account }`, `Program { name, exit:
  Option<u32>, detail }`, `Truncated`, `Malformed`; its
  `CompletionFailure` gains `Refused` and `Unsupported`, `Exhausted` an
  optional reset; `host::Report` (new, re-exported by `lib.rs`) mirrors
  the run's. `ModelFault` and `RunFailure` lose `Copy` and keep `Clone`
  (`CompletionFailure` stays `Copy`); `terminal.rs` passes a reference.
  `answer.rs` decodes each version 4 fault, ending smith 01.2's mapping
  and its comment "smith 02.1 gives the two new classes their domain
  form", which now names 02.3;
  inline-agent's `model_fault` maps the run's, the reset measured at the
  answer, ending 02.1's. `end.rs`'s `answer_status`: `Limited`,
  `Program` and `Completion { Unsupported }` exit 1; `Refused` and
  `Unauthorized` exit 3 naming their own account. Until smith 05.2,
  `stream.rs` and `terminal/failure.rs`'s `failed()` build events
  version 3's nearest, whose words smith-events says: limited as
  exhausted, refused and declined as refused, unauthorized as the
  unauthorized completion, a program as provider. local-shell's
  `words.rs` matches `Limited`. Downstream: the repository downstream of
  jig's `crates/temper-worker-domain/src/translate.rs` matches
  `ModelFault::{Exhausted, Completion, Provider, Refused, Truncated,
  Malformed}`: `Exhausted` is now `Limited`, `Refused` is `Declined`; its
  `src/tests/entrance.rs` builds `ModelFault::Exhausted`.
  With 02.1's `finish.rs`, its repin is about 15 lines (decision 16).
Test shape: unit tests each under 20 ms. In smith-host-protocol's
  `tests.rs`: `each_version_4_fault_reads_as_its_own`, a row per fault.
  In smith-local-domain's `tests.rs` (`end.rs` has no test module;
  `crate::end::answer_status`): `each_model_fault_has_its_exit_status`, rows
  limited 1, refused 3, unauthorized 3, program 1, unsupported 1,
  unauthorized completion 3. In smith-local-protocol:
  `an_exhausted_failure_uses_its_answer_account_and_clock` takes
  `Limited`; `each_model_fault_has_a_last_line`, a row per fault. In
  inline-agent: `the_inline_answer_measures_the_exhausted_accounts_wait`
  takes `Limited`, rows a reset, a reset part passed (the answer 40 ms
  after a notice of 80 ms) and none. Worlds, each under 0.1 s: inline,
  `simulation/inline.rs`, beside
  `an_inline_answer_keeps_its_typed_completion_failure`
  (`inline::limits`, `inline::start`, `World::fail_with`):
  `an_inline_answer_keeps_its_account_failure`, rows exhausted with a
  reset, exhausted with none, refused (an empty code); channel,
  `stories.rs`:
  `a_model_failure_reaches_the_host_domain_in_its_typed_vocabulary`
  gains a row per fault.
Proof: world, `an_inline_answer_keeps_its_account_failure`: a limited
  and a refused account reach the host's failure with their account,
  reset and code; under 0.1 s; unit,
  `each_model_fault_has_its_exit_status`: limited exits 1, refused and
  unauthorized 3; under 20 ms.
Don't: map a fault onto another's exit; drop the report; change a
  word smith-events says (smith 05.2's).
Size: about 280 lines.
Removes: the host's `ModelFault::Exhausted`; smith 01.2's and 02.1's
  fault mappings.

### 02.4 The local host's account notices
Scope: host.md 7, 8; shell.md 4.3, 7.5; design question 7.
Needs: 02.2's notices; 02.3's faults.
Files: `crates/smith-events/src/{lib,words}.rs`,
  `crates/smith-local-domain/src/{boundary,domain,parent_tests}.rs`,
  `crates/smith-local-protocol/src/{terminal.rs,terminal/failure.rs}`,
  `tests/local/tests/{simulation,fuzzy_simulation}.rs`.
Shape: `AccountFailure::Exhausted` becomes `Limited { account,
  reset_at: Option<Wall> }`, the wall clock plus the reset when said;
  `Refused { account, code, reason }` is new; `AccountFailure` loses
  `Copy`. `domain.rs` shows each, ending 02.2's mapping; unauthorized
  stays the credential's refresh (host.md, 7). `terminal.rs` prints
  shell.md 4.3's words (the shell adds `smith: `), the account by its
  number as `failure::credential` does: `run failed: account N reached
  its usage limit, usable again at HH:MM` (no time when no reset was
  said) and `account N: the provider refused it`, then `: CODE` and
  `: REASON` through `failure::detail`, each when not empty; each new
  arm appends `run failed: ` itself, for `failure::credential` and
  today's `Account` arm print none. 4.3's
  `; manage it at <address>` (the preset's usage address, 6.2) is in no
  increment of this plan and is not printed. smith-events publishes
  `clock(at: Wall) -> [u8; 5]`, the `HH:MM` its private `clock` writes
  (which then takes its bytes), and `terminal.rs` appends them through
  `Shown`: never `put` into a `List`, which panics past its room. The run-failed last lines are smith
  05.2's. Downstream: none.
Test shape: unit tests each under 20 ms. In smith-local-protocol's
  `terminal.rs`: `account_failures_keep_their_own_words` keeps its
  rows unavailable and not refreshed, and its exhausted row becomes
  the rows limited with a reset (`run failed: account 7 reached its usage limit, usable
  again at 10:01`), limited without, refused with a reason, refused
  with an empty code. In smith-local-domain:
  `rejection_keeps_the_account_and_exhaustion_keeps_its_notice` becomes
  `each_account_notice_keeps_its_account`. Local world, under 0.1 s,
  `tests/local/tests/simulation.rs`:
  `an_exhausted_account_is_shown_to_the_person` becomes
  `a_limited_account_is_shown_with_its_reset`;
  `a_refused_account_is_shown_with_its_code_and_reason`.
Proof: world, `a_refused_account_is_shown_with_its_code_and_reason`:
  the person sees 4.3's refused line with the account, code and
  reason, and the run fails for it; under 0.1 s.
Don't: say "exhausted" to a person; show a reset not said.
Size: about 150 lines.
Removes: `AccountFailure::Exhausted`; 02.2's local mapping.

## Interface sketches

```rust
// smith_domain_session::llm, re-exported as smith_domain::llm; Report::none() is three Box::default()s: Default is banned
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Report { pub code: Box<[u8]>, pub request: Box<[u8]>, pub field: Box<[u8]> }
// Failure::{Refused, Unsupported, Exhausted { retry_after: Option<Duration> }}
// smith_domain::Event::Failed { owner, failure, evidence, detail, report: llm::Report }
// smith_domain::boundary::Request::{
//     Unauthorized { grant: GrantName },
//     Limited { account: u32, reset: Option<Duration> },
//     Refused { account: u32, code: Box<[u8]>, reason: Box<[u8]> } }
// smith_domain_run::boundary: run::Report mirrors llm::Report; Fault,
// End, Failure and host-domain's channel.rs RunFailure are Clone only; facts.rs's EndKind, FailureKind: Copy.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Fault {
    Completion { failure: CompletionFailure, evidence: CompletionEvidence, model: u32, attempts: u32, report: Report },
    Limited { account: u32, reset_at: Option<Time> },
    Refused { account: u32, code: Box<[u8]> },
    Provider, Declined, Truncated, Malformed,
}
// smith_host_domain::channel; Up and parent::Request take the same notices.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ModelFault {
    Completion { failure: CompletionFailure, evidence: CompletionEvidence, model: u32, attempts: u32, report: Report },
    Limited { account: u32, reset: Option<Duration> },
    Refused { account: u32, code: Box<[u8]> },
    Unauthorized { account: u32 },
    Program { name: Box<[u8]>, exit: Option<u32>, detail: Box<[u8]> },
    Provider, Declined, Truncated, Malformed,
}
// smith_local_domain::boundary
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AccountFailure {
    Unavailable, NotRefreshed,
    Limited { account: u32, reset_at: Option<Wall> },
    Refused { account: u32, code: Box<[u8]>, reason: Box<[u8]> },
}
```

## Questions to raise when reached

- 02.1: if `EndKind`, `FailureKind` and the loss of `Copy` reach over
  ~100 lines outside Files (count them below, `files.txt` holding the
  Files a path a line), say whether the report rides beside the fault:
  ```
  git diff --numstat origin/main...HEAD -- '*.rs' ':!*/generated/*' | awk 'NR == FNR { f[$0]; next } !($3 in f) { n += $1 + $2 } END { print n + 0 }' files.txt -
  ```
- 02.3: if the repository downstream of jig turns a new fault into
  anything but a model failure, name it for the closing repin.

## Done when

code-organization.md 8's two commands show each increment within its
Size and no file over 800 lines grown by more than about 100. Each
command here finds nothing:
```
rg -n --glob '!**/generated/**' '\b(Model)?Fault::Exhausted\b|\b(Request|Up)::(Rejected|Exhausted)\b' crates tests
rg -n --glob '!**/generated/**' '\bAccountFailure::Exhausted\b|A model account is exhausted' crates tests
rg -U -n 'derive\([^)]*\bCopy\b[^)]*\)\]\s*pub enum (End|Fault|Failure|ModelFault|RunFailure|AccountFailure)\b' crates/smith-domain-run/src/boundary.rs crates/smith-host-domain/src/channel.rs crates/smith-local-domain/src/boundary.rs
```
