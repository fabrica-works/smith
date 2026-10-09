# smith

smith is a kit for flexible LLM agents and a standard agent built from
it. A host supplies a charter, tools, messages, result contracts and a
budget; smith supplies the conversation and run mechanics. Its design
starts at [domain/README.md](docs/design/domain/README.md).

## Layout

- `crates/`: the domain and protocol crates, and the `smith` binary.
- `tests/`: worlds and their focused and `fuzzy_*.rs` test binaries; the
  binary's end-to-end and live tests are in `crates/smith/tests`.
- `docs/design/domain/`: the run, session, tools and host contracts.
- `docs/design/protocol/`: the protocol layer: channel, charter,
  transcripts, LLM calls, the agent process and hosts.
- `docs/design/testing.md`: how smith is tested.
- `docs/development/workflow.md`: the checks before merging, the suites'
  budgets and the live suite.
- `docs/development/local-host.md`: running `smith local`.

smith follows skein's foundation documents: programming-model.md for the
Rust subset and ownership, testing-strategy.md for independent worlds,
and notes.md for the memory strategy. Step crates inherit the workspace
lints and use `no_std` with `alloc`; test harnesses are ordinary Rust
whose clocks, seeds and ordered observations make runs reproducible.

## Development

Read [AGENTS.md](AGENTS.md) and the
[workflow](docs/development/workflow.md) before changing code. The gate is:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo nextest run --workspace --profile fuzzy
```

Focused tests have a 15-second workspace budget; fuzzy tests have 60
seconds. `--profile measure -j 1` measures a suite serially without the
gate's timeout. Each world keeps its own fakes and independent
expectations; generic harness mechanisms are reused from `skein_world`
and tested in skein.

Dependencies on skein use `https://git.ekanayaka.io/ai/skein.git` without
`rev`, `branch` or `tag`; `Cargo.lock` pins the revision. smith, jig and
temper must resolve the same skein revision when composed. Changes to
shared mechanisms land in skein first; smith and its consumers then take
that revision through their own gates.
