# Session 11: the closing runs

2026-10-16. The plan's verdict is the newest `parity` run on its final
`main` (benchmarks.md 16.5). The plan holds two closing runs (the user,
decisions.md 12.3): the first after the four changes to what the model
is sent, and a second only after a change that answers the first's
miss. When the last it holds is short of parity, the plan ends with its
aim unmet. Each step lands a committed summary and the baselines cut
from it; none changes code.

## Read first

- smith `docs/design/benchmarks.md`, sections 9, 16.1 to 16.5.
- `decisions.md`, sections 3, 10 and 12.3; `findings/rt-power.md`.
- The README's rules 9 to 13.

## What the design asks

- **A smith arm is run once to a verdict of 0 or 1;** a verdict of 2
  measured nothing whole, and its summary is not committed (16.5).
- **A failed verdict is answered by a change, never by a run:** a later
  run is of a later commit, by a change whose commit body names the
  criteria it addresses, their cause read from the failed run's
  attempts against Codex's in it, and the change's own proof (16.5).
- **A plan bounds its runs:** when its last closing run is short of
  parity, the plan ends with its target unmet, its "Done when" naming
  the criteria that failed (16.5).
- **Baselines are cut from a plan's last `parity` run,** whatever its
  verdict (16.5).

## Choices this plan makes

- 11.1 cuts the baselines on a verdict of 0 or 1, and 11.2, when it
  runs, replaces them: whichever runs last leaves them cut from the
  plan's last `parity` run.
- On 11.1's miss, the supervisor adds the step that answers it,
  `smith-11.s1`, a change that names the failed criteria's cause from
  11.1's attempts (16.5). A change to what the model is sent there runs
  its full `parity-change` first, and carries `Change-Summary:`
  (README, rules 11 to 13). If `smith-11.s1` is dropped, 11.2 is
  dropped with it, and 11.1's run is the plan's last.
- The baselines' commands, each arm's configuration digest from the
  summary:

  ```
  s=<its summary>; for a in smith codex; do d=$(jq -r --arg a "$a" '[.groups[] | select(.agent == $a) | .configuration_sha256] | unique | .[0]' "$s"); heavy cargo run -q -p smith-bench -- baseline "$s" "parity-$a" --group "$a" "$d" gpt-6.1-sol high; done
  ```

## Rules that bind it

- **Live runs** (README, rule 9), **the live record** (rule 10).
- **One run per commit:** no closing run runs again on a commit a
  `parity` run measured.

## Increments

### 11.1 The first closing `parity` run
Scope: benchmarks.md 16.1 to 16.5 on the plan's `main` after its four
  changes.
Needs: smith 10's ranged reads [10.1] landed or dropped, and every step
  in the ledger but 11.1 and 11.2 landed or dropped.
Files: `benchmarks/summaries/parity-*.json`, `benchmarks/baselines/`.
Shape: no code. 06.1's commands 1 to 3 on `origin/main`. On an exit of
  0 or 1, commit the summary, cut both baselines with the commands
  above, and land them together. On 0, the plan's aim holds, and 11.2
  is dropped: the coordinator runs `plan set smith-11.2 state=dropped
  note="dropped: 11.1 on par"`. On 1, the supervisor adds `smith-11.s1`
  (above). On 2, nothing is committed, and the step stops on a question.
Test shape: none: no code; the run is the proof.
Proof: live, once: 68 attempts, about $17 and 6 hours; the same lines in
  the commit body as 06.1's for its `parity` run, and the README's "Done
  when" item 2's command, run once, with its exit.
Don't: run again on the same commit, whatever the verdict.
Size: none.
Removes: nothing.

### 11.2 The second closing `parity` run
Scope: as 11.1, on `main` after `smith-11.s1`.
Needs: 11.1's summary with a verdict of 1, and `smith-11.s1` landed.
Files: `benchmarks/summaries/parity-*.json`, `benchmarks/baselines/`.
Shape: no code. 06.1's commands 1 to 3 on `origin/main`; on an exit of
  0 or 1, commit the summary, cut both baselines again with the
  commands above, and land them together. It is the plan's last
  `parity` run: on 1, the plan ends with its aim unmet, and the README's
  "Done when" item 2 fails, naming the criteria (benchmarks.md 16.5).
  On 2, nothing is committed, and the step stops on a question.
Test shape: none: no code; the run is the proof.
Proof: live, once: 68 attempts, about $17 and 6 hours; as 11.1's.
Don't: run if 11.1's verdict was 0.
Size: none.
Removes: 11.1's baselines, replaced.

## Questions to raise when reached

- None known.

## Done when

- `cd ~/src/rust/smith && git log -1 --format='%(trailers:key=Step,valueonly)' -- benchmarks/baselines/parity-smith.json benchmarks/baselines/parity-codex.json` prints `smith-11.1` or `smith-11.2`.
