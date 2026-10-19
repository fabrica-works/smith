# accounts-sources: an outline

The second plan of LLM accounts' first part, drafted at planning from the
code audits of smith at `e79b9a6b` and its design at `137a1cb4`
(2026-10-19). It starts from the `accounts` plan's `origin/main`, and is
planned then, from the code as it is. The sizes are the audits' guesses.

## What it takes

- **Accounts by source** (smith `shell.md` 4.1 to 4.6, 5.2, 5.3, 7;
  `domain/host.md` 7; `protocol/hosts.md` 5.2, 5.4): the settings'
  account model (provider, source `sign_in`, `key`, `env`, `program` or
  `none`, a redirect port, the issuer's, keys' and revocation URLs,
  trust), numbers from names (7.4), the credentials directory with its
  host id, tokens, keys and programs, `smith login` and `smith logout`,
  the check report and the interactive sign-in. About 1,900 lines new
  and 1,100 removed in 9 increments: the borrowed login
  (`smith-local-protocol/src/borrowed.rs`, `Source::Borrow`,
  `Origin::Borrow`, `CredentialFailure::Borrowed*`) goes once the live
  suite and the benchmarks no longer use it.
- **The presets** (`shell.md` 6.1): `Preset::{Codex, Anthropic}` move
  to `openai` and `anthropic`, the `openai` preset on the public
  Responses route this plan's skein 02 builds. The dialects' names in
  settings and their numbers in the agent's configuration (6.2) are
  this plan's (smith 01).
- **The accounts over the new driver** (`domain/host.md` 7;
  `protocol/hosts.md` 5.4): smith-local-service's credentials on skein
  08's and 09's named table, sign-out, the shared record's lock.
- **Claude Code as a program backend** (`programs.md` 1 to 9;
  `domain/run.md` 4, 9, 9.1; `domain/tools.md` 10): a new crate,
  `smith-protocol-program` (source name, environment, the status check,
  command and settings, the stream decoder, the prompt policy, failures,
  budget and cancel), the MCP server mode relayed over loopback with a
  per-run secret, the program grant, a run spawned per run. About 3,300
  lines in 14 increments, and a fake `claude` (its auth, stream, MCP
  client and signals) in two more.
- **The live suite and the benchmarks' logins** (`testing.md` 2.3, 4;
  `benchmarks.md` 6, 7, 10, 11.1): the suite on `$SMITH_LIVE`, the
  refresh story, Claude's story and a chat across providers; the fake
  issuer at a sign-in account's moved addresses; the benchmarks' arms on
  the credentials directory; the `Codex` to `openai` rename across the
  benchmarks' baselines.

## Gaps found so far, for its planning

1. **design:** the reset of Claude Code's `rate_limit_event` is
   `rate_limit_info.resetsAt`, camelCase, in Unix seconds;
   `programs.md` 5.6 says `resets_at` (`probe-claude.md`).
2. **design:** the OpenAI preset's key set and revocation URLs are not
   written as defaults (`shell.md` 6.1).
3. **design:** the program component's bounds: `limits.md` names none.
4. **design:** a program's completions into smith's domain: each
   assistant message as a completion, against the domain's pairing of
   complete and returned and its reserve at `complete()` (`programs.md`
   5.5, 5.7).
5. **design:** the refresh lead has no surface for the live suite's
   refresh story (`testing.md` 2.3; today
   `skein_oauth_accounts::Limits::refresh_lead`, fixed at 60 s in
   `smith-local-shell`'s `credentials/product.rs`).
6. **planner:** two logins of the same subject at once, and whether
   undeclared records count against the accounts' limit.
7. **planner:** where the fake `claude` lives, its home directory, and
   how the MCP mode is invoked.
8. **planner:** the check report's version (`CHECK_VERSION`, 1) with the
   accounts' new fields.
9. **live:** the `rejected` rate limit, a limit's `api_error_status`,
   SIGTERM's exit code (programs.md 8 says 143) and `-p` reading the
   login: the live suite checks them (`probe-claude.md`, Still open).
10. **live:** signing in to OpenAI needs a person, OpenAI's sanction of
    smith's registration and the MIT licence this plan lands; a capture
    of the registration's reply, the ID token's claims, the keys and the
    revocation URL before the fake issuer is trusted.
