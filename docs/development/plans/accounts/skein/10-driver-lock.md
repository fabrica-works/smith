# skein 10: the driver's lock outcomes and OpenID

The accounts driver ends each write under the owner's lock that finds
no record or another subject, answers each `locked` with one `unlock`
until `closed`, and fails a write whose lock wait passes as busy. Then
OpenID reaches the driver: GET requests on an account's origins, and a
sign-in of another subject than expected refused.

Needs at start: skein 09's lock, `lock_wait` and shared stories [09.3]
on skein's `origin/main`.

## Read first

- skein's design at `49725eca`: `oauth.md` 2.1, 2.2, 6.2 to 6.8.
- skein `docs/foundation/programming-model.md` 4.2, 5.2 to 5.4, 6.2,
  6.3, 7, 9; `testing-strategy.md` 2.5, 2.7, 2.8, 6, 8, 10.
- The README's Rules; code-organization.md 2, 6, 7, 8; decisions.md 10,
  13, 15, 16, 17, 20 and design questions 15 and 16.
- `09-driver-shared.md`'s Rules and 09.3, whose files this session
  changes.

## What the design asks

- oauth.md 2.1 and 2.2: the keys and the ID token, by GET.
- 6.2: each origin the terms name, with its address.
- 6.3: an expected subject; "Another subject"; `signed in`'s subject.
- 6.4: under the lock, no record and another subject.
- 6.6: one `unlock` per `locked` until `closed`; abort releases each.
- 6.7: the lock wait on the monotonic clock.
- 6.8: the worlds' additions, each in the increment that builds it.

## Choices

- The lock is design questions 15 and 16's answers; the repins'
  defaults are decision 16's. Units, worlds and a fuzzy run (10); no
  wait on live text (17); each Shape names its downstream breaks (13).

## Rules that bind it

- skein 08's and 09's Rules hold here: the files they name, the story
  modules' `run` and `facts`, the scripted owner lock.

## Increments


### 10.1 The lock's other outcomes
Scope: oauth.md 6.4, under the lock: no record, another subject; one
  `unlock` per `locked`; 6.6's abort; 6.7's lock wait (design questions
  15 and 16).
Needs: what the session needs.
Files: `crates/skein-oauth-accounts/src/`: `component.rs`,
  `component/lock.rs`, `component/sign_out.rs`, `boundary.rs`,
  `shared_tests.rs`; `tests/oauth-accounts/`: `src/shared.rs` (its
  `observe` hands a lock answer to a helper that matches `Story`, so
  no fn passes 100 lines), `tests/shared.rs`, `tests/memory.rs`,
  `tests/fuzzy_shared.rs` (new).
Shape: no record under the lock: the account holds none, a refresh fails
  its grant as expired, a sign-out ends `SignedOut { revoked: false }`
  with nothing to revoke, a sign-in goes ahead. Another subject (skein
  05's record subject): a refresh or a sign-out fails as
  `Failure::AnotherSubject` (new), the account drops what it held and
  loads again before its next grant; a sign-in goes ahead (its expected
  subject is 10.2's). Each `Locked::Held` gets exactly one `Unlock`,
  also when a cancel, a removal (08.2), a close or an abort ends the
  write meanwhile; a `Held` for no pending lock gets `Unlock` at once.
  An abort unlocks every lock held before `Closed`, and `Closed` waits
  for each pending lock's answer, unlocked at once, or its lock wait.
  After `Closed` the driver answers nothing: a `Held` then is the
  owner's to release (design question 16).
  The lock wait: the deadline `env.now + lock_wait` set when `Lock` is
  emitted, in `next_deadline`; `fire` past it fails the write as busy,
  as `Locked::Busy` does, and a later `Held` for it gets `Unlock` at
  once (design question 15).
  Downstream: `Failure::AnotherSubject` breaks jig's `classify`
  (`crates/jig-protocol-accounts/src/translate.rs`): sign in again
  (decision 16).
Test shape: unit, `src/shared_tests.rs`, over 09.3's helpers, each under
  20 ms: `no_record_under_the_lock_ends_each_write` (rows: refresh,
  sign-out, sign-in), `another_subject_under_the_lock_ends_each_write`
  (rows: refresh, sign-out, sign-in), `each_held_lock_is_unlocked_once`
  (rows: cancel, removal, close, abort),
  `a_locked_for_no_pending_lock_is_unlocked_at_once`,
  `a_lock_with_no_answer_within_its_wait_fails_as_busy`,
  `a_locked_after_the_lock_wait_is_unlocked_at_once`,
  `a_locked_after_closed_is_left_to_the_owner`. World,
  `tests/shared.rs`, both worlds over `world::limits()`, each under
  0.5 s: `a_slow_owner_lock_fails_the_write_as_busy`,
  `a_lock_answered_after_an_abort_is_unlocked_before_closed`,
  `another_process_signing_out_leaves_no_record`,
  `another_subject_under_the_lock_drops_the_account`. `tests/memory.rs`'s
  `every_account_holds_a_maximal_record_within_the_heap_bound` sets
  every limit at once, each account holding a shared read.
  Fuzzy, `tests/fuzzy_shared.rs`, a new file as `tests/fuzzy_private.rs`
  is, over `world::limits()`: the seeded generator draws each lock's
  answer (busy, slow, newer, none, another subject, after a cancel, a
  removal or an abort), and the world's referee holds one `Unlock` per
  `Held` before `Closed`, none after it, one terminal per request,
  `Closed` once and last. It runs as `heavy cargo nextest run
  --profile fuzzy -p skein-oauth-accounts-world -E
  'binary(=fuzzy_shared)'`: 64 seeds, the loop's bound, about 5 s of
  the profile's 60. The file opens as `tests/fuzzy_world.rs` does,
  with `Rng`, `shared::{self, Story}` (`Copy` and `Debug`) and
  `skein_world::Memory`, and no `HEAP`: the run is `Unchecked`.
  Reference:
  ```rust
  #[test]
  fn seeded_lock_answers_settle_with_one_unlock_each_and_replay() {
      let stories = [
          Story::Busy,
          Story::Slow,
          Story::Newer,
          Story::NoRecord,
          Story::Subject,
          Story::Cancelled,
          Story::Removed,
          Story::Aborted,
      ];
      for seed in 0..64_u64 {
          let story = stories[usize::try_from(Rng::new(seed).below(8)).expect("story index")];
          let first = shared::run(seed, story, Memory::Unchecked);
          let second = shared::run(seed, story, Memory::Unchecked);
          assert_eq!(first.trace, second.trace, "{story:?} at seed {seed}");
          assert_eq!(shared::facts(&first.procs), shared::facts(&second.procs));
      }
  }
  ```
Proof: fuzzy,
  `seeded_lock_answers_settle_with_one_unlock_each_and_replay`: over 64
  seeds of lock answers each `Held` gets one `Unlock` before `Closed`,
  none after it, and the run replays; within the fuzzy profile's 60 s.
Don't: no wall clock for the lock wait; no `Unlock` for a `Busy` answer;
  no event after `Closed`.
Size: about 260 lines.
Removes: nothing.

### 10.2 OpenID through the driver
Scope: oauth.md 6.2's origins; 2.1 and 2.2's keys and ID token by GET;
  6.3's expected subject, "Another subject", and `signed in`'s subject.
Needs: skein 07's GET requests, key set fetch and `Unverified` [07.1],
  and its ID token check [07.2]; skein 06.2's ID token, key set and
  metadata for the worlds.
Files: `crates/skein-oauth-accounts/src/`: `accounts.rs`,
  `component.rs`, `component/sign_in.rs`, `component/lock.rs`,
  `component/accounts.rs` (`admit`), `exchange.rs`, `boundary.rs`,
  `lib.rs` (it exports `Origin`), `limits.rs`, `tests.rs`,
  `shared_tests.rs`, `private_tests.rs`; `tests/oauth-accounts/`:
  `src/sign_in.rs`, `src/sign_in/issuer.rs`, `src/sign_in/cuts.rs`,
  `src/world.rs`, `tests/sign_in.rs`, `tests/memory.rs` (its
  `every_listener_slot_keeps_*` test passes 100 lines with `origins`
  and the wrapped `Request::SignIn { account, subject: None }`, so
  its loop that builds the accounts moves into a helper).
Shape: `Account::SignIn` gains `origins: List<Origin>`; `Origin {
  uri, endpoint }` is new, and `accounts.rs` gains `use
  skein_lib::List;`. Each origin the terms name is `uri`, as
  `scheme://host[:port]`, with its `Endpoint`; `Source::SignIn` keeps
  them. `Unusable::Origin` now means a terms endpoint on none of the
  token endpoint's origin and `origins`, which `admit` checks. The
  exchange's web socket sends skein 07's GET requests, for the
  metadata and key set, on their origin's endpoint; a key set's URL on
  an origin not given ends the sign-in as skein 07's `Unverified`, and
  no name is resolved.
  `Request::SignIn` gains `subject: Option<Box<[u8]>>`, the subject it
  expects; a record of another keeps nothing and fails `Failed { ends:
  SignIn, failure: AnotherSubject }`, also under a shared lock (10.1).
  `Event::SignedIn` gains `subject`, the record's. `worst_case` counts
  each account's origins.
  Downstream: `origins` breaks 09.3's `Account::SignIn` literals, an
  empty `List`; `Request::SignIn`'s field, smith-local-service's
  `credentials.rs` (`subject: None`), its only builder; `SignedIn`'s,
  jig's `translate::up` arm again. An ID token failure is unreachable
  (decision 16).
Test shape: unit, `src/tests.rs`, over its helpers, each under 20 ms:
  `a_key_set_on_an_origin_not_given_fails_as_unverified`,
  `the_metadata_and_key_set_go_as_get_on_their_origin`,
  `a_sign_in_of_another_subject_keeps_nothing` (rows: owner, private,
  the private row in `src/private_tests.rs` over its `Owner`),
  `signed_in_names_the_records_subject`. In `src/shared_tests.rs`, over
  09.3's helpers: `a_shared_sign_in_of_another_subject_keeps_nothing`
  (rows: lock held and waiting; answered `locked`, one `Unlock`; wait
  passed, failed as busy and a later `locked` unlocked at once;
  `locked` after `closed`, no `Unlock` and no event: the owner's, by
  design questions 15 and 16). World, `tests/sign_in.rs`, both worlds
  over `world::limits()`, each under 0.5 s:
  `signing_in_again_fetches_the_issuers_keys_before_each_visit`,
  `a_new_registration_signs_in_with_its_issued_client_id`,
  `a_sign_in_of_another_subject_fails_and_keeps_nothing`.
  `tests/memory.rs`'s
  `every_listener_slot_keeps_a_bounded_partial_head_within_the_heap_bound`
  takes the origins and the key set at every limit.
Proof: world,
  `signing_in_again_fetches_the_issuers_keys_before_each_visit`: a
  sign-in sends its GETs for the metadata and key set on their origins
  before the visit; under 0.5 s; world,
  `a_sign_in_of_another_subject_fails_and_keeps_nothing`: the sign-in
  expecting one subject that returns another ends `Failed` as another
  subject, nothing kept; under 0.5 s.
Don't: no name resolved while running; no ID token, nonce or subject
  in a log, a `Debug` or a test's output; no keys kept past their
  sign-in.
Size: about 260 lines.
Removes: `Unusable::Origin`'s rule of the token endpoint's origin
  alone; 07.1's GET ended as `Exchange(Malformed)` (decision 20).

## Interface sketches

```rust
// skein_oauth_accounts::boundary; added variants and fields only.
pub enum Request {
    /// Starts a sign-in, naming the subject it expects (10.2).
    SignIn { account: u32, subject: Option<Box<[u8]>> },
}
pub enum Event {
    /// Ends `SignIn`, naming the required scopes not granted and the record's subject (09.2, 10.2).
    SignedIn { account: u32, generation: u64, missing: Box<[u8]>, subject: Option<Box<[u8]>> },
}
pub enum Failure {
    /// A sign-in or a shared record of another subject than expected (10.1, 10.2).
    AnotherSubject,
}
// skein_oauth_accounts::accounts (10.2)
pub enum Account {
    SignIn { registration: Registration, endpoint: Endpoint, keeper: Keeper, shared: bool, origins: List<Origin> },
}
/// An origin the registration's terms name, with the owner's resolved address and transport.
#[derive(Debug)]
pub struct Origin { pub uri: Box<[u8]>, pub endpoint: Endpoint }
```

## Questions to raise when reached

- 10.2: skein 07's GET requests, if they reach the driver otherwise than
  07.1 says.

## Done when

In skein's checkout of `origin/main`:

```
rg -c -w 'AnotherSubject' crates/skein-oauth-accounts/src/boundary.rs
rg -c -U 'SignIn \{\s*account: u32,\s*subject: Option<Box<\[u8\]>>' crates/skein-oauth-accounts/src/boundary.rs
rg -c -w 'origins' crates/skein-oauth-accounts/src/accounts.rs
```

each print a count of at least 1. The long-file command of
code-organization.md 8 prints no file past 100 lines of growth, and no
new file past 800 lines.
