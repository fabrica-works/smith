# Development workflow

How work reaches main. Work happens on local branches; main moves only by
merging a branch that passed every check below, or that changes only
documentation.

## 1. Before merging to main

On the branch's tip, rebased on main, so that what is checked is what main
becomes:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo nextest run --workspace --profile fuzzy
```

Then, once all four pass:

```sh
git checkout main
git merge --ff-only <branch>
```

A branch that changes only Markdown files (the documents under `docs/`,
`AGENTS.md`) skips the four checks, since nothing it touches is built or
tested, and is merged the same way. Doc comments in `.rs` files are not
documentation in this sense: they are code, and clippy checks them.

## 2. The two test suites

The suites are those of skein's `docs/foundation/testing-strategy.md`
(section 8); `docs/design/testing.md` (6.1) says what they hold in smith.

- **The default suite** is every crate's unit tests and each world's
  focused tests (`tests/<world>/tests/*.rs`): focused tests of
  expected behaviour (scenarios, referee tests, replay, facts changing
  nothing, memory against the worst case), and a cheap random world as a
  smoke test where one helps. It is what `cargo nextest run --workspace`
  runs, and it takes at most **15 seconds**.
- **The fuzzy suite** is the worlds' `tests/fuzzy_*.rs`: randomized tests,
  such as sweeps of random worlds and domains driven at random, over many
  seeds. It is not run by default, only with `--profile fuzzy`, as the
  gate before merging to main, and it takes at most **1 minute**.

The budgets are enforced, not advisory: `.config/nextest.toml` gives each
profile a `global-timeout`, so a suite that runs past its budget fails,
naming the tests it stopped, and a slow period past which a test is
flagged SLOW as it runs. A change that breaks a budget is fixed by making
tests cheaper, or by moving randomized ones to a world's fuzzy tests.
Raising a budget is a decision to take explicitly, not a fix. To see what
each test costs on its own, the `measure` profile has no budget:

```sh
cargo nextest run --workspace --profile measure -j 1
cargo nextest run --workspace --profile measure -j 1 --ignore-default-filter -E 'binary(/^fuzzy_/)'
```

The budgets are wall time on the development machine (4 cores, 8 threads,
which nextest uses all of), with nothing else building. Run the checks on
an idle machine.

## 3. When a fuzzy seed fails

A failure names its seed, and a seed replays to the same run. Fix the bug,
then keep its seed: as a scenario among its world's focused tests if it
shows behaviour worth naming, or among the sweep's pinned seeds. A finding
that cannot be fixed yet joins its world's `FINDINGS`, which an ignored
test replays until it is fixed.

## 4. The live suite

The `live` profile runs the shipped binary against real providers, their
OAuth issuers and, optionally, a git remote: one test at a time, without
retries, with a ten-minute timeout. The default and fuzzy profiles exclude
it, and the merge gate never runs it (`docs/design/testing.md`, 2.3).

It is configured by the environment. A missing variable, marker or
signed-in record fails the test, naming it:

- `SMITH_TEST_LIVE_TOKEN_DIR`: an absolute, private directory reserved for
  this suite, holding each provider's account-zero record in a
  subdirectory and the marker the bootstrap writes. Never point it at
  another tool's login or copy a refresh token from one: issuers rotate
  refresh tokens, and a copy logs that tool out.
- `SMITH_TEST_LIVE_PROVIDERS`: `codex`, `anthropic` or `codex,anthropic`.
- `SMITH_TEST_LIVE_CODEX_ACCOUNT_FILE`, `SMITH_TEST_LIVE_ANTHROPIC_ACCOUNT_FILE`:
  for each selected provider, a JSON file of public registration data
  only, `account_id` and `oauth` as in
  [local-host.md](local-host.md#accounts-and-sign-in). No tokens or client
  secrets; unknown fields are refused.
- `SMITH_TEST_LIVE_CODEX_MODEL`, `SMITH_TEST_LIVE_ANTHROPIC_MODEL`: default
  `gpt-5.5` and `claude-haiku-4-5`, as skein's live reference.
- `SMITH_TEST_LIVE_GIT_REMOTE`, optional: a disposable test remote with
  authentication already configured. The push story pushes a unique
  `smith-live-<provider>-<pid>-<clock>` branch there and removes it after;
  without the variable, that story reports not run.

Sign in once per provider, by hand, to create the suite's token record:

```sh
export SMITH_TEST_LIVE_TOKEN_DIR="$HOME/.local/state/smith-live-tokens"
export SMITH_TEST_LIVE_PROVIDERS=codex
export SMITH_TEST_LIVE_CODEX_ACCOUNT_FILE="$HOME/smith-live-codex-account.json"
export SMITH_TEST_LIVE_BOOTSTRAP_DIR="$HOME/.local/state/smith-live-bootstrap"
cargo nextest run -p smith --test live --profile live -E 'test(=bootstrap_settings)'
cargo run -p smith -- local "$SMITH_TEST_LIVE_BOOTSTRAP_DIR/codex/settings.json" "$SMITH_TEST_LIVE_BOOTSTRAP_DIR/codex"
```

Type a short message, open the sign-in URL shown and complete the
redirect; after the response, close input with Ctrl-D. For Anthropic, do
the same with its account file, `anthropic` selected and the `anthropic`
paths. Then run the suite without the bootstrap directory:

```sh
unset SMITH_TEST_LIVE_BOOTSTRAP_DIR
cargo nextest run --workspace --profile live
```

Each selected provider runs `a_run_refreshes_its_grant_at_the_issuer`,
`a_chat_ends_with_a_commit_in_place`,
`a_second_run_resumes_the_chat_from_its_files` and
`a_configured_push_lands`. Run one alone with
`cargo nextest run -p smith --test live --profile live -E 'test(=<name>)'`.
The stories check outcomes (committed files, token generations, traces),
never the model's wording.
