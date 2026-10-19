# skein 08: the driver's accounts

The accounts driver names its accounts: those `new` is given by index,
others the owner adds under a number no live account holds; any may be
removed. A wanted account is refreshed though nobody holds its grant. A
redirect whose port varies binds a fresh listener for each sign-in, and
the client's `SignIn` carries the bound port, the nonce and the hints.

Needs at start: skein 05.2's record version 3 and 05.3's terms. 08.4
also needs 05.4's hints and 06.1's fake issuer.

## Read first

- skein's design at `49725eca`: `oauth.md` 1, 2, 2.1, 4, 6.1 to 6.8;
  `io.md` 3.2, 5.3; `programming-model.md` 4.2, 5.2 to 5.4, 6.2, 6.3,
  7, 9; `testing-strategy.md` 2.5, 2.7, 6, 8, 10.
- The README's Rules; code-organization.md 2, 6, 7, 8; decisions.md 8,
  10, 13, 14, 16, 17.

## What the design asks

- **oauth.md 1, 6.4:** a record without its issued client id is
  unreadable; a sign-in replaces the file, stating its digest.
- **oauth.md 2, 2.1, 6.1:** `SignIn` carries the bound redirect and the
  hints, its draws seeded in request order; no refresh before the
  earliest refresh time.
- **oauth.md 6.2:** a stable number, never renamed or reused.
- **oauth.md 6.3:** `add` answers only when refused; `remove` cancels
  unsent work, lets sent work finish, then `removed`; a wanted account
  is refreshed with no grant held.
- **oauth.md 6.5:** a varying port binds a fresh listener per sign-in.
- **oauth.md 6.6, 6.7, 6.8:** a removal finishes under close, abort
  cancels it; `accounts` counts added accounts; both worlds cover them.

## Choices

- `Component::new` keeps its list, each account named by its place in
  it (decision 14): no constructor changes downstream.
- No new limit: `accounts` bounds the names (decision 15). Unit and
  world tiers only (decision 10); no wait on live text (17).

## Rules that bind it

- Files over 800 lines grow by at most ~100 lines per increment:
  `component.rs` (1135), `tests/oauth-accounts/src/world.rs` (1224),
  `sign_in.rs` (852). `add`, `remove` and `want` go to
  `component/accounts.rs` (new); the accounts' stories to
  `tests/oauth-accounts/src/accounts.rs` (new), reusing world.rs's
  `Client`, `limits()`, `registration()` and issuer.
- `MAX_OUT_*` stay: an entrance with more to say leaves it to a later
  `fire`, which `has_work` and `next_deadline` announce.
- Each new world story runs in the simulated and the protocol world
  (`world::protocol`, `sign_in`'s cuts), as rows of one test.

## Increments

### 08.1 The named table
Scope: oauth.md 6.2's names; 6.4's unreadable record under an issued
  client id (decision 8, the driver's half).
Needs: skein 05.2's record version 3 (the issued client id it keeps,
  none in a version 2 record) and 05.3's terms (whether it is issued).
Files: `crates/skein-oauth-accounts/src/`: `component.rs`,
  `component/sign_in.rs`, `component/private.rs`, `boundary.rs`,
  `limits.rs`, `lib.rs` (its doc's "knows no account names", false from
  here), `tests.rs`, `private_tests.rs`; `tests/oauth-accounts/`:
  `src/private.rs` (`Story`, and the matches in `run` over it),
  `src/world.rs`, `tests/private.rs`, `tests/fuzzy_private.rs`.
Shape: `Component` gains `names: List<Name>` of capacity
  `limits.accounts`, beside `accounts`, `sources`, `bindings` and
  `keepers`: place k is `Free` or `Live { number }`. `new` names each
  place by its index, so its list, its checks and `Unusable`'s `account`
  stay (decision 14). A private `place(account) -> Option<u32>` scans
  the live names, bounded by `accounts`; each entrance (`down`'s arms,
  `sign_in`, `redirected`, `hand_in`, `grant_request`, `kept`,
  `refused`) maps the name to a place first, and each `Event` maps the
  place back. The client's `SignIn` takes the name as its `key`. A name
  no live account holds is refused as `Refusal::Account`, its doc now
  "names no live account"; `Rejected`, `Release`, `Kept` and `Cancel`
  drop it, as they drop an index past the list today.
  Memory: `worst_case` adds `List::<Name>::worst_case(accounts)`; a
  `Name` is 8 bytes (a tag and a `u32`), so 8 an account and 32 for
  `tests/memory.rs`'s four. 08.1 reports that figure;
  `every_account_holds_a_maximal_record_within_the_heap_bound`
  holds against it.
  Decision 8: `private.rs`'s `loaded` reads a record with no issued
  client id, under a registration whose terms say the id is issued, as
  `Unloaded { at: File, why: Unreadable }`: a grant fails as unloaded,
  and a sign-in goes ahead and stores expecting the digest it loaded,
  as for a damaged file (`sign_in_allowed`). An owner's kept record of
  that kind (`Keeper::Owner { kept }`) is held as none: `State::Empty`,
  a grant fails as expired, a sign-in goes ahead.
  Downstream: nothing breaks; `new`'s accounts keep their indices.
Test shape: unit, with `setup()`, `assert_refused` and `refresh_account`
  of `src/tests.rs` and `Owner` of `src/private_tests.rs` (which gains
  `with_terms(Option<Terms>)`, `new` calling it with `None`), each under
  20 ms: `a_name_no_live_account_holds_is_refused` (rows: sign in,
  redirected, hand in, grant, each with a name past `new`'s list);
  `an_owner_record_without_an_issued_id_is_held_as_none`;
  `a_record_without_an_issued_id_loads_unreadable`;
  `a_sign_in_replaces_it_expecting_the_digest_it_loaded`. World,
  `tests/private.rs`, under 0.1 s, over a new `Story::NoIssuedId` (the
  file planted at version 2, its bytes written by hand as 05.2's tests
  write theirs; `Client::private` takes issued terms for it; it joins
  `fuzzy_private.rs`' list, which draws by `random.below(20)`, a count
  of the list's 20 entries: it draws below the list's length instead, so
  the new story is drawn; `accounts.rs` already imports
  `skein_tls::Name`, so the table's `Name` is named by its path there):
  `a_record_without_an_issued_id_ends_a_grant_unreadable`.
Proof: world, `a_record_without_an_issued_id_ends_a_grant_unreadable`:
  the grant ends `Unloaded { at: File, why: Unreadable }` and the
  person's file is not written; under 0.1 s; unit,
  `a_sign_in_replaces_it_expecting_the_digest_it_loaded`: the store
  names the digest read; under 20 ms. The replacement has no world row:
  it needs the fake issuer to issue an id (06.1), which 08.4 has.
Don't: no second table or account type beside `State`, `Source` and
  `Store`; no hashed map (the scan is bounded); no change to `new`'s
  signature or to the order of `new`'s checks.
Size: about 150 lines.
Removes: nothing.

### 08.2 Accounts added and removed
Scope: oauth.md 6.3, "Accounts come and go"; 6.4's load of an added
  private account; 6.6's removal under close and abort; 6.7's limit.
Needs: 08.1's table.
Files: `crates/skein-oauth-accounts/src/`: `component.rs`,
  `component/accounts.rs` (new), `boundary.rs`, `lib.rs` (its doc names
  `add`), `limits.rs`, `tests.rs`, `accounts_tests.rs` (new, a `#[path]`
  module of `tests.rs` as `private_tests.rs` is), `private_tests.rs`;
  `tests/oauth-accounts/src/`: `accounts.rs` (new), `lib.rs`,
  `world.rs`, `sign_in.rs` (these two and `private_tests.rs` match
  every `accounts::Event`: `Removed` joins their panic arms);
  `tests/accounts.rs` (new), `tests/memory.rs`.
Shape: `Component::add(env, account, configuration: Account, up)` is an
  entrance of its own, not a `Request` variant: `Request` derives
  `PartialEq`, `Eq` and `Hash`, which jig's tests use, and an
  `Account`'s `Registration` and TLS `Config` have none. `new`'s
  per-account checks move into `admit(account, configuration, limits)
  -> Result<(Source, State, Store), Unusable>`, which both call. `add`
  answers only when refused, `Refused { asked: Asked::Add, why }`:
  `Refusal::Taken` (new) for a name a live or removing account holds,
  `Full { bound: accounts }`, `Closed`, or `Refusal::Unusable(Unusable)`
  (new; `boundary.rs` gains `use crate::Unusable;`) for a configuration
  `new` would refuse. Admitted, it takes the first free place, else the
  next. A private keeper starts its load at the next `fire`, as a
  configured one does, not inside `add`: `add` has an `up` queue and no
  files queue, and giving it one would raise the room every owner
  reserves (`MAX_OUT_DOWN.files` is 1).
  `Request::Remove { account }` ends in `Event::Removed { account }`,
  or is refused as `Account` or `Closed`; the place becomes `Removing {
  number }`, so its requests are refused as `Account` and `add` of its
  name as `Taken`. A sign-in waiting for a person (its listener
  closes), a grant held or asked, and a refresh not yet sent end
  `Failed { failure: Exchange(Cancelled) }`; an exchange already sent
  and its keep, the owner's `Kept` or a private `Store`, finish first.
  Then the listener, the exchange's sockets and a private keeper's
  directory close (`Operation::Close`), `Removed` follows and the place
  is `Free`. The record stays with its keeper. Under close a removal
  finishes; an abort cancels its exchange; `Removed` precedes `Closed`.
  Memory: `worst_case` stays at 08.1's figure; `add` fills places.
  Downstream: `Event::Removed` breaks smith-local-service's exhaustive
  `emit` (`credentials.rs`) and jig's `translate::up`; `Asked::{Add,
  Remove}` breaks jig's `match asked` in `up`, and `Refusal::{Taken,
  Unusable}` its `refused` (`crates/jig-protocol-accounts/src/
  translate.rs`). Decision 16: they join the unreachable arms.
Test shape: unit, `src/accounts_tests.rs`, by `use super::*` over
  `limits()`, `crypto()`, `refresh_account` and `assert_refused` of
  `tests.rs`, plus a `roomy()` (new: `setup()` with `accounts: 3` and
  one `HandedIn`), each under 20 ms. Reference test:
  ```rust
  #[test]
  fn an_added_name_is_refused_while_a_live_account_holds_it() {
      let (mut component, env) = roomy();
      let mut up = Queue::with_capacity(MAX_OUT_DOWN.above);
      component.add(&env, 0, Account::HandedIn, &mut up);
      assert_refused(up.pop(), 0, Asked::Add, Refusal::Taken);
      component.add(&env, 5, Account::HandedIn, &mut up);
      assert!(up.pop().is_none(), "an admitted add answers nothing");
      component.add(&env, 5, Account::HandedIn, &mut up);
      assert_refused(up.pop(), 5, Asked::Add, Refusal::Taken);
  }
  ```
  Beside it: `an_add_past_the_accounts_limit_is_refused_as_full`,
  `an_added_configuration_new_refuses_is_refused_as_unusable` (rows:
  each `Unusable` but `Accounts` and `Limits`),
  `an_added_private_account_starts_its_load_at_the_next_fire`,
  `a_removal_cancels_the_unsent_work_of_its_account` (rows: a sign-in
  waiting for a person, a grant held, a grant asked, a refresh due),
  `a_removal_lets_a_sent_refresh_and_its_keep_finish` (rows: owner,
  private), `a_removing_account_refuses_its_requests`,
  `a_removed_name_is_free_for_add`,
  `an_abort_settles_a_removal_before_closed`. World, `tests/accounts.rs`
  over `src/accounts.rs` (`Story`, `Fact`, `run`, `protocol`, on
  `world.rs`'s `Client`), both worlds as rows, each under 0.1 s.
  Reference test:
  ```rust
  #[test]
  fn accounts_added_and_removed_in_every_state_settle_once() {
      for story in [Story::Idle, Story::Loading, Story::SigningIn,
                    Story::Refreshing, Story::Keeping, Story::Holding] {
          let (facts, _) = accounts::protocol(131, story);
          let removed = facts.iter().filter(|f| **f == Fact::Removed(7));
          assert_eq!(removed.count(), 1, "{story:?}");
          assert_eq!(facts.last(), Some(&Fact::Closed));
          drop(accounts::run(131, story, Memory::Checked));
      }
  }
  ```
  Beside it: `a_name_reused_after_removed_starts_from_its_keeper`.
  `tests/memory.rs`: `an_added_and_removed_place_fits_the_heap_bound`.
Proof: world, `accounts_added_and_removed_in_every_state_settle_once`:
  each state's removal ends its unsent work cancelled, then answers one
  `Removed`, and the name is free; under 0.1 s; unit,
  `a_removal_cancels_the_unsent_work_of_its_account`: the four
  cancellations; under 20 ms.
Don't: no reuse of a place's old exchange id or owner tokens by the next
  account; no record forgotten by a removal; no removal of the record's
  file (a sign-out's, skein 09.1, decision 18).
Size: about 280 lines.
Removes: nothing.

### 08.3 A wanted account and the earliest refresh time
Scope: oauth.md 6.3, "A wanted account" and "Refreshed while held";
  2's earliest refresh time.
Needs: 08.1's table; skein 05.2's record version 3
  (`earliest_refresh_at`, seconds since the epoch, an unreadable value
  read as none).
Files: `crates/skein-oauth-accounts/src/`: `component.rs` (`recorded()`
  builds `State::Record`), `component/accounts.rs`, `boundary.rs`,
  `accounts_tests.rs`; `tests/oauth-accounts/src/accounts.rs`,
  `tests/accounts.rs`.
Shape: `Request::Want { account, wanted: bool }` sets a flag of the live
  name, `Live { number, wanted }`; it is refused only as `Account` or
  `Closed`, with `Asked::Want`. `refreshable` stays a refresh-token
  test; `wanted` joins `held` where the component gates on a held grant:
  `Component::fire`'s refresh at the lead, `Component::next_deadline`'s
  `State::Record` arm and `fail_exchange`'s `held(slot)`; a further gate
  on `held` that the lead's refresh passes takes it the same way, listed
  in the commit body. At its lead it refreshes with no grant held, the new record is
  kept, and no `Granted` follows. Its failure, with no grant held, is
  `Failed { ends: Ends::Refresh, failure }` (new), its record dropped as
  for any refresh that fails as sign in again (6.4). An account with no
  record, or a handed-in one, is never refreshed for being wanted. A
  `Cancel` of a wanted refresh ends it as `Ends::Refresh`, cancelled.
  `State::Record` gains `earliest: Option<Time>`, the record's earliest
  refresh time on the monotonic clock, turned from wall seconds where
  `recorded()` turns its expiry. No refresh starts before it, whatever
  asks: the lead, a grant within the lead, a rejection, a want.
  `next_deadline` names it where a refresh waits on it. A grant
  meanwhile is answered with the held token while it is valid; past
  validity the grant waits for that refresh.
  Memory: `Name` stays 8 bytes with `wanted`; `earliest` adds up to 16
  bytes to `State`, so `worst_case` rises by up to 16 an account. 08.3
  reports the new figure, which `tests/memory.rs`'s existing bound test
  holds against.
  Downstream: `Ends::Refresh` breaks jig's `match ends` in
  `translate::up`, and `Asked::Want` its `match asked`
  (`crates/jig-protocol-accounts/src/translate.rs`); decision 16 makes
  both unreachable. jig's `ToComponent::Want` maps to the driver's
  `Grant`, a name it shares with this `Want`; the repin keeps that
  mapping. smith compares `ends == Ends::Grant`, and is unchanged.
Test shape: unit, `src/accounts_tests.rs` with its `roomy()`, each under
  20 ms: `a_wanted_account_is_refreshed_at_its_lead_with_no_grant`,
  `a_wanted_refresh_keeps_its_record_and_lends_nothing` (rows: owner,
  private), `a_wanted_refresh_that_fails_says_failed_as_refresh`,
  `a_wanted_account_without_a_refreshable_record_is_left_alone` (rows:
  no record, handed in), `a_want_answers_only_when_refused` (rows: a
  live name, a free name, after close),
  `no_refresh_starts_before_the_earliest_refresh_time` (rows: lead,
  grant within the lead, rejection, want),
  `a_grant_before_it_is_answered_with_the_held_token`. World,
  `tests/accounts.rs`, both worlds, each under 0.1 s:
  `a_wanted_account_is_refreshed_with_no_grant_held`,
  `a_wanted_account_whose_refresh_fails_is_told_so`.
Proof: world, `a_wanted_account_is_refreshed_with_no_grant_held`: the
  refresh is kept and no `Granted` follows; under 0.1 s; unit,
  `no_refresh_starts_before_the_earliest_refresh_time`: each of the four
  asks waits for it; under 20 ms.
Don't: no interval other than the refresh lead (oauth.md 7's default);
  no `Granted` for a refresh nobody holds; no wall clock in a deadline.
Size: about 200 lines.
Removes: nothing.

### 08.4 A redirect whose port varies
Scope: oauth.md 6.5's ports; 2 and 2.1's `SignIn` fields; 6.1's seeded
  draws.
Needs: 08.1's table; 08.2's `admit`; skein 05.3's terms (a redirect
  whose port varies, its preferred port) and `SignIn` fields, 05.4's
  hints, which the driver's `SignIn` passes empty until here; 06.1's
  fake issuer, which takes a loopback redirect on any port.
Files: `crates/skein-oauth-accounts/src/`: `redirect.rs`,
  `component/sign_in.rs`, `component/accounts.rs` (`admit`),
  `component.rs` (`new`; `up`, which carries `Listening`'s bound port,
  dropped today by `Listening { owner, .. }`; `progress`, which passes
  the bound redirect URI to `Listener::progress` in place of
  `registration.redirect_uri`), `listener.rs`, `exchange.rs`, `limits.rs`,
  `tests.rs`; `tests/oauth-accounts/src/`: `sign_in.rs`,
  `sign_in/browser.rs`, `sign_in/cuts.rs`, `sign_in/protocol.rs` (its
  `Listen` arm asserted the fixed address), `sign_in/issuer.rs`;
  `tests/sign_in.rs`, `tests/memory.rs`.
Shape: `redirect.rs` gains `listen_address(uri, port) ->
  Option<SocketAddr>`, the host `redirect_address` reads with the given
  port, zero included, and `with_port(uri, port) -> Option<Box<[u8]>>`.
  `Redirect` gains `client_id`, read in `field` like `code`,
  percent-decoded within `document.issued_client_bytes` (a repeat or a
  longer value refuses the redirect), and `deliver` sends it in
  `Event::Redirected`, which 05.3 had sent `None`. `parse` and
  `from_head` take the bound URI as `registered`, so `from_head`
  compares the Host header with the bound port, not the preferred one.
  A public sign-in's `begin_sign_in` draws the state, verifier and
  nonce (an OpenID registration) from the component's `CryptoRng` as
  the owner asks, in request order, fills the held record's issued
  client id, ID token and subject (`State::Record` or `Loaded`; none
  for `Empty`), and holds the `SignIn` in its exchange. It pushes
  `Lower::Listen` on `listen_address(uri, preferred)`: for a varying
  redirect the registered port, zero where that is 0; otherwise the
  fixed one. `Listening` steps the client, with `SignIn.port` the
  `Listening` port for a varying redirect and `None` for a fixed one;
  the `Visit` follows at the next `fire`. The `Listener` keeps the
  bound URI (`with_port`) and a once-only fallback: an io `Failed`
  before `Listening` on a nonzero preferred port listens again on zero.
  A listener that fails on zero, or on a fixed port, ends the sign-in
  before its `visit`: `Failed { ends: SignIn, failure:
  Exchange(Unavailable) }`. Each sign-in has its own listener, so two
  at once own two ports, within `listeners`. A confidential sign-in
  steps at once, as today.
  `admit` tests `redirect_address` only for a fixed redirect; for a
  varying one `validate_registration` is the test (05.3 admits port 0
  or a preferred port on `127.0.0.1` and `[::1]`), so `new` and `add`
  admit port 0 for it. That lifts 05.3's refusal of a port-0 varying
  redirect; a fixed redirect with port 0 stays `Unusable::Redirect`.
  Memory: `listener::worst_case` adds the bound URI (`uri_bytes`) and
  the redirect's client id (`issued_client_bytes`), and each exchange
  the held `SignIn` at its caps. 08.4 reports the new figure.
  Downstream: nothing breaks; smith's registrations name a fixed port,
  and no smith test times their `Visit`.
Test shape: unit, `src/tests.rs` (`limits()`, `refresh_account`,
  `crypto()`, `redirect::parse`), each under 20 ms, with
  `varying(account, uri)` (new: the registration's `terms` carry
  `varying_port` and its redirect is `uri`):
  `an_account_with_port_zero_is_configured_then_signed_in` (`new`
  admits it; `Listen` on port 0; `Listening` with a bound port; the
  `Visit` URL carries it);
  `a_fixed_redirect_with_port_zero_is_unusable` (rows: `new`, `add`);
  `a_redirect_is_matched_against_the_bound_uri` (`parse` and
  `from_head`, rows: the bound port, the preferred port);
  `a_redirects_client_id_reaches_the_machine` (rows: present, absent,
  repeated, past its cap);
  `a_public_sign_in_visits_only_after_its_listener_listens` (rows:
  fixed, varying); `a_varying_redirect_falls_back_to_port_zero`;
  `a_listener_that_cannot_bind_ends_the_sign_in_before_its_visit`
  (rows: fixed, zero);
  `two_sign_ins_on_a_varying_redirect_own_two_ports`;
  `signing_in_again_carries_the_held_records_hints` (an owner's kept
  record with an issued client id, ID token and subject; the client's
  `SignIn` compared with the fixture's bytes);
  `the_nonce_is_drawn_from_the_seeded_generator`. World,
  `tests/sign_in.rs`, both worlds, each under 0.5 s (issuer, client and
  browser): `a_varying_port_signs_in_with_its_preferred_port_taken`,
  `two_sign_ins_at_once_each_follow_their_own_port`, over new `Story`
  rows joined to the table in
  `protocol_streams_cover_the_owner_redirect_cancel_close_and_keep_terminals`
  of `tests/sign_in.rs`; the scripted browser follows the port in each
  `visit`'s redirect.
  `tests/memory.rs`: `two_varying_sign_ins_fit_the_heap_bound`.
Proof: world, `a_varying_port_signs_in_with_its_preferred_port_taken`:
  the sign-in binds zero once the preferred port is refused, the
  browser follows it and the record is kept; under 0.5 s; unit,
  `an_account_with_port_zero_is_configured_then_signed_in`: under 20 ms.
Don't: no `localhost` for a varying redirect (skein 05's terms refuse
  it); no shared listener between sign-ins; no hint in a log, a `Debug`
  or a test's output.
Size: about 260 lines.
Removes: the `SignIn` step before `Listening` in `begin_sign_in`; 05.4's
  hints and the terms' fields passed empty by `component/sign_in.rs`.

## Interface sketches

```rust
// skein_oauth_accounts::component (08.2), a method of Component
pub fn add(&mut self, env: &Env<Limits>, account: u32,
           configuration: Account, up: &mut Queue<Event>); // refusals only; the body never todo!() (clippy denies it)
// skein_oauth_accounts::boundary, added variants (08.2, 08.3)
use crate::Unusable;
pub enum Request { Remove { account: u32 }, Want { account: u32, wanted: bool } } // Remove: unsent work fails cancelled
pub enum Event { Removed { account: u32 } } // the name is then free
pub enum Ends { Refresh }                   // a wanted account's refresh
pub enum Asked { Add, Remove, Want }
pub enum Refusal { Taken, Unusable(Unusable) }
```

## Questions to raise when reached

None.

## Done when

- Each command prints what its comment says:
  ```sh
  # 7: the names the boundary gains (0 at the base)
  rg -o -w 'Add|Remove|Removed|Want|Refresh|Taken|Unusable' \
    crates/skein-oauth-accounts/src/boundary.rs | sort -u | wc -l
  # kept: new's signature is unchanged
  rg -U -q 'pub fn new\(accounts: List<Account>,\s*limits: &Limits,\s*crypto: CryptoRng' \
    crates/skein-oauth-accounts/src/component.rs && echo kept
  # nothing: the bind address comes from redirect.rs
  rg -c -w 'redirect_address' crates/skein-oauth-accounts/src/component/sign_in.rs
  ```
- Sizes: the size and long-files commands of code-organization.md,
  section 8, run at each increment's end, as the Rules bind.
