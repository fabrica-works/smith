# The plan's commands, run once at the base

2026-10-19. smith at `c241a7a4` (its code as at `origin/main` `137a1cb4`),
skein at `50f7436` (its fake LLM's files as at `origin/main` `49725ec`),
each in a scratch worktree under `~/src/rust/tmp/plan/live-text-scratch/`.
Each command run once, through `heavy` where it compiles or tests, nothing
live. Test builds unoptimized, as the gate builds them.

## 1. smith's base times

```
$ heavy --gate -- sh -c 'export NEXTEST_DOUBLE_SPAWN=0; cargo nextest run --workspace --no-run >/dev/null 2>&1; cargo nextest run --workspace --profile fuzzy --no-run >/dev/null 2>&1; for i in 1 2 3; do cargo nextest run --workspace 2>&1 | grep -E "^ +Summary"; cargo nextest run --workspace --profile fuzzy 2>&1 | grep -E "^ +Summary"; done'
     Summary [  11.404s] 2356 tests run: 2356 passed, 0 skipped
     Summary [  11.148s] 43 tests run: 43 passed, 0 skipped
     Summary [  10.062s] 2356 tests run: 2356 passed, 0 skipped
     Summary [  11.429s] 43 tests run: 43 passed, 0 skipped
     Summary [  11.282s] 2356 tests run: 2356 passed, 0 skipped
     Summary [  12.762s] 43 tests run: 43 passed, 0 skipped
heavy: task 28 (gate) exit 0, waited 0s, ran 206s, log ~/src/rust/tmp/logs/20261019T064352-28.log
```

Focused median 11.28 s of the 15 s budget; fuzzy median 11.43 s of 60 s.

## 2. smith's focused times by package

One more focused run, each test's time printed (`--status-level pass
--final-status-level none`), summed by package
(`~/src/rust/tmp/logs/20261019T065349-28.log`, 11.111 s in all). The
packages this plan adds to:

| Package | Tests | Sum of test times |
|---|---:|---:|
| smith-protocol-channel | 33 | 0.065 s |
| smith-host-protocol | 14 | 0.033 s |
| smith-host-domain | 15 | 0.033 s |
| smith-channel | 3 | 0.011 s |
| smith-codecs-world | 67 | 0.616 s |
| smith-channel-world | 49 | 0.283 s |
| smith-agent-service | 143 | 2.334 s |
| smith-agent-process-world | 29 | 2.121 s |
| smith-protocol-llm | 35 | 0.402 s |
| smith-protocol-llm-world | 135 | 4.272 s |
| smith-hosts-world | 12 | 0.190 s |

The sum is CPU over a parallel run, not the wall time; the README's Pulse
aims are read from it. smith-local-process-world, which this plan does not
change, takes 5.266 s of it.

## 3. skein's base times

```
$ heavy --gate -- sh -c 'export NEXTEST_DOUBLE_SPAWN=0; cargo nextest run --workspace --no-run >/dev/null 2>&1; cargo nextest run --workspace --profile fuzzy --no-run >/dev/null 2>&1; for i in 1 2 3; do echo "== focused $i"; cargo nextest run --workspace --status-level pass --final-status-level none 2>&1; echo "== fuzzy $i"; cargo nextest run --workspace --profile fuzzy --status-level pass --final-status-level none 2>&1; done'
== focused 1
     Summary [   6.612s] 2859 tests run: 2859 passed, 0 skipped
== fuzzy 1
     Summary [  31.600s] 104 tests run: 104 passed, 0 skipped
== focused 2
     Summary [   6.812s] 2859 tests run: 2859 passed, 0 skipped
== fuzzy 2
     Summary [  30.756s] 104 tests run: 104 passed, 0 skipped
== focused 3
     Summary [   6.449s] 2859 tests run: 2859 passed, 0 skipped
== fuzzy 3
     Summary [  30.292s] 104 tests run: 104 passed, 0 skipped
```

The `PASS` lines between are left out here; they are in
`~/src/rust/tmp/logs/20261019T064728-28.log`.

Focused median 6.61 s, fuzzy median 30.76 s. Summed by package (the
`PASS` lines parsed with `sed -E 's/^ *PASS \[ *([0-9.]+)s\] \( *[0-9]+\/[0-9]+\) ([^ ]+) .*/\1 \2/'`,
since nextest drops the space before a four-digit index):

| Package | Tests | Focused sums, 3 runs | Fuzzy |
|---|---:|---|---|
| skein-fake-llm-domain | 16 | 0.030, 0.031, 0.027 s | none |
| skein-fake-llm-protocol | 2 | about 0.003 s | none |
| skein-fake-peers | 8 | about 0.07 s | none |
| skein-llm-connection-world | 58 | 0.958, 0.869, 0.904 s | 3 tests, about 1.3 s |
| skein-llm-world | 134 | 3.435, 3.972, 3.318 s | 7 tests, about 6.0 s |

## 4. The agent's memory at the base

```
$ cd ~/src/rust/tmp/plan/live-text-scratch/smith-base && heavy -- cargo nextest run -p smith-agent-shell -E 'test(product_values_pass_every_startup_check_and_fit_complete_memory)' --no-capture 2>&1 | grep PRODUCT_COMPLETE
PRODUCT_COMPLETE service=1721347653 startup_header=22966 shell=398848 complete=1721769467 memory=2214592512
```

Log `~/src/rust/tmp/logs/20261019T073941-28.log`. The margin is
2,214,592,512 less 1,721,769,467: 492,823,045 bytes. decisions.md 3.7
sets the floor at 490,000,000.

## 5. smith's fuzzy times by package

```
$ cd ~/src/rust/tmp/plan/live-text-scratch/smith-base && heavy --gate -- sh -c 'export NEXTEST_DOUBLE_SPAWN=0; cargo nextest run --workspace --profile fuzzy --no-run >/dev/null 2>&1; for i in 1 2 3; do echo "== fuzzy $i"; cargo nextest run --workspace --profile fuzzy --status-level pass --final-status-level none 2>&1; done'
     Summary [  11.042s] 43 tests run: 43 passed, 0 skipped
     Summary [  11.036s] 43 tests run: 43 passed, 0 skipped
     Summary [  11.025s] 43 tests run: 43 passed, 0 skipped
```

Log `~/src/rust/tmp/logs/20261019T074918-28.log`, the `PASS` lines left
out here. Summed over the eleven packages of smith's `aims` line, with
README "Done when" item 8's `sed` and `awk`: 17.205, 17.278 and 16.992 s,
median 17.205 s.

## 6. The channel's generated code at the pin

```
$ cd ~/src/rust/tmp/plan/live-text-scratch/smith-base && heavy -- sh -c 'rev=$(grep -A2 "name = \"skein-codegen\"" Cargo.lock | sed -n "s/.*#\([0-9a-f]\{7\}\).*/\1/p"); echo "rev=$rev"; cargo run -q --manifest-path ~/.cargo/git/checkouts/skein-336d5ae5eecfa0f9/$rev/Cargo.toml --target-dir target/codegen -p skein-codegen -- crates/smith-channel/schema/channel-v2.schema <scratch>/gen && cmp <scratch>/gen/v2.rs crates/smith-channel/src/generated/v2.rs && echo "v2.rs identical"'
rev=93052c6
v2.rs identical
```

Log `~/src/rust/tmp/logs/20261019T080254-28.log`. skein's codegen at the
pin writes the committed `v2.rs` byte for byte, so 01.1's `v3.rs` is its
output for `channel-v3.schema` and nothing else.

## 7. smith moved to skein's `main`, unchanged

The output cut to the lines that matter, the other `Updating` lines and
the test's panic report left out:

```
$ cd ~/src/rust/tmp/plan/live-text-scratch/smith-base && heavy -- sh -c 'cargo update -p skein-lib 2>&1 | tail -2; cargo nextest run -p smith-bench -E "test(a_replay_arm_runs_through_the_probe_command_and_prints_one_line)" --no-capture 2>&1 | tail -3; cargo nextest run --workspace --no-fail-fast 2>&1 | grep -E "^ +(FAIL|SIGSEGV|TIMEOUT)|Summary"; git checkout Cargo.lock'
    Updating skein-world v0.0.0 (https://github.com/forgetic/skein.git#93052c6b) -> #aba0c6f2
note: pass `--verbose` to see 16 unchanged dependencies behind latest
assertion `left == right` failed:  source repository is dirty; commit or isolate changes before building an arm
        FAIL [   0.180s] (  12/2356) smith-bench::probe_command a_replay_arm_runs_through_the_probe_command_and_prints_one_line
     Summary [  11.602s] 2356 tests run: 2355 passed, 1 failed, 0 skipped
```

Log `~/src/rust/tmp/logs/20261019T080805-28.log`. The one failure is
the uncommitted `Cargo.lock`: the test builds an arm from the tree, and
refuses a dirty one. On the clean base it passes:

```
$ cd ~/src/rust/tmp/plan/live-text-scratch/smith-base && heavy -- cargo nextest run -p smith-bench -E 'test(a_replay_arm_runs_through_the_probe_command_and_prints_one_line)'
        PASS [   0.436s] (1/1) smith-bench::probe_command a_replay_arm_runs_through_the_probe_command_and_prints_one_line
     Summary [   0.437s] 1 test run: 1 passed, 300 skipped
```

Log `~/src/rust/tmp/logs/20261019T080701-28.log`. skein's commits since
`aba0c6f` change its forge crates and `oauth.md` only, which smith does
not build on. The repin's own commit (02.1) is committed before the gate
runs, so the dirty tree does not arise there.

## 8. The interface sketches compile

The sketches of the three session files, in one scratch crate, each
module standing in for its crate, with `todo!()` bodies and stand-ins
for the types each crate has now:

```
$ cd <scratch>/sketch && heavy -- sh -c 'cargo clippy --all-targets -q -- -D warnings && echo clippy-ok'
clippy-ok
```

Log `~/src/rust/tmp/logs/20261019T083030-28.log`. A deliberate type
error in the same crate failed it (`…T083037-28.log`), so the check
compiles.

## 9. "Done when" at the base

Each session's and the README's commands, at the base: each finds what
the plan has not built yet, and nothing else.

```
$ s=~/src/rust/skein; d=testing/skein-fake-llm-domain/src; git -C $s show origin/main:$d/api.rs | grep -q 'Pieces {' || echo 'no Line::Pieces'; git -C $s show origin/main:$d/domain.rs | grep -q 'Release {' || echo 'no Event::Release'; git -C $s show origin/main:$d/domain.rs | grep -q 'Hold {' || echo 'no Request::Hold'; git -C $s cat-file -e origin/main:testing/skein-fake-llm-protocol/src/pieces.rs 2>/dev/null || echo 'no pieces.rs'; git -C $s show origin/main:testing/skein-fake-peers/src/llm.rs | grep -q 'pub fn release(' || echo 'no Peer::release'
no Line::Pieces
no Event::Release
no Request::Hold
no pieces.rs
no Peer::release
$ git -C ~/src/rust/smith ls-tree --name-only origin/main crates/smith-channel/schema/ crates/smith-agent-service/src/live.rs
crates/smith-channel/schema/channel-v2.schema
$ git -C ~/src/rust/smith grep -c 'live_interval' origin/main -- crates/smith-agent-service/src/profile/quantities.rs crates/smith-agent-service/src/live.rs
```

The README's items 2 to 7 print, in order: `no Line::Pieces`, `no
Event::Release`, `no Peer::release`; `channel-v2.schema` and `0`; `no ask
in the start`, `no OpenEvent::Live`; `no live_interval`, `no
lost_live_bytes` and 12 lines naming `Observation.live_text`; `smith's
skein 93052c6b… has no release`; `0`.
