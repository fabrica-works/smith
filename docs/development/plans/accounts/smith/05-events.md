# smith 05: the event vocabulary's version 4

The events' version 4 names a model's account and backend and tells a
completion's failure by its field, code and request id. It tells an
account's failures in its answer failure and its notices, and its words
are shell.md 4.3's ("reached its usage limit"), never exhausted.

Needs at start: smith 03's charter account and `Spec.account` (03.1),
and from smith 02 the session failures `Refused` and `Unsupported`
with the report carried to the run's completion failure, and the run
faults `Limited` and `Refused` (02.1), the notices (02.2), the host's
faults (02.3) and the local host's notices (02.4), on `origin/main`.

## Read first
- smith's design at `137a1cb4`: `protocol/` `events.md` 3.3, 3.5, 3.9,
  3.10, 6, 8; `llm.md` 6; `README.md` 8; `domain/` `run.md` 10.
  skein's `programming-model.md` 4.4, 6.3, 8; `testing-strategy.md`
  2.2, 2.5, 8, 10.
- The README's Rules; code-organization.md (owners, fixtures, 6, 7);
  decisions.md 3, 7, 10; questions 7, 14.

## What the design asks
- events.md 3.3, 3.5, 3.9, 3.10, 6, 8: models, completions, notices.
- llm.md 6: a completion's failure classes, field, code and request id.

## Choices
- No conversation runs on a program yet (decision 7): a model's backend
  is `direct`, and an answer failure's program fields are written,
  never raised.
- `Limited` and `Refused` are 02.1's names (decision 3).
- Codec and world tiers only (decision 10).

## Rules that bind it
- A format moves in one increment: schema, generated code, goldens (a
  directory per version), version constant, `Versions` literal, its
  rows in `tests/codecs/tests/drift.rs` (events has none in
  `fuzzy_decode.rs`) and `smith check`'s formats line in
  `tests/end-to-end/end_to_end.rs`; the old version goes with it
  (events 4 is split across 05.1 and 05.2).
- Files over 800 lines grow by at most ~100 per increment (code-
  organization.md 8): `service.rs` (2920), `write.rs` (2190), `read.rs`
  (1865), `responses.rs` (1855), smith-agent-shell's `config.rs`
  (1658), `sinks.rs` (1511), `records.rs` (1099).
- The files a change breaks, found at the base, `G` being `--glob
  '*.rs' --glob '!**/generated/**' crates tests benchmarks`:
  ```sh
  rg -l 'golden/v3' G  # 18, the events' goldens
  rg -n '\bevents: 3\b' G  # 3 literals
  ```

## Increments

### 05.1 The events' version 4: models and completions
Scope: events.md 3.3, 3.5, 3.10's model and completion failure, 6, 8;
  llm.md 6. Events version 4 is split: 05.1 writes the model and the
  completion failure, 05.2 the account's failures (the version stays 4
  across both, and `golden/v4/` grows in 05.2).
Needs: what the session needs.
Files: `crates/smith-events/`: `src/{lib,records,read,write,limits,
  words,tests}.rs`, `golden/v4/` (new), `golden/unsupported-v5.jsonl`
  (from `unsupported-v4.jsonl`); in `crates/`: `smith-agent-service/
  src/{observations,events,responses,answers,calls,sinks,service,
  tests}.rs`, `.../src/sinks/{view.rs,tests/view.rs}`, `.../tests/
  sink_memory.rs`, `smith-local-service/src/stream.rs`,
  `smith-local-protocol/src/terminal/failure.rs`, `smith-local-shell/
  src/{front/interactive,startup}.rs`, `smith-agent-shell/src/
  config.rs`; the 18 readers of `golden/v3`, retargeted to `golden/v4`:
  `benchmarks/tests/{events,budget_visibility,cache_probes,
  fuzzy_parsers,large_write_probe,probes,process_tree_probe,
  slow_completion_probe,smith_adapter,tool_output_probes,common/
  events,events/delegation}.rs` and `benchmarks/src/agent/smith/
  observe.rs` (the 13), `tests/codecs/tests/{events,drift}.rs`,
  agent-service's `src/sinks.rs` and `src/sinks/tests/view.rs`,
  smith-events' `src/tests.rs`; `benchmarks/src/agent/smith/events/
  names.rs`, `benchmarks/tests/failure_probes.rs`, the six
  `benchmarks/tests/recorded/*.jsonl` with `"v":3` lines, rewritten to
  version 4; `tests/codecs/tests/support/{events,events_max}.rs`;
  `tests/agent-process/{tests/content,src/tracing}.rs`;
  `tests/end-to-end/end_to_end.rs` (formats line); the `events: 3`
  literals, to 4 (`tests/agent-process/src/tracing.rs`, `smith-local-
  shell/src/startup.rs`, `smith-agent-service/src/service.rs`).
  `tests/codecs/tests/events.rs` retargets its version-4 reads to 5
  (`unsupported-v5.jsonl`, `Error::Version(5)`), and smith-events'
  `src/tests.rs` its `"v":4` line; there too, the `CompletionFailure`
  literals of `every_provider_class_has_its_words` and
  `a_rejected_credential_names_its_account_in_the_shared_words` take the
  new fields (ruling 132).
Shape: `VERSION` 4. `Model` gains `account: Option<u32>` and
  `backend: Backend` (new: `Direct`, `ClaudeCode`, `Unknown`) in
  `run.started`, `conversation.opened` and `response.started`;
  `opening_model` takes them from the charter's LLM, `Backend::Direct`
  here. A program backend is told by its name, `claude_code` as
  `ClaudeCode` and any other name as `Unknown(name)` (events.md 3.10,
  6); nothing writes one yet, so 03.2's `record::Backend` is not
  mapped here. `Tools` gains `withheld`, empty on a direct run.
  `FailureClass` gains `Refused` and `Unsupported`, so `words.rs`,
  the terminal's `failure.rs` and the benchmarks' `events/names.rs`
  match them; `CompletionFailure` gains `field`, `code` and
  `request_id`, written when the provider gave them, so its literals
  without `..` change (in Files); `detail` stays content, under
  `calls` and `everything` only; refused and unsupported
  `response.completed` have no `retry_ms`. `completion_failure` in
  `responses.rs` and `answers.rs` map 02.1's classes and report.
  smith-events has no user downstream.
  Against main (ruling 143): every `"v":3` line literal becomes 4, in
  smith-events' `src/{tests,read}.rs`, `tests/codecs/tests/events.rs`,
  agent-service's `src/tests.rs` and `benchmarks/tests/{events,
  failure_probes}.rs`; `benchmarks/tests/events.rs`'s refusal feed
  `"v":4` becomes 5 and its `contains("version 4")` `"version 5"`
  (`typed_event_stream_classification_usage_and_author_predicates`);
  `tests/codecs/tests/drift.rs`'s
  `event_records_and_every_listed_value_match_frozen_version_three_goldens`
  is renamed to version four, and each new golden gets its sample in
  `support/events.rs` `samples()` (the test counts them). `FailureClass`
  is matched also by smith-events' `write.rs` (`write_failure_class`,
  `validate_failure_class`), `words.rs`' `model` and `class`, and
  agent-service's `sinks/view.rs` `completion_failure`. The fields that
  `Model`, `Tools` and `CompletionFailure` gain are carried, clipped, by
  `sinks/view.rs`' `model` and `completion_failure` and counted in its
  payload bound (`string.checked_mul(3)`), and counted by smith-events'
  `limits.rs` (`heap_model`, `heap_tools`, `heap_completion_failure`
  and the matching `size_*`) and agent-service's `events.rs`
  `worst_case`. In `answers.rs` the mapping is `completion(why,
  evidence)`: its report comes from `run::Fault::Completion { report
  }` (`run::Report { code, request, field }`, an empty box for none),
  and `responses.rs`' `completion_failure` callers, its unit tests
  among them, pass the report. `answers.rs`' comments that give the
  distinct classes to 05.2 now name 05.1.
Test shape: codecs, `tests/codecs/tests/events.rs`: a golden for every
  value of every listed field (an account and null; `direct` and
  `claude_code`; `withheld` empty and `sub_agents`; each class,
  refused and unsupported with field, code and request id); every
  golden read back; each record at its largest;
  `a_version_three_line_is_refused`; drift. Units, each under 20 ms,
  smith-agent-service `src/tests.rs` (`assert_stream_record`):
  `an_unsupported_completion_names_its_field_with_no_retry`;
  `a_refused_completions_detail_is_content`, rows `none`, `calls`.
  Process world, under 0.5 s, `tests/agent-process/tests/content.rs`:
  `a_runs_model_names_its_account_and_backend`.
Proof: world, `a_runs_model_names_its_account_and_backend`: the
  run's `run.started` model line names account 0 and `direct`; under
  0.5 s (a real process); process, `smith_check_json_shows_each_value_
  with_its_layer`: `events` reads 4, beside the drift row; under 0.5 s
  (a real process; about 0.15 s at the base).
Don't: write a field the provider did not give as empty or zero; read
  version 3; write `detail` outside content capture.
Size: about 280 lines.
Removes: the events' v3 goldens and `golden/unsupported-v4.jsonl`;
  02.1's interim completion-class mappings in `responses.rs` and
  `answers.rs` (05.2 removes 02.1's account-fault mappings in
  `answers.rs`). `ls crates/smith-events/golden` printed
  `unsupported-v4.jsonl` and `v3` at the base.

### 05.2 The events' version 4: an account's failures
Scope: events.md 3.9, 3.10's answer failure, 6; questions 7 and 14;
  the text a person reads is shell.md 4.3's, never "exhausted".
Needs: 05.1; 02.1's run faults `Limited` and `Refused`; 02.2's
  notices; 02.3's host faults and 02.4's local notices.
Files: `crates/smith-events/src/{records,read,write,limits,words,
  tests}.rs`, `golden/v4/`; in `crates/`: `smith-agent-service/src/
  {answers,events,responses,service,teardown,sinks}.rs`,
  `smith-agent-service/src/sinks/{view.rs,tests/view.rs}` (their
  `golden/v3` reads are 05.1's), `smith-local-service/src/stream.rs`,
  `smith-local-shell/src/front/interactive.rs`,
  `smith-local-protocol/src/{terminal.rs,terminal/failure.rs}`;
  benchmarks `src/agent/smith/observe.rs` and `tests/{events,
  failure_probes}.rs`; `tests/codecs/tests/support/{events,
  events_max}.rs`.
Shape: `AnswerFailure.account` is `Option<u32>`; `reset_ms` replaces
  `retry_after_ms`; it gains `state: Option<AccountState>` (new:
  limited, refused, unauthorized, `Unknown`), `code`, and the
  program's `program`, `exit` (`AgentExit`) and `detail`, which nothing
  raises yet (decision 7). `NoticeKind::{CredentialUnauthorized,
  AccountLimited, AccountRefused}` replace `CredentialRejected` and
  `AccountExhausted`; `Notice.account` is `Option<u32>`, `reset_ms`
  replaces `wait_ms`, and it gains `code`: the reason is never there,
  it is the failed response's `detail`. `service.rs` writes each notice
  where it sends 02.2's channel notice; `answers.rs` writes 02.1's
  account faults with their state. `AnswerFailure` grows by 128 bytes,
  so `Event::RunCompleted` is 240 bytes over `ResponseCompleted`:
  `Event` takes `#[expect(clippy::large_enum_variant,
  reason = "a record is built once and written directly")]`.
  `words.rs` says shell.md 4.3's words in `model()`, in place of its
  " exhausted" and ": the provider rejected its credential", and
  `account()` takes the `u32` (ruling 132): "account N reached its usage
  limit" and, the reset known, ", usable again at HH:MM"; "account N:
  the provider refused it" and ": CODE"; and "account N unauthorized";
  `class()` words `refused`, `unsupported`, and `limited` for the
  exhausted class. The terminal's failure line follows: "run failed:
  account work reached its usage limit, usable again at 10:01". 4.3's
  `; manage it at <address>` is not printed (02.4).
  smith-events' `src/tests.rs`: `word_failure`'s `retry_after_ms` is
  `reset_ms`, and `an_exhausted_answer_names_its_account_and_time_when_known`
  and `a_rejected_credential_names_its_account_in_the_shared_words` give
  way to `an_account_failure_has_its_words`' rows. The largest samples
  of `tests/codecs/tests/support/{events,events_max}.rs` take
  `Some(u32::MAX)` for the notice's and answer failure's account, which
  were `Some(u64::MAX)`, and `reset_ms` (ruling 132).
Test shape: codecs: goldens for each notice (`account_limited` with
  and without `reset_ms`; `account_refused` with its code) and each
  answer failure shape (account and state, each state; program, exit,
  detail); each at its largest. Units, each under 20 ms, smith-events
  `src/tests.rs`: `an_account_failure_has_its_words`, rows limited with
  a reset, limited without, refused, unauthorized;
  `every_provider_class_has_its_words` grows by refused and
  unsupported; smith-agent-service `src/tests.rs`
  (`assert_stream_record`): `each_account_notice_is_written_when_
  raised`, rows unauthorized, limited, refused;
  `a_limited_run_answers_with_reset_ms`.
Proof: unit, `each_account_notice_is_written_when_raised`: each notice
  line carries the account and, for limited, `reset_ms`; under 20 ms;
  no higher layer reads written events: 02.2's channel world rows
  already raise the notices.
Don't: put the provider's reason in a notice; write `reset_ms` the
  provider did not say; say "exhausted" to a person.
Size: about 280 lines.
Removes: `CredentialRejected`, `AccountExhausted`, `Notice.wait_ms`,
  `AnswerFailure.retry_after_ms`; 02.1's account-fault mappings in
  `answers.rs`; 02.3's interim v3 mappings in local-service's
  `stream.rs` and the terminal's `failure.rs`.

## Interface sketches
```rust
// smith_events, 05.1 (Clone, PartialEq, Eq, Hash, Debug)
/// What a model's completions ran on, as its events name it.
pub enum Backend { Direct, ClaudeCode, Unknown(Box<[u8]>) }
// Model { .., account: Option<u32>, backend: Backend }
// Tools { families, withheld: List<Family>, wait, deliver, host }
// CompletionFailure { .., field, code, request_id: Option<Box<[u8]>> }

// smith_events, 05.2 (Clone, PartialEq, Eq, Hash, Debug)
#[expect(clippy::large_enum_variant, reason = "a record is built once and written directly")]
pub enum Event { .. }
/// An account's state when a model's answer failed for it.
pub enum AccountState { Limited, Refused, Unauthorized, Unknown(Box<[u8]>) }
// AnswerFailure { .., account: Option<u32>, state: Option<AccountState>,
//     reset_ms: Option<u64>, code, program, detail: Option<Box<[u8]>>,
//     exit: Option<AgentExit> }
// Notice { .., account: Option<u32>, reset_ms: Option<u64>, code: Option<Box<[u8]>> }
```

## Done when
Each prints nothing, but the `ls` line (code-organization.md 8's
commands check sizes and long files):
```sh
ls crates/smith-events/golden  # v4, unsupported-v5.jsonl
rg -n -e 'smith-events/golden/v3|unsupported-v4' crates tests benchmarks
rg -n 'golden/v3' crates/smith-events
rg -n --glob '*.rs' -e '\b(CredentialRejected|AccountExhausted|wait_ms)\b' \
  crates tests benchmarks
rg -n 'exhausted' crates/smith-events/src/words.rs
```
