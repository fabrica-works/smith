# accounts: the first part of LLM accounts, its formats and notices

2026-10-19. Start commits: skein `4c0895d` ("Move forge reconciliation
onto checked owner values"), smith `e79b9a6b` ("Design LLM accounts'
smith part: sources, programs, grants, notices"). Every command of this
plan ran once at these commits, or at skein's `84e1d89` before forge-kit's
last two landings, which change no file of this plan
(`findings/base-runs.md`); each session starts from its repository's
`origin/main`.

The first part of LLM accounts, its lower half (decisions.md, 1): the
MIT licence in skein and smith; skein's credential with its scheme, the
failure classes an account meets, with their report; OpenAI's public
Responses route; the fake LLM's failures and its list of credentials;
the OAuth machine's terms, ID token, revocation and record version 3;
the accounts driver's named table, sign-out and shared record; then
smith's grants by scheme, the channel's version 4 and its notices, the
charter's version 3, the transcript's version 6 and its carry-over, and
the event vocabulary's version 4. The withdrawn forms stay, marked for
the second part (decisions.md, 2). smith's accounts by source, its
logins and Claude Code as a program are the next plan's,
`accounts-sources` (`findings/next/`).

## Inputs

- **skein's design,** at `49725eca`: `docs/design/llm.md` (2.5, 2.6, 4,
  4.2, 4.6 to 4.8, 5, 7), `docs/design/llm-connection.md` (3, 4, 4.2),
  `docs/design/fake-llm.md` (2.3 to 7), `docs/design/oauth.md` (1 to 6);
  `docs/foundation/testing-strategy.md`, 8 and 10;
  `docs/foundation/development.md`, 5.
- **smith's design,** at `137a1cb4`: `docs/design/protocol/channel.md`
  (4, 5, 6, 10), `docs/design/protocol/charter.md` (2, 5, 8),
  `docs/design/protocol/transcript.md` (2, 3, 4, 6),
  `docs/design/protocol/events.md` (3.3, 3.5, 3.9, 3.10, 6),
  `docs/design/protocol/llm.md` (6, 7, 10),
  `docs/design/protocol/README.md` (8), `docs/design/protocol/agent.md`
  (4), `docs/design/protocol/limits.md` (2.4),
  `docs/design/domain/session.md` (3, 4, 8.2), `docs/design/domain/run.md`
  (4, 6, 10), `docs/design/domain/host.md` (7, 8),
  `docs/design/protocol/hosts.md` (5.1).
- **`decisions.md`:** the plan's decisions, the design questions with
  their answers, and what the user may veto.
- **`findings/`:** `base-runs.md` (the gates and the suites' times at
  the base); `next/` (the next plan's outline and the Claude Code
  probe).
- **`code-organization.md`:** crates, names, owners, fixtures, the
  "Don't" list, sizes and the count.
- **`later.md`:** what the design names that this plan does not build.

## The sessions

A session waits for what "Needs at start" names; an increment that needs
more says so in its row and its file. Dependencies are by capability:
"skein 01.3" means that increment has landed on skein's `origin/main`.
"live text's …" is a step of the live-text plan, awaited as the Rules
say.

### skein (`skein/`)

| File | Session | Needs at start | Increments that need more |
|---|---|---|---|
| `01-credentials.md` | skein 01: the MIT licence; a credential with its scheme; the failure classes and their report, with its values; a call with no credential | nothing | — |
| `02-public-route.md` | skein 02: OpenAI's public Responses route; the code first on the Codex and Anthropic routes; the public request; its code table | skein 01's `Scheme` [01.2] | 02.2: skein 01's failure classes and report values [01.3, 01.4]; 02.3: skein 01's call with no credential [01.5], and 02.2's code first |
| `03-fake-failures.md` | skein 03: the fake LLM's failing line; each route's failure before a stream; a failure after other lines | live text's fake LLM steps, the last its byte peer's release [`live-text/skein-01.3`] | 03.2: skein 01's report and values [01.3, 01.4], skein 02's public route and code table [02.1, 02.4]; 03.3: skein 02's public request and code table [02.3, 02.4] |
| `04-fake-credentials.md` | skein 04: the fake LLM's credential list, in the protocol and the peer, with the peer's observations; the public route's cache rules; the Exchange with a list, and the stories | skein 01's `Credential` [01.2], skein 02's public route [02.1], skein 03's route tables [03.2] | 04.3: skein 02's public request [02.3]; 04.4: skein 01's call with no credential [01.5] |
| `05-oauth.md` | skein 05: the RS256 check; the OAuth record's version 3; a registration's terms; an issued client id, the hints and `Incomplete`; revocation | nothing | — |
| `06-fake-issuer.md` | skein 06: the fake issuer's terms, revocation and token list; its test key, ID token and documents | skein 05's revocation [05.5], after its record, terms and issued id | — |
| `07-openid.md` | skein 07: an OpenID sign-in's keys, by GET; the ID token's check | skein 05's `OpenId` terms [05.3], skein 06's documents and `Event::Get` [06.2] | 07.2: skein 05's `verify_rs256` [05.1] |
| `08-driver-accounts.md` | skein 08: the driver's named table; accounts added and removed; a wanted account and the earliest refresh time; a redirect whose port varies | skein 05's record version 3 and terms [05.2, 05.3] | 08.4: skein 05's issued client id and hints [05.4], skein 06's fake issuer on any loopback port [06.1] |
| `09-driver-shared.md` | skein 09: signing out; required scopes; the shared record under the owner's lock | skein 08's landing [08.4], skein 06's revocation [06.1] | — |
| `10-driver-lock.md` | skein 10: the lock's other outcomes; OpenID through the driver | skein 09's lock and shared stories [09.3] | 10.2: skein 07's keys and ID token check [07.1, 07.2], skein 06's documents [06.2] |
| `11-fake-loopback.md` | skein 11: the fake issuer's documents and revocation on loopback; its tokens at the LLM peer | skein 06's documents [06.2], after its revocation and token list, and skein 04's Exchange with a list [04.4] | — |

### smith (`smith/`)

| File | Session | Needs at start | Increments that need more |
|---|---|---|---|
| `01-grants.md` | smith 01: the MIT licence; the channel's version 4; smith on skein's credential and failure classes | nothing | 01.2: live text's channel version 3 with its live record [`live-text/smith-01.2`]; 01.3: skein 01's credential, failure classes, report and no credential [01.2 to 01.5], skein 02's public route [02.1], live text's LLM component handover [`live-text/smith-01.3`] |
| `02-notices.md` | smith 02: the session's and the run's account failures; the notices raised; the host's faults; the local host's account notices | smith 01's version 4 [01.2] and skein classes [01.3] | 02.2: smith 04's grant value union [04.1] |
| `03-records.md` | smith 03: the charter's version 3; the transcript's version 6; a window carried over | smith 04's grant value union [04.1], smith 02's notices [02.2] | 03.2: smith 04's dialect numbers [04.2] |
| `04-values.md` | smith 04: the grant value union; the dialects by name and number | smith 01's credential by scheme and `openai` provider [01.3], after its grant value union on the wire [01.2] | — |
| `05-events.md` | smith 05: the event vocabulary's version 4, its models and completions, then an account's failures | smith 03's charter account [03.1], smith 02's run faults, notices, host faults and local notices [02.1 to 02.4] | — |

### The closing repins

development.md 5.7: no session file. Each lands with its repository's
gate, and repairs with decisions 15 and 16's defaults (Rules).

| Step | Starts once | Moves |
|---|---|---|
| `smith-repin` | every skein step has landed | smith's skein to skein's `origin/main`, adding the key row of smith 04.1's `no_grant_value_reaches_a_turn_fact_or_trace` (ruling 124) |
| `jig-repin` | `smith-repin` and every smith step have landed | jig's skein and smith, together, to their `origin/main` |
| `hearth-repin` | `jig-repin` has landed | hearth's skein to skein's `origin/main` |
| the repository downstream of jig's repin | `hearth-repin` has landed | its skein, smith and jig to their `origin/main` |

hearth builds on none of the crates this plan changes (its manifest names
skein's shell, sim, io, scratch, world, heap, lib and json): its repin
should move the pin and repair nothing.

jig's repairs come to about 150 to 170 lines for smith's changes alone,
besides skein's driver arms (decision 16): past 5.7's 200 in one step.
So the supervisor adds `jig-repin.s1` once smith 01 and 04 have landed
(5.7, "When it starts"): it moves jig's smith to `origin/main` and its
skein with `--precise` to the revision smith's `Cargo.lock` pins, which
is the one smith compiles against until `smith-repin`, and repairs smith
01's and 04's breaks, about 60 to 80 lines. The closing `jig-repin`
takes smith 02's, 03's and 05's, and skein's after that revision. The
repository downstream of jig's repin comes to about 80 to 120 lines for
smith's and about 40 for skein's.

### How many at once, and in what order

- **At the start,** three need nothing: skein 01, skein 05 and smith 01,
  whose 01.1 runs at once. skein 03 starts once live text's skein 01.3
  is on `origin/main` (Rules).
- **Then,** as each lands: skein 02 once skein 01.2 has; skein 08 once
  skein 05.3 has; skein 06 once skein 05 has; skein 07 once skein 06.2
  has; skein 09 once skein 08 and 06.1 have; skein 10 once skein 09.3
  has; skein 04 once skein 03.2 has; skein 11 once skein 06 and 04 have;
  smith 02 and smith 04 once smith 01.3 has; smith 03 once smith 04.1
  and 02.2 have; smith 05 once smith 03.1 and smith 02 have; the repins
  as their table says.
- **At most eight sessions are open together** (skein 03, 04, 06, 07
  and 09, smith 02, 03 and 04), the coordinator's eight; smith 04 then
  holds only 04.2, and skein 03 may be waiting within its session, for
  skein 02.4.
- **When slots are short,** start first: skein 01, skein 02, skein 03,
  skein 04 (the longest chain), smith 01, skein 05; then skein 06, skein
  08, smith 04, smith 02, smith 03, smith 05, skein 07, skein 09, skein
  10 and skein 11 as they open.

**The longest chain,** 17 steps: skein 01.1 to 01.4, 02.2, 02.3, 02.4,
03.2, 04.1 to 04.4, 11.1, then `smith-repin`, `jig-repin`,
`hearth-repin` and the downstream repin; at 0.09 to 0.14 h a step, about
1.5 to 2.4 h, plus any wait on live text past skein 02.4's landing.
smith's is 15: skein 01.1 to 01.5, smith 01.3, 04.1 (beside 02.1),
02.2, 02.3, 02.4, 05.1, 05.2, then `jig-repin`, `hearth-repin` and the
downstream repin.

## Rules of this plan

- **The waits on live text** (decisions.md, 17). live text runs on
  another machine, on the same skein and smith `main`. Before it starts
  each of these steps, the coordinator registers its wait, and starts
  the step only once the awaited step is on `origin/main`:
  - skein 03.1: `plan await coordinator live-text/skein-01.3`;
  - smith 01.2: `plan await coordinator live-text/smith-01.2`;
  - smith 01.3: `plan await coordinator live-text/smith-01.3`.
  No other step waits for live text; a conflict with its landings is a
  rebase, as `land` and the implementer's card say.
- **smith takes skein twice** (decisions.md, 13). smith 01.3 repins
  skein at its start, in a commit of its own, to the later of skein
  01.5's and 02.3's landings (ruling 113: 02.1 leaves `Provider::OpenAi`'s
  measure and prepare `Unsupported`, which 02.3 fills): `c=$(git -C ~/src/rust/skein log
  origin/main -E --all-match --grep='^Plan: accounts$'
  --grep='^Step: skein-0(1\.5|2\.3)$' -1 --format=%H)`, then `heavy
  cargo update -p skein-llm --precise $c` and `heavy cargo check
  --workspace --all-targets`, the commit's body naming `$c`. That
  commit repairs what every skein step at or before `$c` broke: skein
  01's and 02's, and those of skein 03 to 09 that landed first, each
  named in its step's Shape ("Downstream"), at most about 200 lines
  besides `Cargo.lock` (development.md, 5.7); more is a question. No
  other step of this plan but `smith-repin` moves the pin, which takes
  the rest. A skein increment that changes an interface used downstream
  names, in its Shape, the downstream lines it breaks, and the repins
  repair them with decisions 15 and 16's defaults.
- **The closing repins** follow development.md 5.7: `smith-repin` once
  every skein step has landed, then jig's, hearth's and that of the
  repository downstream of jig, in that order. Their repairs keep
  today's behaviour downstream (decisions.md, 16) and spell each new
  limit with skein's fixture value (15). A mapping the defaults do not
  cover is a question: the repin lands nothing.
- **The licence reaches every member.** skein 01.1 and smith 01.1 give
  each member on `origin/main` `license.workspace = true`. A member
  another run adds after them takes it too: a member found without it at
  "Done when" is given it by a supplementary step of the session that
  set it.
- **Hotspots,** the files this plan and another run both change:
  - with live text (hel8, on skein's and smith's `main`; its plan at
    smith `0e9c3308`): in skein,
    `testing/skein-fake-llm-domain/src/{api,domain,respond,limits}.rs`,
    `testing/skein-fake-llm-protocol/src/{provider,documents}.rs`,
    `testing/skein-fake-peers/src/llm.rs`, `tests/llm/src/fake.rs`,
    `tests/llm/tests/{fake,fake_memory}.rs` and
    `tests/llm-connection/src/simulated.rs` (skein 01.2's call sites,
    before the wait; skein 03 and 04 after it); in smith,
    `crates/smith-channel`'s schema, generated code and goldens, the
    `component.rs` and `translate.rs` of `smith-protocol-channel` and
    `smith-host-protocol` (smith 01.2, 04.1, 02), `smith-protocol-llm`'s
    `boundary.rs` and `component.rs` (smith 01.3, 02, 03),
    `tests/codecs/tests/drift.rs` and `tests/end-to-end/end_to_end.rs`'s
    formats line (smith 01.2, 03.1, 03.2, 05.1; live text's smith 01),
    and, beside live text's smith 02, which nothing here awaits,
    `smith-agent-service/src/service.rs` (every smith session),
    `smith-local-service/src/process.rs`, `tests/protocol-llm`,
    `tests/hosts`, `tests/agent-process` and `Cargo.lock` (smith 01.3's
    repin and live text's smith 02.1);
  - with forge-kit (carbon-forge, on skein's `main`): skein's root
    `Cargo.toml` and `Cargo.lock` (skein 01.1, and every step adding a
    dependency), `testing/skein-fake-peers/Cargo.toml` (skein 01.1's
    licence) and `src/lib.rs` (skein 11.1's doc lines),
    `docs/design/STATUS.md` (the designer's), and
    `docs/foundation/testing.md` if a step edits it;
  - with tools (debian-16gb-hel1-1, on skein's, jig's and the repository
    downstream of jig's `main`): jig's and the downstream repository's
    `Cargo.lock`, at this plan's closing repins.
  `Cargo.lock` is never merged by hand (development.md, 5.7).
- **No credential value in a test's output** (code-organization.md, 7):
  tests compare routes, schemes and indices; a key's value appears only
  in its fixture.
- **A test helper is written once** (testing-strategy.md, 10, "A setup
  written once"; ruling 110): a helper that a second test file of a
  crate needs moves to that crate's test library (for skein-llm's
  worlds, `tests/llm/src/lib.rs`, beside `fields_holding`), and both
  files call it; the session writing the second copy moves the first.
  `tests/llm/tests/credentials.rs` `header` and `rejection` (retyped
  from `anthropic_admission.rs`, one returning `Option<Error>`, the
  other `Error`) and its twice-written `World::new` / `Start` / `run`
  lines are the first: skein 01 moves them at its next increment.
- **A known timing flake** (ruling 116): skein's
  `tests/world/tests/real.rs`
  `a_keep_time_after_the_last_word_fails_the_world_naming_the_process`
  fails now and then as "did not panic" (once in its 26 runs in this
  run's gate logs, to skein-08.1's of 15:02): its 5 ms deadline is taken
  before `real::World::new` and `spawn`. A gate that fails only there is
  a timeout's failure (implementer.md, 8.5): `land` again once without
  asking; a second failure on the same tree is a question. Its fix is
  `later.md`'s.
- **A table test's rows are data** (testing-strategy.md, 10, "Rows
  carry data" and "The setup the assertion reads"; ruling 119): a row
  holds its inputs and its expected value; the body runs the same lines
  for every row and compares once, with `assert_eq!` after any match,
  never inside an arm. A match or a helper turns what it saw into a
  value (a `Result`, or a small struct) and asserts nothing. Rows that
  need different code are tests of their own, each calling one function
  with what differs. skein 05.4's `issued.rs` and 05.5's `revoke.rs`,
  where it was seen twice, stay as they landed.
- **A test's outcome is named, and its sizes derived** (testing-strategy.md,
  10, "Named fixtures and sizes"; ruling 125): a test reads an outcome of
  more than two facts from a struct whose fields name them, never a tuple
  by position (`type Sent = (Option<Failed>, (bool, u32, bool))`); a
  length or a count in a fixture is derived from what it measures
  (`BODY.len()` in a `Content-Length`), never counted by hand. skein
  08.3 brings 08.2's `accounts_tests.rs`, where it was seen, to it; other
  copies landed, in files a change does not otherwise touch, stay as
  they are.

## Live services

None (decisions.md, 10). The public route and Anthropic's key route are
proven against the fake LLM, with the design's codes (9).

## One test of a suite that needs a built artifact

None: no suite this plan's increments run needs an artifact the gate
builds first.

## Process tier

None: neither skein nor smith has process-tier stories outside its
gate, and this plan's behaviour is a route's rendering, a codec's version
and a component's events, which the worlds prove (decisions.md, 10).

## Repin

Nothing: the plan ends with the closing repins of smith, jig, hearth and
the repository downstream of jig (Rules; development.md, 5.7).

## Kit steps

None: the plan changes nothing in `scripts/dev`.

## Pulse

```
words: actual|genuine|legacy|compat|workaround
types: ^(Legacy|Old|Compat)[A-Z]|V2$
file-lines: 800
budgets: 15 60
aims: skein 16 19 skein-llm skein-llm-world skein-llm-connection skein-llm-connection-world skein-fake-llm-domain skein-fake-llm-protocol skein-fake-peers skein-tls skein-tls-world skein-oauth skein-oauth-world skein-oauth-accounts skein-oauth-accounts-world skein-fake-oauth
aims: smith 60 79 smith-protocol-llm smith-protocol-llm-world smith-agent-service smith-agent-shell smith-agent-world smith-channel smith-channel-world smith-protocol-channel smith-host-protocol smith-local-service smith-codecs-world smith-domain-session smith-domain-run smith-domain smith-host-domain smith-inline-agent smith-local-domain smith-local-protocol smith-local-shell smith-session-world smith-run-world smith-charter smith-transcript smith-events smith-host-world smith-hosts-world smith-local-world smith-agent-process-world smith-local-process-world smith-bench
```

Other runs land on skein's and smith's `main` (live text, forge-kit,
tools), so the aims cover the packages this plan's sessions change,
their tests' times summed (development.md, 5.2). At the base, the
medians of three are 12.85 s focused and 15.19 s fuzzy in skein, 57.67 s
and 75.09 s in smith (`findings/base-runs.md`). The run may add, in all,
3 s and 4 s in skein, about 200 tests at the unit and focused-world
ceilings and the new codecs' fuzzing, and 2 s and 4 s in smith, about 70
tests and the three versions' fuzzing. A single run's sum swung by 5.7 s
at the base, more than any session's share, so no session is given one
(5.6).

## Done when

Each command runs once, after `done.md`, in a worktree of skein and of
smith at `origin/main`, the plan's worktrees removed. Each prints
nothing, or what its item says.

1. **Every step landed or dropped:** `plan status` lists none waiting,
   ready, running or blocked.
2. **Each session's own "Done when" holds:** the commands of each
   session file's "Done when", run once each, in the sessions tables'
   order.
3. **The licence:** in skein and in smith, `cargo metadata --no-deps
   --format-version 1 | jq -r '.packages[] | select(.license != "MIT")
   | .name'` prints nothing, and `head -1 LICENSE` prints `MIT License`.
4. **The test-time aims,** measured once, in skein and in smith, on this
   machine. In each, three runs of each suite:

   ```
   heavy --gate -- sh -c 'export NEXTEST_DOUBLE_SPAWN=0; for i in 1 2 3; do cargo nextest run --workspace --status-level pass --final-status-level none > focused-$i.log 2>&1; cargo nextest run --workspace --profile fuzzy --status-level pass --final-status-level none > fuzzy-$i.log 2>&1; done'
   ```

   then, with `P` the package list of the repository's `aims` line,
   joined by `|`, `grep -L Summary *.log` and `grep -lE
   'Summary.*(failed|timed out)' *.log` print nothing, and the median of
   the three sums, for each suite:

   ```
   for s in focused fuzzy; do for i in 1 2 3; do sed -nE 's/^ *PASS \[ *([0-9.]+)s\] \( *[0-9]+\/[0-9]+\) ([^ ]+) .*/\2 \1/p' $s-$i.log | awk -v p="^($P)$" '{ sub(/::.*/, "", $1) } $1 ~ p { t += $2 } END { printf "%.2f\n", t }'; done | sort -n | sed -n 2p; done
   ```

   prints two times within the Pulse's aims for that repository. An aim
   missed is recorded with the three slowest of its packages' tests by
   name in decisions.md, for the user; the run is done either way.
5. **No budget was raised:** in skein and in smith, `grep -E
   '^global-timeout' .config/nextest.toml | head -2` prints
   `global-timeout = "15s"`, then `global-timeout = "60s"`.
6. Nothing outside these items is checked.
