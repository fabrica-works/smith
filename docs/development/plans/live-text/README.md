# Live-text plan: a reply in pieces, up the channel at the host's ask

2026-10-19. Starts from skein `main` at `49725ec` (its fake LLM as at
`50f7436`) and smith `main` at `137a1cb4` (its code as at `c241a7a4`).

A host asks for live content in the start; the run sends its main
conversation's text up the channel in pieces before the turn's record,
best effort, and the host's half hands them to its owner. skein's fake
LLM learns to write a reply in pieces and hold it at a named point until
its host releases it, so that a test sees a reply partway. smith moves
the channel to version 3, with the ask and the live record, and sends
pieces at the bound or on the live interval, counting what it drops.
jig's and temper's parts are later plans (`later.md`).

## Inputs

- **The design, which is the contract.** skein
  `docs/design/fake-llm.md` (2.2, 3, 5, 6). smith
  `docs/design/protocol/channel.md` (3, 4, 5, 7, 8, 10),
  `docs/design/protocol/limits.md` (2.3, 2.4, 3.6, 3.8, 11),
  `docs/design/protocol/llm.md` (6, 10), `docs/design/domain/host.md`
  (2, 3, 9.1), `docs/design/domain/run.md` (11: a piece is never a
  fact). Each session's "Read first" names its sections.
- **`decisions.md`:** sections 1 to 5 are this plan's choices, then the
  design questions with the director's answers, what goes to the
  designer, and what the user may veto.
- **`code-organization.md`:** crates, modules, tests, fixtures, owners,
  the "Don't" list and size limits.
- **`findings/`:** `planning-audit.md` (the code audit, A1 to A32) and
  `planning-commands.md` (each command run at the base, with its
  output); `findings/README.md` lists them.
- **`later.md`:** what the design names that this plan does not do.

## The sessions

A dependency is named by capability, with its increment in brackets. A
session starts when what it needs at its start has landed; an increment
that needs more waits, and the session does its other increments
meanwhile (implementer.md, section 8.7).

### skein (`skein/`)

| File | Session | Needs at start | Increments that need more |
|---|---|---|---|
| `01-pieces.md` | skein 01: a reply's text in pieces, each its own delta, held at a named point until its host releases it; the server's deltas and keep-alive in both dialects; `Peer::release` | nothing | — |

### smith (`smith/`)

| File | Session | Needs at start | Increments that need more |
|---|---|---|---|
| `01-ask-and-pieces.md` | smith 01: channel version 3 with the start's ask; the live record, the host half's checks and the owner's event; the LLM component's deltas handed to the service, which sends them as pieces | nothing | — |
| `02-best-effort.md` | smith 02: smith on skein's `main`; pieces on the live interval; a retry's restart at offset 0, a channel with no room, and the host's joint story | skein 01's release [01.3] on skein's `origin/main`; smith 01's handover [01.3] | — |

**How many at once.** Two at the start: skein 01 and smith 01. smith 02
opens once both have landed. When slots are short, start smith 01
first: its 01.1 and 01.3 are what the LLM accounts run awaits (rule 3),
and it is on the longest chain; then skein 01.

**The longest chain,** 5 steps: smith 01.1, 01.2, 01.3, 02.1, 02.2; at
0.09 to 0.14 h a step, about 0.5 to 0.7 h. skein 01.1 to 01.3, three
steps, run beside smith 01 with room.

**Other runs on the same `main`s.** temper's tools run (hetzner) changes
skein's request table (`requests.rs`), which the fake crates do not use,
and its smith repin has landed (smith `d56bbe3c`); its jig and temper
repins take this plan's smith (decisions.md, answer 4). temper's
forge-kit run (carbon) lands skein's forge crates, which nothing here
builds on. No file this plan changes is in either run's Files (A9).

## Rules of this plan

What development.md, implementer.md, skein's and smith's `AGENTS.md` and
their `workflow.md` do not already say.

1. **The repin.** smith 02.1's first commit moves smith's skein pin from
   `93052c6b` to skein's `origin/main` with `heavy cargo update -p
   skein-lib`, which moves every skein crate (one git
   source), and adapts only what the move breaks (A30). No other smith
   step moves the pin. Moved to skein's code at this plan's start, the
   pin breaks nothing (`findings/planning-commands.md`, section 7).
2. **One version owner.** smith 01.1 moves the channel from version 2 to
   3, in place (decisions.md 2.2); later increments change version 3
   with no bump. The event vocabulary, the transcript and the charter
   keep their versions. The LLM accounts plan takes the channel to 4.
3. **What the LLM accounts run awaits:** skein 01 (the fake LLM's pieces
   and holds), smith 01.1 (version 3 and the start's ask) and smith 01.3
   (the LLM component's handover). Its plan starts from a `main` that
   holds all three (decisions.md, section 5).
4. **The memory margin.** smith 01.1 asserts a floor of 490,000,000 bytes
   between agent-shell's complete worst case and its shipped memory
   (decisions.md 3.7). Each smith increment's report gives that test's
   line, once, after its last edit:

   ```
   heavy cargo nextest run -p smith-agent-shell -E 'test(product_values_pass_every_startup_check_and_fit_complete_memory)' --no-capture 2>&1 | grep PRODUCT_COMPLETE
   ```

5. **Lint exceptions never land.** Once before `land`, on the rebased
   branch, in the repository it changes:

   ```
   git diff origin/main -- . ':!docs' | grep -nE '^\+.*#\[(allow|expect)\('
   ```

   prints nothing. Each it prints is removed and its lint fixed.
6. **Users outside Files: the sites the audit found.** implementer.md
   8.2's edits outside Files cover them; these are where they fall, so
   that none is a question: in skein, each match on the fake LLM's
   `Event` and `Request` and each `Event::Call` built (A1 to A3, A8);
   in smith, the start's struct literals (A28), each match on
   `OpenEvent` (A29), each destructure of `Policy` (A18), and at the
   repin each match on the fake LLM's `Event` and `Request` in smith's
   tests (A30).

## Live services

**OpenAI, through the user's Codex login,** for smith's `smoke-tools`
only (smith `workflow.md`, section 5): smith reads `~/.codex/auth.json`
read only, the access token only, and no agent signs in. Each smith
landing that changes code runs it once, through `heavy --gate` on the
branch rebased on `origin/main`, and once more only if it fails:

```
heavy cargo run -p smith-bench -- build HEAD
heavy --gate cargo run -p smith-bench -- probe probes/smoke-tools --arm <the directory build printed>
```

Five smith landings, two attempts each at most: 10 attempts, about
100,000 tokens, about $0.06 notional. skein's increments call no live
service. A lapsed login stops the smoke: the coordinator escalates it as
`credentials`, and the user runs `codex` once.

## One test of a suite that needs a built artifact

None: no suite this plan's increments run needs an artifact the gate
builds first.

## Process tier

None: neither skein nor smith has a process-tier script; smith's stories
of the agent process run in its gate (`tests/agent-process`).

## Repin

None, with the reason: jig and temper take this plan's skein and smith
at the tools plan's closing jig-repin and temper-repin, with `live:
None` in the start, an ignored `OpenEvent::Live`, and jig's four
`smith_channel::v3` paths (decisions.md, answer 4; "For the user" 9;
A32). hearth builds on no crate this plan changes. `later.md` names
them.

## Kit steps

None: the plan changes no `scripts/dev`.

## Pulse

The aims are the base's summed times of the packages the sessions
change, on this machine (`findings/planning-commands.md`, sections 2, 3
and 5), plus what the run may add: skein 4.44 s focused and about 7.3 s
fuzzy, plus about 1.2 s and 1.1 s of new units, worlds and joint tests;
smith 10.36 s focused and 17.2 s fuzzy (the median of three), plus about
1.5 s and 1.3 s of new stories. smith's packages that only adapt to the
repin or the start's new field (smith-agent-world, smith-local-world,
smith-session-world, smith-host-world, smith-local-shell,
smith-local-process-world, smith-agent-shell, smith-local-service) are
left out of its aim.

```
words: actually|genuine|immutable|canonical|verified|qualified|robust|properly|python3
types: ^(Legacy|New|Old|Compat)[A-Z]|V2$
file-lines: 800
budgets: 15 60
aims: skein 5.7 8.4 skein-fake-llm-domain skein-fake-llm-protocol skein-fake-peers skein-llm-world skein-llm-connection-world
aims: smith 12 18.5 smith-protocol-channel smith-host-protocol smith-host-domain smith-channel smith-codecs-world smith-channel-world smith-agent-service smith-agent-process-world smith-protocol-llm smith-protocol-llm-world smith-hosts-world
```

## Done when

Each command runs once, after `done.md`, reading `origin/main`, after
`git -C ~/src/rust/<repository> fetch` in skein and smith. Each prints
nothing, or what its item says.

1. **Every step landed or dropped:** `plan status` lists none waiting,
   ready, running or blocked.
2. **skein's fake LLM writes a reply in pieces, held until released:**

   ```
   s=~/src/rust/skein; git -C $s show origin/main:testing/skein-fake-llm-domain/src/api.rs | grep -q 'Pieces {' || echo 'no Line::Pieces'; git -C $s show origin/main:testing/skein-fake-llm-domain/src/domain.rs | grep -q 'Release {' || echo 'no Event::Release'; git -C $s show origin/main:testing/skein-fake-peers/src/llm.rs | grep -q 'pub fn release(' || echo 'no Peer::release'
   ```

3. **smith's channel is at version 3, alone;** prints
   `crates/smith-channel/schema/channel-v3.schema`, then `1`:

   ```
   git -C ~/src/rust/smith ls-tree --name-only origin/main crates/smith-channel/schema/
   git -C ~/src/rust/smith show origin/main:crates/smith-channel/src/kinds.rs | grep -c 'FORMAT_VERSION: u64 = 3;'
   ```

4. **The host asks, and hears the pieces:**

   ```
   m=~/src/rust/smith; git -C $m show origin/main:crates/smith-host-domain/src/channel.rs | grep -q 'pub live: Option<LiveAsk>' || echo 'no ask in the start'; git -C $m show origin/main:crates/smith-host-protocol/src/component.rs | grep -q 'Live {' || echo 'no OpenEvent::Live'
   ```

5. **The agent sends pieces on the interval, counts what it drops, and
   keeps no second switch for live text:**

   ```
   m=~/src/rust/smith; git -C $m show origin/main:crates/smith-agent-service/src/profile/quantities.rs | grep -q 'pub live_interval: Duration' || echo 'no live_interval'; git -C $m show origin/main:crates/smith-protocol-channel/src/component.rs | grep -q 'fn lost_live_bytes' || echo 'no lost_live_bytes'; git -C $m grep -n -E 'live_text:|\.live_text\b' origin/main -- crates tests
   ```

6. **smith pins one skein, and it has the release:**

   ```
   m=~/src/rust/smith; test "$(git -C $m show origin/main:Cargo.lock | grep -o 'forgetic/skein.git#[0-9a-f]*' | sort -u | wc -l)" = 1 || echo 'more than one skein'; s=$(git -C $m show origin/main:Cargo.lock | grep -o -m1 'forgetic/skein.git#[0-9a-f]*' | cut -d'#' -f2); git -C ~/src/rust/skein show "$s:testing/skein-fake-peers/src/llm.rs" | grep -q 'pub fn release(' || echo "smith's skein $s has no release"
   ```

7. **The memory floor holds in smith's test of its shipped values;**
   prints 1 or more:

   ```
   git -C ~/src/rust/smith show origin/main:crates/smith-agent-shell/src/config.rs | grep -cE '490_?000_?000'
   ```

8. **The test-time aims,** measured once in a worktree of skein and of
   smith at `origin/main`, on this machine. In each, three runs of each
   suite:

   ```
   heavy --gate -- sh -c 'for i in 1 2 3; do cargo nextest run --workspace --status-level pass --final-status-level none > focused-$i.log 2>&1; cargo nextest run --workspace --profile fuzzy --status-level pass --final-status-level none > fuzzy-$i.log 2>&1; done'
   ```

   then, with `P` the package list of the repository's `aims` line,
   joined by `|`, the median of the three sums, for each suite:

   ```
   for s in focused fuzzy; do for i in 1 2 3; do sed -nE 's/^ *PASS \[ *([0-9.]+)s\] \( *[0-9]+\/[0-9]+\) ([^ ]+) .*/\2 \1/p' $s-$i.log | awk -v p="^($P)$" '{ sub(/::.*/, "", $1) } $1 ~ p { t += $2 } END { printf "%.2f\n", t }'; done | sort -n | sed -n 2p; done
   ```

   prints two times within the Pulse's aims for that repository. An aim
   missed is recorded with the three slowest of its packages' tests by
   name in decisions.md's rulings, for the user; the run is done either
   way.
9. **Each session's own "Done when" holds:** the command in each session
   file's "Done when", run once each.
10. Nothing outside these items is checked.
