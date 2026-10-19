# skein 09: the driver's sign-out and shared record

The accounts driver signs an account out, revoking its refresh token and
forgetting its record; names the required scopes a sign-in was not
granted, and refuses a grant without them; and reads a shared record
again under the owner's lock before each write.

Needs at start: skein 08's named accounts and the `SignIn` fields its
08.4 passes (the same driver files), skein 05's `Revoke` and terms, and
skein 06.1's revocation in the fake issuer, on skein's `origin/main`.

## Read first

- skein's design at `49725eca`: `oauth.md` 1, 2, 2.3, 3, 4, 6.2 to 6.8;
  `io.md` 5 (`Lock`, `Remove`), 5.3 (private files).
- skein `docs/foundation/programming-model.md` 4.2, 5.2 to 5.4, 6.2,
  6.3, 7, 9; `testing-strategy.md` 2.5, 2.7, 2.8, 6, 8, 10.
- The README's Rules; code-organization.md 2, 6, 7, 8; decisions.md 10,
  13, 15, 16, 17, 18 and design question 15.

## What the design asks

- oauth.md 2.3: revocation where the registration has an endpoint.
- 6.2: a shared record.
- 6.3: sign-out; the required scopes not granted.
- 6.4: the lock before each write; `locked`, busy, a newer generation.
- 6.6: a sign-out under close and abort.
- 6.7: the lock wait among the limits; the worst case's read.
- 6.8: the worlds' additions, each in the increment that builds it.

## Choices

- The lock wait's value is decision 15's; the repins' defaults are
  16's. Units and worlds only (10); no wait on live text (17); each
  Shape names its downstream breaks (13). The lock's other outcomes and
  OpenID through the driver are skein 10's.

## Rules that bind it

- skein 08's Rules hold here: the sizes of `component.rs`, `world.rs`
  and `sign_in.rs`; `MAX_OUT_*` unchanged; names, never places; both
  worlds as rows; no credential value printed.
- Sign-out goes to `component/sign_out.rs`, the lock to
  `component/lock.rs`, their units to `src/sign_out_tests.rs` and
  `src/shared_tests.rs` (`#[path]` from `tests.rs`), their stories to
  `tests/oauth-accounts/src/{sign_out,shared}.rs`, all new, reusing
  world.rs's fixtures and 08's `src/accounts.rs`. Each story module
  gives `run(seed, story, <flags>, memory)`, its flags plain `bool`s as
  `src/sign_in.rs`'s `run(seed, story, faulted, large, memory)` takes
  them (09.1's `sign_out::run(seed, story, private, memory)`;
  `src/private.rs`'s `run(seed, story)` takes neither), `facts(procs)`
  as `src/private.rs`'s `facts` does, and `protocol` as
  `src/world.rs`'s (ruling 121).
- The scripted owner lock is the world's: it plays the other process,
  writing a newer, absent or other-subject record, and never generates
  a key or draws outside the seeded generator.

## Increments

### 09.1 Signing out
Scope: oauth.md 6.3, "Sign-out"; 2.3's revocation; 6.4's forgetting;
  6.6 for a sign-out under close and abort.
Needs: skein 08's named accounts and `SignIn` fields; skein 05's
  `Revoke`, `Revoked` (confirmed, or not) and terms (the revocation
  endpoint); skein 06.1's revocation for the worlds.
Files: `crates/skein-oauth-accounts/src/`: `component.rs`,
  `component/sign_out.rs` (new), `component/private.rs`,
  `component/names.rs` (`named`'s match over `Request`, which has no
  `..`, maps `SignOut` by `lookup(account, Asked::SignOut, up)`),
  `component/accounts.rs` (`admit`'s `Unusable::Origin`), `keeper.rs`,
  `exchange.rs`, `boundary.rs`, `lib.rs`, `tests.rs` (`refusal_of`
  and `failure_of` match `Event` whole), `sign_out_tests.rs` (new),
  `private_tests.rs` (`assert_visit`'s match over `Event`, `Owner::next`
  and the abort test's matches over `FileLower`), `accounts_tests.rs`
  (its match over `Option<FileLower>`);
  `testing/skein-world/src/files.rs`; `tests/oauth-accounts/`:
  `src/sign_out.rs` (new), `src/lib.rs`, `src/world.rs` (`observe`'s
  match over `Event`, `private_io`'s over `FileLower`), `src/sign_in.rs`
  (`private_io`'s `match lower` over `FileLower` gains a `Remove` arm),
  `src/sign_in/owner.rs` (`observe`, about 84 lines: `SignedOut` joins
  its panic arms; its `Keep` arm moves into a `keep` helper if it nears
  100 lines), `src/accounts.rs` (`observe`'s match over `Event`,
  `protocol_files`' over `FileLower`), `tests/sign_out.rs` (new),
  `tests/memory.rs` (its two `FileLower` panic arms). Each match over
  `Event` or `FileLower` found later takes the new arm the same way, and
  the commit body lists it (ruling 136).
Shape: `Request::SignOut { account }` ends in `Event::SignedOut {
  account, revoked }`, or `Failed { ends: Ends::SignOut }`, or
  `Refused { asked: Asked::SignOut }`: `Source` for a handed-in account,
  `NoRecord` (new) for one that holds none once its load settles,
  `Busy`, `Full { bound: exchanges }`, `Account`, `Closed`. It takes an
  exchange (`Purpose::SignOut`, new) in the exchanges limit. A held
  grant ends at once as `Failed { ends: Grant, failure: Expired }`, and
  a grant asked meanwhile ends with it. With a revocation endpoint the
  client steps skein 05's `Revoke` over the web socket; its `Revoked`
  gives `revoked`. Without one, or for an access-only record
  (`SavedToken::refresh_state` gives `None`; oauth.md 2.3: it "has no
  refresh token and is never revoked by the client"), `revoked` is
  false and no connection opens. Then the record is forgotten: the account holds `State::Empty`;
  a private keeper runs `Operation::Remove` (new) of its file under its
  directory, and after it stores expecting the name absent; for the
  owner's store, `SignedOut` is the owner's word to drop its own. A
  removal that fails ends `Failed { ends: SignOut, failure: NotKept }`,
  the account holding no record. A `Cancel` before the owner's answer
  ends a sign-out as cancelled, its record kept.
  io's `file::Request::Remove` takes its root as an `Fd`, which the
  component never holds: `FileLower::Remove { owner, root: Token, name,
  directory, deadline }` (new; `directory` is io's flag, false for a
  file) asks the owner, which maps `root` with `FileIo::descriptor`,
  until io's roots by token (io.md, 5; STATUS.md's io row; decision 18);
  skein-world's `files::Driver` gains `descriptor`.
  A registration whose terms name an endpoint on another origin than the
  token endpoint's is `Unusable::Origin { account }` (new) until 10.2
  gives accounts their origins.
  Downstream: `SignedOut` breaks smith-local-service's `emit`
  (`credentials.rs`) and jig's `translate::up`
  (`crates/jig-protocol-accounts/src/translate.rs`); `Ends::SignOut` and
  `Asked::SignOut` break `up`'s `match ends` and `match asked` there,
  and `NoRecord` that file's `refused`. `FileLower::Remove` breaks each match over `FileLower`:
  smith-local-service's `credentials.rs` and `credentials/private.rs`;
  jig's `tests/protocol-accounts/src/owner.rs` and
  `testing/jig-test-service/src/accounts.rs`; the repository downstream
  of jig's service `accounts.rs`. No one downstream signs out: each new
  arm is unreachable (decision 16).
Test shape: unit, `src/sign_out_tests.rs`, a new file over `setup`,
  `hand_in` and `assert_refused` of `src/tests.rs`, with its `limits()`;
  each under 20 ms. A row that needs an owner with sign-in accounts and
  terms, several accounts, or a taker of `FileLower::Remove` gets it as
  a method of `private_tests.rs`' `Owner` (README, rule 110, ruling
  134): `Owner::with_terms` made `pub(super)`, and `accounts_tests.rs`'
  `owner` or `ports_tests.rs`' `owner` moved there where one serves,
  never a second copy; an arm that a new variant forces in their files
  moves nothing (ruling 136). Reference:
  ```rust
  #[test]
  fn a_sign_out_is_refused_where_it_does_not_belong() {
      let (mut component, env, mut out) = setup();
      hand_in(&mut component, &env, &mut out, 1);
      let mut io = Queue::with_capacity(MAX_OUT_DOWN.io);
      let mut files = Queue::with_capacity(1);
      let sign_out = Request::SignOut { account: 0 };
      component.down(&env, sign_out, &mut out, &mut io, &mut files);
      assert_refused(out.pop(), 0, Asked::SignOut, Refusal::Source);
      assert!(io.is_empty() && files.is_empty(), "nothing to revoke or remove");
  }
  ```
  (Its rows: handed in, no record, an exchange running, exchanges
  full, after close.) Beside it:
  `a_sign_out_says_whether_revocation_was_confirmed` (rows: confirmed,
  unconfirmed, no endpoint, an access-only record: false, no
  connection, the record forgotten), `a_sign_out_forgets_the_record` (rows:
  owner, private), `a_held_grant_ends_as_expired_at_a_sign_out`,
  `a_cancelled_sign_out_ends_as_cancelled_and_keeps_the_record` (rows:
  before the revocation's answer, before the owner's answer to the
  removal, which `FileLower::Cancel` stops),
  `a_file_not_removed_fails_the_sign_out_as_not_kept`,
  `a_sign_in_after_a_sign_out_stores_expecting_no_file`,
  `a_terms_endpoint_on_another_origin_is_unusable`.
  World, `tests/sign_out.rs`, a new file over `world::limits()`, the
  simulated and protocol worlds as rows, each under 0.5 s (the issuer
  runs as a process). `world::Fact` gains `SignedOut(bool)`. The
  worlds' `Terms` set `revocation_endpoint: None` today (`sign_in.rs`,
  `sign_in/issuer.rs`, `private.rs`): `Story::Confirmed` and
  `Unconfirmed` set it on the token endpoint's origin, the issuer's
  loopback one (`issuer_endpoint` allows http only on loopback), else
  `Unusable::Origin` refuses the registration; `NoRevocation` keeps
  `None` (ruling 136). Reference:
  ```rust
  #[test]
  fn a_sign_out_settles_for_both_keepers() {
      let rows = [(Story::Confirmed, true), (Story::Unconfirmed, false), (Story::NoRevocation, false)];
      for (seed, (story, revoked)) in (101..).zip(rows) {
          let settled = [Fact::SignedOut(revoked), Fact::Closed];
          for private in [false, true] {
              let outcome = sign_out::run(seed, story, private, Memory::Checked);
              assert_eq!(sign_out::facts(&outcome.procs), &settled, "{story:?}");
          }
          assert_eq!(sign_out::protocol(seed, story).0, settled, "{story:?} on streams");
      }
  }
  ```
  (The file opens as `tests/sign_in.rs` does, with `HEAP`, `Memory`,
  `world::Fact` and `sign_out::{self, Story}` from the world crate.)
  Beside it: `a_sign_out_under_close_and_abort_settles_once`.
Proof: world, `a_sign_out_settles_for_both_keepers`: a signed-in
  account's sign-out ends `SignedOut` with `revoked` as the issuer
  answered, the record gone from the owner's store and the private
  file, then `Closed`; under 0.5 s.
Don't: no change to io's `Remove` here; no revocation retried; no
  record kept after a sign-out that settled.
Size: about 300 lines.
Removes: nothing.

### 09.2 Required scopes
Scope: oauth.md 6.3, "Granted scopes"; 2.2's scopes.
Needs: skein 08's named accounts; skein 05's terms (the scopes that
  must be granted), its record's granted scopes and `Tokens`' missing
  ones; skein 06's declined scopes for the worlds.
Files: `crates/skein-oauth/src/redirect.rs` and `lib.rs` (`missing_scopes`
  made `pub` and re-exported beside `validate_registration`, its body
  unchanged; ruling 140); `crates/skein-oauth-accounts/src/`: `component.rs`,
  `component/sign_in.rs`, `component/private.rs` (the exhaustive
  `match failure` that sets `sign_in_allowed` takes `NotGranted`),
  `exchange.rs` (`Stage::Keeping` holds `missing`), `boundary.rs`,
  `tests.rs`; `tests/oauth-accounts/src/sign_in.rs`, `tests/sign_in.rs`.
Shape: `Event::SignedIn` gains `missing: Box<[u8]>`, the required
  scopes not granted, as skein 05's `Tokens { record, missing }` names
  them (`crates/skein-oauth/src/machine.rs`); the record is kept.
  `component.rs`'s `oauth::Request::Tokens { record, .. }` arm, which
  drops `missing` today, keeps it in `Stage::Keeping { candidate,
  missing }` until the keep settles and `SignedIn` is pushed. Each
  lend, a grant's answer and a refresh's `Granted`, checks the record's
  granted scopes against the terms' required ones: one lacking ends the
  grant as `Failed { ends: Grant, failure: NotGranted }` (new), nothing
  lent, until a sign-in brings them.
  Downstream: `SignedIn`'s field breaks jig's `translate::up`, whose arm
  names `SignedIn { account: _, generation: _ }` with no `..`
  (`crates/jig-protocol-accounts/src/translate.rs`); smith's arm has
  `..`. `Failure::NotGranted` breaks jig's `classify` there: sign in
  again (decision 16).
Test shape: unit, `src/tests.rs`, over its helpers, each under 20 ms:
  `signed_in_names_the_required_scopes_not_granted` (rows: all granted,
  one missing, none granted),
  `a_grant_of_a_record_without_a_required_scope_fails_as_not_granted`
  (rows: grant, a refresh's re-grant). World, `tests/sign_in.rs`,
  both worlds over `world::limits()`, each under 0.5 s:
  `required_scopes_declined_then_a_grant_is_refused`.
Proof: world, `required_scopes_declined_then_a_grant_is_refused`: a
  sign-in whose scopes the issuer declined ends `SignedIn` naming them,
  and the next grant ends `Failed` as not granted, nothing lent; under
  0.5 s.
Don't: no scope parsed by the driver: skein 05's record and terms say;
  each lend calls `skein_oauth::missing_scopes(&registration, &record)`
  with the account's `Registration` and its record (ruling 140).
Size: about 110 lines.
Removes: nothing.

### 09.3 The shared record under the owner's lock
Scope: oauth.md 6.4, "A shared record": the lock before each write,
  `locked`, busy, a newer generation; 6.2's shared; 6.7's limit and
  worst case.
Needs: 09.1's sign-out (its revocation is a write).
Files: `crates/skein-oauth-accounts/src/`: `component.rs`,
  `component/lock.rs` (new), `component/private.rs`,
  `component/names.rs` (`named` maps `Locked`'s account as `Kept`'s
  is mapped), `exchange.rs`,
  `accounts.rs`, `boundary.rs`, `limits.rs`, `lib.rs`,
  `shared_tests.rs` (new), `tests.rs`, `private_tests.rs`;
  `tests/oauth-accounts/`: `src/shared.rs` (new), `src/lib.rs`,
  `src/world.rs`, `src/sign_in.rs` (`Lock` and `Unlock` join the panic
  arms of `observe`, which 09.1's `keep` helper holds under 100 lines),
  `tests/shared.rs` (new), `tests/memory.rs`.
Shape: `Account::SignIn` gains `shared: bool`. Before a shared account's
  write, a refresh's exchange, a sign-in's keep and a sign-out's
  revocation, the exchange waits (`Stage::Locking`, new) on
  `Event::Lock { account }`. The owner answers `Request::Locked {
  account, answer }`: `Locked::Held { record }`, the record it read
  under the lock for its own store, or none; a private keeper loads its
  file again. `Locked::Busy` ends the write as `Failed { failure: Busy
  }` (new), transient, the account holding what it held, with no
  `Unlock`: no lock was taken. Under the lock, a newer generation of
  the same subject is held with no exchange: a refresh's grant is
  answered `Granted` with it where valid past the lead, and refreshed
  under this lock where not. Otherwise the write goes ahead, a sign-in's
  keep at a generation above the one read. `Event::Unlock` follows once
  the write is kept or has failed. A grant, a load and lending never
  lock. `Limits` gains `lock_wait: Duration`, which `usable` requires
  above zero; its deadline is 10.1's. `worst_case` adds a record per
  account, the read under its lock.
  skein's fixture value: `lock_wait: Duration::from_secs(1)` in
  `src/tests.rs`'s `limits()`, `tests/oauth-accounts/src/world.rs`'s
  `limits()` and `tests/memory.rs`'s literal.
  Downstream: `lock_wait` breaks each `Limits` literal; no host shares a
  record, and each takes skein's fixture value, `Duration::from_secs(1)`
  (decision 15): smith-local-shell's `credentials/product.rs`,
  smith-local-service's `credentials/tests.rs`; jig's
  `crates/jig-protocol-config/src/profile/small.rs` (`DRIVER`),
  `crates/jig-protocol-accounts/src/tests/fixtures.rs`,
  `tests/protocol-accounts/src/fixtures.rs`. `shared` breaks each
  `Account::SignIn` literal, `shared: false`: smith-local-service's
  `credentials.rs`; jig's `testing/jig-test-service/src/accounts.rs`,
  `tests/protocol-accounts/src/owner.rs`; the repository downstream of
  jig's service `accounts.rs`. `Lock` and `Unlock` break smith's `emit`
  and jig's `translate::up` (unreachable); `Busy`, jig's `classify`:
  `Later { after: None }`, as `NotKept` maps (decision 16).
Test shape: unit, `src/shared_tests.rs`, a new file over
  `refresh_account`, `assert_event` and `limits()` of `src/tests.rs`,
  with new helpers `shared_component()` (that account with `shared:
  true`), `queues()` and `shared_record(generation)`; each under 20 ms.
  Reference:
  ```rust
  #[test]
  fn each_write_of_a_shared_record_locks_first() {
      let (mut component, env) = shared_component();
      let (mut above, mut io, mut files) = queues();
      component.down(&env, Request::Grant { account: 0 }, &mut above, &mut io, &mut files);
      assert_event(above.pop(), Event::Lock { account: 0 });
      assert!(io.is_empty(), "no connection opens before the lock is held");
      let answer = Locked::Held { record: Some(shared_record(0)) };
      component.down(&env, Request::Locked { account: 0, answer }, &mut above, &mut io, &mut files);
      assert!(above.is_empty() && !io.is_empty(), "the exchange starts under the lock");
  }
  ```
  (This is its refresh row; the others are a sign-in's keep and a
  sign-out's revocation.) Beside it:
  `only_writes_lock_a_shared_record` (rows: grant, load, lend),
  `a_newer_generation_of_the_same_subject_is_held_with_no_exchange`,
  `a_newer_generation_within_the_lead_is_refreshed_under_this_lock`,
  `a_sign_in_keeps_above_the_generation_read`,
  `a_busy_lock_fails_the_write_as_busy_and_keeps_the_record`,
  `the_lock_is_released_once_the_write_settles` (rows: kept, failed),
  `a_lock_wait_of_zero_is_unusable`. In `src/private_tests.rs`, whose
  `Owner` gains `shared()` beside `new()`:
  `a_private_keeper_loads_its_file_again_under_the_lock`.
  World, `tests/shared.rs`, a new file over `world::limits()`, both
  worlds, under a scripted owner lock, each under 0.5 s. Reference:
  ```rust
  #[test]
  fn a_busy_owner_lock_fails_the_refresh_as_busy() {
      let facts = [Fact::Lock, Fact::Failed(Failure::Busy), Fact::Closed];
      let outcome = shared::run(111, Story::Busy, Memory::Checked);
      assert_eq!(shared::facts(&outcome.procs), &facts, "no Unlock: no lock was taken");
      assert_eq!(shared::protocol(111, Story::Busy).0, facts);
  }
  ```
  (`world::Fact` gains `Lock` and `Unlock`; the header is
  `tests/sign_out.rs`'s, with `Failure`.) Beside it:
  `a_shared_refresh_finds_a_newer_generation_and_exchanges_nothing`.
  `tests/memory.rs`'s
  `every_account_holds_a_maximal_record_within_the_heap_bound` holds a
  shared read per account.
Proof: world,
  `a_shared_refresh_finds_a_newer_generation_and_exchanges_nothing`: a
  shared account's refresh emits `Lock`, takes the owner's newer record,
  lends it with no connection opened, and `Unlock`s once; under 0.5 s.
Don't: no lock for a record not shared; no io `Lock` taken by the
  component (the lock is the owner's); no wait on the lock for a grant.
Size: about 290 lines.
Removes: nothing.

## Interface sketches

```rust
// skein_oauth_accounts::boundary; added variants and fields only.
pub enum Request {
    /// Revokes and forgets the account's record; `SignedOut`, `Failed` or `Refused` (09.1).
    SignOut { account: u32 },
    /// The owner's answer to `Lock`, for an account whose record is shared (09.3).
    Locked { account: u32, answer: Locked },
}
/// The owner's answer to a shared record's `Lock` (oauth.md, 6.4).
#[derive(PartialEq, Eq, Hash)]
#[expect(missing_debug_implementations, reason = "credential values must never occur in traces")]
pub enum Locked {
    /// The lock is held, with the owner's record read under it; none for none or a private keeper.
    Held { record: Option<SavedToken> },
    /// The owner could not take its lock; the write fails as busy, with no `Unlock`.
    Busy,
}
pub enum Event {
    /// Ends `SignOut`: the record is forgotten; `revoked` says whether revocation was confirmed.
    SignedOut { account: u32, revoked: bool },
    /// Take the shared record's lock and read it under the lock; answered by `Locked`.
    Lock { account: u32 },
    /// Release the lock one `Held` answer took.
    Unlock { account: u32 },
    /// Ends `SignIn`, naming the required scopes not granted (09.2).
    SignedIn { account: u32, generation: u64, missing: Box<[u8]> },
}
// Ends::SignOut and Asked::SignOut, the owner's sign-out; Refusal::NoRecord, no record to sign out.
pub enum Failure {
    /// The record lacks a scope the registration requires; nothing is lent.
    NotGranted,
    /// The shared record's lock was not taken in time; transient.
    Busy,
}
// skein_oauth_accounts::component (09.1); transitional, until io's roots by token.
// The file gains `use alloc::boxed::Box;` and names `skein_lib::Token` in full.
pub enum FileLower {
    /// Remove the named file (or empty directory, when `directory`) under this private `root`; the owner maps `root` with `FileIo::descriptor`.
    Remove { owner: skein_lib::Token, root: skein_lib::Token, name: Box<[u8]>, directory: bool, deadline: Time },
}
pub enum Unusable {
    /// A registration's endpoint lies on an origin the account was not given.
    Origin { account: u32 },
}
// skein_oauth_accounts::accounts (09.3)
pub enum Account {
    SignIn { registration: Registration, endpoint: Endpoint, keeper: Keeper, shared: bool },
}
// skein_oauth_accounts::limits (09.3)
pub struct Limits {
    /// How long a shared write waits for `Locked` before it fails as busy.
    pub lock_wait: Duration,
}
```

## Questions to raise when reached

None: ruling 121 gives `missing`'s type, `Box<[u8]>`, as skein 05's
`Tokens` has it.

## Done when

In skein's checkout of `origin/main`:

```
rg -o -w 'SignOut|Locked|SignedOut|Lock|Unlock|NotGranted|NoRecord' crates/skein-oauth-accounts/src/boundary.rs | sort -u | wc -l
rg -c -U 'Remove \{\s*owner: (skein_lib::)?Token,\s*root: (skein_lib::)?Token' crates/skein-oauth-accounts/src/component.rs
rg -c 'lock_wait: Duration::from_secs\(1\)' crates/skein-oauth-accounts/src/tests.rs tests/oauth-accounts/src/world.rs tests/oauth-accounts/tests/memory.rs
```

print 7, then 1, then each of the three files with 1. The long-file
command of code-organization.md 8 prints no file past 100 lines of
growth, and no new file past 800 lines.
