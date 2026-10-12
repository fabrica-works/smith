# Session 04: limits from declared quantities

Provisional, 2026-10-12. The LLM path's limits are derived on `main`, the
startup checks refuse by name, and the product's values fit 2 GiB, but
four things are left. Nobody can read the effective limits: the stream's
header carries an empty `limits`. The transcript, the channel's queue
and framing, the host kit and io still take hand-set values or schema
ceilings, with three named checks never run. `memory` is a required
setting with no default. And the `standard` profile still declares the
transitional values, so the product ships limits that shell.md, section
6 does not name. This session finishes them, so that what smith runs
with is what the design derives, and a reader of the stream sees it.

## Read first

- protocol/limits.md, sections 2, 3 (3.3, 3.6, 3.7 and 3.8), 4, 5, 9 and
  11;
- protocol/events.md, sections 3.2 and 6;
- protocol/agent.md, sections 4 and 6;
- shell.md, sections 4.4, 6.2, 6.3 and 6.5;
- testing.md, section 2.5;
- decisions.md, 3.1, 3.7 and 3.10;
- findings S8, S11 and S14 (`findings/limits.md`).

## What the design asks

- **Effective limits** (limits.md, section 9): "The declared values, the
  policy values and every derived limit, each under its name in its
  unit … a derived limit is named by its layer and its field in that
  layer's `Limits`, such as `llm.request` or `session.history_bytes`. A
  value per endpoint or per model adds them by their configured names,
  never by a number." `smith check` prints them, and
  `session.started`'s `limits` carries them (events.md, 3.2), the
  product's included. "Their names and units are part of the event
  vocabulary."
- **The transcript** (limits.md, 3.3): a turn's bytes from a completion's
  answer, its calls' inputs, its results at their render bounds and its
  reasoning items; the resume limit at least one window's turns.
- **The channel** (3.6): its turn, start and message frames, and "its
  queue: the window's turns, the calls in flight (at most
  `conversations`), the notices, the long operations, the answer and a
  reserve for facts, charged by rule 5"; by direction, by rule 6.
- **Checks** (section 4): `turn-in-transcript`, `resume-holds-window`,
  `turn-in-channel` and `cancel-over-close`, each refused by its name.
- **Memory** (section 5; shell.md, 6.3): "`memory` | 2 GiB | decided as
  at most 2 GiB; the derived worst case must fit it." The LLM path's
  worst case is the pool, each connection's own state and the tables;
  the rest at derived limits.
- **The `standard` profile** (shell.md, 6.3 and 6.5) declares the
  product's deployment and policy values.
- **No pinned values** (section 11): properties of the derivation, one
  refusal per relationship, each profile deriving and fitting `memory`.

## Choices this plan makes

- **The effective limits are one total function per service,**
  `profile::effective`, destructuring each layer's `Limits` exhaustively,
  so a new field fails to build until it is named. It is split by layer
  (04.7, 04.21) to stay within 600 lines; the local layers come last
  (04.22), once the product writes a stream.
- **Names follow limits.md, section 9:** `<layer>.<field>`, then
  `[endpoint=<name>]` or `[endpoint=<name>,model=<name>]` by configured
  names where the limit is one; durations are integer milliseconds; the
  compaction threshold is a fraction.
- **Derivation stays in `smith-agent-service::profile`,** where 04.17 put
  the transcript's and the channel's; each crate's own `derive` is used
  where one exists.
- **`exit_grace`** keeps its transitional value: smith 06 derives it.

## Rules that bind it

- **Never raise `memory`, or lower a decided value, to make a worst case
  fit** (decisions.md, 3.10). A profile that does not fit is a question
  (Questions, 1).
- **One owner for the profile's derivation** (code-organization.md,
  section 7): extend `smith-agent-service::profile`; no second derivation
  beside it.
- **Formats keep their versions** (README, rule 5): the effective limits'
  names enter events version 1, with no bump.

## Landed in the first run

| Increment | Commit | Proof |
|---|---|---|
| 04.1 | 8cb89164 | merged, unproven: smith-13 |
| 04.2 | 839ac739 | deterministic |
| 04.3 | cca5c09f | merged, unproven: smith-13 |
| 04.4 | 7bd5d14b | merged, unproven: smith-13 |
| 04.5 | 2a514521 | proven |
| 04.6 | 61d74a29 | merged, unproven: smith-00.16 |
| 04.8 | 61d74a29 | merged, unproven: smith-00.16 |
| 04.10 | c883e2e1 | merged, unproven: smith-13 |
| 04.11 | e826a2bf | merged, unproven: smith-13 |
| 04.12 | a1b6dc7c | merged, unproven: smith-13 |
| 04.optional-usage | 89b16539 | merged, unproven: smith-13 |
| 04.15 | e548d632 | proven |
| 04.16 | e548d632 | proven |
| 04.17 | e548d632 | proven |
| 04.18 | 60004433 | proven |
| 04.19 | 086098ac | proven |
| 04.20 | 61b34463 | proven |

## Increments

### 04.7 Effective limits: declared, policy and smith's own layers
Scope: a total function from the profile, the configuration and the
derived limits to the effective limits (limits.md, section 9): each
declared and policy value under section 2's name, per endpoint or model
where it is one, and each field of the derived `Limits` of the run, the
session, the tools, the root domain, rendering and the service, as
`<layer>.<field>`. The agent writes them in `session.started`'s `limits`
(events.md, 3.2); the event sink's records and the header's memory hold
them. 04.21 adds the other layers.
Needs: nothing beyond the session's start.
Files: `crates/smith-agent-service/src/profile/effective.rs` (117 lines)
and a new `src/profile/effective/` module; `src/profile.rs`
(re-export); `src/events.rs` (`EventLimits::derive`);
`crates/smith-agent-shell/src/agent_shell.rs` (the header at line 194,
its memory at 154–170), `src/config.rs`. Users of
`EventLimits::derive`: `crates/smith-agent-service/src/service.rs:2566`,
`crates/smith-agent-service/tests/sink_memory.rs:57`,
`crates/smith-agent-shell/src/config.rs:238`, `:1225`, `:1379`,
`tests/agent-process/src/tracing.rs:34`, `:195`. The world's header:
`tests/agent-process/src/tracing.rs:115`.
Shape: see "Interface sketches, 04.7". `effective` counts, then fills a
`List` of that capacity; `None` on overflow. `header(&[Limit],
&channel::Endpoints, &Configuration) -> Option<List<EffectiveLimit>>`
spells names and values. `EventLimits::derive` takes the header's
entries, so `records.items` holds their count and `records.string` the
longest name.
Test shape: `crates/smith-agent-service/src/profile/effective/tests.rs`,
new; reuse `profile()` and `declarations()`
(`crates/smith-agent-service/src/profile/tests.rs:11`, `:20`), and add
`names()` beside them, naming endpoint 7 `codex`. Reference test:
```rust
#[test]
fn a_models_head_is_named_by_its_endpoint_and_model_in_milliseconds() {
    let limits = derive(&profile(), &declarations()).expect("fixture derives");
    let named = effective(&profile(), &declarations(), &limits).expect("bounded list");
    let header = header(&named, &names(), &declarations()).expect("names fit");
    let head = header.iter().find(|limit| limit.name.as_ref() == b"declared.head[endpoint=codex,model=one]");
    assert_eq!(head.map(|limit| limit.value), Some(LimitValue::Integer(5_000)));
}
```
Tests: that one; `effective_names_are_unique_across_two_endpoints_and_models`;
`the_compaction_threshold_is_written_as_its_fraction`;
`the_sinks_records_hold_the_headers_entries_and_longest_name`; end to end
(`tests/end-to-end/end_to_end.rs`, reusing `support::run` and
`support::read_trace`, `tests/end-to-end/support/mod.rs:286`, `:354`):
`the_agents_trace_header_carries_its_effective_limits`. Ceilings: unit 20
ms; end to end 2 s.
Proof: process, `the_agents_trace_header_carries_its_effective_limits`:
the real `smith-agent`'s trace starts with a header whose `limits` hold
`declared.memory` and `session.history_bytes`; it fails on the parent,
whose `limits` is empty. The passive headroom checks (benchmarks.md,
11.2) would show it live; not run in this plan.
Don't: list a field twice under two names; name a schema's ceiling as
a derived limit; print `smith check` (session 10).
Size: at most 600 lines of production code (decisions.md, 13.9; was 450).
Removes: the comment "Effective projection is owned by 04.7" and the empty
`limits` in `agent_shell.rs`.

### 04.9 The transcript, the channel, the host kit and io derived (the rest)
Scope: 04.17 (d43cfe32) and 3735e86c derived the channel's bodies, start
and output by rule 6, the transcript's `message_blocks` and the check
`calls-in-transcript`. Left (limits.md, 3.3, 3.6 and section 4):
- the transcript's limits from the derivation, each at most its schema
  ceiling: a turn's fields from the session's and tools' derived limits,
  and the resume limit at least one window's turns;
- the channel half's queue (`turns`, the fact reserve, `grants`) by rule
  5, and skein-channel's `chunk`, `skip` and `output_frames` by rule 6;
- the host kit's limits (`host_limits`,
  `crates/smith-local-service/src/profile.rs:10`) from the same
  derivation, in place of its hand-set bytes;
- io's `sockets` from the connections, the channel and the file slots,
  and the service's `file_slots`, `file_read` and `file_entries` from the
  tools' limits;
- checks `turn-in-transcript`, `resume-holds-window` and
  `turn-in-channel`, each refused by name;
- the two ceilings the standard profile passes, as the designer ruled on
  2026-10-12 (limits.md, rule 6 and 3.6; channel.md, section 3;
  `4c7b15e6`), under transcript 3 and channel 2 (the README's rule 5),
  with the codecs and goldens regenerated by skein-codegen
  (`tests/codecs/tests/drift.rs` names the three):
  - transcript 3's `Listed` from `list 256 Entry` to `list 512 Entry`
    (`crates/smith-transcript/schema/transcript-v3.schema:28`):
    shell.md's `list_entries`. The generator refuses 1,024 ("encoded
    worst case for Turn exceeds u32");
  - channel 2's `Start.transcript` from `list 64 bytes 8388608` to one
    piece, `bytes 536870912`, the same 512 MiB in all
    (`crates/smith-channel/schema/channel-v2.schema:55`): each turn's
    bytes after its big-endian `u32` length. The host kit joins its
    kept turns into it; the agent splits it, counting turns against the
    resume limit's turns, and refuses as malformed a piece whose lengths
    do not end at its end, and as too large one past the limit;
  - transcript 3's `Found`, as the designer ruled on 2026-10-12
    (transcript.md, section 7): `hits` from `list 256 Hit` to one piece,
    `bytes 2097152`, and `Hit.text` from `bytes 4096` to `bytes 2097152`
    (`crates/smith-transcript/schema/transcript-v3.schema:21`, `:29`).
    `Hit` stays a record: the piece is the hits as the codec encodes
    each `Hit`, in order. `turn.rs:246` encodes them into it;
    `transcript.rs:259` decodes them one by one with the generated `Hit`
    decoder, counting them against the tools' `search_hits`, and refuses
    as malformed a piece whose hits do not end at its end. The generator
    accepts the piece at 2 MiB and refuses it at 4 MiB;
- the resume limit's turns from the session's `history_messages`
  (46,255 for the standard profile's 272,000-token window), never from
  `max_turns`, and its bytes from one window's turns at their largest
  (limits.md, 3.3).
Needs: nothing beyond the session's start.
Files: `crates/smith-agent-service/src/profile.rs` (`transcript`,
`derive`, `io_limits`), `src/profile/validate.rs`, `src/profile/tests.rs`;
`crates/smith-local-service/src/profile.rs`; `crates/smith-local-shell/src/local_shell.rs:96`
(`host_limits`' caller); `tests/channel/src/lib.rs`,
`tests/channel/tests/stories.rs`, `tests/hosts/src/lib.rs`. The ceilings:
`crates/smith-transcript/schema/transcript-v3.schema`,
`src/generated/v3.rs` and `golden/v3/`;
`crates/smith-channel/schema/channel-v2.schema`, `src/generated/v2.rs`
and `golden/v2/`; the piece: `decode_transcript`
(`crates/smith-protocol-channel/src/transcript.rs:18`, called at
`component.rs:802`), the start's parts in
`crates/smith-host-protocol/src/translate.rs:100`, and
`channel_bodies` and `channel_start_bytes` (`profile.rs:233`, `:244`);
the search piece: `crates/smith-protocol-channel/src/turn.rs:246` and
`src/transcript.rs:259`.
Shape: `validate::turn_in_transcript(required, available) ->
Result<(), Refusal>`, `resume_holds_window`, `turn_in_channel`, each
naming both sides as `validate::calls_in_transcript` does
(`src/profile/validate.rs:296`). `host_limits(&Profile, &service::Limits)
-> host::Limits`. `decode_transcript(piece: &[u8], turns: u32, limits:
&wire::v3::Limits, endpoints: &Endpoints)`.
Test shape: unit in `src/profile/tests.rs`, one refusal per new check,
like `transcript_blocks_hold_the_largest_completion_or_name_the_ceiling`
(`:387`); property: `random_declarations_refuse_by_name_or_hold_the_checked_relationships`
(`:254`) extended to the three checks; world (`tests/channel/tests/stories.rs`):
`a_completion_of_every_call_it_may_make_is_told_and_acknowledged`,
`a_window_that_holds_one_turn_of_the_largest_size`,
`a_window_of_more_than_sixty_four_turns_resumes`; unit in
`smith-protocol-channel`: `a_search_hit_holding_all_of_search_bytes_is_told_and_resumed`,
`a_found_piece_whose_hits_pass_its_end_is_malformed`,
`a_piece_whose_lengths_pass_its_end_is_malformed`,
`a_piece_of_more_turns_than_the_resume_limit_is_too_large`; `tests/channel` and
`tests/hosts`' `memory.rs` at their derived worst cases. Ceilings: unit
20 ms, world 0.5 s, the property at `main`'s number of cases.
Proof: world, `a_completion_of_every_call_it_may_make_is_told_and_acknowledged`:
a completion of `calls_per_response` calls is told and acknowledged; and
the three refusals by name, each failing on the parent, which never runs
them. `parallel-read` would show it live; not run in this plan.
Don't: reserve a schema's ceiling as memory (rule 6); lower a value of
shell.md's to fit a ceiling; raise a ceiling past what the generator
accepts.
Size: at most 350 lines of production code.
Removes: `smith_transcript::CEILINGS` and `smith_channel::CEILINGS` as
the agent's limits; the hand-set bytes of `host_limits`; io's `sockets:
32`; the channel's `start_transcript_item` and `Start`'s `list 64`.

### 04.13 `memory` made real
Scope: skein's memory pool is the connection component's `memory`
(`llm_pool`) on `main`, so a call waits at admission (9bbd957b). Left
(limits.md, section 5): `Declared::memory` is a value, which `standard()`
declares as 2 GiB and the configuration may override; the `memory` check
compares the service's summed worst case with it; the process worlds declare
their own memory and pool, at most 2 GiB and `standard()`'s pool.
Needs: nothing beyond the session's start.
Files: `crates/smith-agent-service/src/profile/quantities.rs:168`,
`src/profile.rs:56`; `crates/smith-agent-shell/src/config.rs:244`,
`:563`, `:980`; `docs/development/local-host.md:120`, `:231`. Users of
`declared.memory`: `crates/smith-agent-service/src/tests.rs`,
`src/calls.rs:488`, `src/profile/tests.rs:13`,
`src/profile/room/tests.rs:125`, `tests/sink_memory.rs:36`;
`crates/smith-local-service/src/tests.rs:464`;
`crates/smith-local-shell/src/local_settings.rs:615`;
`tests/agent-process/src/lib.rs:131`, `tests/agent-process/tests/fuzzy_profile.rs:12`;
`tests/hosts/src/lib.rs:99`; `tests/session/tests/window_room.rs:13`,
`tests/session/tests/startup_receiving.rs:12`;
`tests/end-to-end/support/mod.rs:259`, `support/live.rs:158`.
Shape: `pub memory: u64` in `Declared`.
Test shape: memory (`tests/agent-process/tests/memory.rs`, beside `:50`):
`every_conversation_open_with_the_pool_full_fits_the_worst_case`, against
skein's counting allocator; property (`src/profile/tests.rs`):
`the_worst_case_does_not_grow_with_connections_times_the_request`.
Ceilings: memory test 0.5 s; property 0.5 s.
Proof: world, `every_conversation_open_with_the_pool_full_fits_the_worst_case`:
peak allocation within `worst_case`, with `memory` at `standard()`'s
2 GiB. The passive `peak_rss_bytes` check would show it live; not run in
this plan.
Don't: declare `u64::MAX` or 1 TiB in a world to pass a check; unit
tests of one other relationship may.
Size: at most 120 lines of production code.
Removes: `memory: Option<u64>` and "required until the real pool lands";
"profile.declared.memory is required"; the worlds' 1 TiB and 8 TiB
`memory` and 1 TiB `llm_pool`.

### 04.21 Effective limits: the LLM path, machine, channel and io
Scope: the rest of limits.md, section 9's list: per endpoint, skein's
derived LLM limits (`llm.<field>`, with its `http` and `sse` parts) and
the connection component's; the machine's; the transcript's and the
channel half's; the event sink's; io's.
Needs: 04.7; 04.9, so that the transcript's and channel's limits listed
are derived ones.
Files: `crates/smith-agent-service/src/profile/effective/`;
`crates/smith-agent-shell/src/agent_shell.rs`, `src/config.rs`;
`tests/agent-process/src/tracing.rs:115`.
Shape: `effective` gains `endpoints: &llm::Endpoints`, for each
endpoint's own derived client limits.
Test shape: the tests file of 04.7; tests
`each_endpoint_names_its_own_request_bound`,
`the_channels_turn_frame_is_named_under_channel`. Ceilings: 20 ms.
Proof: unit, `each_endpoint_names_its_own_request_bound`: two endpoints
of different windows give two `llm.request` entries with their own
values; it fails on the parent, which names none.
Don't: name a schema ceiling as a derived limit.
Size: at most 400 lines of production code.
Removes: nothing.

### 04.14 The `standard` profile at shell.md's values
Scope: `standard()` declares shell.md, 6.3's deployment values
(`tool_payload` 40 KiB, `file_bytes` 1 MiB, `read_window` 32 KiB, `guide`
16 KiB, `shell_output` and `shell_head` 16 KiB, `shell_tail` 48 KiB,
`search_hits` 100, `search_bytes` 16 KiB, `list_entries` 512) and 6.5's
policy values (`inbox` 64, `check_timeout` 5 minutes, `group_stop` 1 s,
`cancel_grace` 10 s; `sections`, `host_tools`, `verdicts`, `items` and
`fields` at the charter schema's ceilings), with 6.2's `connection_keep`
of 30 s. With 6.1's models declared, it derives, passes the startup
checks of limits.md, section 4, and fits `memory`. It adds the check
`cancel-over-close` (section 4), refusing by name a `cancel_grace` under
twice io's `close` (decisions.md, 12.23).
Needs: 04.9 (its three checks) and 04.13.
Files: `crates/smith-agent-service/src/profile/quantities.rs`
(`standard`), `src/profile/validate.rs`; `crates/smith-agent-shell/src/config.rs:1073`
(`product_document`'s overrides go); `crates/smith-local-shell/src/local_shell.rs:278`;
the process worlds' profiles (`tests/agent-process/src/lib.rs:120`).
Shape: `standard()`, with values from `smith_charter::CEILINGS` where
6.5 says "the charter schema's ceilings"; `validate::cancel_over_close(
required, available) -> Result<(), Refusal>`, `required` twice `close`.
Test shape: `crates/smith-agent-shell/src/config.rs:1202`
(`product_values_pass_every_startup_check_and_fit_complete_memory`) runs
on `standard()` with 6.1's models and no overrides; the same in
`crates/smith-local-shell/src/local_shell.rs:278`. Neither asserts a
shipped value. Unit in `src/profile/tests.rs`:
`cancel_grace_under_twice_close_is_refused_as_cancel_over_close`
(`close` 5 s, `cancel_grace` 9,999 ms). Ceilings: 20 ms each.
Proof: unit, `product_values_pass_every_startup_check_and_fit_complete_memory`
on `standard()` unchanged: the startup checks pass and the worst case fits
2 GiB. `tool-output` and `large-write` would show it live; not run.
Don't: change a per-model value (configuration's, and session 10's
presets); change `exit_grace` (smith 06).
Size: at most 90 lines of production code.
Removes: the transitional values and the doc line "Today's transitional
bundle".

### 04.22 The local host's effective limits
Scope: the product's `session.started` carries its effective limits
(events.md, 3.2): the agent's list, then the local domain's, the inline
agent's and, for a spawned agent, the host kit's and the process
adapter's, as `local.<field>`, `inline.<field>`, `host.<field>`.
Needs: 04.21; smith 03's product stream (03.7).
Files: `crates/smith-local-service/src/profile.rs`;
`crates/smith-local-shell/src/local_host.rs` (the product's header).
Shape: `smith_local_service::profile::effective(&Profile,
&Configuration, &agent_service::Config, &Limits) -> Option<Box<[Limit]>>`,
the agent's list (from the configuration's limits and endpoints)
followed by the local layers'.
Test shape: `tests/local-process/tests/events.rs` (smith 03.7's file);
test `the_products_header_carries_its_local_limits`. Ceiling: 0.5 s.
Proof: world, `the_products_header_carries_its_local_limits`: the
product's trace header holds `local.line_bytes` and the agent's
`declared.memory`; it fails on the parent, whose product header is empty.
Don't: rename the agent's entries in the product's list.
Size: at most 180 lines of production code.
Removes: nothing.

### 04.s1 A small window lowers the material's bounds, not the run
Added during the run (decisions.md, 13.55). Scope: smith 00.16's
`history/small-window` (`probes-history-small-window-ad9c80ac-20261012T174018Z/01`)
ends `Refused(invalid host_part)` before any request: its window derives
`host_part` 8,874 bytes, and the charter's host bytes reserve the declared
`guide`, 16,384 per directory, with no `AGENTS.md` in the seed.
session.md, 8.1: "Where a declared window would not hold that material,
the derivation lowers the material's bounds, never the window
(protocol/limits.md)." The derivation lowers `guide` (and any other
material bound the opening reserves) so that the opening fits
`host_part`; a run is refused only where no material bound can fit.
Needs: nothing.
Files: `crates/smith-agent-service/src/profile.rs` (the derivation, `:173`,
`:272`, `:559`); `crates/smith-domain-run/src/prompt.rs` (`host_bytes`,
:172), `charter.rs` (`check_window_parts`, :297), `run.rs` and
`prepare.rs` (the run's private guide bound, decisions.md 13.57); their
unit tests.
Shape: no new type.
Test shape: a unit test, `a_small_window_lowers_the_guide_bound_and_admits_the_run`,
at small-window's declared window: the derived `guide` fits `host_part`
and the charter is admitted; it fails on the parent. Under 20 ms.
Proof: that test, once, failing on the parent; then 00.16's
`history/small-window` calibration is its live proof.
Don't: lower the window or `host_part`; pin a smaller `guide` in the
probe's settings to pass. If the refusal comes from elsewhere, stop and
ask with where.
Size: README rule 9's.

### 04.s2 A timed-out command's terminal says `timed_out`
Added during the run (decisions.md, 13.65). Scope: smith 13.5's live
`tool-output/timeout` (3 of 3 attempts): a `sleep 600` under
`timeout_seconds` 1, then 9,999 (clamped), renders "command timed out" to
the model, but each `tool.completed` says `exited`, with no exit.
events.md, 3.6: "A tool's terminal tells what the tool observed when it
ended", and its verdicts include `timed_out`. `verdict_kind`
(`smith-agent-service/src/calls.rs:426`) maps every `Exited` to `exited`,
and `command_exit` (:391) drops `Exit::TimedOut` to none. A command
whose exit is `TimedOut` is projected as `timed_out`; every other case
stays.
Needs: nothing.
Files: `crates/smith-agent-service/src/calls.rs`; its unit tests.
Shape: no new type.
Test shape: a projector unit test,
`a_timed_out_command_is_projected_as_timed_out_with_its_bound`, failing on
the parent. Under 20 ms.
Proof: that test, once; then `tool-output/timeout` once, live (one
batch), its counts in the commit body.
Don't: change the rendered result, the clamp or the checker.
Size: at most 40 production lines.

## Interface sketches

**04.7.** The types on `main` stay: `LimitName`, `Layer`, `Per`, `Value`,
`Limit` (`crates/smith-agent-service/src/profile/effective.rs`).

```rust
// crates/smith-agent-service/src/profile/effective/list.rs
/// The declared, policy and derived values of the agent, each under its name (limits.md, section 9).
pub fn effective(profile: &Profile, configuration: &Configuration, limits: &service::Limits) -> Option<Box<[Limit]>>;

// crates/smith-agent-service/src/profile/effective/header.rs
/// The list as `session.started`'s `limits`: names spelled, durations in milliseconds.
pub fn header(
    limits: &[Limit],
    names: &smith_protocol_channel::Endpoints,
    configuration: &Configuration,
) -> Option<List<smith_events::EffectiveLimit>>;

// crates/smith-agent-service/src/events.rs
impl EventLimits {
    /// `header`: the effective limits the stream's first record carries.
    pub fn derive(
        limits: &crate::Limits,
        capture: Capture,
        destinations: u32,
        option_bytes: u32,
        header: &[smith_events::EffectiveLimit],
        write_deadline: Duration,
    ) -> Option<EventLimits>;
}
```

**04.9.** The three checks name both sides, as the others do:

```rust
// crates/smith-agent-service/src/profile/validate.rs
/// `turn-in-transcript`: the largest turn's fields within the transcript schema's.
pub fn turn_in_transcript(required: u64, available: u64) -> Result<(), Refusal>;
/// `resume-holds-window`: the resume limit holds one window's turns.
pub fn resume_holds_window(required: u64, available: u64) -> Result<(), Refusal>;
/// `turn-in-channel`: the largest turn, start and message within the channel's frames.
pub fn turn_in_channel(required: u64, available: u64) -> Result<(), Refusal>;
```

**04.13.**

```rust
// crates/smith-agent-service/src/profile/quantities.rs (Declared)
/// The process's real memory, in bytes: `standard()` declares 2 GiB.
pub memory: u64,
```

## Questions to raise when reached

1. **The 2 GiB target** (04.14). If `standard()` at shell.md's values,
   with 6.1's models and the charter's ceilings as policy, does not fit 2
   GiB, raise which value moves, with the numbers. Do not shrink a
   decided value, or raise `memory` past 2 GiB.

## Done when

- The agent's header carries its limits: `heavy cargo nextest run -p
  smith --test end_to_end -E 'test(effective_limits)'` passes.
- No empty header is written: `rg -c 'limits: List::with_capacity\(0\)'
  crates/smith-agent-shell/src/agent_shell.rs` prints nothing.
- The four named checks run: `rg -o 'Check::(TurnInTranscript|ResumeHoldsWindow|TurnInChannel|CancelOverClose)'
  crates/smith-agent-service/src/profile | sort -u | wc -l` prints 4.
- No transitional memory: `rg -c 'memory: None|Option<u64>' crates/smith-agent-service/src/profile/quantities.rs`
  prints nothing, and `rg -c '1_099_511_627_776|8_796_093_022_208|1_u64 << 40' crates tests`
  prints nothing.
- The standard profile fits: `heavy cargo nextest run -p smith-agent-shell
  -E 'test(product_values_pass_every_startup_check_and_fit_complete_memory)'`
  passes, and `rg -c '"tool_payload":40960' crates/smith-agent-shell/src/config.rs`
  prints nothing (its document's overrides are gone; 1 on `main`).
- The product's header carries its local limits: `heavy cargo nextest run
  -p smith-local-process-world -E 'test(the_products_header_carries_its_local_limits)'`
  passes.
