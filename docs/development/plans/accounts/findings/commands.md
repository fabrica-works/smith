# The plan's commands at its base

Each command the plan prescribes was run once at skein 12305e5 and smith c0c89ddc (origin/main), on 2026-10-19, on x1, through `heavy` for every cargo command; identical package sets ran once.
Verdicts, 90 commands: 48 OK, 37 EXPECTED, 5 DEFECT (two defects: the `wc -l` check at four sites, and one `grep -L`).
Not run: `plan await coordinator live-text/...` (3, README 128-130; they register a wait) and `scripts/check.sh` (named in the Done when of skein 01-04). No Proof asks for `cargo doc` or a count (`rg -c`).

## DEFECT

```
$ wc -l testing/skein-fake-oauth/src/*.rs testing/skein-fake-peers/src/oauth.rs
  531 testing/skein-fake-oauth/src/lib.rs
  661 testing/skein-fake-peers/src/oauth.rs
 1192 total
$ wc -l crates/skein-oauth/src/*.rs
  820 crates/skein-oauth/src/machine.rs
 2785 total
```
DEFECT 1: `wc -l` over several files prints a `total` line, which is past 800 whenever the files sum past 800 (1192 and 2785 at base), so "prints none past 800" cannot hold even when every file does. Sites: skein/06-fake-issuer.md:377-378, 07-openid.md:325, 08-driver-accounts.md:353-355, 09-driver-shared.md:399-400 (08's and 09's files do not exist yet: `wc` prints "No such file" and `0 total` at base, and their sums will pass 800 the same way). Fix: end each with `| awk '$2 != "total" && $1 > 800'` (09's brace expansion also needs bash, not sh). machine.rs at 820 is the plan's known case, not a defect: 05.3 moves code out first (05:68-70).

```
$ grep -L 'Summary.* 0 failed' *.log
fuzzy-1.log  fuzzy-2.log  fuzzy-3.log  focused-1.log  focused-2.log  focused-3.log
  (every log of a clean run ends: Summary [   5.616s] 3071 tests run: 3071 passed, 1 skipped)
```
DEFECT 2: README.md:260 expects `grep -L 'Summary.* 0 failed' *.log` to print nothing, but nextest prints no `0 failed` when nothing failed (findings/base-runs.md 51-60 show the same Summary lines), so it lists all six logs of a clean run. Fix: `grep -lE 'Summary.*(failed|timed out)' *.log` prints nothing for a clean run (checked on the six logs, and it lists a log with `1 failed`); if a missing Summary must be caught, add `grep -L Summary *.log`.

## OK

```
$ heavy cargo check --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 39s   (smith; skein 1m 41s)
```
OK: README 138 (smith 01.3's repin); passes in both worktrees.

skein 01 (Done when; Proofs 01:174, 01:220):
```
$ heavy cargo nextest run -p skein-llm -p skein-llm-world -p skein-llm-connection -p skein-llm-connection-world -p skein-tls-world -p skein-fake-llm-protocol -p skein-fake-peers
     Summary [   3.464s] 420 tests run: 420 passed, 0 skipped
$ heavy cargo nextest run -p skein-llm -p skein-llm-world -p skein-llm-connection -p skein-llm-connection-world
     Summary [   3.406s] 329 tests run: 329 passed, 0 skipped
```
OK: both pass.

skein 02 and 03 (02:120, 03:258 and 03:149-225, 03 Done when):
```
$ heavy cargo nextest run -p skein-llm -p skein-llm-world
     Summary [   2.634s] 241 tests run: 241 passed, 0 skipped
$ heavy cargo nextest run -p skein-fake-llm-domain
     Summary [   0.014s] 16 tests run: 16 passed, 0 skipped
$ heavy cargo nextest run -p skein-llm-world -p skein-fake-llm-protocol -p skein-fake-peers -p skein-llm
     Summary [   1.887s] 255 tests run: 255 passed, 0 skipped
$ heavy cargo nextest run -p skein-fake-llm-domain -p skein-fake-llm-protocol -p skein-fake-peers -p skein-llm -p skein-llm-world
     Summary [   1.455s] 271 tests run: 271 passed, 0 skipped
```
OK: all four pass.

skein 04 (04:148-250, 04:305, 04:149-206, Done when, and 03:150):
```
$ heavy cargo nextest run -p skein-fake-llm-protocol
     Summary [   0.002s] 2 tests run: 2 passed, 0 skipped
$ heavy cargo nextest run -p skein-fake-peers -p skein-llm-world
     Summary [   1.360s] 146 tests run: 146 passed, 0 skipped
$ heavy cargo nextest run -p skein-llm-world
     Summary [   1.901s] 134 tests run: 134 passed, 0 skipped
$ heavy cargo nextest run -p skein-fake-llm-protocol -p skein-fake-peers -p skein-llm-world -p skein-llm-connection-world -p skein-tls-world
     Summary [   2.698s] 283 tests run: 283 passed, 0 skipped
```
OK: all four pass.

skein 05 (05:104, 05:177 = 07 Done when, 05:245, 05:325, Done when):
```
$ heavy cargo nextest run -p skein-tls
     Summary [   0.084s] 66 tests run: 66 passed, 0 skipped
$ heavy cargo nextest run -p skein-oauth -p skein-oauth-world -p skein-oauth-accounts -p skein-oauth-accounts-world -p skein-fake-peers
     Summary [   0.684s] 124 tests run: 124 passed, 0 skipped
$ heavy cargo nextest run -p skein-oauth -p skein-oauth-world -p skein-oauth-accounts -p skein-oauth-accounts-world
     Summary [   0.559s] 112 tests run: 112 passed, 0 skipped
$ heavy cargo nextest run -p skein-oauth -p skein-oauth-accounts
     Summary [   0.025s] 51 tests run: 51 passed, 0 skipped
$ heavy cargo nextest run -p skein-tls -p skein-oauth -p skein-oauth-world -p skein-oauth-accounts -p skein-oauth-accounts-world -p skein-fake-peers
     Summary [   0.390s] 190 tests run: 190 passed, 0 skipped
```
OK: all five pass.

skein 06, 08, 09 (06 Done when and 06:260; 08:128-190; 08 and 09 Done when):
```
$ heavy cargo nextest run -p skein-fake-oauth -p skein-oauth-world -p skein-oauth-accounts-world -p skein-fake-peers
     Summary [   0.268s] 73 tests run: 73 passed, 0 skipped
$ heavy cargo nextest run -p skein-fake-peers -p skein-oauth-world
     Summary [   0.110s] 20 tests run: 20 passed, 0 skipped
$ heavy cargo nextest run -p skein-oauth-accounts
     Summary [   0.016s] 24 tests run: 24 passed, 0 skipped
$ heavy cargo nextest run -p skein-oauth-accounts-world
     Summary [   0.219s] 53 tests run: 53 passed, 0 skipped
$ heavy cargo nextest run -p skein-oauth-accounts -p skein-oauth-accounts-world
     Summary [   0.257s] 77 tests run: 77 passed, 0 skipped
$ heavy cargo nextest run -p skein-oauth-accounts-world --profile fuzzy
     Summary [   0.463s] 3 tests run: 3 passed, 0 skipped
```
OK: all six pass (`-p skein-fake-oauth` alone is EXPECTED below).

smith 01 (01:178, 01:227, 01:286, 01:312, Done when):
```
$ heavy cargo nextest run -p smith-codecs-world -p smith-channel-world -p smith-protocol-channel -p smith-host-protocol -p smith-hosts-world
     Summary [   0.645s] 175 tests run: 175 passed, 0 skipped
$ heavy cargo nextest run -p smith-protocol-llm -p smith-agent-shell -p smith-agent-service -p smith-protocol-llm-world
     Summary [   3.787s] 340 tests run: 340 passed (1 slow), 0 skipped
$ heavy cargo nextest run -p smith-agent-world -p smith-protocol-llm-world -p smith-channel-world -p smith-local-world -p smith-protocol-channel -p smith-agent-service
     Summary [   1.856s] 561 tests run: 561 passed, 0 skipped
$ heavy cargo nextest run -p smith-agent-shell -p smith-local-shell
     Summary [   2.307s] 135 tests run: 135 passed, 0 skipped
$ heavy cargo nextest run -p smith-codecs-world -p smith-channel-world -p smith-agent-world -p smith-protocol-llm-world -p smith-local-world -p smith-hosts-world -p smith-agent-shell -p smith-local-shell
     Summary [   4.207s] 599 tests run: 599 passed (1 slow), 0 skipped
```
OK: all five pass.

smith 02 (02:138, 02:183, 02:232, 02:267, Done when):
```
$ heavy cargo nextest run -p smith-domain-session -p smith-domain -p smith-protocol-llm -p smith-protocol-llm-world -p smith-agent-world -p smith-run-world -p smith-inline-agent
     Summary [   2.727s] 526 tests run: 526 passed, 0 skipped
$ heavy cargo nextest run -p smith-domain -p smith-host-domain -p smith-channel-world -p smith-host-world -p smith-hosts-world -p smith-agent-world
     Summary [   1.256s] 334 tests run: 334 passed, 0 skipped
$ heavy cargo nextest run -p smith-host-protocol -p smith-host-domain -p smith-inline-agent -p smith-local-domain -p smith-local-protocol -p smith-local-shell -p smith-host-world
     Summary [   0.915s] 383 tests run: 383 passed, 0 skipped
$ heavy cargo nextest run -p smith-local-domain -p smith-local-protocol -p smith-events -p smith-local-world
     Summary [   0.276s] 249 tests run: 249 passed, 0 skipped
$ heavy cargo nextest run -p smith-domain-session -p smith-domain -p smith-agent-world -p smith-channel-world -p smith-host-world -p smith-hosts-world -p smith-local-world -p smith-local-protocol
     Summary [   1.033s] 598 tests run: 598 passed, 0 skipped
```
OK: all five pass.

smith 03 (03:138-326, Done when):
```
$ heavy cargo nextest run -p smith-codecs-world -p smith-agent-world -p smith-agent-shell -p smith-protocol-llm-world
     Summary [   3.866s] 339 tests run: 339 passed (1 slow), 0 skipped
$ heavy cargo nextest run -p smith --test end_to_end
     Summary [   2.877s] 52 tests run: 52 passed, 0 skipped
$ heavy cargo nextest run -p smith-codecs-world -p smith-session-world -p smith-domain-session -p smith-protocol-channel
     Summary [   0.521s] 348 tests run: 348 passed, 0 skipped
$ heavy cargo nextest run -p smith-session-world -p smith-agent-world -p smith-protocol-llm-world -p smith-channel-world
     Summary [   2.582s] 427 tests run: 427 passed, 0 skipped
$ heavy cargo nextest run -p smith-codecs-world -p smith-events -p smith-agent-service -p smith-agent-world
     Summary [   1.999s] 338 tests run: 338 passed, 0 skipped
$ heavy cargo nextest run -p smith-codecs-world -p smith-events -p smith-agent-world -p smith-local-protocol
     Summary [   0.534s] 268 tests run: 268 passed, 0 skipped
$ heavy cargo nextest run -p smith-codecs-world -p smith-session-world -p smith-agent-world -p smith-protocol-llm-world -p smith-channel-world -p smith-events -p smith-local-protocol
     Summary [   2.406s] 585 tests run: 585 passed, 0 skipped
```
OK: all seven pass (every package of every `-p` exists at base).

README Done when 4, the aims (machine shared with other sessions' builds):
```
$ heavy --gate -- sh -c 'export NEXTEST_DOUBLE_SPAWN=0; for i in 1 2 3; do cargo nextest run --workspace --status-level pass --final-status-level none > focused-$i.log 2>&1; cargo nextest run --workspace --profile fuzzy --status-level pass --final-status-level none > fuzzy-$i.log 2>&1; done'
heavy: task 45 (gate) exit 0, waited 0s, ran 148s   (skein; smith: task 50, ran 172s)
$ for s in focused fuzzy; do for i in 1 2 3; do sed -nE 's/^ *PASS \[ *([0-9.]+)s\] \( *[0-9]+\/[0-9]+\) ([^ ]+) .*/\2 \1/p' $s-$i.log | awk -v p="^($P)$" '{ sub(/::.*/, "", $1) } $1 ~ p { t += $2 } END { printf "%.2f\n", t }'; done | sort -n | sed -n 2p; done
16.83  19.73   (skein; aims 16 and 19)       71.03  96.90   (smith; aims 60 and 79)
```
OK: both run and print two times per repository. Note, not a defect: with other sessions building on x1 the base medians came out above the aims and above README's recorded 12.85, 15.19, 57.67, 75.09, so the aims check is meaningful only on a quiet machine.

```
$ git diff --numstat origin/main...HEAD -- 'crates/*.rs' 'testing/*.rs' 'benchmarks/*.rs' ':!*/generated/*' ':!*tests.rs' ':!*/tests/*'
$ git diff --numstat origin/main...HEAD -- '*.rs' ':!*/generated/*' | awk '{ n = 0; c = "wc -l < " $3 " 2>/dev/null"; c | getline n; close(c); if (n > 800) print $3, n, $1 - $2 }'
(no output, in skein and in smith)
$ grep -E '^global-timeout' .config/nextest.toml | head -2
global-timeout = "15s"
global-timeout = "60s"
$ plan status
44 of 44 steps landed; 0 running, 0 blocked, 0 ready, 0 to check, 0 waiting.
```
OK: code-organization.md 8's two size counts (empty diff), README 5 (both repositories print the two values), README 1 (it reports this machine's other plan, vacuously clean).

```
$ rg -n 'ring::|rustls::' crates/skein-oauth/src
$ rg -n 'generate' testing/skein-fake-oauth/src
$ rg -n 'fails before its check exists' tests/oauth
(each prints nothing, exit 1)
```
OK: three "finds nothing" commands that hold at base (05 and 07 share the first); they guard later increments.

## EXPECTED

```
$ heavy cargo nextest run -p skein-llm-world -E 'test(each_scheme_goes_where_its_route_takes_it) | test(exhausted_without_a_reset_carries_none_never_zero) | test(a_report_value_past_its_bound_is_empty) | test(a_call_without_a_credential_sends_no_credential_field)'
$ heavy cargo nextest run -p skein-llm-world -E 'test(anthropic_failures_are_classed_by_type_first) | test(a_key_and_a_token_send_byte_identical_bodies) | test(each_code_of_the_table_gives_its_class) | test(a_mid_stream_code_outside_the_table_is_unavailable)'
$ heavy cargo nextest run -p skein-llm-world -E 'test(each_scripted_class_reaches_the_client_as_scripted) | test(a_failure_after_text_ends_the_stream_as_its_route_fails)'
$ heavy cargo nextest run -p skein-llm-world -E 'test(both_schemes_on_one_endpoint_are_answered_by_index) | test(a_revoked_token_stays_accepted_until_a_line_names_it)'
     Summary [   0.001s] 0 tests run: 0 passed, 134 skipped
error: no tests to run          (heavy: exit 4, each of the four)
$ heavy cargo nextest run -p skein-fake-oauth
     Summary [   0.000s] 0 tests run: 0 passed, 0 skipped
error: no tests to run          (heavy: exit 4)
```
EXPECTED (5): the filters parse and match nothing because the tests are new: 01.2-01.5 (01:160, 214, 251, 292), 02.2-02.4 (02:151, 197, 245, 257), 03.2-03.3 (03:216, 253), 04.4 (04:286, 298); `skein-fake-oauth` has no test yet, and 06.1 creates its `src/tests.rs` (06:86-87).

```
$ cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.license != "MIT") | .name'     (skein: 71 of 71 names)
$ head -1 LICENSE                  (no such file, in skein and in smith)
$ cargo metadata --format-version 1 --no-deps | jq -e '[.packages[].license] | all(. == "MIT")'     (smith: false, exit 1)
```
EXPECTED (3): the licence is skein 01.1 and smith 01.1 (01:88-90, smith 01:98-106); every member's `license` is null at base.

```
$ rg -n 'OpenAi =>' crates/skein-llm/src/replay.rs
$ rg -n 'subscription_sharing_usage_limit_exceeded' crates/skein-llm/src/openai/common.rs
$ rg -n 'Fail \{' testing/skein-fake-llm-domain/src/api.rs
$ rg -n 'pub fn accept' testing/skein-fake-peers/src/llm.rs tests/llm/src/fake.rs
$ rg -n 'Credential \{ connection' testing/skein-fake-peers/src/llm.rs
$ rg -n 'RECORD_VERSION: u16 = 3' crates/skein-oauth/src
$ rg -n 'fn authorization_url' crates/skein-oauth/src/redirect.rs
$ rg -n 'pub fn granted' testing/skein-fake-oauth/src testing/skein-fake-peers/src
$ rg -n '^rustls|^skein-tls' testing/skein-fake-oauth/Cargo.toml
$ rg -n 'AwaitKeys' crates/skein-oauth/src/machine.rs
$ rg -n '^skein-tls' crates/skein-oauth/Cargo.toml
$ rg -n 'Remove \{ account|Want \{ account|Removed \{ account|Refresh,|Taken,' crates/skein-oauth-accounts/src/boundary.rs
$ rg -n 'pub fn add' crates/skein-oauth-accounts/src/component/accounts.rs
$ rg -n 'SignOut \{ account|Locked \{ account|SignedOut \{|Lock \{ account|Unlock \{ account|NotGranted|AnotherSubject' crates/skein-oauth-accounts/src/boundary.rs
$ rg -n 'lock_wait: Duration::from_secs\(1\)' crates/skein-oauth-accounts/src/tests.rs tests/oauth-accounts/src/world.rs tests/oauth-accounts/tests/memory.rs
(no match, exit 1; the redirect.rs and component/accounts.rs commands print "No such file", exit 2)
```
EXPECTED (15): each names what one increment creates: `Provider::OpenAi` 02.1 (02:93), the code row 02.4 (02:225), `Fail` 03.1 (03:102), `accept` and the `Credential` observation 04.4 and 04.2 (04:346, 177), `RECORD_VERSION` 05.2 (05:121), `redirect.rs` 05.3 (05:193), `granted` 06.3 (06:290), the fake's `rustls` and `skein-tls` 06.2 (06:197), `AwaitKeys` 07.1 (07:93), the skein-tls dependency 07.2 (07:209), the driver's variants 08.2-08.3 (08:159, 204) and `component/accounts.rs` 08.2 (08:142), the sign-out and lock variants 09.1-09.3 and 09.4 (09:85, 147, 177, 240), `lock_wait` 09.3 (09:188).

```
$ rg -n '\.access_token|access_token:|Cap::AccessToken' --type rust crates/skein-llm ... testing/skein-fake-peers     (51 matches, 24 files)
$ rg -n 'server_error' testing/skein-fake-llm-protocol/src     (provider.rs:807)
$ rg -n 'struct Secret|envelope\.put|send_rejected|send_exhausted|account >= ' crates/smith-protocol-channel ...     (8 matches)
$ rg -n 'Fault::Exhausted|Request::(Rejected|Exhausted)|Up::(Rejected|Exhausted)' crates tests     (89 matches)
$ rg -n 'AccountFailure::Exhausted' crates tests     (6 matches)
$ rg -n 'is exhausted' crates/smith-local-protocol/src     (2 matches, terminal.rs:164, 437)
$ rg -n 'Refusal::(Endpoint|Dialect)|CredentialRejected|AccountExhausted|wait_ms' crates tests     (103 matches)
$ rg -n 'exhausted' crates/smith-events/src/words.rs     (2 matches, words.rs:118, 216)
$ rg -n 'endpoint\.account|options\.account' crates     (6 matches)
```
EXPECTED (9): "finds nothing" fails because base still has the code that these increments remove: skein 01.2 (01:181, `access_token`), 03.2 (03:231, `server_error`), smith 01.4 (`Secret`, the hand-packed envelope, `config.rs:453`; 01:292), 02.1, 02.3 and 02.4 (02:144, 238, 271: the faults, `AccountFailure::Exhausted` and the `is exhausted` words), 03.3 (03:247, `Refusal::{Endpoint, Dialect}`), 03.5 (03:331 and 312: `CredentialRejected`, `AccountExhausted`, `wait_ms`, words.rs), and 03.1 (03:100-108, `endpoint.account`; the one call site in smith-local-shell `resolve/local.rs:103` follows from `ConfiguredEndpoint.account` going).

```
$ ls crates/smith-channel/golden      v2
$ ls crates/smith-channel/schema      channel-v2.schema
$ ls crates/smith-charter/golden crates/smith-transcript/golden crates/smith-events/golden
v2 / v5 / v3, unsupported-v4.jsonl
```
EXPECTED (3): each moves up one version: channel to v4 in 01.2 from live text's v3 (01:118, awaited at README 128-130), charter v3 in 03.1, transcript v6 in 03.2, events v4 in 03.4 (03:90, 152, 258).

```
$ git -C ~/src/rust/skein log origin/main -E --all-match --grep='^Plan: accounts$' --grep='^Step: skein-0(1\.5|2\.1)$' -1 --format=%H
(no output)
$ heavy cargo update -p skein-llm --precise $c
(probe without --precise: heavy cargo update -p skein-llm --dry-run  ->  Updating skein-world ... #93052c6b -> #f1a62194 ... not updating lockfile due to dry run)
```
EXPECTED (2): `$c` is empty until skein 01.5 and 02.1 land with their `Plan:` and `Step:` trailers; the dry-run shows `skein-llm` resolves in smith's lockfile.
