# Live-text plan: later

2026-10-19. What the design names, or the run will need, that this plan
does not do, and when it should be done.

| Item | Design | When |
|---|---|---|
| jig's part (jig `f19a100f`): the engine and worker half follows smith's; the client wire's `kind` waits for the tools plan's jig 03 and 05, which edit the client wire v9 in place | jig's design at `f19a100f` | jig's own live-text plan, after this plan and the tools plan's jig 03 and 05 |
| temper's part (temper `4613daa11`): it needs the real loop and the terminal client from the tools plan's next part | temper's design at `4613daa11` | temper's own live-text plan, after the tools plan's next part |
| In process, the LLM component handing deltas to the host's owner directly, with no channel: smith's inline agent has no LLM component, and no host in smith runs one in process (A25) | smith `domain/host.md` 2 and 9.1 | with jig's part, whose engine runs the agent in process |
| The byte peer run as a process of its own, refusing a script with a hold at its start | skein `fake-llm.md` 2.2 (answer 2) | when such a process exists; the designer rewords 2.2 and 6 first (decisions.md, "For the designer") |
| Reasoning and a tool call's input as live content: the schema carries both kinds (decisions.md 2.3), and nothing produces them; `Measure::Delta` gains a kind with the second kind | smith `protocol/channel.md` 4, `protocol/llm.md` 6 | the plan that builds a second kind |
| The live bytes not projected in the event stream: a field beside `loss.channel` | smith `protocol/channel.md` 7, `protocol/events.md` 3.2 and 6 (answer 1) | after the designer's next pass, outside the accounts' version 4 |
| A scripted failure after a reply's pieces (decisions.md 5.2) | skein `fake-llm.md` 2.3 | the LLM accounts' skein part |
| jig's and temper's repins of smith: `live: None` in the start's 5 struct literals, and an arm for `OpenEvent::Live` in both `worker_route.rs` (A32) | development.md 5.7 (answer 4) | the tools plan's closing jig-repin and temper-repin, or their next repin of smith |
