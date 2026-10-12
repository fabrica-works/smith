#!/bin/sh
# The gate (docs/development/workflow.md, section 1): formatting, lints as
# errors, then both test suites in full, the focused one, then the fuzzy
# one. It stays deterministic: none of its checks runs an agent
# (workflow.md, section 5).
set -eu

cd "$(dirname "$0")/.."

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo nextest run --workspace --profile fuzzy
