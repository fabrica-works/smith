# Findings: prompts and tool descriptions

2026-10-14. What smith's model is told against what Codex's is told, the system text, the tool set and the tool descriptions, and which differences the fixture and repository records tie to measured behaviour. Written by a read-only sub-agent for the parity plan's designer (skein's development.md, section 4.1).

Sources: smith `main` at `689c15d0`; skein at its checkout; raw attempts under `~/.local/state/smith-bench/runs/`: `compare-a57ca3a8-20261013T221135Z` (smith 01-05, 11-15, 21-25; Codex 06-10, 16-20, 26-30) and `repository-a64f0523-20261013T230741Z` (smith 01-03, Codex 04-06). On smith's side the records read are `response.started`, `response.completed` and `tool.*` in `trace.jsonl`. On Codex's side they are `session_meta`, `response_item` and `token_usage_record` in `rollout-*.jsonl`. `~/.codex/models_cache.json` was also read, and the strings of the pinned binary `~/.local/state/smith-bench/tools/codex-0.160.0/bin/codex`, read with `strings` and never run. Every number comes from a `jq` or `python3 -I` query over those files. Codex's instructions are summarised here, not quoted. Line numbers "base l.N" count lines of the `base_instructions.text` recorded in compare 06's rollout.

## 1. In one page

- **smith's model works without instructions.** The compare and repository arms run with `instructions = ""`, inherited from the first-request probe's template (harness findings, T4). Even the product default is a single 96-byte line. Codex's model gets 21,429 bytes of base instructions, the same bytes in all 15 fixture rollouts (sha256 `d4b5b114…`), plus 4,415 + 2,429 + 271 bytes of developer messages. smith's whole system text is 2,035 bytes, and only 1,641 of them are smith's own mechanics. (P1)
- **Codex's text gives guidance on 9 topics where smith's says nothing:** persistence and scope, planning, `rg` first, which tests to run, shell hygiene, progress updates, the final answer's form, when to delegate, and compaction. On batching, smith has only its descriptions' one sentence per tool. smith's text covers mechanics only: tools, checkout, sub-agents, finishing and the budget. The gaps that line up with measured behaviour are orientation and the habits the model brings from Codex. (P2)
- **The working directory is never named as `.`, and `""` is refused without a fix.** In 6 of 15 smith fixture attempts the first call was `list {"path":""}`. Each got the answer `invalid field value "path"` and cost one completion. smith needed 39 responses before its first change over the 15 attempts; Codex needed 30, two in each of its 15. The one-directory `list` description still offers "List / for directory names", and attempt 02 took it. (P3)
- **The descriptions are terse, and one mixes its units.** `timeout_seconds` is described as "Seconds; default deadline 120000 ms; effective maximum 1200000 ms". `search` never says it runs `rg` or that its pattern is a regex. No description says what happens to later calls in a batch after one of them fails. No probe tests the descriptions: `tool-output/timeout` dictates its values in its prompt, and the trace records tool names only. In 123 shell calls no model asked for a confused timeout, so the cost here is risk, not measured harm. (P4)
- **Most of smith's extra reasoning goes into making the change.** Over 15 fixture attempts each, smith used 8,236 reasoning tokens and Codex 4,437. Of the 3,799 extra tokens, 2,053 (54%) went to the first change, 990 (26%) to later changes, 541 (14%) to the finish and 215 (6%) to tests; orientation used none. smith rewrote the main source with a whole-file `write` in 10 of 10 attempts on fixtures 02 and 03, where Codex used one `apply_patch` Update. smith made no batch of two or more changes or commands in any of its 117 fixture responses. Codex's patch and its test ran in one `exec` in 15 of 15 attempts. On the repository task smith changed files through `python3 - <<'PY'` and `cat >>` scripts in the shell: 4, 8 and 8 times, with 12, 0 and 0 `edit` calls. Codex never edited through the shell and made 5 to 11 `apply_patch` calls per attempt. (P5)
- **Finishing costs reasoning that Codex's final message does not.** On fixture 02, smith spent 518 reasoning tokens in its `finish` responses against Codex's 31. The contract text tells a report with no required fields about "host-required fields: (none)" and "Extra fields are allowed …". (P6)
- **The repository grade failure is not a prompt gap.** smith 03 checked the raw spelling of `cwd` against the bound before normalising it. The hidden test `feature_normalized_path_limit_is_inclusive` wants a normalised path at the bound to spawn. Nothing in either agent's text bears on it. (P7)
- **Nobody owns the text, and nothing guards it.** The design names the system text's sections (run.md 3.3) and the descriptions' facts (llm.md 3), but not their words. The role is "smith's own" (shell.md 5.2) and specified nowhere. The edit-format experiment (benchmarks.md 11.3) has never run; this machine holds no `edit-format` run directory. (P8)
- **What to do first for parity:** the role text (P1), the working directory (P3), then the edit-format experiment (P5). Measure before adopting: a patch-format edit tool, and instructions longer than a few kilobytes.

## 2. What each model sees

| Part | smith (bytes, source) | Codex (bytes, source) |
|---|---|---|
| Role instructions | 0 in the arms (`instructions: ""`, `benchmarks/agents/smith/first-request.json:20` through `compare.pin.toml:5`); the product default is 96 bytes, `crates/smith-local-shell/src/settings/resolve/local.rs:147-149` | 21,429 bytes, about 5,360 tokens: `session_meta.payload.base_instructions`, provenance `model gpt-6-astra`; byte for byte `models_cache.json`'s `instructions_template` (`cmp` equal) |
| Mechanics | 1,641 bytes, about 410 tokens: `## Tools` 155, `## Checkout` 170, `## Sub-agents` with the `wait` line 574, `## Finishing` 376, `## Budget` 366 (compare 01, response 1, `prompt.system`); `prompt.rs:344-392` | developer messages: skills, permissions, collaboration mode and plugins, 4,415; `multi_agent_role` 2,429; `multi_agent_mode` 271 (compare 06) |
| AGENTS.md | 394 bytes in the system text, under `## AGENTS.md in the working directory` (`prompt.rs:432-451`); 2,912 bytes of guide on the repository task (harness T9) | a user message, `# AGENTS.md instructions for <cwd>`, with the text inside `<INSTRUCTIONS>`, then `<environment_context>`: 1,870 bytes (compare 06) |
| First user message | `person: ` and the task, 633 bytes of JSON (compare 01) | the task, 571 bytes |
| Tools offered | 9: `read`, `list`, `search`, `write`, `edit`, `shell`, `wait`, `finish`, `sub_agent`. Descriptions 1,708 bytes and schemas 3,238 bytes, rebuilt from `descriptions.rs:48-181` and `tools.rs:90-121` with `check.json`'s effective values, plus `finish`'s contract schema. All are JSON function tools with `strict: false` (skein `translate.rs:53-67`) | a freeform `exec`, which runs JavaScript, and the six collaboration tools that `multi_agent_role` names. `exec_command`, `write_stdin` and `apply_patch` are called inside `exec` (357, 43 and 46 calls in 24 rollouts). No definition is recorded in a rollout. |
| First request | 11,027 bytes; 1,664 input tokens (compare 01, response 1) | 14,756 input tokens (compare 06). The recorded text is 31.0 KB, about 7,750 tokens, so about 7,000 tokens are tool definitions and framing |
| Per-turn additions | none in the 15 fixture attempts: no text block after the opening in any `prompt.messages`. Repository: two compaction summaries per attempt | none: one `turn_context`, one `world_state` |
| Model settings | effort `high`, `text.verbosity` `low`, `parallel_tool_calls` true, no reasoning summary (skein `translate.rs:70-100`) | effort `high`, summary `none`, `default_verbosity` `low`, `tool_mode` `code_mode_only`, `apply_patch_tool_type` `freeform`, `shell_type` `unified_exec`, `truncation_policy` 10,000 tokens (`models_cache.json`, written by client 0.162.1, a later client than the run's 0.160.0) |

Each topic, side by side:

| Topic | Codex | smith |
|---|---|---|
| Persistence, scope | base l.15-25: infer intent and scope, carry the task to completion, no partial solution | none (product line: "Carry out the person's request …") |
| Planning | no plan tool; base l.21: do not stop at proposing a plan | none |
| Searching | base l.114: `rg`, `rg --files` first | `search`: "Search files; max 100 matches, 16384 bytes" |
| Parallel calls | base l.115-116: batch independent reads and searches in one `exec` with `Promise.allSettled`; keep edits and dependent steps sequential | descriptions: "Batch independent reads, lists and searches side by side." and "Batch writes, edits and commands; run serially in call order." |
| Reading in chunks | none; output is truncated at 10,000 tokens | `read`: byte windows, "Next line, cuts and clamps are reported." |
| Editing | the freeform `apply_patch` grammar; none in the text | `write` needs a read; `edit` needs one match; a large file is written in pieces |
| Tests | base l.125-126: meaningful tests only; run those the change needs, broaden only for cause | none (the seed's AGENTS.md asks for tests) |
| Shell hygiene | base l.117-122: no echo separators, quoting, no sleeps over 60 s, keep `HOME` | none |
| Progress | base l.61-81: commentary channel, at most 60 s without an update | none |
| Final answer | base l.83-110: outcome first; formatting rules | `## Finishing` and the contract |
| Delegation | developer: spawn only when asked; 7 slots | `## Sub-agents`: how to delegate, not when |
| Compaction | base l.69-71: compaction does not end the task | the summary instruction only |
| Sandbox | developer: `workspace-write`, approval `never`, the writable roots | `## Checkout`: which directories may change |
| Reverting user changes | none in this version | none |

## 3. Entries

### P1 smith's model works without instructions

- **Symptom:** compare 01's system text opens at "## AGENTS.md in the working directory"; there is no role text before it. The repository attempts open the same way. Codex opens with 21,429 bytes. Both agents received the same seed `AGENTS.md`.
- **Root cause:**
  - The arms take `"instructions": ""` from the first-request probe's template (`first-request.json:20`; harness T4 covers the pin).
  - The product's own role is one line, `local.rs:147-149`. shell.md 5.2 (`:567`) gives the default as "smith's own, for a chat or for a task", but the code has one text for both, and no design text says what it holds.
  - run.md 3.1 (`:72-74`) defines instructions as "the role, as text: a chat, a researcher, a coordinator, …". run.md 3.3 (`:140-150`) fixes the mechanics that follow them. Neither puts working guidance anywhere.
  - benchmarks.md 6.1 (`:338-339`) says "smith's pinned settings add nothing". In fact they remove smith's one line.
- **The clean fix:**
  - The arms run on the product default (harness T4).
  - The default becomes a task role that smith writes in its own words and on its own topics: persistence and scope, finding files, batching, how to change files with smith's tools, which tests to run, progress, the report, and compaction (P2). It names only tools smith offers.
  - shell.md 5.2's default cell becomes "the preset's task role (6.6)". A new shell.md 6.6, "Instructions", states the topics and owns the text per model family (P8).
  - benchmarks.md 6.1's sentence becomes "smith runs with its shipped role; its pin adds nothing".
- **Its tests:**
  - `smith-local-shell`, `src/settings/tests.rs`: `the_default_task_role_names_each_topic_shell_md_lists` asserts that every topic's heading is present and that only offered tool names appear.
  - `smith-domain-run`, `src/prompt.rs` tests: `main_system_text_puts_the_role_before_guides_and_mechanics`.
  - `benchmarks/tests/smith_adapter.rs`: `a_comparison_arm_renders_no_instructions_key` (harness T4's test covers the rest).
  - Live: `compare`, 01, 02 and 03 at 5 attempts per arm, interleaved. Arms: smith with `""`, smith with the new default, and Codex. Count responses, responses before the first change, reasoning tokens, and grades.
- **Confidence:** high that the gap exists. Medium that closing it moves the counts: the model already behaves much as Codex does without being told (it writes commentary text, for example in compare 14). Two arms of 5 would show it.

### P2 Codex's guidance smith lacks, and what it plausibly explains

- **Symptom:**
  - **Orientation:** smith used 39 responses before its first change in 15 fixture attempts; Codex used 30 (2 in each). The extra 9 were 6 invalid `list ""` calls (P3), one `list /` (02), and `pwd; ls -la; git status --short` as a response of its own (02, 04).
  - **Batching:** no smith response holds two or more of `write`, `edit`, `shell` and `finish` (0 of 117 fixture responses; 4 of 171 on the repository task). Every Codex attempt applies a patch and runs a command in one `exec` (15 of 15).
  - **Shell habits on the repository task:** smith reads through `cat` and `sed -n` in 20, 25 and 27 shell calls per attempt, and changes files through `python3 - <<'PY'` with `str.replace` and `cat >>` in 4, 8 and 8. It made 12, 0 and 0 `edit` calls; Codex's 3 attempts had 0 shell edits. Of smith's shell outputs, 6 of 26, 13 of 26 and 16 of 36 were cut, at budgets the model chose of 2,500 to 7,000 bytes.
- **Root cause:**
  - smith's text carries none of the topics in section 2's table. The model, which is trained on Codex's harness, falls back to Codex habits: shell reads, patch-like edits through Python, and `cat` of several files at once.
  - Codex composes calls in JavaScript (`tool_mode: code_mode_only`). There a failed `apply_patch` throws and stops the script. smith's descriptions invite batching but never say what happens to the calls after a failed one, and domain/session.md 5 is silent too. A model that cannot tell whether the test would run against a failed edit has a reason not to batch.
  - Link strength: medium for orientation; low to medium for batching, since the format (P5) explains it as well; high that the repository habits come from the edit format and the absent guidance together.
- **The clean fix:**
  - smith's role (P1) tells the model:
    - to find files with `list` or `search` before reading;
    - to read several files in one response;
    - to change files with `edit` or `write`, never through shell scripts;
    - to put a change and its test in one response;
    - to give `shell` a budget large enough for what it reads.
  - The descriptions of `write`, `edit` and `shell` state what happens after a failed call in the same response. If later calls still run, the design must decide whether they should; that is section 4, Q5.
- **Its tests:**
  - `smith-protocol-llm`, `tests/protocol-llm/tests/production_tools.rs`: `mutating_descriptions_say_what_follows_a_failed_call`.
  - Live: as P1. Add the repository suite at 3 attempts per arm, counting shell-modifying commands (the regex in this file's queries: `python3? - <<|cat >>? |sed -i|tee`), `edit` calls, and cut shell outputs.
- **Confidence:** medium. The habits are measured; that instructions change them is not.

### P3 The working directory: `.` unnamed, `""` refused, `list /` offered with one directory

- **Symptom:**
  - Compare 02, 04, 05, 13, 21 and 24 opened with `list {"path":""}` (`response.completed.invalid` 1). Each got `invalid field value "path"` (compare 13, response 2's `result`) and spent one completion on it.
  - Compare 02 then called `list {"path":"/","depth":2}` and got 4 bytes back, the hidden directory's name.
  - Fixture 01's smith responses had a median of 7, against Codex's 5.
- **Root cause:**
  - `descriptions.rs:105` describes a path as "Relative to the working directory, e.g. src/lib.rs". `prompt.rs:478` says "Use `src/lib.rs`, without a directory-name prefix." Neither names `.`.
  - The decoder refuses an empty relative path (`decode.rs:447-449`), and the refusal does not say what to write instead.
  - tools.md 2 (`:61-64`) says "The directory itself is an empty path within it, and `.` at io …", so the domain accepts what the wire refuses.
  - `descriptions.rs:60` appends "List / for directory names." in every namespace. tools.md 2 (`:51-54`) says "One directory hides its name".
- **The clean fix:**
  - Path descriptions say "`.` is the working directory" (with several directories, also "`/name` is a directory's root").
  - `""` decodes as `.` (Q4); failing that, its refusal says "use `.` for the working directory".
  - "List / for directory names" is written only for `Namespace::Several`.
  - llm.md 3's paths sentence (`:128-131`) gains: "with one directory, `.` names it, and `list /` is not offered as a way to find names".
- **Its tests:**
  - `production_tools.rs`: `one_directory_path_descriptions_name_dot_and_omit_list_slash`, `several_directories_keep_list_slash_for_their_names`, `an_empty_path_decodes_as_the_working_directory_itself`.
  - Live: `compare` 01, smith arm, 5 attempts before and after: invalid calls (3 of 5 before) and responses before the first change (median 3 before). Also `paths/single`, 3 attempts.
- **Confidence:** high. The calls and their answers are recorded.

### P4 Tool descriptions: terse, one unit mixed, and never proven

- **Symptom:**
  - The shell schema (`descriptions.rs:166-178`) renders `timeout_seconds` as "Seconds; default deadline 120000 ms; effective maximum 1200000 ms, within the remaining session time."
  - `search` says "Search files; max 100 matches, 16384 bytes; cuts are marked." Its pattern is described as "Search pattern."
  - `shell` says "Run a command; files may change." It does not say that the command runs under `sh`, where it runs, that the exit code comes back, or that what is kept is a head and a tail.
  - `read` does not say that its lines come numbered `N: ` or that a page continues from `offset`.
  - `edit.maxItems` is the tool payload in bytes (40,960) used as a count.
  - `sub_agent`'s `llm` has no description; its models are listed only in the system text.
  - No measured harm: of 123 shell calls, the requested `timeout_seconds` were absent (77), 120 (31), 180 (7), 200 (7) and 150 (1), and none was clamped.
- **Root cause:**
  - llm.md 3 (`:123-127`): "Every parameter has a description, and each limit it meets is given with its unit and its maximum". The code meets the facts but not the units, and nothing checks the words.
  - The `tool-output/timeout` prompt states "timeout_seconds=1, then … timeout_seconds=9999" (`benchmarks/tasks/probes/tool-output/task.toml:139`), so it tests clamping, not reading.
  - events.md 3.11 (`:430-431`) records "the names of the tools offered", so no attempt shows what a description said.
  - Codex's parameters state their defaults with their units (binary strings: "Maximum command runtime. Defaults to 10000 ms.").
- **The clean fix:**
  - Each parameter states its unit once, the parameter's own: "seconds; default 120, at most 1200".
  - `search` names `rg`, its regex syntax and its default path `.`; `shell` names `sh`, the working directory, the exit code and the head and tail; `read` names its line numbers and how to page.
  - The trace records a digest and the byte length of each offered tool in `conversation.opened`, so that an attempt shows which text it saw.
  - llm.md 3 gains: "Each parameter's limits are stated in that parameter's unit."
- **Its tests:**
  - `production_tools.rs`: `the_shell_timeout_is_described_in_seconds_throughout`, `search_description_names_rg_regex_and_default_path`, `every_offered_schema_matches_its_reviewed_golden_text` (the golden holds all nine definitions at the standard profile).
  - Probe: a `tool-output/timeout-told` variant whose prompt says only "use the longest timeout you are allowed". It passes when the request equals the stated maximum in seconds and is not clamped.
- **Confidence:** high on the defects; low that fixing them moves the fixture counts.

### P5 The edit format: whole-file writes, Python patches, and the extra reasoning

- **Symptom:**
  - Fixtures 02 and 03: smith's first change was a whole-file `write` of `src/decoder.rs` (2,054-2,208 bytes) or `src/planner.rs` (3,386-3,606 bytes) in 10 of 10 attempts. Codex's was an `apply_patch` `*** Update File` in 10 of 10.
  - Reasoning at that response: smith 2,550 tokens against Codex's 1,748 on fixture 02 (5 attempts each), and 1,476 against 526 on fixture 03. Output was alike: 1,072-1,448 tokens against 792-1,161, except Codex 18 and 30, which wrote both files at once (2,999 and 3,041).
  - Fixture 01: smith used two `edit` responses (`src/lib.rs`, then `tests/regression.rs`) in 5 of 5; Codex used one patch over both files plus a test, in one `exec`, in 5 of 5.
  - Over all 15 attempts each, of smith's 3,799 extra reasoning tokens, 3,043 (80%) went to responses that change files.
  - Edits failed rarely: 0 of 32 fixture writes and edits failed. On the repository task, 2 of 12 `edit`s in attempt 01 matched nothing: a Markdown line indented by 3 spaces where the file had 4. Codex's 46 `apply_patch` calls ran in scripts that all completed.
  - The repository habits are in P2.
- **Root cause:**
  - llm.md 3 (`:141-145`): "Until a measurement chooses otherwise, every dialect is offered `edits` as old and new text." No measurement has been made: benchmarks.md 11.3's `edit-format` (`:598`) has no run on this machine, and its task text ("add a doc comment to each public function") is not 11.4's `03-planner`.
  - decisions.md 7.4: "The formats are chosen by measurement (T4)."
  - llm.md 12 (`:471-472`): "a patch as a free-form tool needs skein's Codex dialect to offer one". skein offers only `"type":"function"` tools (`translate.rs:57-58`); `rg` finds no custom-tool support in `skein-llm`.
  - For gpt-6-astra, Codex's model entry says `apply_patch_tool_type: freeform`.
- **The clean fix:**
  - Run `edit-format` on `compare` 01-03, with these arms:
    - (a) today's `edits`;
    - (b) a `patch` tool whose one string parameter takes Codex's patch grammar (Begin and End markers; Add, Update and Delete; `@@` hunks with context), decoded to the same typed `Edit` or `Write`. This needs no skein change;
    - (c) the same grammar as a freeform tool, once skein's Codex dialect offers custom tools.
  - Judge by reasoning tokens in change responses, change responses per attempt, failed changes, changes made through the shell (repository), and grades.
  - Adopt per dialect by the result, as tools.md 3 and decision 7.4 already say. Rewrite 11.3's task text to name the fixtures.
- **Its tests:**
  - If (b) is built: `production_tools.rs` gets `a_patch_updating_two_files_decodes_to_two_typed_edits`, `a_patch_adding_a_file_decodes_as_a_create_write`, `a_patch_whose_context_does_not_match_fails_whole_naming_the_hunk`, and `a_patch_within_the_payload_fits_one_call_bound`.
  - `smith-domain-tools`: `a_multi_file_patch_stores_each_file_once_or_none`.
  - Live: `edit-format`, 5 attempts per arm per fixture, interleaved, and the repository suite with 3 per arm.
- **Confidence:** medium that the format drives the reasoning gap and the shell edits; the experiment would settle it.

### P6 The finishing text and the final turn

- **Symptom:**
  - smith finished in a response of its own in 15 of 15 attempts, as Codex's final message does.
  - Reasoning in those responses: 23 against 0 (fixture 01), 518 against 31 (02, of which 324 in attempt 14), 31 against 0 (03).
  - smith's `finish` inputs are `{"form":"report","text":…,"fields":{}}` (compare 01, 13, 14).
- **Root cause:** `prompt.rs:515-546` writes "Report: text up to 4096 bytes, with these host-required fields:\n(none)\nExtra fields are allowed within the aggregate result byte limit; no field name may repeat." `finish`'s fixed description is "Declare the final result under the host's result contract." (`tools.rs:137-142`). Neither says what a good report holds. Codex's base l.49 does: what changed, why, how it was tested, risks.
- **The clean fix:**
  - When the contract requires no fields, the text says "Call `finish` with `form` `report` and a short report as `text`: what changed, how you checked it."
  - The field rules appear only when there are fields.
  - run.md 3.3's "how to finish (the contract, as the LLM must meet it)" (`:144-145`) gains "in the fewest words for the contract it has".
- **Its tests:**
  - `smith-domain-run` prompt tests: `a_report_with_no_fields_is_told_in_one_sentence`, and `field_rules_appear_only_when_the_contract_has_fields`.
  - Live: as P1, counting reasoning in `finish` responses.
- **Confidence:** low to medium: 518 tokens come from 5 attempts, and 324 of them from one.

### P7 The repository grade failure is not a prompt gap

- **Symptom:**
  - Repository 03 failed one hidden test: `feature_normalized_path_limit_is_inclusive` (`grade-0-stdout.log:15`). It panicked with "expected Spawn" because the call at the bound was answered `TooLong` (`:25-26`). The test's cwd is `/work/` followed by 123 `x` (`grader/src/lib.rs:191-201`).
  - Its diff rejects `cwd_bytes > limits.path_bytes` on the raw spelling before `authority::locate` (`diff.patch:91-95`). Its reviewer brief records the choice as "cwd_bytes original spelling length to reject redundant-slash overlong raw inputs" (third `sub_agent` input).
  - Smith 01 and 02 and Codex 04-06 passed.
- **Root cause:** a design reading of the task ("Reuse the same path normalization …"), made in main and passed on to the children. Neither agent's text bears on it.
- **The clean fix:** none in the prompts. The repository task's three attempts per arm cannot separate this from chance. Treat it as one failure in six until repeated.
- **Its tests:** none here.
- **Confidence:** medium that prompts are not the cause.

### P8 Nobody owns the text, and nothing guards it

- **Symptom:** in this run's records, the words the model sees are spread over five places:
  - the domain's literals (`prompt.rs:27`, `:45` and `:453-546`);
  - the protocol's descriptions (`descriptions.rs:48-181`, `tools.rs:90-142`);
  - the shell's one-line role (`local.rs:147-149`);
  - the seed's guide;
  - the harness's pin.

  No test or probe notices when any of them changes.
- **Root cause:**
  - llm.md 3 says what descriptions must state. run.md 3.3 says which sections appear and that "the domain decides what each says". shell.md 6 owns presets' values but not text.
  - benchmarks.md 4.2 requires a probe for "every improvement", but a change to the text names none, since no probe measures it.
- **The clean fix:**
  - Mechanics stay the domain's (run.md 3.3) and the descriptions the protocol's (llm.md 3), as now. Each is pinned by a golden file at the standard profile, so a change to the words is a reviewed diff.
  - The role becomes a preset per model family in `smith-local-shell` (shell.md 6.6, Q1), versioned with the other presets.
  - benchmarks.md 4.2 gains: "A change to the role, a section's words or a tool's description names the `compare` arms it ran, before and after, and its effect on responses, reasoning tokens and grades."
  - The trace's tool digest (P4) lets an attempt show which text it saw.
- **Its tests:**
  - `the_standard_system_text_matches_its_golden` (`smith-domain-run`).
  - `every_offered_schema_matches_its_reviewed_golden_text` (P4).
  - `the_codex_preset_role_matches_its_golden` (`smith-local-shell`).
- **Confidence:** high that the gap exists; the shape of the fix is the designer's (Q1).

## 4. Questions for the designer

- **Q1 Who owns smith's role text?**
  - Options: (a) domain literals, like the mechanics; (b) a preset per model family in `smith-local-shell`, specified in shell.md 6.6; (c) settings only, with an empty default.
  - Recommendation: (b).
  - Why: run.md 3.1 makes instructions the host's choice of role. Model families differ in what they need: Codex's own text is per model ("provenance: model"). The mechanics are what the domain enforces, so they stay in run.md 3.3.
- **Q2 Do the arms run with the shipped role?**
  - Options: (a) shipped role; (b) empty, as now.
  - Recommendation: (a), with an empty arm only inside P1's proof.
  - Why: benchmarks.md 12 says "It runs smith as a person would". Codex runs with its product text.
- **Q3 Which edit formats go into the experiment, and does skein's custom-tool support join the parity plan?**
  - Options: (a) old and new only; (b) add patch-in-a-JSON-string; (c) also freeform, which needs a skein increment first.
  - Recommendation: (b) now. Put (c) in the plan as a skein increment that is adopted only if (b) shows a gain and (c) beats (b).
  - Why: (b) costs no skein change and tests the grammar. (c) tests the channel the model is tuned to (`apply_patch_tool_type: freeform`).
- **Q4 What does `""` mean as a path?**
  - Options: (a) the working directory, decoded as `.`; (b) refused, with a message that says to use `.`.
  - Recommendation: (a).
  - Why: tools.md 2 already calls the directory itself an empty path. It cost 6 of 15 attempts one completion each.
- **Q5 Should a failed call in a batch stop the calls after it?**
  - Options: (a) later calls run, as now, and the descriptions say so; (b) later mutating calls are answered as not run.
  - Recommendation: (b), measured by P2's batch rate.
  - Why: it is what Codex's composition gives the model, and it makes "a change and its test in one response" safe.
- **Q6 How long may the role be?**
  - Options: (a) a few hundred bytes; (b) up to a stated bound, such as 8 KiB, checked as part of smith's own part (protocol/limits.md, `fresh-window`).
  - Recommendation: (b).
  - Why: a cached prefix is cheap. Codex sends 14,756 tokens in its first request against smith's 1,664, and was no slower on fixture 01 (median 49 s against 50 s).
- **Q7 May smith reuse Codex's wording?**
  - Options: (a) adapt its text; (b) write smith's own on the same topics.
  - Recommendation: (b).
  - Why: Codex's text names Codex and tools smith lacks (`functions.exec`, skills, the commentary channels). smith's has to describe smith's own tools and contract.

## 5. What other themes must say

- **Harness (T4):** with the arms on the product presets, P1's live proof has a baseline. Until then, the 131,072 window (`benchmarks/src/agent/smith/settings.rs:148`) and the 4,096 output cap (`first-request.json:22`) confound any comparison of prompts: the repository attempts compacted twice each.
- **Tools:** `list .` on the repository failed in 3 of 3 attempts with the fixed message `operation failed: io error` (repository 01, 02 and 03; 5 times in 02, children included). tools.md 4 says "a failure comes back with its output, not a fixed message".
- **Harness:** `counts.tool_failures` counts `list` and `read` failures but not `edit` `no_match`. Repository 01 shows `list: 1` and leaves out 2 failed edits.
- **Events:** record each offered tool's digest and byte length (P4). events.md 3.11 records names only.
- **Benchmarks:** 11.3's `edit-format` task text needs to match 11.4's fixtures. `tool-output/timeout` dictates its values and so cannot test descriptions (P4).
- **Delegation:** smith's `## Sub-agents` says how to delegate but not when. Repository 02 and 03 spent 178 s and 131 s of main's wall time in `sub_agent` calls. Codex's developer message forbids spawning unless the user asks (the repository prompt allowed it).
- **Latency:** on the fixtures, smith's wall time is model time (compare 11: 98 s of responses in a 100 s run; 1 s of shell). Its excess is extra responses (P3, P5) and reasoning (P5, P6), not tools. On the repository task, shell time (76-288 s) and children (0-178 s) add to model time (356-385 s).
- **Shell output:** the head-and-tail split at the model's small budgets cut 35 of 88 repository shell outputs. If smith keeps a read path through the shell, the tools theme should weigh a budget in tokens or a whole-head default for `cat`-like reads. Codex truncates at 10,000 tokens.
