# Findings: packaging and the standalone agent (theme P)

## Summary

`crates/smith` is one ordinary-Rust crate. It is the library for the agent shell, the local shell, JSON configuration, the `standard` limits profile, the chat store, the token store, the OAuth transport and the trace writer. It is also the only binary, and it links skein's test harness (`skein-world`) just for the `Host` trait.

The user's suspicion is right in substance. `smith local` is a supervising host (local domain + `smith-host-domain` + process adapter) that spawns `smith agent` (the same executable, via `current_exe`) once per activation and talks to it over a framed pipe channel. It is not a network server.

The spike did not change the packaging. It fixed defects of which about half exist only because of the spawned path: the 7 s teardown, the trace drain at agent exit, facts lost after the channel answered, and missing spawn and stderr diagnostics. It also made instrumentation worse. The benchmark had to regex-parse Rust `Debug` strings in a JSONL trace that only the spawned agent writes, and it could not use the in-process mode because that mode writes no trace at all.

The clean shape:
- `crates/` holds libraries only, with shell crates that mirror skein's echo layering.
- The root package's `./src` binaries are glue.
- The product is one single-process `smith` binary built on smith's own inline agent (the twin of `smith-host-domain` from the one-host draft), with a versioned event vocabulary and both headless and interactive front ends.
- The spawned path stays a tested library capability, for temper and for an optional isolated mode.
- Several shell mechanisms belong in skein first: the `Host` trait and loop driver, an append sink, durable file replace, the OAuth io driver and the private token store.

Refs: `main` is `30259d8`; the spike tip is `bf96e94`. Paths without a ref are at the tip.

---

## 1. Inventory of `crates/smith`

Sizes are lines at `main` / tip. "Worlds" are crates under `tests/`; "e2e" is `crates/smith/tests`.

| Module | Lines | Layer | What it is | Used by | Should move to |
|---|---:|---|---|---|---|
| `main.rs` | 35/35 | binary | positional dispatch: `agent CONFIG` and `local SETTINGS STATE [WS]` | binary | `./src/main.rs` (glue only) |
| `lib.rs` | 20/20 | — | module list | — | gone |
| `agent_shell.rs` | 249/258 | shell | `Agent`, a `skein_world::Host` over `smith_agent_service::Service`: drains trace facts, content and prompts, opens or adopts roots, prints start and loss diagnostics; `run` is the kernel loop | binary; e2e `agent_library.rs`. **Not used by worlds:** `tests/agent-process/src/process.rs` re-implements it | `smith-agent-shell`; the loop to skein-shell (P5) |
| `config.rs` | 539/692 | config + profile | serde agent document, endpoint validation, DNS resolution, TLS trust (`rustls-native-certs`), **and `standard_limits`** (`config.rs:321`) | binary; `local_shell`; e2e `completion_deadlines`, `receiving_limits`, `run_capacity`, `agent_library`; `tests/real/src/settled.rs:67` | the document to `smith-agent-shell`; the profile to `smith-agent-service` (P3); trust loading to skein |
| `limits.rs` | 103/106 | profile | domain `LIMITS` (spike: 64→256 turns, `call_timeout` 60→300 s) | `config.rs` | `smith-agent-service` (P3) |
| `local_shell.rs` | 222/235 | glue + profile | `smith local` composition: reads settings, opens roots, writes `STATE/agent.json`, resolves the token directory, **hard-codes `host_limits`** (`:190`), builds the launch from `current_exe`, runs the loop, prints diagnostics with `eprintln!` | binary only | `smith-local-shell::compose`; `host_limits` to `smith-local-service`; the loop to skein |
| `local_host.rs` | 493/498 | shell | `Local` host pass: routes the local service, terminal writer, chat `Store`, `Tokens`, OAuth `Auth` | binary; worlds `local-process` (`make_local`) and `real`; e2e `local_library`, `live` | `smith-local-shell` |
| `local_settings.rs` | 571/588 | config | serde settings, workspace merge, `policy`, charter encoding | `local_shell`; `tests/local-process` | `smith-local-shell` (`policy` possibly into `smith-local-protocol`, per hosts.md §5.2) |
| `local_store.rs` | 473/473 | store | chat files using **blocking `std::fs` + fsync on the loop thread** (`:8`, `:156`, `:254`) | `local_host`; worlds `local-process/src/world.rs:643`, `real/src/lib.rs:242`, `settled.rs:71` | step code in the local protocol layer over skein io `Create/Write/Sync/Rename` (P2) |
| `local_tokens.rs` | 164/164 | credentials | private 0600/0700 token record files | `local_host`, worlds, e2e, live | skein, beside `skein-oauth` (P2) |
| `local_auth.rs` | 541/541 | credentials | OAuth driver: loopback listener, refresh, grant lending | `local_host` | generic part to skein; grant envelope stays in `smith-local-protocol` |
| `local_auth_http.rs` | 438/438 | credentials | one token POST over `skein-http` | `local_auth` | skein |
| `local_auth_tests.rs` | 349/349 | tests | — | — | follows `local_auth` |
| `trace.rs` | 251/410 | trace | writer **thread** (`:73`), 32-record channel; records are `format!("{fact:?}")` / `format!("{usage:?}")` (`:111`, `:167`) | `agent_shell`, `agent_library` | schema to `smith-events` (P12, P13); sink to skein io (P2) |

E2e tests, at `main` / tip:
- `end_to_end.rs` (93/151, spawned only), `live.rs` (223/264), `live_configuration.rs` and `support/*` move to root-package `[[test]]` targets.
- `agent_library.rs` (191/197) moves to `smith-agent-shell`; `local_library.rs` (205) to `smith-local-shell`.
- The spike's `completion_deadlines.rs` (343), `receiving_limits.rs` (370) and `run_capacity.rs` (56) are profile tests and move to `smith-agent-service` or `tests/protocol-llm`.

Totals: src 4,448/4,807 lines; tests 1,531/2,451.

### Proposed crate map

This follows smith's naming and skein's `examples/echo/{domain,protocol,service,shell}`, where the service owns "its limits and the sum of their worst cases".

| Crate | Layer | Content |
|---|---|---|
| `smith-agent-service` (exists) | service | plus `standard()`, the profile as step code; profile tests |
| `smith-local-service` (exists) | service | plus the host and local profiles |
| `smith-inline-agent` / `smith-host-inline` (new) | domain | the one-host twin (P18) |
| `smith-events` (new) | codec | versioned event vocabulary, JSON via `skein-json`; goldens and drift in `tests/codecs` |
| `smith-local-protocol` (exists) | protocol | plus the chat store over skein io, the credential envelope, and maybe `policy` |
| `smith-agent-shell` (new) | shell | agent document; the one `Agent` adapter for both binary and worlds |
| `smith-local-shell` (new) | shell | settings, `compose`, `Local`, front ends, CLI argument types |
| root package `smith` | binaries | `src/main.rs`, optionally `src/bin/smith-agent.rs` |

Shell crates are ordinary Rust with echo-shell lints and their own `clippy.toml`.

**To skein first** (AGENTS.md; domain/README.md §4 already earmarks OAuth, supervised processes and fakes):
1. `Host` (today in `skein-world/src/host.rs`, a testing crate the binary links) plus a `drive` loop, into `skein-shell` (P5).
2. A ring-driven, bounded, drop-counting append sink (P2).
3. Durable whole-file replace and a private secret store on io's `Create/Write/Sync/Rename` (skein `kernel.rs` docs, lines 132-145).
4. The OAuth io driver that `skein-oauth` leaves to its caller (skein `oauth.md` intro); temper's forge sign-in needs it too.
5. Loading machine trust roots or a DER file (today `config.rs:288`).

---

## 2. The two agent paths

### P1. One crate fuses five layers, and the shipped binary links a test harness

- **Origin:** latent; the spike grew it (`config.rs` +153, `trace.rs` +159).
- **Symptom:** every profile change in the spike (`ee512a1`, `d75190b`, `cfb7408`, `5848e59`, `44e5120`) was tested through `smith::config::parse` (`completion_deadlines.rs:13`, `receiving_limits.rs:11`, `run_capacity.rs:6`). `crates/smith/Cargo.toml:9` makes `skein-world` a normal dependency, used only for `Host` (`agent_shell.rs:14`, `local_host.rs:12`).
- **Root cause:** accretion in the binary crate.
- **Assessment:** redo properly. The design already assigns configuration, transcripts and sign-in to `smith-local-protocol` (protocol/README.md §5). `smith-local-protocol/src/store.rs:1-6` says "The caller stores each `File` with Skein file `Store`"; the shell uses std IO instead.
- **Clean fix:** the crate map above. No testing crate in the binary.
- **Contracts:** domain/README.md §4 (row `binary | smith | the agent process a host spawns, and the local host with an agent`), protocol/README.md §5, testing.md §7. None edited by the spike.
- **Layer:** shell crates; skein-shell.
- **Tests:** move with their modules.
- **Benchmark:** none needed.

### P2. The shell does blocking and threaded IO that programming-model.md §2.1 rules out

- **Origin:** latent; extended by `a54ba26` and `136b9c4`.
- **Symptom:**
  - `trace.rs:73` spawns a thread.
  - `local_store.rs:156` and `:254` fsync on the loop thread for every turn.
  - `local_shell.rs:161-176` writes `agent.json` with std IO.
  - `crates/smith/Cargo.toml` allows all `disallowed_*` lints.
- **Root cause:** §2.1 says anything that can be a ring op is one, with no close-on-drop handles and one loop per process. skein's echo shell enforces this with its own `clippy.toml`; smith opted out wholesale.
- **Spike change:** a 50 ms bounded drain (`trace.rs:23`) that detaches a stalled thread, and exact-sized records.
- **Assessment:** a sound patch on a mechanism that shouldn't exist in this form. The fsync stall's effect on loop latency was not measured.
- **Clean fix:**
  - The sink is a protocol-layer writer over skein io output streams (the Room/Send discipline `local_host.rs` `writer_down` already uses for the terminal), on a file opened by the ring. Exact drain at exit through settlement; no grace timer.
  - The stores use the `Create/Write/Sync/Rename` idiom, with skein-sim crash cuts replacing `Store::crash_after` (`local_store.rs:205`).
  - If skein lacks file append output or a durable-replace helper, add it there first.
- **Contracts:** protocol/agent.md §5 (the spike's 50 ms and "no thread join" text becomes moot); protocol/hosts.md §5.3 and §5.4.
- **Layer:** skein io; `smith-local-protocol`; `smith-events`.
- **Tests:** sim with filesystem faults and cuts at each `Sync`/`Rename`; allocator test for the sink; no real-thread sleeps.
- **Benchmark:** none needed.

### P3. The `standard` profile is buried in the JSON module, so the benchmark needed a binary registry

- **Origin:** latent; the spike moved it five times (arguments 2→32 KiB, call timeout 60→300 s, request envelope 256 KiB→1 MiB, turns 64→128→256).
- **Symptom:**
  - The harness keeps `smith-binary-profiles.json` and `binary_profiles.py`, binding a manually reviewed `standard_max_turns` to each binary's SHA-256.
  - One 128-turn attempt was refused on a 64-turn binary (benchmark report, "Remaining gaps").
  - `/tmp/smith-benchmark-smith-research/findings.txt` reverse-engineers the caps from source.
- **Root cause:** compiled-in limits are observable only as a refusal.
- **Assessment:** keep the profile, move it. Whether the values are right is another theme.
- **Clean fix:**
  - The profile is a function of the service crates.
  - `smith profile --json` prints effective limits, schema versions (channel, charter, transcript, events) and build identity.
  - Admission errors name the bound and the configured value.
- **Contracts:** protocol/agent.md §4; local-host.md "Verification and later work".
- **Tests:** profile tests in the service crates; a golden of `smith profile --json`.
- **Benchmark:** the harness reads `smith profile --json` instead of keeping a digest registry.

### P4. Two adapters for one hosted agent: the shipped one isn't what the worlds run

- **Origin:** latent; the spike's spawn-path fixes landed only in the shipped adapter.
- **Symptom:** `tests/agent-process/src/process.rs:11-108` is a second `Host` over `Service`: stderr closed unused, facts kept in a `Vec`, no trace, no diagnostics, no `finish`. The local-process and hosts worlds spawn it (`tests/local-process/src/process.rs:667`). So `agent_shell`'s trace drain (`a54ba26`), loss accounting (`136b9c4`), root opening and diagnostics are tested only by `agent_library.rs` and the binary e2e.
- **Root cause:** `agent_shell::Agent` wasn't factored for worlds; `local_host::Local` was, and the worlds use it.
- **Assessment:** redo. Once the binary stops spawning (P10), this would leave the agent-process shell untested.
- **Clean fix:** `smith-agent-shell::Agent` (it already takes `Resources`) is the process the agent-process, hosts and local-process worlds spawn. Delete the test-only adapter. Make stderr an observable writer so the hosts referee can check the tail reported in `Diagnostic::Gone`.
- **Contracts:** testing.md §2.1.
- **Tests:** agent-process stories for exit with a full trace queue, refusal lines and loss counters.
- **Benchmark:** none needed.

### P5. The kernel loop is hand-written three times, and `Host` lives in a testing crate

- **Origin:** latent.
- **Symptom:** `agent_shell.rs:241-258`, `local_shell.rs:125-158` and skein `examples/echo/shell/src/main.rs:131-150` are the same loop. `skein_world::Host` (`skein-world/src/host.rs:10`) is implemented by production code. The spike's diagnostics drain inside `local_shell`'s loop (`ecb7562`) shows the copies drifting.
- **Clean fix:** `skein_shell::Host` (re-exported by skein-world) plus `skein_shell::drive` with a per-iteration hook for shell drains.
- **Contracts:** skein `shell.md` §6.
- **Layer:** skein.
- **Tests:** a skein-shell real-loop test of `drive`.
- **Benchmark:** none needed.

### P6. The in-process placement has no instrumentation, and the agent's `trace` setting is silently ignored

- **Origin:** latent. It explains why the benchmark used the spawned path (`findings.txt`: "In-process mode may have different timing").
- **Symptom:**
  - In-process, `smith-local-domain` owns `smith_domain::Domain` (`domain.rs:146`). Nothing pops its facts or content (`smith-domain/src/domain.rs:304`, `:309`); the only callers are the framed agent service (`smith-agent-service/src/service.rs:493`, `:1344`, `:1370`), checked with grep and the graph.
  - `local_shell.rs:102` passes `agent_config.service` and drops `agent_config.trace`.
  - `Effects::new` forces `capture_prompts = false` (`service.rs:181`).
  - The binary doesn't drain local facts either.
- **Root cause:** the trace was designed as a property of the agent process (protocol/agent.md §5), not of whoever owns the domain.
- **Spike change:** none; every trace fix went into the spawned path.
- **Assessment:** the largest blocker to making in-process the only mode. A parsed-then-ignored setting is also a bug on its own.
- **Clean fix:** the domain's owner drains every pass, independently of any channel (P13). With the inline agent, that's the `pop_fact(client)` / `pop_content(client)` jig's twin already has (`jig-inline-agent/src/domain.rs:94`, `:99`).
- **Contracts:** protocol/agent.md §5 becomes placement-neutral or moves to a new `protocol/events.md`; domain/host.md §9 "What stays" should list facts and traces.
- **Tests:** local-process in-process stories asserting complete usage and tool events, and exact loss counts under a stalled sink.
- **Benchmark:** probe "read README.md, report its title" with `--json`. Assert one `response.completed` per provider response, totals equal to the summary, zero loss.

### P7. In-process failures are invisible; the spike's diagnostics exist only on the spawned path

- **Origin:** `ecb7562`, `e69bfc3`, `8466afb` (spawn-only); the in-process half is latent.
- **Symptom:**
  - The benchmark diagnosed the line-decoder failure only once `Diagnostic::RunFailed` printed `Model(Completion { failure: Limit, evidence: Response })`.
  - All three `Diagnostic` variants come from `route_host` only (`smith-local-service/src/service.rs:704`, `:735`, `:753`).
  - In-process, `finish_answer` collapses `Model`, `Budget`, `Policy` and `Stale` into "Run failed" (`smith-local-domain/src/domain.rs:951-956`), and the process exits 0.
- **Assessment:** keep the principle (operator-only, never in the transcript or prompts, bounded). Redo the mechanism: typed failure first for both placements, the stderr tail only where a process exists, and rendering from the event vocabulary rather than `{failure:?}`.
- **Clean fix:** `run.completed {status:"failed", failure:{kind:"model", completion:"limit", evidence:"response"}}`; the terminal prints one human line from it. Process `Fault`/`End`/tail stays an extra event only in the spawned capability.
- **Contracts:** protocol/hosts.md §5.1 (the spike's paragraph, made placement-neutral).
- **Tests:** a local-process in-process `Limit` failure gives the typed event and a nonzero headless exit.
- **Benchmark:** covered by P14.

### P8. In-process stop semantics are weaker than the interrupt contract

- **Origin:** latent.
- **Symptom:**
  - protocol/hosts.md §5.1 says a second interrupt "stops its tree". In-process, `StopProcess` becomes another `Interrupt` (`smith-local-service/src/service.rs:536-541`), which in `Ending` only sets `chat.closed` (`smith-local-domain/src/domain.rs:709`).
  - Nothing drops a run that doesn't wind down, and there is no watchdog or kill.
  - A panic aborts the whole host.
- **Root cause:** domain/host.md §9 accepts losing containment but never defined the replacement. jig's twin did: cancel, then drop after `cancel_grace` (`jig-inline-agent/src/domain.rs:276-279`, `limits.rs:18`).
- **Assessment:** must be designed before in-process is the only mode. A drop is safe only if LLM bindings, process trees and file ops below it still settle.
- **Clean fix:**
  - Cancel, grace, drop. On drop, the root aborts or cancels every lower owner of the run and holds exit until they settle.
  - The second interrupt means "drop now". Wall time remains a bound.
  - Panics stay fail-stop: the store is durable and resume works. Say so in the contract.
- **Contracts:** domain/host.md §9; protocol/hosts.md §5.1.
- **Tests:** a never-answering provider plus two interrupts gives a drop within the grace with all owners settled; a fuzzy sweep with stalls.
- **Benchmark:** none needed.
- **Questions:** Q3.

### P9. Spawn-only defects in the spike, and one cliff the spike moved there

- **Defects that lived only (or mainly) on the spawned path:**
  - **7 s teardown per activation** (`a2c2e53`). Host grace 5 s plus kill 2 s (`local_shell.rs:216-217`) while idle LLM bindings waited out their 15 s keep (benchmark `shutdown-fix-notes.txt`). The root cause, the pool never closed (now `service.rs:888`), is shared with `Effects`. In-process the symptom would have been one delayed exit per invocation; that's reasoned, not measured.
  - **Trace lost at agent exit** (`a54ba26`). In-process there is no trace at all.
  - **Terminal facts lost after the channel answered** (`136b9c4`, `6a1dfb2`). Local capture was gated on channel room (`service.rs:1338-1376`) because one pop feeds both the channel and the trace. That coupling exists only on the framed path.
  - **Spawn, admission and stderr diagnostics** (`ecb7562`, `8466afb`) are process-only. The typed half (`e69bfc3`) also matters in-process (P7).
  - **First-task race** (`140c351`) is domain-level, but the spawned Admitted round trip widens it (`findings.txt`).
- **Latent cliff (not verified by test):** `d75190b` set `call_timeout` to 300 s, and the host's `no_progress` is also 300 s (`local_shell.rs:213`). Any channel record counts as progress (`smith-host-domain/src/domain.rs:848`). A non-streaming completion sends nothing between `CompletionStarted` and its terminal. So the agent's own deadline wins only by pipe and loop latency, and facts are best-effort (`6a1dfb2`). This is spawn-only.
- **Assessment:** keep the pool close (shared). The trace drain and channel gating are artefacts of the spawned trace design and disappear with P2, P6 and P13.
- **Clean fix:** derive `no_progress` from the agent profile (`call_timeout` + backoff + margin), or report LLM calls as long operations (domain/host.md §4: the watchdog "stretches to the span of a long operation"). The single-process product has no watchdog, only a wall bound.
- **Contracts:** domain/host.md §4; protocol/agent.md §6.
- **Tests:** hosts world: a provider answering at the timeout minus epsilon with the fact queue full must not stop the run.
- **Benchmark:** none needed.

### P10. Keeping the spawned path covered once the binary stops using it

- **Today:** spawned coverage comes from `tests/agent-process`, `tests/hosts`, `tests/local-process` (resume and replay are spawned-only), `tests/real` (3 of 4 stories), the binary e2e (all spawned) and the live suite (spawned only).
- **What would be lost:** the real-process tier — real spawn, pipes, signals, stderr tail, `current_exe`.
- **Clean fix:**
  1. Worlds host the shipped `smith-agent-shell::Agent` (P4).
  2. Placement becomes a parameter in local-process and real; resume and replay run on both placements.
  3. Optionally ship `src/bin/smith-agent.rs` (about 20 lines) and keep one e2e story spawning it. Otherwise the hosts world in the real loop takes that role (testing.md §8 open question).
- **Contracts:** testing.md §2.2 and §2.3; domain/README.md §4.
- **Tests:** the four binary controls took 0.322 s (`crates/smith/README.md`), so one more story fits the budget.
- **Benchmark:** none needed.
- **Questions:** Q2.

**In-process coverage today:**

| Tier | Coverage |
|---|---|
| `tests/local` domain world | in-process only (`world.rs:624`), broad |
| `tests/local-process` | equivalence, interrupt, commit, 4 auth stories, fuzzy, memory, 1 referee test; **no** resume or replay |
| `tests/real` | 1 of 4 stories |
| binary e2e | none |
| live | none |
| benchmark | none |

Also missing: P6, P7, P8, and settings cleanup (`in_process` goes; channel-only fields become profile-internal). There is dead work in-process too: `agent.json` and `current_exe` (`local_shell.rs:48-49`, `:63-64`). Directories are opened twice by design (delivery vs effect roots must not alias); keep that.

---

## 3. Root package layout

### P11. A root package for `./src` binaries

- **Today:** a virtual workspace; `tests/` holds 15 world crates and a `clippy.toml`; the root `clippy.toml` holds step-crate rules ("Applies to every crate below this directory that has no clippy.toml of its own").
- **Making the root a package** (`[package] name = "smith"` beside `[workspace]`):
  - `default-members` becomes the root package. The gate uses `--workspace` (workflow.md lines 15-18), but set `default-members` explicitly so bare commands keep today's meaning.
  - `cargo run -- …` and `cargo install --path .` work directly.
  - The name `smith` frees up once `crates/smith` is split. temper depends only on domain, codec and host crates (`temper/Cargo.toml:78-87`), so nothing outside breaks.
- **Auto-discovery:**
  - `src/main.rs` becomes bin `smith`; `src/bin/*.rs` become further bins; `tests/*.rs` and `tests/*/main.rs` become integration tests.
  - No world crate has a `tests/<name>/main.rs` (checked), and nested packages stay separate.
  - Still set `autotests = false` (and `autobenches`, `autoexamples`) and declare `[[test]]` with explicit paths, as skein's echo shell does (`path = "../../../tests/echo/tests/end_to_end.rs"`).
- **`CARGO_BIN_EXE_<name>`** exists only for the root package's own test targets, so e2e and live must be root `[[test]]` targets. Put them in `tests/end-to-end/`; a directory with no `main.rs` isn't auto-discovered. The nextest filters `binary(end_to_end)` and `binary(=live)` keep working.
- **Lints:**
  - The root package gets its own `[lints]` table (echo's ordinary-Rust one), not `workspace = true`.
  - Clippy finds `clippy.toml` by walking up from each manifest directory. Move the step-crate file to `crates/clippy.toml` and put a shell `clippy.toml` at the root with echo's bans (`File`, `OwnedFd`, `std::process`, `Instant`, `SystemTime`, `HashMap`, `thread::spawn`).
  - Shell library crates under `crates/` carry their own copy of the shell file; `tests/clippy.toml` is unchanged.
  - Keeping today's root file and allowing all `disallowed_*` lints loses exactly the bans P2 needs.
- **Glue size:** `main.rs` parses arguments into typed commands (parser and types live in `smith-local-shell`, so they get step tests), opens fds, signals and seed, then `compose` → `drive` → exit code. That stays about 40 to 60 lines.
- **Which binaries:**
  - `smith`, the product.
  - `smith-agent` only if a host needs a prebuilt one (Q2). temper's worker design spawns "an agent process" (`worker-domain.md:107`), but temper may prefer to embed the shell in its own binary.
  - No test-helper binaries: skein-world hosts fakes inside the loop.
- **Contracts:** testing.md §7; workflow.md commands still work if the package is named `smith`.
- **Questions:** Q1, Q2.

---

## 4. The standalone agent as a product

### P12. The trace is `Debug` strings, so every consumer reverse-engineered it

- **Origin:** latent; the spike's `136b9c4` changed records, not the format.
- **Symptom:**
  - `trace.rs:111` writes `format!("{fact:?}")`; `:167` writes `format!("{usage:?}")`.
  - The harness regexes Rust syntax, e.g. `Run \{ fact: Called \{ run: Token\((\d+)\), … ask: SubAgent` and `(\w+_tokens): (\d+)` (`smith_smoke.py`, `terminal.py`).
  - smith's own live referee does too: `fact.starts_with("Run { fact: Returned {") && fact.contains("result: Accepted")` (`support/live_run.rs:94-99`).
  - `findings.txt` lists what the harness had to work out:
    - `at_ns` is boot-relative.
    - Usage is duplicated (the `Used` fact and the `usage` record).
    - Tool records have no call id, so durations were paired FIFO.
    - The model came from the prompt's first line.
    - The prompt record isn't the request, so a recording proxy was needed for bytes, first-token latency and effort.
    - `Ended` facts could drop, so acceptance came from `Returned … Accepted`.
    - Loss counts were parsed from a stderr sentence.
- **Root cause:** the trace is a debugging aid; protocol/agent.md §5 defines capture levels, not a schema.
- **Assessment:** redo. Any rename of a domain type silently breaks the benchmark, the live suite and every user script.
- **Clean fix:** P13.
- **Contracts:** new `protocol/events.md`.
- **Tests:** codec goldens and drift; the live referee asserts on typed events.
- **Benchmark:** collectors become plain JSON readers.

### P13. Instrumentation as a contract: one versioned event vocabulary

- **Codex comparison** (`/tmp/smith-benchmark-codex-research/source/exec/src/exec_events.rs`):
  - Codex emits `thread.started`, `turn.started`, `item.{started,updated,completed}` (agent_message, reasoning, command_execution {command, aggregated_output, exit_code, status}, file_change, mcp/collab tool calls, web_search, todo_list, error), then `turn.completed {usage: input, cached_input, cache_write_input, output, reasoning_output}`, `turn.failed` and `error`.
  - Usage is **cumulative only**; the harness read SQLite rollouts for per-response usage and child threads.
- **Claude comparison:** `claude -p --output-format stream-json` has an init record, messages with usage, and a final `result` with durations, turns, cost and `is_error`. This is from memory, not checked in this environment.
- **What smith already has:**
  - Typed facts (`smith-domain-session/src/facts.rs:18`; run facts).
  - Authoritative totals in the answer: `Answer::Accepted|Failed { spent: Spend, turns }` (`smith-domain-run/src/boundary.rs:774-785`).
  - The local domain discards `spent` in `finish_answer`.
- **Clean fix:**
  - **One `smith-events` vocabulary** of JSON lines with `"v":1`. Consumers ignore unknown types and fields. Additions don't bump the version; renames and removals do. It is served by `smith exec --json` on stdout and by `--trace PATH`, with the same records. Capture policy removes content fields, never changes shape.
  - **Authoritative records**, built from domain terminals and shell measurements, never dropped, with reserved queue room:
    - `session.started`;
    - `run.completed`: status, typed failure, result, `Spend`, turns, durations;
    - `session.ended`: exit code, teardown, per-class loss counts, `getrusage` self+children CPU and max RSS.
  - **Detail records** (best-effort, exact loss counts):
    - Per response: conversation and parent, configured model, endpoint and effort, attempt, request bytes, start, first byte and end times, stop, blocks, calls, invalid, usage `{input, cached_input, cache_write, output, reasoning?}`, spend.
    - Per tool: provider call id, tool, times, verdict, exit code, bytes; content only under `calls`/`everything`.
    - Budget snapshot: turns used and remaining (global and child quota), spend, time.
  - **Fields that need provider support:** `reasoning` needs `Usage` (`llm.rs:266`) and skein-llm to carry it. `cache_write` is reported as unknown, not 0, until observed.
  - **Timestamps:** milliseconds since process start on every record; wall time once at start.
  - **Fan-out:** pop each fact once and hand it to the encoder and, in the spawned capability, the channel, each with its own queue and loss counter. This removes the gating at `service.rs:1338`.
  - **Layering:** encoding is step code over `skein-json`; the sink is skein io (P2).
- **Contracts:** `protocol/events.md`; domain/run.md §11 (call ids on tool facts if needed); protocol/agent.md §5.
- **Tests:** goldens and drift; a local-process world checking events against the fake provider's requests and that usage sums equal `Spend`; encoder memory test.
- **Benchmark:** probe: a two-file edit plus one shell call. Assert matching call ids, `run.completed.spent` equal to the summed responses, and `teardown_ms` < 100.
- **Questions:** Q4.

### P14. No headless mode: exit happens only on stdin EOF, and the status doesn't say whether the run succeeded

- **Symptom:**
  - The harness sends the task as one line and refuses anything over 4,088 bytes.
  - It tails the trace for the root `Returned … Accepted`, scans stdout for strings like "Run failed", then closes stdin to make the host exit.
  - A failed run still exits 0 (`smith-local-domain/src/domain.rs:968-970`).
- **Clean fix:** `smith exec [PROMPT|-]`, modeled on codex (`exec/src/cli.rs`: `--json`, `-o`, `--output-schema`, `--ephemeral`, `-C`, `-m`, `-c`, `resume --last`, stdin prompt).
  - The prompt travels **in the start**, so there is no Admitted round trip and no wait opening. That removes the `140c351` race by construction.
  - No `wait` authority by default; a run that would wait ends as `needs_input`.
  - One run, then exit, with a documented code table (Q5).
  - Result on stdout or `-o`; human status on stderr; `--json` puts the P13 stream on stdout.
  - `--chat NAME` to resume, `--ephemeral` for a temporary state dir; the default chat id is reported in `session.started`.
  - Workspace defaults to cwd with git detection; `--read-only` for reviews.
  - Edits in place, no commit by default; `--deliver` for today's change contract.
- **Contracts:** protocol/hosts.md §5.1, domain/host.md §8, domain/run.md §3 if the first message moves into the start.
- **Tests:** a local-process scenario per exit code; e2e `smith exec --json` against the fake provider.
- **Benchmark:** the runner becomes `smith exec --json -C WS -m MODEL - < prompt`, and process exit is the terminal.
- **Questions:** Q5.

### P15. The interactive mode is a line pipe, not a chat UI

- **Symptom:**
  - One line is one message, with backslash continuation.
  - Text appears per turn, not streamed; tools show as `Tool: name` (`person.rs:47`).
  - No status line, no commands, no history or editing; the sign-in URL is printed inline.
- **Assessment:** fine as the contract's minimal person, insufficient for daily use, not a benchmark blocker.
- **Clean fix:**
  - Render the P13 stream: text deltas if the LLM component exposes them (cross-theme), tool verdicts and durations, a status line after each response.
  - A small command set (`/model`, `/effort`, `/new`, `/resume`, `/quit`) in the terminal protocol.
  - TTY raw mode lives in the shell; `--plain` keeps today's pipe mode for worlds and scripts.
- **Contracts:** protocol/hosts.md §5.1.
- **Tests:** terminal step tests; local-process person scripts.
- **Benchmark:** none needed.

### P16. Settings: everything required, nothing discovered, no flags

- **Symptom:**
  - Required: `agent`, `chat`, `instructions`, `models` (three prices plus `price_unit`), `budget`, `waiting_seconds`.
  - The `agent` object needs `profile` (one valid value), `memory_bytes` above a "roughly 360 GB" bound (users write `1099511627776`), `grace_ms` and `environment`. Each endpoint needs `number`, `dialect`, `account`, `provider`, `address` and `server_name`.
  - The state dir is positional; workspace settings are an explicit path; there are no flag overrides.
  - So the harness generated settings per attempt (`smith_smoke.py` `settings()`).
- **Clean fix:**
  - Provider presets (`codex`, `anthropic`), so users write `model` and `effort`.
  - `memory_bytes` defaults to the profile's checked bound.
  - Defaults for profile, grace, waiting, instructions and budget; prices optional; chat `default`; state under `$XDG_STATE_HOME/smith`; workspace = cwd.
  - Layering: built-ins < user config < `<repo>/.smith/` (discovered) < `-c key=value` < flags. Keep strict refusal of unknown fields.
  - `smith config show` prints the effective settings, credential-free.
- **Contracts:** protocol/hosts.md §5.2 leaves the format to the binary; local-host.md is rewritten.
- **Tests:** merge step tests; one e2e refusal per layer.
- **Questions:** Q6.

### P17. Credentials: no sign-in for known providers; the benchmark hand-encoded token records

- **Symptom:**
  - OAuth needs a user-supplied issuer registration.
  - The reliability session used an import script.
  - The benchmark hand-wrote `SKOT` records with a dummy refresh token, because "SKOT codec requires nonempty refresh" (`smith_smoke.py` `token_record`), and copied `account_id` from Codex's `auth.json`.
- **Clean fix:**
  - `smith login <provider>` using the preset where a public client is sanctioned; device or paste flow otherwise.
  - `smith auth import codex`: access token only, read-only. skein's record codec should admit access-only records.
  - `SMITH_<PROVIDER>_TOKEN` and API-key environment variables for CI.
  - `smith auth status` showing accounts, expiry and source, never values.
  - Keep the rule: never copy a rotating refresh token.
- **Contracts:** domain/host.md §7; protocol/hosts.md §5.4; skein `oauth.md` §1.
- **Tests:** local-process import and env stories.
- **Benchmark:** the harness passes an env token instead of writing record files.
- **Questions:** Q7.

---

## 5. Relation to jig's one-host draft

### P18. Build on smith's inline agent, not on today's in-process local service

**Today's in-process composition.** `smith-local-domain` owns `smith_domain::Domain` (`domain.rs:146`), with 29 `Placement::` branches and paired `Final::InProcess` / `Final::External` arms. `smith-local-service` carries both `effects` and `host` (42 `effects` references). Two lifecycles live in one domain.

**Option A: complete today's in-process local service.**
- For: it exists, it is typed with no encoding (protocol/README.md §4: "a charter is never encoded"), and the domain world covers it.
- Against:
  - Two code paths forever, and every fix lands twice (the spike already wrote "Both placements…").
  - P6, P7 and P8 must each be invented for this placement.
  - jig keeps its own twin, so smith has three one-process compositions (draft §1).

**Option B: the inline agent (recommended).** A domain child that speaks `smith-host-domain`'s parent vocabulary and composes one `smith-domain` per run. LLM and machine requests go up to the root, which routes them to `Effects`.
- For:
  - The local domain loses `Placement` and keeps only the external path, which is the spawned capability's lifecycle too.
  - One composition shared with jig.
  - The twin already has the cancel-grace-drop ladder (`domain.rs:276-279`) and per-run fact and content drains (`:94-99`), the two biggest in-process gaps.
  - Placement becomes a root-level choice, so a later `isolate` setting is cheap.
  - Turns are saved as bytes anyway, so the extra encoding is small.
- Against:
  - Not yet in smith. jig's twin has **no workspace** (`unreachable!("inline charter has no workspace")`, `domain.rs:429-435`), so smith must add machine requests, checks, delivery relays and checking notices.
  - Drop needs effect settlement (P8).
  - It contradicts protocol/README.md §4's "never encoded".
  - The draft's open questions must be answered.

**Assessment:** B.

**Sequence:**
1. Answer the draft's §5 questions. Recommended: a shared vocabulary crate with `smith-host-domain`, bytes for charter and turns, a wall bound and no progress watchdog.
2. Build `smith-inline-agent` from jig's, adding the workspace and the drop ladder.
3. Recompose `smith-local-service` as local domain + (inline | host) + `Effects`, and delete `Placement`.
4. Events on the twin's drains.
5. Headless and interactive front ends.
6. jig repins smith and retires its twin.

**Contracts:** the draft's §4 list (host.md §§1, 4, 8, 9, 10; protocol/hosts.md §5.6; README tree), plus protocol/README.md §4 and testing.md §2.

**Tests:** the host world runs both kinds of agent; local-process placement becomes a choice of agent kind.

**Benchmark:** rerun one fixture with the single-process binary. Expect about 0 ms teardown, one TLS handshake per invocation, and no `agent.json`.

---

## Questions for the user

1. **Root package `smith`, with `src/main.rs` and the step `clippy.toml` moved to `crates/clippy.toml`?** *Recommendation:* yes.
2. **Ship `src/bin/smith-agent.rs`?** *Recommendation:* keep it until temper decides whether its worker embeds the agent shell. It is the only real-process proof of the channel.
3. **Is spawning a product option (`isolate`) or only a library capability?** *Recommendation:* library first, `isolate` later once the inline agent makes it cheap and contained trees exist. Today's plain children give little extra containment.
4. **Is the event vocabulary a public versioned contract from day one, with the trace file being the same stream?** *Recommendation:* yes. Authoritative records never drop; detail records drop with exact counts.
5. **Headless exit codes (0 accepted, 1 failed, 2 config, 3 credentials, 4 budget, 130 interrupted) and edit-in-place without commit by default?** *Recommendation:* yes, with `--deliver` for today's behavior.
6. **Settings format and discovery?** *Recommendation:* TOML for human files (strict, bounded), JSON accepted for generated files, `<repo>/.smith/` discovery on.
7. **Credentials policy: provider public clients and read-only Codex import?** *Recommendation:* import the access token only, never refresh; `login` only where the provider sanctions a public client; env tokens for CI.
8. **Order of work?** *Recommendation:* inline agent first, then the product binary. Until then benchmarks keep the spawned binary and the `Debug` trace for one more round.

## Cross-theme dependencies

- **Pool close and shutdown** (`a2c2e53`, skein `416cffd`/`f5fca44`): shared by both placements; the P8 drop ladder depends on `Component::close` and abort settlement.
- **Trace and facts fixes** (`136b9c4`, `6a1dfb2`, `a54ba26`): replaced by P2, P6 and P13. Record bounds should target `smith-events` plus a skein sink.
- **Limits** (arguments, deadlines, envelope, turns): P3 moves and exposes them; P9 ties the host watchdog to `call_timeout`.
- **First-task race** (`140c351`): P14's prompt-in-start may remove `Awaiting` for headless runs.
- **Diagnostics** (`ecb7562`, `e69bfc3`, `8466afb`): P7 makes them placement-neutral and event-based.
- **Sub-agents and budgets** (`3fae871`, `44e5120`): P13 needs parent linkage and own-turn quotas in facts.
- **skein-llm usage:** reasoning tokens and real cache-write counts for P13.
- **Cache affinity:** with presets, the binary should generate and persist the per-chat `cache_key` and `session-id` itself.
- **skein:** `Host` and `drive`, append sink, durable replace, OAuth io driver, access-only records, trust loader — each lands in skein before its smith consumer.