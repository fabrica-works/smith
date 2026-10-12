# What this plan does not do

2026-10-09, revised 2026-10-12. Work that the design names but defers,
and when each item should be taken up. Nothing here is a session. Entry
IDs cite `findings/`.

| Item | Design | When |
|---|---|---|
| Messages offered mid-work, between completions, and threshold notes on the budget | domain/run.md, sections 6 and 9.2 | After `05-messages` and `07-budgets`. The `mid-work` probe comes with it. |
| A read-only parent working while one writing child holds the lease; then scoped write sets; then asynchronous children | domain/run.md, section 5.3 (the lease named); a later document (D7) | After contained trees, in a design document of its own |
| Contained trees: write scopes, read-only git directories, network policy, per-tree limits, build profiles | skein `docs/design/io.md`; smith `shell.md`, section 8; `protocol/agent.md`, section 3 (T9, T10) | A skein design and plan of their own. The `build-environment` probe comes with them. |
| The product's `isolate` option: the spawned agent as a containment boundary | `shell.md`, section 9; domain/host.md, section 12 | After contained trees |
| A background-process tool (start, poll, stop) | domain/tools.md, section 8 (T7) | After process groups and contained trees |
| A child starting from its parent's transcript prefix | domain/run.md, section 15 (D8) | When the `parallel-children` and `budget` probes show children re-orienting at a material cost |
| Provider-native compaction | domain/session.md, section 11 (C8) | When the `compaction` experiment favours it |
| Codex routing extras (turn state, `reasoning.context`) and WebSocket transport | skein `llm.md`, Later (C2, C4) | Only if the `affinity` experiment, which no plan runs yet, shows a material gain; WebSocket on skein's roadmap |
| Edit formats chosen per dialect | protocol/llm.md, section 12 (T4) | After the `edit-format` experiment, which no plan runs yet |
| Local `sub_agents` on by default | decisions.md, 5.8 (D9) | Reconsidered by `07-budgets`' last increment; the user decides |
| temper and jig adapting to smith's and skein's changed APIs (the typed agent face, the inline agent, removed modes), and their pins and Cargo URLs moving to GitHub | domain/host.md, section 11 | Separately, after this plan; this plan never edits them (decisions.md, 12.8) |
| Benchmarks: concurrent attempts, task kinds beyond code, a commit hook for smoke runs | benchmarks.md, section 15 | When the suites outgrow serial runs |
| Benchmarking against Claude Code, and every probe variant run on Anthropic (`cache-reuse` on `anthropic`, `large-write` and `slow-completion` on Anthropic models). smith's Anthropic support is built and tested deterministically in this plan; only its live evidence waits | benchmarks.md, sections 6, 7 and 10; the first run's `smith/11-claude.md` | The next pass, after this plan's session 12, as its own plan. It uses the user's `~/.claude` login the way Codex's is used. |
| `cache-reuse/idle-gap`: a gap past the cache's lifetime before a message | benchmarks.md, section 11.1 | With the Anthropic pass: only Anthropic lets a request set the cache's lifetime, and a scripted person's line has no delay yet |
| A notice for lines a crash lost | domain/host.md, section 8 | When the design names one (decisions.md, 12.19) |
| A probe's next task version, for each probe session 13 records as not calibrated | benchmarks.md, section 11.1 | After this plan, one version per probe, with the reason session 13 recorded (decisions.md, 12.4) |
| smith-bench's offline tier within "under a second" (95 tests take about 8 s serially) | benchmarks.md, sections 4.1 and 14 | When the harness's tests are next reworked; the suites' budgets hold meanwhile (decisions.md, 11.13) |
| A repository bundle kept durably, so that another machine can run `shell-cwd` (this plan keeps it in `$XDG_STATE_HOME/smith-bench/bundles/` on debian-16gb-hel1-1) | benchmarks.md, section 15, "Repository bundles" | When a second machine runs the repository suite; the designer chooses where, and the user if it needs a hosted service (decisions.md, 13.27) |
