# Base runs

At skein `84e1d89` and smith `e79b9a6b`, on x1 (kernel 7.2.6), 2026-10-19,
each in a detached worktree at that commit.

## The gates

skein:

```
$ heavy --gate --tail 30 scripts/check.sh
     Summary [  23.506s] 38 tests run: 38 passed, 0 skipped
heavy: task 45 (gate) exit 0, waited 0s, ran 377s
```

smith:

```
$ heavy --gate --tail 30 scripts/check.sh
     Summary [  11.519s] 973/2356 tests run: 971 passed (2 slow), 2 failed, 0 skipped
        FAIL [   0.194s] ( 966/2356) smith-real-world::stories a_second_run_resumes_the_chat_from_its_files
        FAIL [   0.227s] ( 968/2356) smith-real-world::stories the_same_chat_runs_with_its_agent_in_process
heavy: task 45 (gate) exit 100, waited 1s, ran 324s
$ heavy --tail 15 cargo nextest run -p smith-real-world
     Summary [   0.255s] 5 tests run: 3 passed, 2 failed, 0 skipped
```

Both failures panic in skein-world's real loop (`testing/skein-world/src/real.rs`):
"io_uring is not usable here, so the real loop cannot run: io_uring could
not be set up: Cannot allocate memory (os error 12)". Kernel 7.2 charges
io_uring rings to locked memory, which was capped at 8 MiB (hetzner and
hel8 pass on kernel 6.12 with the same cap). The director raised the cap
to unlimited on x1's running processes, pueued included; the persistent
drop-in waits for the user, so a reboot of x1 resets it. Again, with the
cap raised:

```
$ heavy --gate --tail 30 scripts/check.sh
     Summary [   6.647s] 2356 tests run: 2356 passed, 0 skipped
     Summary [  11.327s] 43 tests run: 43 passed, 0 skipped
heavy: task 45 (gate) exit 0, waited 1s, ran 24s
```

## The suites' times, for the aims

At skein `4c0895d` and smith `e79b9a6b`, three runs of each suite in
each repository, the passing tests' lines kept:

```
$ heavy --gate --tail 8 -- sh -c 'export NEXTEST_DOUBLE_SPAWN=0; for i in 1 2 3; do cargo nextest run --workspace --status-level pass --final-status-level none 2>&1 | grep -E "^ +PASS|Summary" > skein-focused-$i.txt; cargo nextest run --workspace --profile fuzzy --status-level pass --final-status-level none 2>&1 | grep -E "^ +PASS|Summary" > skein-fuzzy-$i.txt; done'
skein-focused:  Summary [   3.113s] 2963 tests run: 2963 passed, 1 skipped
                Summary [   5.038s] 2963 tests run: 2963 passed, 1 skipped
                Summary [   4.733s] 2963 tests run: 2963 passed, 1 skipped
skein-fuzzy:    Summary [  29.743s] 104 tests run: 104 passed (3 slow), 0 skipped
                Summary [  30.915s] 104 tests run: 104 passed (4 slow), 0 skipped
                Summary [  33.377s] 104 tests run: 104 passed (5 slow), 0 skipped
smith-focused:  Summary [  10.023s] 2356 tests run: 2356 passed (1 slow), 0 skipped
                Summary [   9.359s] 2356 tests run: 2356 passed (1 slow), 0 skipped
                Summary [   8.898s] 2356 tests run: 2356 passed (1 slow), 0 skipped
smith-fuzzy:    Summary [  11.864s] 43 tests run: 43 passed, 0 skipped
                Summary [  15.017s] 43 tests run: 43 passed, 0 skipped
                Summary [  11.959s] 43 tests run: 43 passed, 0 skipped
heavy: task 45 (gate) exit 0, waited 1s, ran 72s
```

(smith's the same, in its worktree.) The tests' times of the packages
this plan's sessions change (README, Pulse), summed per run, in seconds,
with `P` each `aims` line's packages joined by `|`:

```
$ for s in focused fuzzy; do for i in 1 2 3; do sed -nE 's/^ *PASS \[ *([0-9.]+)s\] \( *[0-9]+\/[0-9]+\) ([^ ]+) .*/\2 \1/p' smith-$s-$i.txt | awk -v p="^($P)$" '{ sub(/::.*/, "", $1) } $1 ~ p { t += $2 } END { printf "%.2f\n", t }'; done | tr '\n' ' '; echo " $s"; done
62.49 57.67 54.39  focused
75.09 91.37 74.26  fuzzy
```

| Suite | Run 1 | Run 2 | Run 3 | Median |
|---|---|---|---|---|
| skein focused | 7.72 | 13.45 | 12.85 | 12.85 |
| skein fuzzy | 16.29 | 14.90 | 15.19 | 15.19 |
| smith focused | 62.49 | 57.67 | 54.39 | 57.67 |
| smith fuzzy | 75.09 | 91.37 | 74.26 | 75.09 |

skein's focused sum swung by 5.7 s between runs with the gate alone on
the machine: its packages' tests run beside the whole suite's, so the
sum moves with the parallel load. The median of three is what "Done
when" compares. `skein-fake-oauth` has no tests of its own at the base.
