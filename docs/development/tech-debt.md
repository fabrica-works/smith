# Technical debt

What the code owes, and what paying it back takes. Each entry says what
was seen, why it matters, and what pays it back. An entry leaves this
file when it is paid.

## From the reliability plan, 2026-10-10

A quick look at the plan's work so far: about 247 smith commits since
`30259d8`, and 76 skein commits since `e8bc018`.

**The verdict:**
- **Proper work, not hacks.** The spikes' raised constants became the
  mechanisms the design asks for:
  - a run identity sent as Codex's cache affinity;
  - a finishing reserve, child shares and a final turn;
  - deadlines by phase;
  - limits derived from declared quantities and checked at startup.
- **Consistent with the design.** Commit messages have bodies, the
  suites keep their budgets, and design questions were raised, not
  guessed.

Four kinds of debt came with it.

### Implementation follows the design's letter

- **Seen.** Where the design is silent, the code takes the most literal
  reading. "Constants set from captures" became bounds at exactly the
  largest capture: a response head of 1,935 bytes and 42 fields. At
  `61d74a29` that failed every live Codex call.
- **Why it matters.** Every gap in the design becomes a defect, not a
  question.
- **Pays it back:**
  - **The design states margins and intent,** not only sources. The
    first instance is skein's `llm.md`, sections 4.2 and 7: bounds on
    what a provider sends have headroom over their captures.
  - **A design review looks for every bound** taken from an observation,
    and gives each its margin.

### Proven against fakes, not providers

- **Seen.** Every merged behaviour was proven by deterministic worlds
  over fake providers, a fake machine and a fake TLS transport, and
  none of it live. The fakes drifted from reality without anyone
  seeing:
  - the fake TLS transport dropped the rest of a record past 4,096
    bytes;
  - the fake provider's response heads were smaller than Codex's.
- **Why it matters.** The risk sits where the fakes meet providers, and
  no deterministic test can see it.
- **Pays it back:**
  - **A change is proven before it merges** (workflow.md, section 6):
    `smoke-tools` live on every merge, and each improvement's probe
    before and after;
  - **the fakes take their shapes from real captures,** such as response
    heads, their header counts and event sizes, and are checked against
    fresh captures as providers change;
  - **the plan's proving phase** proves each improvement merged before
    2026-10-10 on current `main`. Until then, those increments are
    "merged, unproven".

### Volume and churn

- **Seen.** smith grew about 84,000 lines, less 17,000 removed, in about
  a day and a half. Many commits were fixture and test plumbing,
  re-preserving fixtures as the code beneath them moved.
- **Why it matters.** It is hard to review, and the weight of the tests
  may slow later changes more than it protects them.
- **Pays it back:**
  - **a review pass per area** once the proving phase ends, looking for
    tests that repeat one another, fixtures that pin implementation
    details, and helpers duplicated across worlds;
  - **shared mechanisms move into skein's test kit,** as AGENTS.md
    already requires.

### Strict-subset exceptions grew

- **Seen.** In the production domain crates, the `while` loops and the
  `expect(clippy::…)` suppressions grew from 90 at `30259d8` to 102.
  The rules forbid `while` (AGENTS.md; skein's programming-model.md).
- **Why it matters.** The exceptions are few, but each one is a rule the
  compiler cannot check, so it erodes without review.
- **Pays it back:** one audit of the domain crates. Remove each
  exception, or justify it beside the code.
