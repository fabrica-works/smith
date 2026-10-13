# The local shell

The product runs a workspace chat with `smith`, or one headless activation
with `smith exec`. Its strict TOML settings, commands, accounts and durable
outputs are described in [Running the product](../../docs/development/local-host.md).

The binary and hosted worlds share native local composition and one Skein
loop. Inline composition takes the generated configuration as a value and
writes no agent file. A library host choosing `compose::AgentKind::Spawned`
launches `smith-agent` with generated JSON, its derived exit grace and an
agent trace at the product trace path with `.agent` appended.

The focused binary tier is `tests/end-to-end/end_to_end.rs`. Cargo supplies
the binaries; the shared real-ring loop drives the attached pseudo-terminal
person, fake provider, issuer and browser. Fixtures own scratch HOME, XDG
settings, private tokens and the checkout. The referee observes public
terminal and peer outcomes, durable chat records and real git changes.
A first sign-in story ends with a checked commit; a second invocation
preserves the chat and shows its history reaching the provider. Neither
story reads service state or uses a real issuer.
