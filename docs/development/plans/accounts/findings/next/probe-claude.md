# Claude Code probe, for accounts-sources

This probe answers the designer's open point for programs.md, sections 5.4 to 5.6 and 8. It ran on x1 on 2026-10-19, with Claude Code 2.1.303 on a Max subscription and the `haiku` model. The next plan's program backend and fake `claude` take these shapes, and its live suite checks them again.

The runs strip every `CLAUDE_CODE_*` and `CLAUDECODE` variable, plus `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN` and `CLAUDE_CODE_OAUTH_TOKEN`, from the environment (`$STRIP`). They run from an empty directory. The MCP server is a 40-line Python stdio server. It logs each line both ways, serves one tool, `prompt`, denies its first call and allows the rest.

## Sign-in status

```
$ claude auth status
{"loggedIn": true, "authMethod": "claude.ai", "apiProvider": "firstParty",
 "analyticsDisabled": false, "projectsDirectory": "...", "configDirectory": "...",
 "email": "<redacted>", "orgId": "<redacted>", "orgName": "<redacted>",
 "subscriptionType": "max"}
exit 0
$ CLAUDE_CONFIG_DIR=<empty dir> claude auth status
{"loggedIn": false, "authMethod": "none", "apiProvider": "firstParty", ...}
exit 1
```

Signed in exits 0, and not signed in exits 1. Both print JSON on standard output. "Signed in otherwise" (programs.md, section 3) can be told from `authMethod` and `subscriptionType`.

## The prompt tool

```
$ cd cwd && env $STRIP claude -p --model haiku --output-format stream-json --verbose \
    --max-turns 6 --tools Bash --mcp-config ../mcp.json --strict-mcp-config \
    --permission-prompt-tool mcp__probe__prompt --permission-mode default \
    --setting-sources project --settings ../ask-settings.json --json-schema "$SCHEMA" \
    "Use the Bash tool to run: echo probe-one. If that is denied, ... echo probe-two. ..."
exit 0
```

Here `ask-settings.json` is `{"permissions":{"ask":["Bash"]}}`, and `$SCHEMA` is `{"type":"object","properties":{"answer":{"type":"string"}},"required":["answer"]}`.

- **The call it gets:**

  ```
  {"method":"tools/call","params":{"name":"prompt","arguments":{"tool_name":"Bash",
   "input":{"command":"echo probe-one","description":"Print probe-one"},
   "tool_use_id":"toolu_01KM..."},"_meta":{"claudecode/toolUseId":"toolu_01KM...",
   "progressToken":2}},"jsonrpc":"2.0","id":2}
  ```

  The arguments are `tool_name`, `input` (the tool's input, as the model wrote it) and `tool_use_id`.
- **Its answer** is one text content block holding JSON:
  - to deny: `{"behavior":"deny","message":"<text>"}`. The model sees a tool result with `is_error: true` whose content is the message. The final `result`'s `permission_denials` lists the call, as `{tool_name, tool_use_id, tool_input}`.
  - to allow: `{"behavior":"allow","updatedInput":{...the input unchanged...}}`. The tool runs.
- **The ask rule is needed.** Without it (the same run with no `--settings`), `echo` is allowed as read-only and never reaches the prompt tool. `touch` does reach it. With `ask: ["Bash"]`, `echo` reaches it too. This confirms programs.md 5.3: every built-in call reaches the prompt only when the settings say ask.

## The MCP handshake

Claude Code first sends `server/discover`, with `_meta` protocol version `2026-07-28` and client `claude-code` 2.1.303. It then sends `initialize`, with `protocolVersion` `2025-11-25`, capabilities `roots` and `elicitation`, and then `notifications/initialized` and `tools/list`. A server that answers `initialize` with `2025-11-25` and ignores `server/discover` is enough.

## The stream

- **Message types:** `system/init`, `system/status` (`requesting`), `stream_event` (with `--include-partial-messages`: `message_start`, `content_block_start`/`_delta`/`_stop`, `message_delta`, `message_stop`), `assistant`, `user` (tool results), `rate_limit_event`, and `result`. With user settings, `system/hook_started`, `system/hook_response` and `system/thinking_tokens` come too.
- **`system/init`'s keys** include `tools` (`["Bash","StructuredOutput"]` for `--tools Bash` with `--json-schema`), `mcp_servers` (`[{"name":"probe","status":"connected","source":"dynamic"}]`), `permissionMode`, `apiKeySource` (`none`), `claude_code_version` and `capabilities`. `capabilities` is a list of words, such as `interrupt_receipt_v1` and `sdk_mcp_tools_list_changed`. The check (programs.md 5.5) must allow `StructuredOutput` among the tools when `--json-schema` is given.
- **`rate_limit_event`:**

  ```
  {"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1792404000,
   "rateLimitType":"five_hour","overageStatus":"rejected","overageDisabledReason":
   "org_level_disabled","isUsingOverage":false,"unifiedWindows":{"five_hour":
   {"utilization":0.35,"resetsAt":1792404000},"seven_day":{"utilization":0.09,
   "resetsAt":1792713600}}},"uuid":"...","session_id":"..."}
  ```

  The status is under `rate_limit_info`. The reset is `resetsAt`, camelCase, in Unix seconds. programs.md 5.6 says `resets_at`: that is a design gap for the next plan. The `rejected` status was not seen, since the account was under its limit. The live suite captures it.
- **`result`:** `subtype` (`success`), `is_error`, `terminal_reason` (`completed`), `api_error_status` (`null`), `permission_denials`, `structured_output` (the `--json-schema` object, which the model gives through a `StructuredOutput` tool call), `errors`, `num_turns`, `stop_reason`, `session_id`, `total_cost_usd`, `usage` and `modelUsage`.

## Still open

- the `rejected` rate limit and an `api_error_status` of a limit;
- SIGTERM's exit code (programs.md 8 says 143);
- `-p` reading the login past `--bare`'s change of default.

The next plan's live suite checks all three.
