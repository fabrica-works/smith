# Skein interface sketches under clippy

Sessions 01 to 09 of the accounts plan's skein sketches (`skein/01-credentials.md` to `09-driver-shared.md`) were applied cumulatively to skein `origin/main` 12305e519b43ef0f1b93e1b67c88f3ad131af59c in a scratch worktree on 2026-10-19, with `todo!()` bodies (the `todo` lint relaxed) and with callers fixed only where a sketch change broke them. After each session `heavy --tail 40 cargo clippy -p <changed crates> --all-targets -- -D warnings` ran. Each block shows the first run on the sketch as written (`# literal`) and the last lines of the clean run after the fixes named under it. Line numbers are those of the sketch files on 2026-10-19.

### 01 credentials
```
$ heavy --tail 40 cargo clippy -p skein-llm -p skein-llm-connection -p skein-fake-llm-protocol -p skein-fake-peers -p skein-llm-world -p skein-llm-connection-world -p skein-tls-world --all-targets -- -D warnings
# literal: error: use of a disallowed macro `core::default::Default` (types.rs:360:45)
# literal: error: this method could have a `#[must_use]` attribute (openai/common.rs:215:12)
    Checking skein-fake-llm-protocol v0.0.0 (.../sk-sketch/testing/skein-fake-llm-protocol)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.05s
heavy: task 46 (build) exit 0, waited 0s, ran 3s, log /home/free/src/rust/tmp/logs/20261019T092730-46.log
```
Verdict: compiles after three sketch fixes; the block's bodiless `;` fns (`measure` :369, `reset` :372) need `{ todo!() }`.
- 01:204 says `Report`'s `Default` is empty; `derive(Default)` is a `disallowed_macros` entry (programming-model 10.2). Fix: name an associated `Report::none()` instead of `Default`.
- 01:372 `RateLimit::reset` needs `#[must_use]` (`must_use_candidate`).
- 01:366 `Report` (and :364 `Credential`) carry no derives; `Report` needs `Clone, PartialEq, Eq, Hash, Debug` because `Event` derives them.

### 02 public route
```
$ heavy --tail 40 cargo clippy -p skein-llm -p skein-llm-connection -p skein-fake-llm-protocol -p skein-fake-peers -p skein-llm-world -p skein-llm-connection-world -p skein-tls-world --all-targets -- -D warnings
# literal: error: item in documentation is missing backticks (types.rs, "OpenAI's")
    Checking skein-llm-connection v0.0.0 (.../sk-sketch/crates/skein-llm-connection)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.67s
heavy: task 45 (build) exit 0, waited 0s, ran 2s, log /home/free/src/rust/tmp/logs/20261019T093756-45.log
```
Verdict: one doc lint.
- 02:277 `/// OpenAI's public Responses route` trips `doc_markdown` (two capitals). Fix: backtick the name, `` `OpenAI`'s ``.

### 03 fake failures
```
$ heavy --tail 40 cargo clippy -p skein-llm -p skein-llm-connection -p skein-fake-llm-domain -p skein-fake-llm-protocol -p skein-fake-peers -p skein-llm-world -p skein-llm-connection-world -p skein-tls-world --all-targets -- -D warnings
# literal: error: use of a disallowed macro `core::default::Default` (api.rs:308:45)
# literal: error: this method could have a `#[must_use]` attribute; error: function `encode_failed` is never used
    Checking skein-llm-connection-world v0.0.0 (.../sk-sketch/tests/llm-connection)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.24s
heavy: task 48 (build) exit 0, waited 88s, ran 15s, log /home/free/src/rust/tmp/logs/20261019T094435-48.log
```
Verdict: four sketch fixes.
- 03:361 `#[derive(Default)]` on the fake `Report` is `disallowed_macros` (the root `clippy.toml` covers `testing/*`). Fix: drop it and give `Report` an associated `none()`.
- 03:362-367 `Report`, `Line` and `Failure` have no derives; the fake domain's `Event`/`Request` need `Clone, PartialEq, Eq, Hash, Debug` on them (E0277, E0369).
- 03:373 `Domain::scripts` needs `#[must_use]`.
- 03:385 `encode_failed` trips `dead_code` (the `response` module is private); 03:321 must say `openai.rs` re-exports it.

### 04 fake credentials
```
$ heavy --tail 40 cargo clippy -p skein-llm -p skein-llm-connection -p skein-fake-llm-domain -p skein-fake-llm-protocol -p skein-fake-peers -p skein-llm-world -p skein-llm-connection-world -p skein-tls-world --all-targets -- -D warnings
# literal: error: docs for function returning `Result` missing `# Errors` section
# literal: error: this function has too many lines (102/100) (tests/llm/src/fake.rs, Exchange::take)
    Checking skein-fake-peers v0.0.0 (.../sk-sketch/testing/skein-fake-peers)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.59s
heavy: task 45 (build) exit 0, waited 1s, ran 4s, log /home/free/src/rust/tmp/logs/20261019T100016-45.log
```
Verdict: three sketch fixes and one omission.
- 04:355-356 `Exchange::accept` returns `Result` and needs an `# Errors` doc (`missing_errors_doc`).
- 04:184-185 "`tests/llm/src/fake.rs` takes `presented`" makes `Exchange::take` 102/100 lines. Fix: say `take` moves the head arm into a helper (`observe_head(headers, presented)`).
- 04:319-328 `credentials.rs` items must be re-exported by `provider.rs` (`MAX_CREDENTIALS`, `Presented`, `route`); the Files line at :162-163 does not say so and the block shows them only in `credentials.rs`.
- 04:265-266 name `prepared` and `prepared_with_options` but not `prepared_with_codex_echo` (tests/llm/src/fake.rs:375 in skein), which calls `prepared` and must also take `Option<Credential>`.

### 05 oauth
```
$ heavy --tail 40 cargo clippy -p skein-llm -p skein-llm-connection -p skein-fake-llm-domain -p skein-fake-llm-protocol -p skein-fake-peers -p skein-llm-world -p skein-llm-connection-world -p skein-tls-world -p skein-tls -p skein-oauth -p skein-oauth-accounts -p skein-oauth-world -p skein-oauth-accounts-world --all-targets -- -D warnings
# literal: error: this argument (272 byte) is passed by value (tests/oauth-accounts/src/world.rs:261, sign_in.rs:163)
# literal: error: this function has too many lines (106/100) (tests/oauth-accounts/tests/memory.rs:10)
    Checking skein-oauth-accounts-world v0.0.0 (.../sk-sketch/tests/oauth-accounts)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.82s
heavy: task 50 (build) exit 0, waited 20s, ran 3s, log /home/free/src/rust/tmp/logs/20261019T101418-50.log
```
Verdict: three findings; the `Limits`, `SavedToken` and `TokenResponse` literal breaks (E0063) downstream of 05.2 were fixed mechanically and are not sketch defects.
- 05:390 `verify_rs256(...) -> bool;` has no body. Fix: `{ todo!() }`.
- 05:103 `sig.to_vec()` is `disallowed_methods` (`slice::to_vec`) in the reference test. Fix: `let mut bad = sig.clone();`.
- 05:394 and :423 (four `Limits` fields, three `ClientLimits` fields) grow `accounts::Limits` to 272 bytes: `with_limits(.., limits: accounts::Limits, ..)` in `tests/oauth-accounts/src/{world,sign_in}.rs` must take `&accounts::Limits`. 05:132-136 lists those files but not the signature change, nor that `every_account_holds_a_maximal_record_within_the_heap_bound` (`tests/memory.rs`) passes 100 lines and needs a `handed_in_record` helper.

### 06 fake issuer
```
$ heavy --tail 40 cargo clippy -p skein-llm -p skein-llm-connection -p skein-fake-llm-domain -p skein-fake-llm-protocol -p skein-fake-peers -p skein-llm-world -p skein-llm-connection-world -p skein-tls-world -p skein-tls -p skein-oauth -p skein-oauth-accounts -p skein-oauth-world -p skein-oauth-accounts-world -p skein-fake-oauth --all-targets -- -D warnings
# literal: error: this method could have a `#[must_use]` attribute (testing/skein-fake-oauth/src/lib.rs:188, :190); 4 errors in all
# literal: error: this function has too many lines (101/100) (tests/oauth/tests/fuzzy_world.rs:56, scenario)
    Checking skein-oauth-accounts-world v0.0.0 (.../sk-sketch/tests/oauth-accounts)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.88s
heavy: task 45 (build) exit 0, waited 1s, ran 1s, log /home/free/src/rust/tmp/logs/20261019T101605-45.log
```
Verdict: three findings.
- 06:305 `Issuer::carried` and :307 `Issuer::granted` need `#[must_use]`.
- 06:311 "The fake's OpenID issuer" trips `doc_markdown`. Fix: `` `OpenID` ``.
- 06:77 lists `tests/fuzzy_world.rs`, whose `scenario` becomes 101/100 lines with the new `Config`/`Limits` fields; say it splits a `token_plan` helper.

### 07 openid
```
$ heavy --tail 40 cargo clippy -p skein-llm -p skein-llm-connection -p skein-fake-llm-domain -p skein-fake-llm-protocol -p skein-fake-peers -p skein-llm-world -p skein-llm-connection-world -p skein-tls-world -p skein-tls -p skein-oauth -p skein-oauth-accounts -p skein-oauth-world -p skein-oauth-accounts-world -p skein-fake-oauth --all-targets -- -D warnings
# literal: error[E0603]: function `check_claims` is private (crates/skein-oauth/src/machine.rs:267); `pick_key` likewise
    Checking skein-oauth v0.0.0 (.../sk-sketch/crates/skein-oauth)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.81s
heavy: task 45 (build) exit 0, waited 1s, ran 2s, log /home/free/src/rust/tmp/logs/20261019T101938-45.log
```
Verdict: two findings; the machine glue that called the sketched fns was my scaffolding, so its lint errors are not counted.
- 07:355-356 (`issuer_key_set`, `fitting_keys`) and :367, :376, :377 (`verify_id_token`, `check_claims`, `pick_key`) are private fns called across modules. Fix: `pub(crate) fn`.
- 07:268 the reference test's `claims` uses `format!`, a `disallowed_macros` entry. Fix: build it with `skein_lib::Writer` and `skein_lib::Decimal::of(exp)`; and name `use crate::jwt::{Expected, check_claims};` for the unit test.

### 08 driver accounts
```
$ heavy --tail 40 cargo clippy -p skein-llm -p skein-llm-connection -p skein-fake-llm-domain -p skein-fake-llm-protocol -p skein-fake-peers -p skein-llm-world -p skein-llm-connection-world -p skein-tls-world -p skein-tls -p skein-oauth -p skein-oauth-accounts -p skein-oauth-world -p skein-oauth-accounts-world -p skein-fake-oauth --all-targets -- -D warnings
    Checking skein-oauth-accounts v0.0.0 (.../sk-sketch/crates/skein-oauth-accounts)
    Checking skein-oauth-accounts-world v0.0.0 (.../sk-sketch/tests/oauth-accounts)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.34s
heavy: task 46 (build) exit 0, waited 2s, ran 7s, log /home/free/src/rust/tmp/logs/20261019T102311-46.log
```
Verdict: clean, apart from the bodiless `add` at 08:369-370 (needs `{ todo!() }`); the variant lines at :372-377 are not Rust and were applied as the comments say.
- 08:377 `Refusal::Unusable(Unusable)` needs `use crate::Unusable;` in `boundary.rs`; `Event::Removed` (:374) joins the panic arms of `private_tests.rs`, `tests/oauth-accounts/src/{world,sign_in}.rs`.

### 09 driver shared
```
$ heavy --tail 40 cargo clippy -p skein-llm -p skein-llm-connection -p skein-fake-llm-domain -p skein-fake-llm-protocol -p skein-fake-peers -p skein-llm-world -p skein-llm-connection-world -p skein-tls-world -p skein-tls -p skein-oauth -p skein-oauth-accounts -p skein-oauth-world -p skein-oauth-accounts-world -p skein-fake-oauth --all-targets -- -D warnings
# literal: error: item in documentation is missing backticks (boundary.rs:61, component.rs:74; 3 errors in all)
# literal: error: this function has too many lines (102/100) (tests/oauth-accounts/src/sign_in.rs:238, observe)
    Checking skein-fake-peers v0.0.0 (.../sk-sketch/testing/skein-fake-peers)
    Checking skein-oauth-accounts-world v0.0.0 (.../sk-sketch/tests/oauth-accounts)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.05s
heavy: task 51 (build) exit 0, waited 1s, ran 9s, log /home/free/src/rust/tmp/logs/20261019T103144-51.log
```
Verdict: two findings.
- 09:298 ("SignedOut, Failed or Refused"), :313 ("Ends SignOut"), :319 ("Ends SignIn"), :331 ("FileIo::descriptor") trip `doc_markdown`. Fix: backticks on each.
- 09:69 and :205 list `tests/oauth-accounts/src/sign_in.rs` without saying `observe` (102/100 after the `Removed`, `SignedOut`, `Lock`, `Unlock` panic arms) moves its Keep arm into a `keep` helper. Minor: :332 `Token` is `skein_lib::Token` in `component.rs` and `Box` needs `use alloc::boxed::Box;`.

### Cumulative and workspace
```
$ heavy --tail 40 cargo clippy -p <the 14 packages of session 09> --all-targets -- -D warnings
    Checking skein-fake-peers v0.0.0 (.../sk-sketch/testing/skein-fake-peers)
    Checking skein-fake-oauth v0.0.0 (.../sk-sketch/testing/skein-fake-oauth)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.36s
heavy: task 51 (build) exit 0, waited 1s, ran 8s, log /home/free/src/rust/tmp/logs/20261019T103202-51.log
```
`cargo check --workspace --all-targets --keep-going` is also clean after 09.

## Decision 15 against skein's limit checks
Values: `granted_bytes` 1024, `id_token_bytes` 8192, `modulus_bits` 2048, `lock_wait` 1 s, every other new limit the smallest valid value (0).
- Admitted. Scratch unit tests `machine_tests::decision_15_values_are_admitted` (skein-oauth) and `tests::decision_15_values_are_usable` (skein-oauth-accounts) pass with smith's host values plus these: `Client::new`, `client_worst_case`, `worst_case`, a terms-less `validate_registration`, `limits::usable`, `limits::worst_case` and `Component::new` accept them.
- Two checks the sketches only state in prose were written in scratch: `Client::new` refusing `modulus_bits` outside 2048..=8192 (2047 and 8193 are refused), and `usable` refusing `lock_wait` 0. A third, `validate_registration` refusing OpenID terms with `keys`, `key_set_bytes` or `metadata_bytes` 0 (without a `key_set` URL), is also prose-only.

## Not covered
- `10-driver-lock.md` and `11-fake-loopback.md`: outside the requested 01 to 09.
- 08's `Name`, `listen_address`, `with_port` and `Redirect.client_id` appear only in Shape prose and were not applied.
- Smith and jig downstream sites could not be checked in this repository.
- Reference tests in the prose compiled only where cheap (07.2's unit test); 07's world reference test (private fixtures in `tests/oauth/tests/world.rs`) was not.
