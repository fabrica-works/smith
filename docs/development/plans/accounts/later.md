# accounts: later

What the design names that this plan does not build, and what it leaves
to the repositories downstream (development.md, 5.7).

| Item | Design | When |
|---|---|---|
| smith's accounts by source, `smith login` and `logout`, the presets and smith's own registration, the credentials directory, numbers from names | smith `shell.md` 4.1 to 4.6, 5.2 to 5.4, 6.1, 6.2, 7, 9; `domain/host.md` 2, 7, 9.1; `protocol/hosts.md` 2, 5.2 to 5.4, 5.6 | `accounts-sources`, from this plan's `origin/main` (decisions.md, 1; outline in `findings/next/`) |
| Claude Code as a program backend, `smith-protocol-program`, the fake program | smith `programs.md` 1 to 9; `domain/run.md` 4, 9, 9.1; `domain/tools.md` 10; `protocol/agent.md` 1, 4; `protocol/README.md` 5; `protocol/events.md` 3.10's program completions | `accounts-sources`; a program-source grant is refused until then (decisions.md, 7) |
| The live suite on smith's own accounts, and the benchmarks' logins | smith `testing.md` 2.3, 4; `benchmarks.md` 6, 7, 10, 11.1 | `accounts-sources` (decisions.md, 10) |
| A capture of OpenAI's public route: its error codes, resets and headers, checked against llm.md 4.8's table and the fake LLM's rendering | skein `llm.md` 4.8; `fake-llm.md` 2.3, 7 | when the user has an OpenAI API key (decisions.md, 9) |
| A live check of Anthropic's key route | skein `llm.md` 4.7 | `accounts-sources`' live suite |
| The withdrawn forms removed: skein's Codex route with its account id and Anthropic's bearer route; smith's id-and-bearer grant and the `codex` dialect | skein `llm.md` 4.6, 4.7; smith `protocol/channel.md` 6, `protocol/llm.md` 7 | the second part of LLM accounts, once their last caller has moved (decisions.md, 2) |
| jig's part of LLM accounts: the driver's new failures and events, the refused notice, a grant by scheme instead of the withdrawn form, the OAuth record's new fields and the lock wait's value | skein `oauth.md` 6; smith `protocol/channel.md` 4, 6 | the second part; until then jig-repin maps them onto today's outcomes (decisions.md, 16) |
| The same, for the repository downstream of jig, with the real loop's use of the fake LLM's credential list and the in-process issuer | skein `fake-llm.md` 6; `oauth.md` 6.8 | the second part, and that repository's real-loop plan; its repin keeps today's behaviour (decisions.md, 16) |
| The fixed hash that numbers accounts in charters and transcripts | smith `shell.md` 7.4 | `accounts-sources` (decisions.md, Design questions 14) |
| skein's `a_keep_time_after_the_last_word_fails_the_world_naming_the_process` (`tests/world/tests/real.rs`) made steady: its 5 ms deadline taken after the world's setup, not before | none: a test defect | the next skein plan that changes `tests/world` (ruling 116) |
| The driver's `FileLower::Remove`, gone once io's requests take their roots by token | skein `io.md` 5; `oauth.md` 6.4 | the plan of io's roots by token (skein's `STATUS.md`, section 3; decisions.md, 18) |
| jig's race in its client: `tests/client-browser/tests/browser_tier.rs` `under_a_prefix_the_inbox_s_links_carry_the_prefix` misses `Cancelled` at person step 9 (snapshot `Live`, "Behind") now and then, on the old pins too; 20 focused runs found no cause. Look first at the order in which the client applies the durable `Cancelled` answer and the task watch's `Gone` terminal | none: a defect in jig's client, below the browser | jig's next plan that changes its client or task watch; until it lands, the README's "A known timing flake" lets that story's gate run again (rulings 139, 145) |
