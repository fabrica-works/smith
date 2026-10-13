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

- **The harness follows the binary** (README, rule 4): 10.15 and 10.24
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
`window`, `output` and prices; a preset model whose list price rises
past a prompt size priced at the highest rates a prompt within its
resolved window can reach, after settings resolve the window, and prices
a person sets used as set (shell.md, 5.2, settled 2026-10-12); `resolve`
builds the agent configuration
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
Split (decisions.md, 13.66): 10.13 lands the presets and the native
agent value and typed refusals they build, with the golden and the Haiku
test; 10.17 takes `resolve`'s full run, accounts and limits, the
relationship report in `smith check`, trust arrays and custom presets'
inheritance, with the end-to-end test and the process Proof below.
Test shape: unit in `settings/tests.rs`: the built-in layer against a
byte-stable golden, one refusal per relationship the presets check,
`haiku_takes_its_long_prompt_rates_only_past_a_100000_token_window`
(the preset's 200,000 and a declared 100,000). Ceilings: 20 ms.
Proof: unit, the golden and the Haiku test; once.
Don't: the profile's values stay the service crates' `standard()`.
Size: README rule 9's. Removes: nothing.

### 10.17 The run resolved from settings, and `smith check`'s relationships
Split from 10.13 (decisions.md, 13.66). Scope: 10.13's Scope from
"`resolve` builds the agent configuration" to its end: the agent's
`Profile` and the local `Config` as values, accounts and limits
assembled, each relationship checked by name, `smith check` naming a
relationship with both sides and their layers, trust arrays and custom
presets' inheritance. Its candidate's resolver parts start from
10.13's (`6e64b9d9`). The run's models and accounts, as the designer
ruled on 2026-10-12 (shell.md, 5.2, "The run's models and accounts"):
the local `Config.models` holds main's model and, with `sub_agents` on,
every catalog or declared model whose endpoint's account names a
source; `Config.accounts` holds those models' accounts. Main's account
without a source is refused naming `accounts.<name>.source` (exit 2); a
needed account's unusable credential exits 3; an account not needed is
reported with warnings only. The agent's configuration, the derivation
and every relationship use the run's models; a model outside the run is
checked as values only (priced above zero, output below window), and
`smith check` names it and why it is out. Only the run's endpoints are
resolved.
Needs: 10.13.
Files: 10.13's.
Shape: `resolve(settings: &Settings, command: &Command) ->
Result<Resolved, Refusal>`, `Resolved { agent, local, accounts }`.
Split again (decisions.md, 13.73): 10.17 lands the native trust arrays,
custom presets' inheritance and the selection of the run's models and
accounts (13.69's rule), with their unit tests (among them 13.69's
three) and a unit Proof; 10.18 takes the rest: `resolve`'s entry, the
native agent's and the local `Config`'s composition, limits, the other
catalog models' checks, origin-aware relationship rendering and `smith
check`'s report, with the end-to-end test and the process Proof below.
Test shape (10.18's): end to end
`smith_check_names_both_sides_of_a_budget_past_the_profile_ceiling`,
expecting 5.3's line `smith: budget.turns = 5000 (from -c) exceeds the
standard profile's ceiling of 4096 turns`; unit refusals for the
relationships `resolve` checks; unit
`a_codex_only_first_run_needs_no_anthropic_credential`,
`sub_agents_on_adds_the_models_of_every_account_with_a_source`,
`mains_account_without_a_source_is_refused_naming_the_key`. Ceilings: 20 ms, 2 s.
Proof: 10.17, unit: 13.69's three tests and the trust-array tests, once; 10.18, process, that test; once, under 2 s.
Don't: the profile's values stay the service crates' `standard()`.
Size: README rule 9's. Removes: nothing.

### 10.18 The run composed from settings, and `smith check`'s report
Split from 10.17 (decisions.md, 13.73). Scope: what 10.17's section names
as 10.18's. Its candidate's parts start from 10.17's (`4697786a`).
Needs: 10.17.
Files: 10.13's.
Shape, Test shape and Proof: 10.17's section, as 10.18's.
Size: README rule 9's. Removes: nothing.

### 10.7 `borrow` and `env`
Scope (shell.md, 7; hosts.md, 5.4): `smith-local-protocol` decodes
Codex's `auth.json` with skein-json's collector: the access token and the
account id, expiry from the token's claims; the local service owns
`skein-oauth-accounts`' `Component` with handed-in accounts: `borrow`
read at startup and again through io's `Load` when a grant is due, `env`
once; the driver's `Expiring` and expiry become shell.md 7's notice; no
run starts on an expired login; `smith check` reports these accounts,
exit 3 when only the credentials of accounts the run needs fail, and a
warning for an account it does not need (shell.md, 5.2, settled
2026-10-12). Until 10.6, `smith local`'s
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
Split (decisions.md, 13.87): 10.7 lands the protocol decoder and the
driver that owns the borrowed file, with their unit tests and a unit
Proof; 10.19 takes the shell's credential inspection, the product's
composition and `smith check`, with the world test above as its Proof.
Size: README rule 9's, each.
Removes: nothing.


### 10.19 Credentials composed into the product and `smith check`
Split from 10.7 (decisions.md, 13.87). Scope: 10.7's Scope for the
shell's credential inspection, the product's composition and `smith
check`'s report; its candidate's parts start from 10.7's (`b8b04eeb`).
Needs: 10.7.
Files, Test shape, Proof and Don't: 10.7's, as named for 10.19.
Size: README rule 9's.
### 10.9 The build environment
Scope (shell.md, 8): `[environment]` `inherit` (the core list by default,
names only) and `set`; `[build]` preset `rust` (`CARGO_HOME`,
`RUSTUP_HOME`; `$CARGO_HOME/registry` and `$CARGO_HOME/git` as caches)
and `caches`; `smith check` checks each cache with skein's
`directory_access` (skein's `shell.md`, 6.1; shell.md, 8, settled
2026-10-12), a cache it cannot confirm a warning naming it and the
kernel's answer, and warns of a configured
wrapper whose cache is not among them, naming what configured it, and of
a credential-like name. The result is the agent configuration's
environment, smith 09's single owner.
Needs: skein's `directory_access` landed and pinned (a skein step its
supervisor names); the rest of 10.9 does not wait for it.
Files: `smith-local-shell` (`settings/`, `presets.rs`, `check.rs`),
`tests/end-to-end/end_to_end.rs`.
Shape: `check::Warning::{WrapperCache, CredentialLikeName}`.
Test shape: unit in `settings/tests.rs`:
`inherit_takes_named_variables_never_a_pattern`,
`the_rust_preset_adds_its_homes_and_caches`,
`a_credential_like_name_is_warned`,
`an_unwritable_cache_is_warned_with_the_kernels_answer`; end to end
`smith_check_json_warns_of_a_wrapper_cache_outside_the_caches`.
Ceilings: 20 ms, 2 s.
Proof: process, the end-to-end test: exit 0, the warning naming
`RUSTC_WRAPPER`; once, under 2 s.
Don't: `smith check` creates no cache; read no mode bit or owner in
place of `directory_access`.
Size: at most 300. Removes: nothing.

### 10.s2 The inline product counts each part once
Added during the run (decisions.md, 13.104), from the designer's answer
of 2026-10-13T00:49:24Z (smith 1c14b678, limits.md section 5). Scope:
`Effects` gets its own constructor and worst case, never a whole agent
service built and stripped (`Effects::new`, `effects_worst_case`); the
local service and the shell price one io, and no host channel in inline
mode; smith repins skein at skein-07.s7's commit or later; the product's
memory check at default settings reports its worst case by part (the
local domain, the effects' parts, io, the store, the terminal, delivery
and the front) and fits shell.md 6.3's 2 GiB.
Needs: skein-07.s7.
Files: `crates/smith-agent-service/src/service.rs` (`Effects`), the local
service and shell's memory accounting, `Cargo.toml`, `Cargo.lock`; their
tests.
Shape: no new type beyond `Effects`' own constructor.
Test shape: the product's memory check at default settings fits 2 GiB,
with the breakdown by part asserted against the measured parts; it
fails on the parent. Under 0.5 s.
Proof: that test, once.
Don't: lower `memory`; if the default still passes 2 GiB, stop and report
by part (the designer then lowers a provisional value, `conversations`
first).
Size: README rule 9's.

### 10.s3 `-o`'s file in the teardown bound
Added during the run (decisions.md, 13.110). Scope: shell.md 4.4 has
`smith check ... exec -o FILE` report the teardown bound `smith exec`
with those flags would have, `-o`'s file among its destinations; the
product's `service.teardown` today is the agent's alone (two io closes
and the events write deadline), while `-o`'s result file has its own
FileIo write and close deadlines before `session.ended` (10.20). The
local effective limits gain the result file's deadlines in the product's
teardown bound when `-o` is given, through the one limits owner, and
`smith check` reports it.
Needs: 10.15 (the invocation-aware check).
Files: the local effective limits' owner and the result file's
preparation; their tests.
Shape: no new type.
Test shape: a unit test: `smith check ... exec -o FILE` reports a
teardown bound that includes the file's deadlines, and without `-o` the
bound is unchanged; it fails on the parent. Under 20 ms.
Proof: that test, once.
Don't: change the agent's teardown bound or a deadline's value.
Size: README rule 9's.

### 10.5 `smith exec`
Scope (shell.md, 4.2 and 4.3; events.md, 5.2): the command and its usage
errors; the headless local domain composed inline (`compose`); chats by
workspace, a new one named in `session.started` and on standard error,
`--chat`, `--ephemeral`, `--deliver`; the result on standard output and
`-o FILE`; standard error's status from what the local host hears of
the run (its told turns, notices and answer), never from the records,
provider text clipped (shell.md, 4.2, settled 2026-10-12); the last line from smith 06's words; the code from
`ExitStatus`; a forced exit's warning; `--json`, `--trace` and `[trace]`
sinks, standard output under `--json` adopted by what it is (a regular
file by `adopt_append`, a pipe or a terminal by `adopt_write_pipe` under
the same write deadline) through the agent service's one sink owner; `session.started` with mode `exec` and `versions.chat_store`.
Split (decisions.md, 13.101): 10.5 lands the command, its parser and versions, the inline startup, typed headless rendering, the one bounded stdin phase and standard output adopted by its kind, with its process Proof; 10.20 takes `-o`'s durable output and `--ephemeral`'s private files and their cleanup, with their memory and close accounting and tests.
Needs: smith 03's product stream [03.7]; smith 06's words function,
`smith_events::words` [06.2]; for `--json` on a pipe only, skein's write
pipe with a stated write deadline (skein's `io.md`, 5.1), a skein step
its supervisor names, pinned; the inline product's memory counted once
(limits.md, section 5, "One process counts each part once") and skein's
replay values charged at their total (skein's `llm.md`, 4.2), steps its
supervisor names, as the designer ruled on 2026-10-13, so that the
product's default worst case fits shell.md's 2 GiB.
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
function beside smith 06's; no reading of the product's own event
records, and no in-process pipe or second producer for them.
Size: at most 600.
Removes: nothing.


### 10.20 `smith exec`'s `-o` output and `--ephemeral` files
Split from 10.5 (decisions.md, 13.101). Scope: 10.5's Scope for `-o`
(the result written durably to a file) and `--ephemeral` (a private chat
directory, removed at exit), with their memory and close accounting;
its candidate's parts start from 10.5's (`b92c18a0` pin). `--ephemeral`'s
directory is removed as the designer ruled on 2026-10-13 (shell.md, 4.2,
"Removed by the names it wrote"): the store's known names through
FileIo's `Remove`, then the directory with `directory: true`, never a
listing or a traversal, and never `std::fs`; a directory left behind is
a warning before the last line. `-o`'s file, as the designer ruled on
2026-10-13 (shell.md, 4.3, "The result's file is the product's, not the
run's"): written and synced after the answer and before
`session.ended`; after an accepted answer a write or sync failure exits
1 with `smith: run accepted, but its result could not be written to
<file>: <the kernel's error>`, `run.completed` unchanged and
`session.ended`'s `exit` 1; after any other answer, a warning before
the answer's last line.
Split (decisions.md, 13.107): 10.20 lands `-o`'s durable output with its failure policy (shell.md 4.3, the designer's faaba005) and its tests; 10.22 takes `--ephemeral`'s private files and their known-name cleanup (the designer's 536b3a2d) with their tests.
Needs: 10.5; skein's `Remove` with `directory` (skein's `io.md`, 5), a
skein step its supervisor names, pinned.
Files: 10.5's, as they concern these two.
Test shape: unit or process tests of each: `-o`'s file holds the result
and nothing else; `--ephemeral` leaves no file behind, on success and on
failure; and a file the store did not write, placed in the directory,
keeps the directory with a warning naming it; an `-o` file that cannot
be written after an accepted answer exits 1 with that last line, and
`session.ended`'s `exit` is 1. Under 0.5 s.
Proof: process, those tests, once.
Size: README rule 9's.


### 10.22 `smith exec --ephemeral`'s private files and their cleanup
Split from 10.20 (decisions.md, 13.107). Scope: 10.20's Scope for
`--ephemeral`: the private chat directory, the store keeping its written
names in ephemeral mode, their removal through io by those names, then
the directory, empty (skein-07.s8's `Remove` with `directory`), and a
directory left behind as a warning before the last line; never a listing
or a traversal. Its candidate's parts start from 10.20's (`c61212fa`).
Needs: 10.20.
Files: 10.20's, as they concern `--ephemeral`; the store's
`machine.rs` and `boundary.rs`.
Test shape: `--ephemeral` leaves no file behind on success and on
failure; a file the store did not write is kept, with its directory and
the warning. Under 0.5 s.
Proof: process, those tests, once.
Size: README rule 9's.
### 10.21 `smith exec`'s last text and named failure forms
Split from 10.5 (decisions.md, 13.103). Scope: 10.5's rendering left at
its 950-line stop: the last assistant text kept across later tool-only
turns for the result, and smith 06's words and notices carrying shell.md
4.3's named forms (the account and endpoint, the budget used against its
limit), through the existing words owner. As the designer ruled on
2026-10-13 (run.md, section 10; channel.md, section 4; events.md, 3.10;
shell.md, 4.3): the answer's failure carries the values its forms need,
and no notice is paired with it:
- an exhausted account: the account's number and, when its provider
  said, how long until it may be used again, measured when the run
  answers (`run::Fault::Exhausted`, the channel's `ModelFault`, the
  host's and the inline agent's, and the trace's `retry_after_ms`);
- a budget failure: what ran out, with `used` and `bound` in its unit,
  time being the run's wall time as its budget counted it, never the
  product's elapsed;
- `words` stays one function, of the answer failure and the two facts
  only the host has: each account's name and its clock when the answer
  came.

Those crossings are a step its supervisor names, under channel 2 and
events 1 (the README's rule 5), with the codecs and goldens regenerated.
Needs: 10.5; that step.
Files: 10.5's rendering files; smith 06's words owner, for those forms.
Test shape: unit tests with literal expected lines for each form, and
one for a result whose last text precedes tool-only turns. Under 20 ms.
Proof: unit, those tests, once.
Size: README rule 9's.
### 10.15 The harness on `smith exec`
Scope (benchmarks.md, 6 and 10): for commits from this one, the smith
adapter writes its pinned settings as the attempt's user layer, state in
its `XDG_STATE_HOME`, and an account borrowing the user's
`~/.codex/auth.json`; runs `smith check --json exec` with the attempt's
own `exec` flags, its prompt aside, before an arm's first attempt, a
nonzero exit refusing the arm as setup; runs `smith exec
--json -C <workspace> -m <endpoint>/<model> --effort <effort> -` with no
trace file, so the run has one event destination, its standard output
(benchmarks.md, section 6; shell.md, 4.4, as the designer ruled on
2026-10-13);
classifies by the exit code held against `run.completed`. The live
suite runs through `smith exec --json`, borrowing the file its caller
names (testing.md, 2.3). It adds the variant `probes/first-request/headless`,
the task in the start, to the task and `suites/probes.toml` (decisions.md, 12.21).
Gains (decisions.md, 13.89): the harness reads an exec or chat arm's teardown bound from `smith check`, through 10.14's decoder, with `an_exec_arms_teardown_bound_comes_from_check`.
Needs: session 00's one command per probe run [00.27], for its proof;
`smith check` taking the invocation it checks (shell.md, 4.4), a step its
supervisor names; the standard profile's `conversations` at 5 (shell.md,
6.3, as the designer ruled on 2026-10-13) in `standard()`
(`crates/smith-agent-service/src/profile/quantities.rs:155`), so that the
default product's worst case fits 2 GiB.
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
takes `sign_in` accounts with the registration their settings give,
never a preset's (shell.md, 7, as the designer ruled on 2026-10-13:
`client_id`, `authorization_url`, `token_url`, `redirect_uri`, `scope`,
`token_format` and `trust`, a missing key refused naming it);
`Keeper::Private`
under `$XDG_STATE_HOME/smith/tokens`; the account id from the claims.
`smith login ACCOUNT` exits 0, 3, or 2 for an unknown account; `smith
local`'s accounts move onto the driver; at the end a waiting sign-in is
cancelled before the driver closes.
Split (decisions.md, 13.112): 10.6 lands the native account driver's private-file and sign-in routing with its tests, removing nothing; 10.23 takes the registration from settings (the designer's 4d808f08), `smith login`, the legacy accounts' migration and the old mechanism's removal, and both live-arm moves to `exec` (13.108).
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


### 10.23 Sign-in's registration from settings, `smith login`, the old accounts gone
Split from 10.6 (decisions.md, 13.112). Scope: 10.6's Scope for the
registration under `[accounts.<name>]` (shell.md 7, the designer's
4d808f08: `client_id`, `authorization_url`, `token_url`, `redirect_uri`,
`scope`, `token_format`, `trust`), missing keys refused by name at
`smith check` and startup, `smith login` and its words, the legacy
accounts' consumers migrated and the old mechanism removed whole, the
issuer-refresh story and the ordinary live stories' Anthropic arms moved
to `exec` (13.108); the 168-line registration module of 10.6's candidate
(`95b4f6d1`) starts it.
Needs: 10.6.
Files: 10.6's.
Test shape: 10.6's named authentication stories and process tests, as
they concern these.
Proof: 10.6's process Proof.
Size: README rule 9's; past 700, it stops and asks.
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
address. As the designer ruled on 2026-10-13 (shell.md, 4.1 and 4.2;
events.md, 5.2; hosts.md, 5.3):
- the live view comes from the one event sink: the front end asks it
  for the records it shows, typed and in process, through a bounded
  queue counted in the startup footprint, whatever the destinations'
  capture, best effort (dropped and counted when behind, never holding
  the run back), never read back from a destination; the answer and
  its failure in words come from what the local host hears;
- the chat store's root is the workspace's chats; it holds one chat at
  a time and selects another only when idle, after the live activation
  has answered or parked, forgetting the old chat's remembered state;
  `/new` makes the new chat's directory, private; `/resume` of an
  absent chat, or of a name that is not one path component within the
  store's bounds, is refused naming it; an `--ephemeral` store selects
  no other. The harness runs `probes/first-request` at `smith --trace <file>`
on a pseudo-terminal; its `headless` variant stays on `smith exec`.
Split (decisions.md, 13.113): 10.8 lands the sink's typed, bounded, best-effort live view (`EventLimits.front`) and the store's selection of one chat at idle (`StoreRequest::SelectChat`), with their routing through the domain, the services and the local host, and their unit tests; 10.24 takes the rest, from the preserved drafts of `6e6790d1`.
Needs: smith 05's typed notices [05.11], and lines carried, held until
read, unread by outcome [05.13]; for its Proof, smith 00's one command
per probe run [00.27].
Files: `smith-local-shell` (`front/interactive.rs`, `command.rs`,
`compose.rs`, `local_host.rs`); `smith-local-protocol/src/terminal.rs`;
`smith-local-domain` (`/` commands at an activation's boundary);
`smith-agent-service` (`src/events.rs`, `src/sinks.rs`, the front end's
queue); `smith-local-protocol` (`src/store/boundary.rs`,
`src/store/machine.rs`, the chat's selection);
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
Proof (10.24's, decisions.md 13.113): live, `heavy cargo run -p smith-bench -- build HEAD`, then `heavy
--gate cargo run -p smith-bench -- probe probes/first-request --arm <the
printed directory>`, one attempt typed at `smith --trace` on a
pseudo-terminal: it passes. No before: the parent passes it through `smith local`. At most
$0.20 and 5 minutes; once. `follow-up` (smith 00.14) is not its proof.
Don't: no raw input (10.10); no `/` command reaches the run.
Size: at most 600.
Removes: nothing.
10.8's own Test shape and Proof (decisions.md, 13.113): its units of the
sink's view and the store's selection; Proof: unit,
`the_live_view_drops_without_holding_a_none_capture_destination` and
`selecting_a_new_chat_makes_a_private_directory_and_keeps_its_record_layout`;
once each, under 20 ms. Size: README rule 9's; past 700, it stops and asks.

### 10.24 The interactive `smith`'s screen, commands and live Proof
Split from 10.8 (decisions.md, 13.113). Scope: 10.8's Scope beyond the
sink's live view and the store's selection: interactive startup in line
mode with the workspace's chats adopted as the store's root, `front` at
the header's and the footprint's derivation, line mode and `--plain`
detected, the screen per record kind with notices and the budget joined,
the status line, the `/` lines parsed and executed at the domain's
activation boundary (`/model`, `/effort`, `/new`, `/resume NAME`,
`/quit`), interrupts and end of input, a `sign_in` account without a
record showing its address through the existing driver, and the
harness's Chat face running `probes/first-request` at `smith --trace
<file>` on a pseudo-terminal. It starts from 10.8's preserved drafts:
the screen and the entered parser of `6e6790d1` (on `2834aedf`).
`/model` and `/effort`, as the designer ruled on 2026-10-13 (shell.md,
4.1 and 5.2; charter.md, section 2; agent.md, section 4; session.md,
sections 3 and 8.2; transcript.md, section 2):
- the agent's configuration holds every model main's endpoint declares,
  checked and priced at startup with main's account; `/model` chooses
  among them for the next activation's charter, and a model on another
  endpoint is refused, the chat kept, naming `smith --chat NAME -m
  ENDPOINT/NAME`; nothing is composed again in the process;
- effort is a run's choice, the charter LLM's `effort`, no longer an
  endpoint option; `smith-protocol-llm` sets each prompt's effort from
  it, and skein's `Prompt` already carries one;
- a turn records its model and effort (transcript 3 in place); a
  session opened from a window whose last turn's model is not its
  charter's compacts first with the window's model.

Those crossings (the charter, the agent configuration, the transcript,
the session's trigger, `smith-protocol-llm`) are a step its supervisor
names, which 10.24's `/model` and `/effort` need.
Split (decisions.md, 13.120): 10.24 lands all but `/model` and
`/effort`'s execution: it parses them as local commands and refuses them
in words, naming them, as not yet available; 10.25 executes them.
Needs: 10.8; 10.s4 (decisions.md, 13.121); for `/model` and `/effort`, that step: 08.s1 (decisions.md, 13.114).
Files: 10.8's, and `benchmarks/agents/smith/faces.toml` for one appended
Chat entry (decisions.md, 13.118).
Shape: 10.8's.
Test shape: 10.8's units of each `/` line and of the renderer per record
kind, its four world stories and its three end-to-end tests. The four world
stories run on the scripted `Terminal` with the test world's interactive
launch flag; the end-to-end tests keep `Terminal::attached`
(decisions.md, 13.119).
Proof: 10.8's live Proof.
Don't: 10.8's.
Size: README rule 9's; past 700, it stops and asks.
Removes: nothing.

### 10.25 `/model` and `/effort`
Split from 10.24 (decisions.md, 13.120). Scope: 10.24's `/model` and
`/effort`, as the designer ruled on 2026-10-13 (shell.md, 4.1 and 5.2):
`/model ENDPOINT/NAME` sets the next activation's charter model to one of
main's endpoint's models, which 08.s1 composes at startup; another
endpoint's model is refused in words, the chat kept, naming `smith --chat
NAME -m ENDPOINT/NAME`; `/effort LEVEL` sets the next activation's
charter effort. Efforts are the model's, as the designer ruled on
2026-10-13 (shell.md, 4.1, 5.2 and 6.1; charter.md, section 2;
agent.md, section 4): each model declares its `efforts`, optional,
beside its other declared values (presets from their provider's
documentation, with the source; settings' `[models."<e>/<n>"].efforts`);
`smith check`, `--effort`, `/effort` and the run's entrance refuse an
effort outside the main model's list, naming it and the list; a model
that declares none takes any effort within 32 bytes, its provider's
refusal being the request's failure; skein holds no list;
both take effect only once the live activation has answered or parked;
`sub_agents`' models and the needed accounts are unchanged; 10.24's
interim refusal of both goes. It starts from 10.24's preserved WIP (the
startup inventory's model resolution and the next charter's private
model and effort state).
Needs: 10.24; 08.s1.
Files: 10.24's.
Shape: the local host's next-start charter fields and 08.s1's `Effort`;
a model's declared `efforts` in the settings schema, the presets, the
agent configuration's models (`smith-agent-shell`) and the run's
entrance check (`smith-domain-run`), Files beyond 10.24's, which the
supervisor sizes.
`Slash::Model.selected: Option<usize>` (decisions.md, 13.123): the named
model's position among main's endpoint's models, `None` when it is not
one. Files beyond 10.24's: `smith-local-shell` (`settings/`, `presets.rs`,
`check.rs`), `smith-agent-shell` (`config.rs`), `smith-domain-run` (the
entrance check). No preset declares efforts in 10.25 (13.123).
Test shape: unit: each `/model` and `/effort` line's choice or refusal,
its words. World, on the scripted `Terminal` (13.119):
`slash_model_applies_from_the_next_activation`,
`slash_model_on_another_endpoint_is_refused_and_the_chat_kept`,
`slash_effort_outside_the_models_efforts_is_refused_naming_them`. End to
end: `slash_effort_reaches_the_next_request`. Ceilings: 20 ms, 0.5 s, 2 s.
Proof: process, `slash_effort_reaches_the_next_request`; once, under 2 s.
Don't: nothing composed again within the process; no endpoint but
main's; no live run (10.24's live Proof covers the product).
Size: README rule 9's; past 700, it stops and asks.
Removes: 10.24's interim refusal of `/model` and `/effort`.

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
follows 10.24 and 10.15.
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
Moved (decisions.md, 13.89): the harness reading an exec or chat arm's teardown bound, and `an_exec_arms_teardown_bound_comes_from_check`, go to 10.15; 10.14 lands `smith check`'s report and a decoder of its bound in `smith-bench`, unit-tested, used by no arm yet.
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

### 10.s1 skein repinned at its `main`
Split from 10.10 (decisions.md, 13.75). Scope: 10.10's first commit,
landed on its own: smith's skein pin moves from 26502c96 to skein
`origin/main`, which holds 02.6's terminal modes, 07.s2 (skein-world
skips `EINVAL`) and 07.s3 (a summary's parts are not output items);
smith's code takes what the new pin changes under README rule 8, and
nothing of 10.10's interactive work.
Needs: nothing.
Files: `Cargo.toml`, `Cargo.lock`, and what the repin breaks (README rule 8).
Shape: no new type.
Test shape: none new.
Proof: the gate and the smoke, once; then 13.6's `slow-completion` once,
as 13.70 says.
Don't: implement 10.10's raw terminal input here.
Size: README rule 9's.

### 10.s4 The live view holds only what the screen shows
Added (decisions.md, 13.121), before 10.24 lands. Scope (shell.md, 4.1;
events.md, 5.2): 10.8's live view queues, for each record the screen
shows (`text.delta`, a tool's start and terminal with its verdict and
duration, notices and failures, a response's model, usage and spend, the
budget's standing), a typed projection of what it shows, bounded on its
own; a request's prompt and any record the screen does not show never
enter the queue; the startup footprint prices the view as its slots
times the projection's bound, not a record's encoded line. Destinations,
their capture and `smith exec` are unchanged. It starts from 10.24's
uncompiled one-sink projection work, which leaves 10.24's diff.
Needs: 10.8.
Files: `smith-agent-service` (`src/sinks.rs`, `src/events.rs`, the
profile's footprint owner and its tests); `smith-events` (the projection
type, if the screen's records need one); `smith-local-shell`
(`local_host.rs`, `local_host/memory.rs`, `check.rs`, where the view's
part is priced); their tests.
Shape: `sinks::View` (the projection) and its bound beside
`FRONT_RECORDS`; the front end pops `View`s, not `Record`s.
Test shape: unit: a request's record never enters the view; each shown
record's projection stays within its bound; the footprint's view part is
`FRONT_RECORDS` times that bound. Process:
`the_interactive_default_fits_its_memory` (decisions.md, 13.122): for
each preset's default interactive composition, every check passes and
the complete worst case, from the existing `smith_local_service::worst_case`
owner, fits the default `memory` with at least 64 MiB to spare, each
number recorded in the test (limits.md, 3); and `smith check --json` for
the interactive invocation on those defaults, in scratch `HOME` and
`XDG_*`, exits 0. No effective limit is added.
Ceilings: 20 ms, 2 s.
Proof: process, `the_interactive_default_fits_its_memory`; once, under
2 s. The body states each preset's interactive worst case and its
margin under 2 GiB.
Don't: no change to `memory` or any default; no destination change; no
screen (10.24).
Size: README rule 9's; past 700, it stops and asks.
Removes: the view's whole-record slots.

### 10.10 Raw terminal input
Scope (shell.md, 4.1; skein shell.md, 6.3): with a terminal and without
`--plain`, raw input at startup with the interrupt key kept as a signal;
the modes restored once `drive` returns, on every path, before the last
line; `Resized` heard and `Window` read for the status line's width.
Needs: smith 10.24's interactive owner (`front/interactive.rs`, the status line, `--plain`; decisions.md, 13.113) and 10.s1's repin (decisions.md, 13.75); skein's terminal modes (skein 02.6): `raw_input`,
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

- ~~**Public clients** (shell.md, 11; 10.6)~~: settled by the designer on
  2026-10-13 (shell.md, 7): no preset carries a registration; `sign_in`
  takes it from the account's settings.
- ~~**Repository trust** (11; 10.3)~~: settled by the designer on
  2026-10-12 (shell.md, 5.1, "The line" and "No approval"): the
  authority keys are refused, and nothing else; no approval step.
- ~~**Tiered list prices** (11; 10.13)~~: settled by the designer on
  2026-10-12 (shell.md, 5.2, "Tiered list prices"): the preset prices a
  model at the highest rates a prompt within its window can reach.

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
  passing, and 10.24's interactive `first-request`: `git log origin/main
  --format=%B -E --grep='^Step: smith-10\.(15|24)$' | rg -c '^Proof: live'`
  prints 2.
