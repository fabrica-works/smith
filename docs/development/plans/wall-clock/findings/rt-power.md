# RT's power at five, ten and fifteen attempts per arm

2026-10-16. The designer's own check, for decisions.md, 3. RT is the upper bound of the 95%
percentile bootstrap of the ratio of the arms' median task walls on `shell-cwd`, against a
margin of 1.20 (benchmarks.md, 16.2).

## The spread of `shell-cwd`'s task wall

From the three committed `parity` summaries (`benchmarks/summaries/parity-*.json`), seconds,
sorted, with the standard deviation of the log of each arm's walls:

| Run | smith | sd log | Codex | sd log |
|---|---|---|---|---|
| baseline `744e0820` | 494, 665, 703, 721, 836 | 0.194 | 536, 541, 543, 624, 632 | 0.083 |
| first closing `b7b78fb1` | 719, 731, 805, 849, 932 | 0.108 | 537, 569, 696, 713, 802 | 0.167 |
| last closing `87a8d1e8` | 590, 618, 654, 709, 818 | 0.129 | 393, 466, 493, 523, 634 | 0.174 |

About 0.15 for both arms.

## The simulation

Each arm's walls drawn log-normal with a standard deviation of 0.15 on the log scale, smith's
median a given ratio of Codex's; RT computed as the verdict computes it, with 400 bootstrap
resamples; 250 simulated runs a cell (1,000 bootstrap draws and 1,000 runs would narrow each
share by a few points). Pure Python, seed 7; the script was in the designer's scratch directory
and is not kept: it is these two paragraphs.

Share of runs in which RT holds, by smith's true ratio to Codex:

| Attempts per arm | 0.85 | 0.90 | 0.95 | 1.00 | 1.05 | 1.10 | 1.20 |
|---|---|---|---|---|---|---|---|
| 5 | 0.82 | 0.60 | 0.46 | 0.31 | 0.16 | 0.09 | 0.00 |
| 8 | 0.96 | 0.92 | 0.65 | 0.52 | 0.28 | | |
| 10 | 0.98 | 0.96 | 0.84 | 0.60 | 0.43 | 0.20 | 0.01 |
| 15 | 1.00 | 0.98 | 0.95 | 0.72 | 0.49 | 0.24 | 0.03 |
| 20 | 1.00 | 1.00 | 0.98 | 0.86 | 0.60 | | |

- At five, a smith as fast as Codex passes about one run in three, and one 10% faster about
  three in five: the user's "about 10% faster to pass at five".
- At the margin (1.20), the share stays at or below 0.03 at every size: more attempts raise the
  power without loosening the margin's promise.
- Each `shell-cwd` attempt took 393-932 s in these runs; five more per arm add about 1.7 hours
  to a `parity` run, in the gate lane where the parity run's live suites ran.
