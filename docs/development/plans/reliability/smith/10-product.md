# Session 10: the product

Provisional, 2026-10-12. What is left of the product of shell.md,
sections 4 to 9: the chat store and startup reads over skein; TOML
settings, presets and `smith check`; credentials on skein's OAuth driver
and `smith login`; `smith exec`; the interactive `smith`; the build
environment; raw terminal input; and the end of `smith local`, its JSON
settings and `smith agent`. It matters because a run still cannot say how
it ended (P14), a first run needs expert JSON and a hand-written token
record (P16, P17, T11, T12), and the shell keeps std files and an OAuth
loop of its own beside skein's.

## Read first

- smith `shell.md`, sections 1 to 11; `domain/host.md`, sections 7 and
  8; `protocol/hosts.md`, 5.1 to 5.5; `protocol/events.md`, 3.2, 3.5 and
  5.2; `domain/run.md`, 3.2, 6 and 10; `testing.md`, 2.3;
  `benchmarks.md`, sections 6 and 10.
- skein `oauth.md`, section 6; `io.md`, 5.2, 5.3 and 7; `shell.md`, 6.1
  to 6.3 and 13.
- decisions.md, 6.7, 7.9 to 7.11, 8.4 to 8.7 and 12.11; findings P14 to
  P17, T10 to T12, O13 and O16.

## What the design asks

- **One process** (shell.md, 9): the local domain owns the inline agent;
  "the agent configuration is a value the shell builds from presets and
  settings, never a file"; the loop is `drive`.
- **Four commands** (4.1 to 4.5) with shared flags; exit codes 0 to 5
  and 130 by the answer's class, and one `smith: ` last line "rendered
  from `run.completed`'s typed failure or the startup refusal, matched
  exhaustively" (4.3).
- **Settings** (5.1 to 5.4): TOML, strict and bounded, layered built-ins
  < user < `.smith/` < `-c` < flags; "tables merge, values replace"; the
  repository layer "shapes runs, not authority"; refusals name file,
  line, key and relationship. Presets and run defaults: 6.1, 6.2, 6.4.
- **Credentials** (7; hosts.md, 5.4): one source per account; `borrow`
  takes "the access token and the account's identifier, and nothing
  else"; the expiry notice names the file and the program.
- **The build environment** (8) and **terminal modes** (2.4: "Until
  skein has them, the interactive front end runs in line mode").

## Choices this plan makes

- **New modules of `smith-local-shell`:** `command.rs`, `settings/`
  (`layers.rs`, `schema.rs`, `resolve.rs`), `presets.rs`, `check.rs`,
  `credentials.rs`, `compose.rs`, `front/` (`headless.rs`,
  `interactive.rs`). `local_host.rs` stays the `Local` adapter.
- **TOML with `toml` 0.9 and `serde`,** as `benchmarks/` has; each file
  layer read with `skein_shell::read_file`.
- **Chats by workspace:** `$XDG_STATE_HOME/smith/chats/<digest of the
  workspace's canonical path>/<chat>/`, the path recorded inside once.
- **`Command::Legacy`** keeps `smith agent` and `smith local` as on
  `main` until 10.16; the spawned kind stays a composition parameter.
- **`resolve` runs `smith-agent-shell`'s configuration checks,** so the
  value and the spawned kind's JSON pass one set of checks.
- **Splits and order.** The old 10.3 is 10.3, 10.13 and 10.14 (size, and
  10.14 waits on smith 04); the old 10.5 is 10.5 and 10.15 (the harness);
  the old 10.8 is 10.8 and 10.16 (the removals). 10.7 comes before 10.15,
  so the harness moves straight to a borrowed login; 10.6 after it, so no
  harness converts a record for the new keeper.
- **Formats** (README, rule 5): the chat store's header version 1 (10.1),
  `session.started`'s `versions.chat_store` (10.5), the check document's
  version 1 (10.3), all under events 1.

## Rules that bind it

- **The harness follows the binary** (README, rule 4): 10.15 and 10.8
  move `smith-bench`'s smith adapter; the legacy face is never edited.
- **Never touch the user's logins** (README, rule 7): an end-to-end test
  sets `HOME`, `XDG_CONFIG_HOME` and `XDG_STATE_HOME` to directories in
  its scratch; a borrowed login in a test is a fixture file there.
- **Removing a mechanism removes its code, limits, accounting, facts,
  tests and docs** (code-organization.md, 7); a shell gets no new
  `#[expect(clippy::disallowed_*)]`.

## Landed in the first run

| Increment | Commit | Proof |
|---|---|---|
| 10.11 startup refusals | `f7892de3` | proven |
| 10.12 the reserve's entry | `066ca827` | proven |

## Increments

### 10.1 The chat store over io
Scope: a chat's state, turns and delivery records kept by
`smith-local-protocol` through io's `Store` (skein io.md, 5.2), `Load`,
`Scan`, `MakeDirectory` and `Remove`, within hosts.md 5.3's bounds; the
local service routes the domain's store requests to it, and its io
requests to its `FileIo`; the shell opens the state directory as a root
and owns no file once the loop runs. A fresh history keeps today's
journal order (`local_store.rs:191`). Where smith 08's window turns
resumed by hosts [08.2] have landed, the store keeps their
`opens_window` flag and their bound on the current window's files.
Needs: nothing.
Files: `smith-local-protocol` (`store.rs`, `lib.rs`);
`smith-local-service` (`service.rs`, `process.rs:85`); `smith-local-shell`
(`local_host.rs`, `local_shell.rs`); users of `local_store::Store`:
`tests/real/src/{lib.rs,settled.rs}`, `tests/local-process/src/world.rs`.
`Resources::state_directory: PathBuf` becomes `state_root: kernel::Fd`,
opened before `Local::new` (decisions.md, 13.7); its users:
`tests/local-process/src/process.rs` (`local_roots`, `make_local`, :656),
`crates/smith-local-shell/tests/local_library.rs:120`. The worlds' own
`Launch::state_directory`, the path a test reads or faults, stays.
Shape: `smith_local_protocol::{Store, StoreRequest, Action, StoreFault,
StoreLimits}` (sketch), with its `Expect` rule.
Test shape: unit in `crates/smith-local-protocol/src/tests.rs`:
`a_turn_file_is_stored_expecting_its_name_absent`,
`a_store_of_unknown_durability_is_sent_again_once`, each record's
refusals. World `tests/local-process/tests/fuzzy_store.rs` (new), with
`World::cut(seed, files, before)` beside `World::new` (`world.rs:304`)
over `skein_world::World::spawn_restartable` and `cut(.., Cut::PowerLoss)`:
```rust
#[test]
fn a_power_loss_at_any_store_operation_resumes_without_reusing_an_activation() {
    for before in 0..12 {
        let files = Files::new();
        World::cut(70 + u64::from(before), &files, before).settle();
        let mut second = World::new(90, &files, Placement::InProcess, b"Second answer");
        second.settle();
        assert!(second.activations().windows(2).all(|pair| pair[0] < pair[1]), "cut {before}");
    }
}
```
Ceilings: unit 20 ms; this sweep 5 s, in the fuzzy suite.
Proof: world, the test above: a chat resumed after a power loss at any
store operation never reuses an activation; once, within 5 s.
Don't: never `Expect::Any`; no std file in the shell once the loop runs.
Size: at most 1,050 lines of production code, in one landing (decisions.md, 13.15 and 13.29; was 550); README rule 9's 600 does not apply to 10.1.
Removes: `smith-local-shell/src/local_store.rs` and its two
`#[expect(clippy::disallowed_types)]`.

### 10.2 Startup reads and trust roots from skein
Scope (skein shell.md, 6.1 and 6.2): `smith_agent_shell::config::read`
and `local_settings::read` read with `skein_shell::read_file`;
`config::trust` reads the machine's roots or one DER file with
`skein_shell::read_trust_roots` and `skein_tls::Config::from_der`.
Needs: nothing.
Files: `smith-agent-shell/src/config.rs` (`read` :187, `trust` :771;
callers :445 and `smith-local-shell/src/local_auth.rs:119`),
`smith-local-shell/src/local_settings.rs:532`; three manifests.
Shape: `trust` keeps its signature; bounds from `TRUST_BYTES` (:768).
Test shape: unit in `config.rs`'s tests (:806):
`a_trust_file_that_is_not_a_regular_file_is_refused_naming_it`,
`a_machine_bundle_past_its_certificate_bound_is_refused_naming_the_bound`;
20 ms each.
Proof: process, `the_shipped_local_and_agent_answer_on_a_terminal`
(`tests/end-to-end/end_to_end.rs:7`) passes with the fake provider's DER
root read by skein; once, under 2 s.
Don't: no TLS crate in the shells but `skein-tls`.
Size: at most 120.
Removes: `rustls-native-certs` (root, agent shell and local shell
manifests) and the `#[expect]` at `config.rs:774`.

### 10.4 Headless runs in the local host
Scope (domain/host.md, 8; shell.md, 4.2): `Config.mode` replaces
`Config.waiting`. Headless, the start carries the prompt as its one
message labelled `person`, nothing more is relayed, `wait` has a waiting
time of zero, and the host ends at the answer; the contract as today.
`ExitStatus` follows the answer's class (table under the sketches).
Needs: nothing.
Files: `smith-local-domain` (`config.rs:62`, `boundary.rs`, `domain.rs`,
`end.rs`, `tests.rs:66`, `parent_tests.rs`); users:
`smith-local-service/src/tests.rs:91`, `smith-local-shell` (`local_settings.rs:319`,
`local_host.rs:510` and :622, `tests/local_library.rs`), `tests/local`
(`src/world.rs:645`, `tests/memory.rs:57`, `simulation.rs`,
`fuzzy_simulation.rs`), `tests/local-process/src/process.rs:332`.
Hotspot: `domain.rs`.
Shape: `smith_local_domain::{Mode, ExitStatus}` (sketch).
Test shape: `tests/local/tests/simulation.rs`, with `World::headless(seed,
prompt)` beside `World::new` (`tests/local/src/world.rs:245`), written as
`a_question_is_answered_and_the_chat_waits_for_the_next` (:41), checking
`exit()` (:773): `a_headless_run_that_would_wait_parks_at_once_and_needs_input`,
`a_headless_prompt_opens_the_run_and_its_answer_ends_the_host`,
`a_headless_run_out_of_turns_ends_with_the_budget_status`,
`a_headless_run_whose_model_fails_ends_run_failed`,
`a_headless_run_without_a_credential_starts_nothing`,
`an_interrupt_ends_a_headless_run_interrupted`,
`a_headless_delivery_commits_in_place`; 0.5 s each.
Proof: world, `a_headless_run_that_would_wait_parks_at_once_and_needs_input`:
the run parks at once and the host exits `NeedsInput`; once, under 0.5 s.
Don't: the shell never decides a class; it numbers the domain's.
Size: at most 300.
Removes: `ExitStatus::{Success, Failed}`.

### 10.3 Settings and `smith check`
Scope (shell.md, 4.4 and 5.1 to 5.3): `parse` for `check` and the shared
flags; the layers merged with each value's origin; a file at most 1 MiB;
an unknown key, a wrong type or a value past its bound refused naming
file, line and key path; the repository layer's forbidden keys refused
naming the file, and nothing else of it refused or held for approval
(shell.md, 5.1, settled 2026-10-12): its `[environment].set` values are
accepted and listed by name with their layer; 5.2's and section 7's schema. `smith check [--json]`:
the settings with their layers, credentials redacted, each format's
version, the build's identity, `sh`, `rg` and `git` resolved, the
commands' environment by name; exit 0 or 2. Presets and relationships
are 10.13's, credential states 10.7's and 10.6's, warnings 10.9's,
limits 10.14's.
Needs: nothing.
Files: `smith-local-shell` (`command.rs`, `settings/`, `check.rs`;
`Cargo.toml`), `src/main.rs`, `tests/end-to-end/{end_to_end.rs,support/mod.rs}`.
Shape: `command::{Command, Shared, Usage, parse}`; `settings::{Layer,
Origin, Sourced, Source, Settings, Refusal, read}`; `check::{Report,
Verdict, report, render_text, render_json, CHECK_VERSION}` (sketch).
Test shape: unit in `smith-local-shell/src/settings/tests.rs` (new) and
`command.rs`, at most 12: layers merged, one refusal per strict rule, a
forbidden key, the argument forms, a usage error. End to end through a
new `support::smith(scratch, arguments, input) -> Ran` over
`Mode::Pipes`, beside `refusal_code` (`support/mod.rs:455`), setting the
scratch's `HOME` and XDG directories (`start`, :217, sets none):
```rust
#[test]
fn a_refusal_in_the_user_layer_names_its_file_line_and_key() {
    let scratch = Scratch::new();
    let file = support::user_layer(&scratch, b"model = \"codex/gpt-6-luna\"\nmodle = 1\n");
    let ran = support::smith(&scratch, &["check"], b"");
    assert_eq!(ran.code, 2);
    let line = String::from_utf8(ran.stderr).expect("UTF-8");
    assert!(line.starts_with("smith: ") && line.contains(&format!("{}:2", file.display())) && line.contains("modle"));
}
```
Also `smith_check_json_shows_each_value_with_its_layer`,
`a_repository_layer_setting_an_account_is_refused_naming_the_file`,
`an_override_past_its_bound_is_refused_naming_the_override`. Ceilings:
20 ms, 2 s.
Proof: process, the test above; once, under 2 s.
Don't: no test reads the user's own configuration or state.
Size: at most 600.
Removes: nothing.

### 10.13 Presets, and the run resolved from settings
Scope (shell.md, 5.2 to 5.4, 6.1, 6.2, 6.4): presets `codex` and
`anthropic`: an endpoint and an account of each name, their models'
declared values and notional prices; a model no preset knows declares
`window`, `output` and prices; `resolve` builds the agent configuration
(smith 04's `Profile`, `[deployment]` over `standard()`) and the local
`Config` as values; each relationship checked by name
(`smith_agent_service::profile::derive`), the budget against the ceilings
and one completion's cost, each model priced above zero, each address
resolved; `smith check` names a relationship with both sides.
Needs: nothing.
Files: `smith-local-shell` (`presets.rs`, `settings/resolve.rs`,
`check.rs`), `smith-agent-shell/src/config.rs`, `tests/end-to-end/end_to_end.rs`.
Shape: `presets::{Preset, built_in}`; `resolve(settings: &Settings,
command: &Command) -> Result<Resolved, Refusal>`, `Resolved { agent,
local, accounts }`.
Test shape: unit in `settings/tests.rs`: the built-in layer against a
byte-stable golden, one refusal per relationship. End to end
`smith_check_names_both_sides_of_a_budget_past_the_profile_ceiling`,
expecting 5.3's line `smith: budget.turns = 5000 (from -c) exceeds the
standard profile's ceiling of 4096 turns`. Ceilings: 20 ms, 2 s.
Proof: process, that test; once, under 2 s.
Don't: the profile's values stay the service crates' `standard()`.
Size: at most 500. Removes: nothing.

### 10.7 `borrow` and `env`
Scope (shell.md, 7; hosts.md, 5.4): `smith-local-protocol` decodes
Codex's `auth.json` with skein-json's collector: the access token and the
account id, expiry from the token's claims; the local service owns
`skein-oauth-accounts`' `Component` with handed-in accounts: `borrow`
read at startup and again through io's `Load` when a grant is due, `env`
once; the driver's `Expiring` and expiry become shell.md 7's notice; no
run starts on an expired login; `smith check` reports these accounts,
exit 3 when only credentials fail. Until 10.6, `smith local`'s
`sign_in` accounts keep the shell's own OAuth.
Needs: nothing.
Files: `smith-local-protocol` (`borrowed.rs`, `lib.rs`, `Cargo.toml`);
`smith-local-service` (`service.rs`, `Cargo.toml`); `smith-local-domain`
(`CredentialFailure`, and its users `smith-local-service/src/service.rs`,
`smith-local-shell/src/local_host.rs`); `smith-local-shell`
(`settings/schema.rs`, `check.rs`, `credentials.rs`); root `Cargo.toml`;
`tests/local-process`
(`src/world.rs`, `tests/borrowed.rs`).
Shape: `smith_local_protocol::{LoginFormat, Borrowed, LoginRefusal,
decode_login}`; `check::Credential` (sketch).
Test shape: unit in `smith-local-protocol/src/tests.rs`, logins shaped as
`benchmarks/tests/codex_adapter.rs:177`: valid, expired, malformed,
`the_refresh_token_of_a_borrowed_login_never_reaches_the_record`. World
`tests/local-process/tests/borrowed.rs` (new), `World::borrowed(seed,
files, Borrowing)` beside `World::authenticated` (`world.rs:435`):
```rust
#[test]
fn an_expired_borrowed_login_starts_no_run_and_names_its_program() {
    let files = Files::new();
    let mut world = World::borrowed(41, &files, Borrowing::Expired);
    world.settle();
    assert!(world.queries().is_empty());
    assert!(world.shown().windows(16).any(|bytes| bytes == b"Run `codex` once"));
    review(&world.seen(), &world, Ending::Unavailable).assert_passed(41);
}
```
Also `a_borrowed_login_lends_its_grant_and_its_file_is_never_written`,
`a_login_refreshed_by_its_program_is_read_again_when_a_grant_is_due`,
`an_env_account_lends_its_variable`. Ceilings: 20 ms, 0.5 s.
Proof: world, the test above; once, under 0.5 s. The `credentials`
probe (smith 00.30) shows it live; it is not this increment's proof.
Don't: never a refresh token in smith's state, a write to a login file,
or a refresh of another tool's login; no test reads `~/.codex`; no
credential value in settings shown, events, traces, lines or checks.
Size: at most 550.
Removes: nothing.

### 10.9 The build environment
Scope (shell.md, 8): `[environment]` `inherit` (the core list by default,
names only) and `set`; `[build]` preset `rust` (`CARGO_HOME`,
`RUSTUP_HOME`; `$CARGO_HOME/registry` and `$CARGO_HOME/git` as caches)
and `caches`; `smith check` checks each cache, warns of a configured
wrapper whose cache is not among them, naming what configured it, and of
a credential-like name. The result is the agent configuration's
environment, smith 09's single owner.
Needs: nothing.
Files: `smith-local-shell` (`settings/`, `presets.rs`, `check.rs`),
`tests/end-to-end/end_to_end.rs`.
Shape: `check::Warning::{WrapperCache, CredentialLikeName}`.
Test shape: unit in `settings/tests.rs`:
`inherit_takes_named_variables_never_a_pattern`,
`the_rust_preset_adds_its_homes_and_caches`,
`a_credential_like_name_is_warned`; end to end
`smith_check_json_warns_of_a_wrapper_cache_outside_the_caches`.
Ceilings: 20 ms, 2 s.
Proof: process, the end-to-end test: exit 0, the warning naming
`RUSTC_WRAPPER`; once, under 2 s.
Don't: `smith check` creates no cache.
Size: at most 300. Removes: nothing.

### 10.5 `smith exec`
Scope (shell.md, 4.2 and 4.3; events.md, 5.2): the command and its usage
errors; the headless local domain composed inline (`compose`); chats by
workspace, a new one named in `session.started` and on standard error,
`--chat`, `--ephemeral`, `--deliver`; the result on standard output and
`-o FILE`; standard error's status from the records, provider text
clipped; the last line from smith 06's words; the code from
`ExitStatus`; a forced exit's warning; `--json`, `--trace` and `[trace]`
sinks; `session.started` with mode `exec` and `versions.chat_store`.
Needs: smith 03's product stream [03.7]; smith 06's words function,
`smith_events::words` [06.2].
Files: `smith-local-shell` (`command.rs`, `compose.rs`,
`front/headless.rs`, `local_host.rs`); `smith-local-service`;
`smith-events` (`records.rs:401`, `read.rs`, `write.rs`) and the users of
`Versions`: `smith-agent-shell/src/agent_shell.rs`,
`tests/agent-process/src/tracing.rs`, `tests/codecs/tests/support/`;
`src/main.rs`; `tests/end-to-end`.
Shape: `front::headless::{ExitCode, Ending, Headless}`;
`compose::{start, open, compose, AgentKind, Refused}`.
Test shape: unit in `front/headless.rs`: each `ExitStatus` to its code
and last line, 4.3's forms as literals. End to end with the fake provider
through `support::exec_with_provider`, beside `run_with_provider`
(`support/mod.rs:290`):
```rust
#[test]
fn a_headless_run_that_would_wait_exits_5_naming_its_chat() {
    let scratch = Scratch::new();
    support::user_layer(&scratch, &support::fake_provider_settings(&scratch));
    let ran = support::exec_with_provider(&scratch, &["exec", "--chat", "plan", "-"], b"Ask first\n", Reply::Wait);
    assert_eq!(ran.code, 5);
    assert!(ran.stderr.ends_with(b"smith: needs input: answer with `smith exec --chat plan`\n"));
}
```
Also `smith_exec_json_ends_accepted_with_its_events_and_exit_0`,
`an_account_without_a_credential_exits_3_before_any_request`,
`an_empty_prompt_is_a_usage_error_exiting_2`,
`the_trace_file_is_read_as_it_grows`. Ceilings: 20 ms, 2 s.
Proof: process, the test above; once, under 2 s; live in 10.15.
Don't: `exec` never signs in; no agent configuration file; no words
function beside smith 06's.
Size: at most 600.
Removes: nothing.

### 10.15 The harness on `smith exec`
Scope (benchmarks.md, 6 and 10): for commits from this one, the smith
adapter writes its pinned settings as the attempt's user layer, state in
its `XDG_STATE_HOME`, and an account borrowing the user's
`~/.codex/auth.json`; runs `smith check --json` before an arm's first
attempt, a nonzero exit refusing the arm as setup; runs `smith exec
--json -C <workspace> -m <endpoint>/<model> --effort <effort> -`;
classifies by the exit code held against `run.completed`. The live
suite runs through `smith exec --json`, borrowing the file its caller
names (testing.md, 2.3). It adds the variant `probes/first-request/headless`,
the task in the start, to the task and `suites/probes.toml` (decisions.md, 12.21).
Needs: session 00's one command per probe run [00.27], for its proof.
Files: `benchmarks/src/agent/smith/` (`prepare.rs:84`, `settings.rs`,
`faces.rs`, `events.rs`, `events/classify.rs`),
`benchmarks/agents/smith/faces.toml`, `benchmarks/src/credential.rs`,
`benchmarks/tasks/probes/first-request/task.toml`, `benchmarks/suites/probes.toml`,
`benchmarks/tests/smith_adapter.rs`, `benchmarks/README.md`,
`tests/end-to-end/{live.rs,support/live.rs}`. Hotspot: `benchmarks/src/`.
Shape: `events::Shape::Exec`'s invocation; `CredentialShape::Borrowed`.
Test shape: offline in `benchmarks/tests/smith_adapter.rs`:
`an_exec_arm_borrows_the_login_file_without_copying_it`,
`an_exec_arm_refused_by_check_is_a_setup_refusal`,
`an_exec_arm_classifies_exit_5_as_needs_input`; 0.5 s each.
Proof: live, `heavy cargo run -p smith-bench -- build HEAD`, then `heavy
--gate cargo run -p smith-bench -- probe PROBE --arm <the printed
directory>`, one attempt each, for PROBE `probes/failures/one-turn`,
`probes/failures/bad-setting`, `probes/failures/unknown-model`,
`probes/failures/derived-limit` and `probes/first-request/headless`:
each passes. Before: the parent has no `smith exec`; the
closest evidence is 10.5's end-to-end tests (benchmarks.md, 4.2). No
Codex. At most 5 attempts, $0.50 and 15 minutes; once.
Don't: never convert, copy or write a login; never run `smith login`;
earlier registry entries and the legacy face unchanged; the live suite
names no login path of its own, and this plan never runs it.
Size: at most 400.
Removes: the "awaits its shipped product invocation" refusal for `Exec`.

### 10.6 Sign-in on skein's driver, and `smith login`
Scope (shell.md, 4.5 and 7; hosts.md, 5.4): the service's component
takes `sign_in` accounts: the preset's registration where the provider
sanctions its public client, settings' otherwise; `Keeper::Private`
under `$XDG_STATE_HOME/smith/tokens`; the account id from the claims.
`smith login ACCOUNT` exits 0, 3, or 2 for an unknown account; `smith
local`'s accounts move onto the driver; at the end a waiting sign-in is
cancelled before the driver closes.
Needs: the harness on `smith exec` with a borrowed login (10.15).
Files: `smith-local-service`; `smith-local-shell` (`credentials.rs`,
`command.rs`, `local_host.rs`, `local_shell.rs:260`, `lib.rs`);
`smith-local-protocol` (`credential.rs`, `lib.rs`); `src/main.rs`;
`tests/local-process` (`tests/authentication.rs`, `src/world.rs:435`,
`src/process.rs:222`, `src/referee.rs`); `tests/real/tests/stories.rs`;
`tests/end-to-end` (`end_to_end.rs:15`, `support/mod.rs:230`,
`support/live.rs:123`, `live.rs`).
Shape: `credentials::{Account, LoginOutcome, credential_words}`.
Test shape: `tests/local-process/tests/authentication.rs`'s four stories
(:8, :23, :37, :53) keep their names over skein's driver, the record
written in skein's private encoding; end to end
`smith_login_signs_in_against_the_fake_issuer_and_exits_0` and
`smith_login_of_an_unknown_account_exits_2`, with `BrowserProcess`
(`process.rs:395`). Ceilings: 0.5 s, 2 s.
Proof: process, `smith_login_signs_in_against_the_fake_issuer_and_exits_0`;
once, under 2 s.
Don't: no test signs in to a real issuer.
Size: at most 450.
Removes: `local_auth.rs`, `local_auth_http.rs`, `local_auth_tests.rs`,
`local_tokens.rs` and its two `#[expect]`s, `smith-local-protocol`'s
`credential.rs`, and the `$XDG_CONFIG_HOME/smith/tokens` default.

### 10.8 The interactive `smith`
Scope (shell.md, 4.1), in line mode: the workspace's chat resumed; lines
as messages, a backslash continuing one, `//`; the run shown from its
records: `text.delta` whatever the capture, each tool with its verdict
and duration, notices and failures in words, a status line after each
response; the waiting prompt only once the relayed lines are read; `/model`,
`/effort`, `/new`, `/resume NAME`, `/quit`; interrupts cancel, then
drop; end of input lets the run answer or park, and lines it left
unread open one more activation, then exit 0 (4.1); `--plain`
and no terminal; a `sign_in` account without a record shows its
address. The harness runs `probes/first-request` at `smith --trace <file>`
on a pseudo-terminal; its `headless` variant stays on `smith exec`.
Needs: smith 05's typed notices [05.11], and lines carried, held until
read, unread by outcome [05.13]; for its Proof, smith 00's one command
per probe run [00.27].
Files: `smith-local-shell` (`front/interactive.rs`, `command.rs`,
`compose.rs`, `local_host.rs`); `smith-local-protocol/src/terminal.rs`;
`smith-local-domain` (`/` commands at an activation's boundary);
`smith-local-service`; `src/main.rs`; `tests/local-process`;
`tests/end-to-end`; `benchmarks/src/agent/smith/{prepare.rs,faces.rs}`.
Shape: `front::interactive::{Interactive, Screen, StatusLine}`;
`terminal::{Entered, Slash}`.
Test shape: unit: each `/` line, the renderer per record kind. World, in
`tests/local-process/tests/simulation.rs` with `Terminal::attached`
(`src/terminal.rs:134`): `a_follow_up_typed_as_the_run_finishes_is_read_next`,
`lines_then_end_of_input_answer_then_exit_0`,
`an_interrupt_with_a_line_unread_shows_it_not_delivered`,
`slash_new_starts_a_chat_and_slash_resume_reopens_it`. End to end:
`smith_on_a_terminal_signs_in_and_commits_in_place`,
`a_second_smith_resumes_the_chat_from_its_files`,
`an_interrupt_cancels_the_run`. Ceilings: 20 ms, 0.5 s, 2 s.
Proof: live, `heavy cargo run -p smith-bench -- build HEAD`, then `heavy
--gate cargo run -p smith-bench -- probe probes/first-request --arm <the
printed directory>`, one attempt typed at `smith --trace` on a
pseudo-terminal: it passes. No before: the parent passes it through `smith local`. At most
$0.20 and 5 minutes; once. `follow-up` (smith 00.14) is not its proof.
Don't: no raw input (10.10); no `/` command reaches the run.
Size: at most 600.
Removes: nothing.

### 10.16 `smith local`, `smith agent` and the JSON settings go
Scope: `Command::Legacy`, `smith local`, `smith agent`, the JSON
settings, `in_process` and the agent configuration file written in one
process go; worlds and the sample host spawn `smith-agent` through
`AgentKind::Spawned`, with generated JSON and its `exit_grace`
(shell.md, 5.4), and the agent's trace at the product's trace path with
`.agent` appended (events.md, section 2); tests on `smith local` move to
`smith`, `smith exec` or
the composition; `docs/development/local-host.md` is rewritten for the
product, and `workflow.md`'s `cargo run -- local` line goes.
Needs: smith 03's product stream with a spawned agent [03.9]; it
follows 10.8 and 10.15.
Files: `smith-local-shell` (`local_settings.rs`, `local_shell.rs`,
`local_host.rs`, `compose.rs`, `tests/local_library.rs`); `src/main.rs`;
`tests/local-process/src/process.rs`; `tests/real`; `tests/end-to-end`;
the harness tests reading `local_settings`
(`benchmarks/src/agent/smith/settings.rs:347`,
`benchmarks/tests/{smith_adapter.rs:536,smith_producer.rs:29}`);
`docs/development/{local-host.md,workflow.md}`.
Shape: `compose::AgentKind::Spawned { program: PathBuf }`.
Test shape: the moved stories keep their names (`end_to_end.rs:7`, :15,
:36); `smith_agent_spawned_by_the_sample_host_answers` (testing.md,
2.3). Ceilings: 2 s.
Proof: process, `a_first_run_signs_in_and_ends_with_a_commit_in_place`
(`end_to_end.rs:15`) on `smith` at a pseudo-terminal; once, under 2 s.
Don't: no compatibility path for JSON settings.
Size: at most 200.
Removes: what Scope names, and the `#[expect]` at `local_shell.rs:239`.

### 10.14 The effective limits in `smith check`
Scope (shell.md, 4.4; limits.md, 9): `smith check` and its JSON print
the effective limits under limits.md's names and units, as
`session.started` does; the harness reads an exec or chat arm's teardown
bound from them (benchmarks.md, 10).
Needs: smith 04's effective limits by name [04.7].
Files: `smith-local-shell/src/check.rs`;
`benchmarks/src/agent/smith/{prepare.rs,settings.rs:209}`;
`benchmarks/tests/smith_adapter.rs`; `tests/end-to-end/end_to_end.rs`.
Shape: `Report.limits: Vec<smith_agent_service::profile::Limit>`.
Test shape: end to end `smith_check_json_lists_the_effective_limits_by_name`
(`tool_payload` of 40,960 bytes, shell.md 6.3); offline
`an_exec_arms_teardown_bound_comes_from_check`. Ceilings: 2 s, 0.5 s.
Proof: process, the end-to-end test; once, under 2 s.
Don't: no list of names beside smith 04's.
Size: at most 150.
Removes: `settings::teardown` for event faces.

### 10.10 Raw terminal input
Scope (shell.md, 4.1; skein shell.md, 6.3): with a terminal and without
`--plain`, raw input at startup with the interrupt key kept as a signal;
the modes restored once `drive` returns, on every path, before the last
line; `Resized` heard and `Window` read for the status line's width.
Needs: skein's terminal modes (skein 02.6): `raw_input`,
`Kept::restore`, `open_signals(Watch::TerminationAndWindow)`, io's
`Resized` and `Window`. Repin skein in its own commit first.
Files: `smith-local-shell` (`compose.rs`, `front/interactive.rs`,
`local_host.rs`), `smith-local-service`, `src/main.rs`, `Cargo.lock`,
`tests/end-to-end`.
Shape: `front::interactive::Screen::Terminal { columns: u16 }`.
Test shape: end to end at a pseudo-terminal, with skein's
`resize_terminal` and `terminal_modes`:
`the_terminal_modes_are_restored_after_smith_exits`,
`the_terminal_modes_are_restored_after_an_interrupt`,
`a_window_size_change_reaches_the_status_line`; 2 s each.
Proof: process, `the_terminal_modes_are_restored_after_an_interrupt`;
once, under 2 s.
Don't: no terminal syscall in smith.
Size: at most 200.
Removes: nothing.

## Interface sketches

The store is a step machine the local service owns: domain requests in,
io requests out, one terminal each. Its one decision, what each record
expects of its file: the state and a delivery record `Expect::Absent`
until the store has read or written it, then `Expect::Digest` of that
version; a turn `Expect::Absent` always, so a second writer conflicts;
never `Expect::Any`. A store answered "committed, durability unknown" is
sent once more with `Expect::Digest` of its own bytes (io.md, 5.2); a
second failure is `StoreFailed` with `NotDurable`.

```rust
// crates/smith-local-protocol/src/store.rs
pub const CHAT_STORE_VERSION: u64 = 1;
pub struct StoreLimits { pub file_bytes: u32, pub turns: u32, pub deliveries: u32, pub scan_bytes: u64 }
/// One local-domain store request; its one terminal is `Action::Domain`.
pub enum StoreRequest {
    Load,
    SaveState { state: ChatState, history: History },
    SaveTurn { number: u32, read: Option<Token>, turn: smith_domain::Turn },
    SaveDelivery { record: Box<DeliveryRecord> },
}
/// `Fresh` replaces the earlier turns in the same request.
pub enum History { Kept, Fresh }
pub enum RecordName { State, Turn(u32), Delivery(smith_domain::run::CallName) }
/// Why a request failed, for the operator's line; the domain hears `StoreFailure`.
pub enum StoreFault { Io { file: RecordName, error: kernel::Error }, Conflict { file: RecordName },
                      TooLarge { file: RecordName }, Malformed { file: RecordName }, NotDurable { file: RecordName } }
/// To io, to the domain, the fault behind a `StoreFailed`, and the close's one terminal.
pub enum Action { Io { request: file::Request, deadline: Time }, Domain(local::Event), Fault(StoreFault), Closed }
pub struct Store { /* each file's last digest, requests in flight */ }
impl Store {
    pub fn new(owner: Token, chats: Token, chat: Box<[u8]>, limits: StoreLimits, endpoints: Endpoints) -> Option<Store>;
    pub fn request(&mut self, request: StoreRequest, now: Time, out: &mut Queue<Action>);
    pub fn from_io(&mut self, event: file::Event, now: Time, out: &mut Queue<Action>);
    pub fn close(&mut self, out: &mut Queue<Action>);
}

// crates/smith-local-domain/src/{config.rs,boundary.rs}
pub enum Mode { Chat { waiting: Duration }, Headless { prompt: Box<[u8]> } }
/// How the invocation ended, by its answer's class; the shell numbers it.
pub enum ExitStatus { Accepted, RunFailed, Budget, Credentials { account: u32 }, Refused, NeedsInput, Interrupted }

// crates/smith-local-shell/src/{command.rs,settings/layers.rs,check.rs,front/headless.rs}
/// One invocation, parsed before anything is read; `Legacy` until 10.16.
pub enum Command { Check { shared: Shared, json: bool }, Exec(Exec), Login { shared: Shared, account: Box<str> },
                   Chat { shared: Shared, plain: bool }, Legacy(Vec<OsString>) }
/// Where a value came from, as `check` shows it and a refusal names it.
pub enum Origin { BuiltIn, File { layer: Layer, path: PathBuf, line: u32 }, Override { index: u32 }, Flag { flag: Box<str> } }
/// An account as `check` and `login` report it, never with a value.
pub enum Credential { Usable { until: Option<Wall> }, NotSignedIn, Expired { at: Wall, file: PathBuf, program: Box<str> },
                      Unreadable { file: PathBuf }, Unset { variable: Box<str> } }
/// The product's exit codes, 0, 1, 2, 3, 4, 5 and 130 (shell.md, 4.3).
pub enum ExitCode { Accepted, RunFailed, Usage, Credentials, Budget, NeedsInput, Interrupted }
impl ExitCode { pub const fn code(self) -> u8; pub const fn from_status(status: ExitStatus) -> ExitCode; }

// crates/smith-local-protocol/src/borrowed.rs
pub enum LoginFormat { Codex }
/// `record.refresh_token` is `None`: the driver lends it, never refreshes or keeps it.
pub struct Borrowed { pub record: skein_oauth::SavedToken, pub account_id: Box<[u8]> }
pub enum LoginRefusal { TooLarge, Malformed, NoAccessToken, NoAccountId, NoExpiry }
pub fn decode_login(format: LoginFormat, bytes: &[u8], limits: &skein_oauth::Limits) -> Result<Borrowed, LoginRefusal>;
```

`ExitStatus` from `run.completed`: `accepted` is `Accepted`; `parked`,
`NeedsInput`; `failed` of class `budget`, `Budget`; `cancelled`,
`Interrupted`; `model` with `unauthorized` and no grant to replace it,
`Credentials`; any other, `RunFailed`; `refused` as `invalid`,
`Refused`; as `busy`, `RunFailed`; no answer because the chat store
failed, `RunFailed` (shell.md, 4.3).

## Questions to raise when reached

- **Public clients** (shell.md, 11; 10.6): until a provider sanctions
  smith's use of its public client, propose that `sign_in` takes a
  registration from settings.
- ~~**Repository trust** (11; 10.3)~~: settled by the designer on
  2026-10-12 (shell.md, 5.1, "The line" and "No approval"): the
  authority keys are refused, and nothing else; no approval step.
- **Tiered list prices** (11; 10.13): propose Haiku 5.5's higher rate
  whenever its window passes 100,000 tokens.

## Done when

- The commands' end-to-end tests: `heavy cargo nextest run -p smith
  --test end_to_end -E
  'test(/smith_check|smith_exec|smith_login|smith_on_a_terminal|exits_|terminal_modes/)'`
  passes, with the twelve tests of those names that 10.3 to 10.14 add
  among them (on `main`, 3 tests match, of `smith local` and
  `smith-agent`).
- No module of the old local shell is left: `rg -n
  '\blocal_(auth|tokens|store|settings|shell)\b|\bin_process\b|rustls_native_certs'
  crates src tests --type rust` prints nothing (76 lines on `main`).
- The shells need no exception to their bans: `rg -n
  'clippy::disallowed_' crates/smith-local-shell/src
  crates/smith-agent-shell/src src` prints nothing (6 lines on `main`).
- 10.15's commit records `failures` and `first-request/headless`
  passing, and 10.8's interactive `first-request`: `git log origin/main
  --format=%B -E --grep='^Step: smith-10\.(8|15)$' | rg -c '^Proof: live'`
  prints 2.
