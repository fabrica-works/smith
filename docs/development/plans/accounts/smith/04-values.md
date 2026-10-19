# smith 04: the grant value union and the dialects

The agent installs a value by its tag, refuses one it cannot install
as `grants`, and counts accounts. The dialects have a fixed name and
number, `openai` 3, `anthropic` 2 and the withdrawn `codex` 1, and a
configuration's `"dialect":0` is refused.

Needs at start: smith 01.3's credential by scheme and its `openai`
provider, and smith 01.2's grant value union on the wire, on
`origin/main`.

## Read first

- smith's design at `137a1cb4` and skein's at `378fd8c`, the sections
  below; skein `docs/foundation/programming-model.md` 2.1, 4.4, 6.2,
  7, 10.4 and `testing-strategy.md` 2.1, 2.2, 8, 10.
- The README's Rules; code-organization.md 6, 7, 8; decisions.md 2,
  7, 10, 13, 16, 22 and design questions 6, 13, 14.

## What the design asks

- channel.md 6, 10: a grant's value is a tagged union, never exposed.
- hosts.md 2, host.md 2, 7: the host fills a value in by its scheme.
- run.md 4: an uninstallable value refuses a start or fails the run.
- llm.md 7, 10: the component sends a credential by its scheme.
- agent.md 4, shell.md 6.2: the dialects' names and numbers.

## Choices

- The withdrawn forms stay, doc-commented "withdrawn": the id and
  bearer grant and the `codex` dialect, for the second part of LLM
  accounts (decision 2). A program's grant decodes and is refused as
  uninstallable (decision 7).
- smith takes skein twice (README, Rules; decision 13): smith 01.3
  repins at its start and `smith-repin` takes the rest. jig's breaks
  are `jig-repin.s1`'s (decision 22); the repository downstream of
  jig's are its own repin's, on decision 16's defaults: a grant goes as
  the withdrawn form, `"dialect":0` takes its dialect's number.
- No new live or process tier (decision 10).
- An account's number is a name: the agent counts distinct accounts
  against `ACCOUNTS` (question 14).

## Rules that bind it

- Transitional code names, in a comment, the increment that removes it.
- No value in a `Debug`, fact, turn, trace or test output; tests
  compare tags, schemes, accounts and lengths.
- Files over 800 lines grow by at most ~100 per increment:
  `smith-agent-service/src/service.rs` (2920), `config.rs` (1658),
  smith-local-service's `process.rs` (1420) and `service.rs` (1340),
  smith-protocol-channel's `component.rs` (1002),
  `tests/channel/tests/stories.rs` (1255).

## Increments

### 04.1 The grant value union
Scope: channel.md 6, 10; hosts.md 2; host.md 2, 7; run.md 4; llm.md 7,
  10; decisions 2, 7; questions 6, 14.
Needs: what the session needs.
Files: `crates/smith-protocol-channel/src/`:
  `{credentials,component,lib}.rs`; `crates/smith-host-protocol/src/`:
  `{translate,component,process,tests}.rs`;
  `crates/smith-agent-service/src/{service,profile}.rs`;
  `crates/smith-protocol-llm/src/{grants,endpoints,component}.rs`;
  `crates/smith-agent-shell/src/config.rs`;
  `crates/smith-local-service/src/{service,process,tests}.rs`,
  `src/service/accounts.rs`; `tests/agent-process/src/{lib,world}.rs`,
  `tests/agent-process/tests/grants.rs` (new) and `content.rs`;
  `tests/protocol-llm/tests/endpoints_grants.rs`;
  `tests/channel/{src/lib.rs,tests/{stories,composed}.rs}`;
  `tests/hosts/src/lib.rs`.
Shape: `smith_protocol_channel::GrantValue` (new) replaces `Secret` and
  the raw bytes beside it in `Credentials`: `Bearer`, `Key`, `Program`
  and `Withdrawn { account_id, bearer }`, its `Debug` redacted.
  `program_name` in `credentials.rs` takes 1 to 32 of `a-z`, `0-9`, `-`
  and refuses the rest as `Error::Grants`; the host's encode and the
  agent's decode both call it. `grant_value` returns
  `Option<&GrantValue>`; host-protocol's `Values.credentials` is
  `Box<[GrantValue]>` and `send_grant` takes one, each encoding its own
  arm. agent-service's `credential()` takes `&GrantValue`: `Bearer`
  gives `Credential::bearer`, `Key` `Credential::key`, `Withdrawn` with
  an empty id a bearer and with an id `Credential::codex(value, id)`;
  `Program` is not installable. `Effects::grant` takes `&GrantValue`.
  The install check reads the dialect of the endpoint configured with
  the grant's account (smith 03.1 reads the charter's LLM instead):
  `codex` takes `Withdrawn` with an id; `openai` and `anthropic` a
  bearer, a key or `Withdrawn` with an empty id; anything else, an
  empty value included, is `GrantError::Scheme` (new). `component.rs`'s
  exhaustive match on `grants.read` gains `Scheme` beside `Bounds`,
  `Stale` and `Full`: an `Invalid` failure.
  `refuse_start(reason)` answers `Invalid::Grants` for an
  uninstallable value and `Workspace` as before otherwise; a later
  grant fails the run as `Failure::Credential`. Question 14: `Grants`
  keeps any `u32` and is `Full` past `accounts` distinct accounts;
  `Endpoints::new` refuses more distinct accounts than its limit as
  `EndpointError::Account` (smith 03.1 removes it). The account bound
  in `config.rs` (message "endpoint account is outside the profile")
  goes in this increment alone, and smith 03.1 does not remove it;
  `ACCOUNTS`' doc says a count. `accounts.rs` stops packing the
  envelope: an empty identifier lends `Bearer`, a sign-in's id
  `Withdrawn`, until `accounts-sources` moves the presets;
  `StartValues.credentials` and the service's `values` hold
  `GrantValue`. The agent-process world sends wire values: `bearer(..)`,
  `key(..)`, `withdrawn(id, ..)` and `program(..)` in `lib.rs` replace
  `grant_bytes`; `scenario_grant` replaces the packing in
  `scenario_credential`, which keeps building the fake LLM's
  credential; `World::granting(value)` sets the Start's value and
  `World::openai()` the endpoint (`configuration_for` takes a dialect,
  not a bool); `World::refreshing(value)` plays a `GrantRefresh` of
  generation 2 once the host has seen `Admitted`, as `acknowledge`
  plays after a turn, with the fake LLM's reply held back one second
  (`fake::configured`'s latency), and settles as `hang_up` does: no
  Answer, exit 1. Downstream: jig builds `Values` and calls
  `send_grant` with bytes (`jig-repin.s1`'s) and so does the
  repository downstream of jig (its own repin's); both send
  `Withdrawn` (decision 16).
Test shape: agent-process world, `grants.rs` (each under 0.5 s: a real
  client and two processes; reference below):
  `an_uninstallable_grant_refuses_the_start_as_grants`, rows empty
  bearer, key on `codex`, withdrawn with an id on `openai`, program;
  `a_later_uninstallable_grant_fails_the_run_as_credential`, row a key
  on `codex`: no Answer, and the last stderr line is `smith: the run
  could not answer: a host credential could not be installed`. Unit,
  `service.rs`'s `mod tests` (under 20 ms):
  `each_grant_value_installs_its_scheme`, rows bearer, key, withdrawn
  with an id, withdrawn without; it asserts the installed Credential's
  scheme and id, and replaces
  `grant_envelope_splits_account_and_bearer_without_leaking_the_header`.
  World, `endpoints_grants.rs` (each under 0.1 s):
  `a_sparse_account_number_is_granted_and_used` (0x9f3a_0001);
  `a_fifth_distinct_account_is_refused` (four granted, the fifth
  `Full`; five accounts at a limit of four are
  `EndpointError::Account`);
  `duplicate_names_and_out_of_range_accounts_fail_at_startup` becomes
  `duplicate_names_fail_at_startup`, its account row moved to the
  fifth-account test. Channel world, `stories.rs` (under 0.1 s):
  `each_grant_form_reaches_the_table_by_its_tag`. Unit,
  protocol-channel (each under 20 ms):
  `a_grant_values_debug_shows_no_bytes`, a row per arm;
  `a_program_name_outside_its_charset_is_refused`, rows empty, 33
  bytes, uppercase, underscore, non-ASCII. Agent-process world,
  `content.rs` (under 0.5 s):
  `no_grant_value_reaches_a_turn_fact_or_trace`, rows bearer, key (with
  `private_grant()`, as `reconstruct` does). Reference, `grants.rs`:

  ```rust
  use smith_agent_process_world::{World, charter, key};
  use smith_channel::{InvalidStart, RunResult, StartRefusal};
  #[test]
  fn an_uninstallable_grant_refuses_the_start_as_grants() {
      let mut world = World::new(31, &charter());
      world.granting(key(b"not-for-codex"));
      assert!(world.settle(), "the refusal was answered");
      assert!(world.observed().iter().all(|frame| frame.kind != 0x0106));
      let answer = world.answer().expect("host saw the refusal");
      let RunResult::Refused(refused) = answer.result() else { panic!("refused") };
      let StartRefusal::Invalid(invalid) = refused.reason() else { panic!("invalid") };
      assert_eq!(invalid.value(), &InvalidStart::Grants);
      assert_eq!(world.exit(), Some(skein_io::kernel::Exit::Code(0)));
      world.assert_agent_clean();
  }
  ```
Proof: world, `an_uninstallable_grant_refuses_the_start_as_grants`: the
  host reads an Answer refused as `Invalid::Grants` and no `Admitted`;
  under 0.5 s; unit, `each_grant_value_installs_its_scheme`: each tag
  installs its scheme and id; under 20 ms (unit, because skein 04's
  worlds prove the header each scheme puts on the wire and smith's pin
  reaches them only at `smith-repin`, decision 13).
Don't: pack id and bearer bytes by hand; install a program grant;
  bound an account's number; keep `Secret` beside the union.
Size: about 260 lines.
Removes: `Secret`; the hand-packed envelope; `config.rs`'s account
  bound; `Grants`' and `Endpoints`' number checks.

### 04.2 The dialects by name and number
Scope: agent.md 4; shell.md 6.2; question 13.
Needs: what the session needs.
Files: `crates/smith-agent-shell/src/config.rs`;
  `crates/smith-local-shell/src/settings/resolve/{models,document}.rs`,
  `src/settings/tests.rs`;
  `tests/local-process/src/process/configuration.rs`;
  `tests/agent-process/tests/stories.rs`,
  `tests/agent-process/fixtures/read-past-payload.json`;
  `tests/end-to-end/end_to_end.rs`.
Shape: `config.rs` fixes each dialect's number, `openai` 3,
  `anthropic` 2 and the withdrawn `codex` 1, and refuses a name with
  another number, naming `dialect`. The configurations in smith with
  `"dialect":0` take their dialect's number: `config.rs`'s own test,
  `configuration.rs`, `stories.rs`, `read-past-payload.json` and
  `end_to_end.rs` (three configurations) each name the `codex`
  provider, so each takes 1. The settings take `openai`: `models.rs`
  gives it `codex`'s idle bound, 300 s, and `document.rs` writes it
  with 3. `codex` stays in settings, withdrawn, so `built-in.golden`
  is unchanged; the presets' move is `accounts-sources`'. Downstream:
  `"dialect":0` is refused from here. jig's
  `testing/jig-test/tests/support/mod.rs` and
  `tests/channel/src/fixtures.rs` take their dialect's number (about 8
  lines), `jig-repin.s1`'s. In the repository downstream of jig,
  `crates/temper/tests/support/{configuration,sign_in}.rs` (`sign_in.rs`
  sets `anthropic`: 2), `docs/examples/owned/agent.json` and
  `docs/examples/owned.json` (about 12 lines) are its own repin's
  (decision 16).
Test shape: unit, `config.rs` (each under 20 ms):
  `a_dialect_name_and_number_must_match`, rows `openai` 3, `anthropic`
  2, `codex` 1 taken, `openai` 1 and `anthropic` 3 refused naming
  `dialect`. Unit, settings:
  `an_openai_endpoint_is_written_with_its_number`.
Proof: unit, `a_dialect_name_and_number_must_match`: `openai` with 1 is
  refused naming `dialect`; under 20 ms (unit: the increment's tests
  all read configuration, and no world reads it).
Don't: renumber a dialect; change a preset.
Size: about 80 lines.
Removes: nothing.

## Interface sketches

```rust
// smith_protocol_channel, credentials.rs (04.1)
#[derive(Clone, PartialEq, Eq)]
pub enum GrantValue {
    Bearer(Box<[u8]>),
    Key(Box<[u8]>),
    Program(Box<[u8]>),
    /// Withdrawn: the `codex` dialect's id and bearer.
    Withdrawn { account_id: Box<[u8]>, bearer: Box<[u8]> },
}
impl core::fmt::Debug for GrantValue { /* "[redacted grant value]" */ }
// fn program_name(name: &[u8]) -> Result<&[u8], Error>
// Component::grant_value(&self, name: GrantName) -> Option<&GrantValue>

// smith_host_protocol (04.1)
pub struct Values { pub paths: Box<[Box<[u8]>]>, pub credentials: Box<[GrantValue]> }
// Component::send_grant(grant: channel::Grant, value: GrantValue, ..)

// smith_protocol_llm, grants.rs (04.1)
pub enum GrantError { Bounds, Stale, Full, Missing, Lapsed, Scheme }
```

## Done when

- `rg -n 'struct Secret|envelope\.put' crates/smith-protocol-channel
  crates/smith-local-service/src/service crates/smith-protocol-llm/src
  crates/smith-agent-shell/src` and
  `rg -n 'endpoint account is outside the profile'
  crates/smith-agent-shell/src` find nothing.
- `rg -l '"dialect":0' --glob '*.rs' --glob '*.json' crates tests`
  prints nothing.
- For the size of the files this plan grows, code-organization.md 8's
  commands print nothing new.
