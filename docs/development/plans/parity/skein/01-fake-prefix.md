# Session 01: a changed tool choice keeps an automatic query's prefix

2026-10-14. skein's fake LLM puts every query's tool choice in its
cached prefix, so a smith compaction asked with `ToolChoice::None` would
read nothing from the fake's cache, where the Codex wire reads its whole
prefix (findings `compaction.md` K1). fake-llm.md 2.1 now puts the choice
in a marked query's prefix only. One increment; smith 07.1 repins it.

## Read first

- skein `docs/design/fake-llm.md`, section 2.1 ("A query's prefix").
- skein `programming-model.md`, section 3; `testing-strategy.md`,
  sections 2.1 and 4.1.

## What the design asks

"A query's prefix is its model, its tools and its system text, then, for
a marked query alone, its tool choice, then its messages. A changed choice
therefore keeps an automatic query's whole prefix, as the Codex wire does,
and only a marked query's tools and system text, as Anthropic's does."

## Rules that bind it

- **No smith edit** from this session; smith 07.1 repins.
- **Dependents' tests** (smith README, rule 6), in skein:
  `heavy cargo nextest run --workspace -E 'rdeps(skein-fake-llm-domain)'`.

## Increments

### 01.1 The tool choice in a marked query's prefix only
Scope: fake-llm.md 2.1.
Needs: nothing.
Files: `testing/skein-fake-llm-domain/src/cache.rs` (`tokens`, `:113`;
  `read`, `:126`; `write`, `:149`; `Prefix::system`, `:205`; the digest,
  `:248`) and its tests (`:320-402`).
Shape: `prefix.choice` enters the digest only under `Caching::Marks`;
  an automatic query's prefix is its model, tools, system text and
  messages.
Test shape: in `cache.rs`'s tests, beside
  `marked_reads_stop_at_the_frontier_and_choice_changes_keep_only_system_hits`
  (`:369`), which keeps passing: `an_automatic_query_keeps_its_whole_prefix_across_a_choice_change`,
  written as its neighbours are. Under 20 ms.
Proof: unit, that test; skein's gate.
Don't: change marked caching.
Size: 20 lines.
Removes: the choice in an automatic query's prefix.

## Done when

- `heavy cargo nextest run -p skein-fake-llm-domain` passes, once.
