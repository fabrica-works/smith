# accounts: decisions

The decisions this plan rests on, numbered. The supervisor's rulings
during the run follow them, from 101. Then the design's questions with
their answers, and the choices left to the user. The user is away, and
the director acts for them. The planner took each decision below as the
planner's, choosing the most reversible option where the choice could
be the user's (development.md, 10.4).

## The plan's own

1. **Two plans, split by layer.** The first part of LLM accounts
   (skein's and smith's `STATUS.md`, section 3: the licences, skein,
   smith) comes to about 77 increments. This plan, `accounts`, takes the
   licences, all of skein's part (llm.md, llm-connection.md, fake-llm.md
   2.3 to 6, oauth.md) and smith's formats and notices: a credential's
   scheme, the channel's version 4, the charter's 3, the transcript's 6,
   the event vocabulary's 4, an account's failures as notices, a window
   carried over by compaction. The next plan, `accounts-sources`, starts
   from this one's `origin/main`: smith's accounts by source, `smith
   login` and `logout`, the presets and smith's own registration, the
   credentials directory, Claude Code as a program backend with the fake
   program, the live suite on smith's own accounts and the benchmarks'
   logins. Its outline and the gaps found so far are in
   `findings/next/`. Only this plan's STATUS rows are `ready`. The
   director decided it (the inbox's question 1, 2026-10-19).

2. **The withdrawn forms stay.** The designer's call C1: skein keeps the
   Codex route with its account id and Anthropic's bearer route; smith
   accepts the id-and-bearer grant and the `codex` dialect. Each keeps
   its tests and says "withdrawn" in its doc comment, naming the second
   part of LLM accounts as the work that removes it once its last caller
   has moved. So the closing repins stay within development.md 5.7: the
   repositories downstream keep sending what they send.

3. **smith's fault names.** An account's failures and the model's own
   answer meet in `ModelFault`. The model's own refusal, today
   `ModelFault::Refused`, becomes `ModelFault::Declined`: its wire tag 3,
   the event vocabulary's word `refused` (events.md, 3.9) and run.md
   10's *refused* stay. An account the provider refuses is
   `ModelFault::Refused { account, code }`, and one spent is
   `ModelFault::Limited { account, reset }`, today's `Exhausted`, with
   the reset optional (run.md, section 10; channel.md, section 4). The
   designer's open point asked for the name.

4. **The replay tags.** skein's replay tag keeps 1 for the Codex route
   and 2 for Anthropic, and the public route takes 3 (llm.md, section 7):
   the tags written before this plan still read the same.

5. **The credential's value is `value`.** `Credential { value, scheme,
   account_id }` (llm.md, 2.6), and its limits follow:
   `CredentialLimits::access_token` and `Cap::AccessToken` take the name
   `value`, in skein 01.2, with their users.

6. **A credential's admission.** skein refuses at a call's admission, as
   `Invalid`, a key or a credential without an account id on the Codex
   route, whose credential is a bearer with one, and an account id on
   any other route (llm.md, 2.6 and 4.6). A bearer on Anthropic is the
   withdrawn bearer route, sent as today (4.7). The fake LLM
   refuses at configuration a key with an account id, and a list of
   credentials that mixes routes, whenever the list is given, at start
   or by `accept` (fake-llm.md, section 3).

7. **A program source's grant is refused.** A grant naming a program
   source decodes in channel version 4, and the agent refuses it as a
   kind it cannot install, the refusal the channel already has for a
   grant it cannot use, until `accounts-sources` gives the agent a
   program backend (channel.md, section 6).

8. **A version-2 OAuth record under an issued client id** reads as no
   record: the driver's grant fails as unloaded, and a sign-in replaces
   the file, saying the digest it read (oauth.md, sections 1 and 6.4).
   The codec half lands with the record's version 3 (skein 05.2), the
   driver's half with the named table (skein 08.1).

9. **The OpenAI public route's codes are the design's.** No OpenAI API
   key is on the machine, and llm.md 4.8 states the codes. The fake LLM
   renders them as stated, and `later.md` keeps the capture for when the
   user has a key. The director decided it (question 2).

10. **No live suite and no process tier.** This plan's behaviour is a
    route's rendering, a codec's version and a component's events: the
    worlds and the fakes prove each. The live suite on smith's own
    accounts is `accounts-sources`' (smith testing.md, 2.3 and 4), and
    the Claude Code probe it rests on ran at planning
    (`findings/next/probe-claude.md`; the director, question 3).

11. **The licence** (the designer's call C2). One `LICENSE` file at
    each repository's root, MIT's text, with the line `Copyright (c)
    2026 Free Ekanayaka`; `license = "MIT"` in the root manifest's
    `[workspace.package]`, and `license.workspace = true` in each
    member's. smith's `benchmarks/tasks/` repositories are fixtures,
    not smith's packages, and keep no licence of their own. "Done when"
    reads each package's licence with `cargo metadata`.

12. **The public route's code table is an increment of its own**
    (skein 02.4). llm.md 4.8's rows for the public route, each a case of
    llm.md 5, are about 200 lines with their tests, beside 02.2's code
    first rule on the Codex and Anthropic routes.

13. **skein's breaks reach smith in two places.** smith 01.3 repins at
    its start, to the later of skein 01.5's and 02.1's landings
    (`cargo update --precise`, README, Rules), and repairs what every
    skein step at or before that commit broke: skein 01's and 02's,
    about 120 to 150 lines, and those of skein 05 or 08 that landed
    first. The closing smith-repin takes the rest: the fake LLM's
    credential list, the OAuth record's fields, the driver's events and
    limits, about 95 lines in all. Repairs past about 200 lines are a
    question (development.md, 5.7). Until smith-repin, smith's tests
    prove what smith hands skein (the installed credential's scheme, a
    call with none) at the unit layer, and skein 04's worlds prove the
    header each scheme puts on the wire. Each skein increment that
    changes an interface smith, jig or the repository downstream of jig
    uses names, in its Shape, the downstream lines it breaks, so the
    repins find them.

14. **The driver keeps its constructor.** `Component::new` takes the
    accounts as a list, as today, each named by its place; an account
    added while running takes a free name, and one removed leaves its
    name free (oauth.md, 6.2). So no downstream constructor
    changes.

15. **New limits downstream take skein's values.** The OAuth machine's
    and the driver's new limits (the record's issued client, granted
    scope, subject and ID token caps, the OpenID documents' caps, the
    skew, the lock wait) are spelled out by every literal downstream. A
    repin writes:
    - in a cap on a field today's issuers already send, the cap that
      bounds its like today: `granted_bytes` the host's `scope_bytes`
      (1024 in smith), `id_token_bytes` its `token_bytes` (8192);
    - in a cap the host does not use (no terms, no OpenID, no shared
      record), the smallest valid value, 0, or 2048 for `modulus_bits`,
      so that no memory ceiling moves for it;
    - in the lock wait, skein's fixture value, 1 s.
    The skein increment that adds each limit names its fixture and its
    downstream value in its Shape; the memory ceilings downstream are
    checked again by their gates.

16. **The repins' defaults for what downstream does not use yet.** jig's
    and the downstream repository's parts of LLM accounts come later,
    as the second part (skein's `STATUS.md`, section 3), so their
    repins keep today's behaviour:
    - the driver's new events (signed out, removed, lock, unlock) join
      the existing unreachable arms: nothing downstream asks for them;
    - the driver's new failures map onto the nearest outcome there is:
      busy as a transient exchange failure, not granted and another
      subject as sign in again;
    - the refused notice reaches no event downstream, as a refused
      completion raised no notice before smith 02.2: the run's failed
      answer carries it (ruling 130);
    - a grant goes as the tagged withdrawn form (decision 2), which the
      agent accepts until the second part;
    - an ID token failure (`Unverified`), and `Incomplete`, which no
      downstream registration can meet, join the unreachable arms;
    - a new model fault is a model failure, as today's `Refused` is;
    - an opaque block's dialect, a number from smith 03.2, goes into
      jig's contract, whose `dialect` is text, as its decimal digits;
    - a configuration's `"dialect":0`, refused from smith 04.2, takes
      its dialect's number, as smith's own tests do.

17. **The waits on live text are few and narrow.** live text takes
    smith's channel version 3, the deltas smith's LLM component hands
    over (smith's protocol/llm.md, 6), and skein's fake-llm.md 2.2. A
    step of this plan that changes the same files first waits, with
    `plan await`, for the live-text step that changes them last (its plan
    at smith `0e9c3308`): skein 03.1 for `skein-01.3`, the byte peer's
    release, since 03.1 changes the files of live text's three skein
    steps; smith 01.2 for `smith-01.2`, the channel version 3's live
    record; smith 01.3 for `smith-01.3`, the LLM component's handover.
    live text's planner named its smith 01.1, the schema; its 01.2
    changes the version 3 schema again, from which version 4 is made,
    so smith 01.2 waits for that. The later steps of those sessions
    follow from these. The work that does not wait goes first: skein 01,
    02 and 05 to 09, and smith 01.1.

18. **A sign-out's file is removed through the owner** (skein 09.1).
    io's `Remove` names its root by descriptor, which the driver never
    holds; the design has io take roots by token (io.md, 5), which waits
    for a plan of its own (skein's `STATUS.md`, section 3). Until then
    the driver asks its owner with `FileLower::Remove`, naming the root's
    token, and the owner maps it with `FileIo::descriptor`, as it does
    for the requests on open files. The variant says so in its doc
    comment, and goes when io's roots by token land (`later.md`).
    Downstream, its arm is unreachable: no one there signs out
    (decision 16).

19. **smith's local host keeps its routes until `accounts-sources`.** It
    borrows Codex's and Claude's logins today, on the Codex route and
    Anthropic's bearer route. Until the next plan moves it to the
    presets, it lends a sign-in with an account id as the withdrawn
    id-and-bearer form, and any other credential as a bearer (smith
    04.1), so that what it sends does not change. The design's "written
    by none of smith's" holds from `accounts-sources` on.

20. **Interim mappings end at a named step.** Where a format lands
    before the code that fills it, the gap is mapped onto what there is
    and ended by a later increment, which names the mapping in its
    Removes: smith 01.3 maps skein's `Refused` and `Unsupported` onto
    `Invalid` until 02.1; 01.2 sends the endpoint and dialect refusals
    as malformed until 03.3; 02.1 writes the run's failed lines in event
    vocabulary 3 until smith 05.1 and 05.2. In skein, the driver's `SignIn`
    passes 05.4's hints and the terms' fields empty until 08.4, and ends
    07.1's GET demand as `Exchange(Malformed)` until 10.2.

21. **The client's readings where llm.md leaves a choice** (skein 01,
    02). One `REPORT_BYTES`, 128, serves every route: 4.2 gives each
    route the same value, and smith destructures the limits. Where an
    error gives both, `resets_in_seconds` comes before `resets_at`;
    Codex's `usage_limit_reached` takes no `retry-after` (4.6 names
    none), while the public route's
    `subscription_sharing_usage_limit_exceeded` falls back to it (4.8);
    the fake LLM never sends one for `Exhausted`. The public route
    ends a stream on 4.8's four events alone: `response.done` stays
    Codex's, and an unnamed mid-stream code, `authentication_error`
    included, is `Unavailable`. A zero bound on a credential's value is
    still refused at admission, as `Cap::Value`, whether or not the call
    carries one.

22. **jig repins twice** (README, The closing repins). Its repairs for
    smith's changes come to about 150 to 170 lines, past 5.7's 200 with
    skein's, so `jig-repin.s1` takes smith 01's and 04's once both have
    landed, with jig's skein on smith's pin, and the closing repin the
    rest.

## Design questions

Questions 6 to 16 of the inbox
(`~/src/rust/tmp/director/inbox/accounts-planner.md`, 2026-10-19),
relayed by the director to the accounts' designer, who settled each as
recommended: skein `d4ef1680` (6, 9 to 12), `378fd8cf` (15) and
`49725eca` (16), smith `137a1cb4` (6 to 8, 13, 14). The design at those
commits is this plan's.

- 6. **An LLM with no account** (smith charter.md 2; skein
  llm-connection.md 4.2, llm.md 2.6). Lands in skein 01.5, smith 03.1.
  Answer: a call's credential is optional; with none, no credential
  field is sent, and the Codex route refuses the call as `Invalid`; a
  fake peer configured for it observes the scheme none. In smith, an LLM
  with no account has no grant, no grant's value is empty, and its
  failure ends the session naming no account (session.md 4).
- 7. **The limited and refused notices' fields** (channel.md 4). Lands
  in smith 01.2, 05.2.
  Answer: the reset is an optional `duration` (absent: not said; zero:
  said zero); the code at most 128 bytes, left empty past that, never
  clipped; the reason at most 512 bytes, cut at a character (limits.md
  2.4). In events 3.10 the account answer failure's `retry_after_ms`
  becomes `reset_ms`; the completion failure keeps its own;
  `account_refused` keeps the code alone.
- 8. **The transcript refusals `endpoint` and `dialect`** (channel.md
  5). Lands in smith 01.2, 03.3.
  Answer: channel version 4 drops them, and run.md 6 says so;
  transcript.md 4's one endpoint and dialect across a window's turns
  stays, as a check for malformation.
- 9. **A mid-stream failure whose code is not in llm.md 4.8's table.**
  Lands in skein 02.4.
  Answer: `Unavailable`, the code kept in the report, as the code
  already does. llm.md 4.6 to 5 also now read as the code does:
  `usage_limit_reached` in `response.failed`; `Overloaded` as a 529 or a
  5xx `overloaded_error`; Anthropic's `ContextTooLong` as "prompt is too
  long" or `request_too_large`.
- 10. **fake-llm.md 2.3's lines after a Fail.** Lands in skein 03.1.
  Answer: only a Fail that names no credential must be its turn's last.
- 11. **Resets and rows per route** (fake-llm.md 2.3, 7). Lands in skein
  03.2.
  Answer: the public and Codex routes render `resets_in_seconds`,
  rounded up; Anthropic gives a reset on the bearer route only, in the
  unified-reset header, before any stream, on bearer lines only;
  anything else is refused at configuration, and `Peer::accept` refuses
  a key at such an index. A class with no row on a route is refused;
  only `Unsupported` on Anthropic has none. Which field the public route
  sends stays open until a capture (decision 9).
- 12. **The fake issuer's tokens at the fake LLM** (fake-llm.md 6).
  Lands in skein 04.4, 06.1, 11.1.
  Answer: the issuer mints no token; its script lists every token, each
  refresh's included, and the world gives the LLM peer that list at
  start. Every credential is accepted before the peer serves, and a
  revoked token stays accepted unless a story scripts `Unauthorized`.
- 13. **The public Responses dialect's name and number.** Lands in smith
  04.2, 03.2.
  Answer: `openai`, 3, held by agent.md 4, not the charter, beside
  `anthropic` 2 and the withdrawn `codex` 1, fixed for good; a name that
  mismatches its number is refused. Turns and opaque blocks carry the
  number (transcript.md 2).
- 14. **The fixed hash of shell.md 7.4.**
  Answer: the first four bytes of SHA-256 of the name, as a big-endian
  `u32`; every declared account, `none` included, is checked, and a
  collision exits 2 naming both. The hash is `accounts-sources`'. The
  account number is a `u32` in charter 2, channel 6 and events 3.10, and
  a number, not an index: smith 04.1 makes the agent's admission count
  accounts instead of bounding their numbers (`profile::ACCOUNTS`, the
  configuration's check), and smith 05.2 makes `AnswerFailure`'s account
  a `u32`.
- 15. **The driver's lock wait** (skein oauth.md, 6.4 and 6.7). Lands in
  skein 10.1.
  Answer: a `Duration` among the limits the owner gives, a deadline on
  the injected monotonic clock starting at the emitted `lock`; when it
  passes, the write fails as busy, and a later `locked` gets `unlock` at
  once, until `closed`.
- 16. **A `locked` after the driver's `closed`** (skein oauth.md, 6.4,
  6.6). Lands in skein 10.1.
  Answer: `closed` ends the driver's answers; a lock the owner answers
  `locked` after it is the owner's to release, and the driver answers
  none.

## Rulings

101. **skein 06's Shapes against main** (ready check of skein-06.1).
     skein 06's ready check, against skein `887302f8`, corrected `skein/06-fake-issuer.md`:
     - 06.1 Files: `tests/oauth/src/lib.rs` also takes `response`, `time`, `wall`, `registration` and `error` from `tests/oauth/tests/world.rs` (05.2 moves only `issuer`, `sign_in`, `success`, `client_step`, `fake_step`, `record` and the limits); skein-fake-peers' `oauth::worst_case` joins the Files.
     - 06.1 Shape: `Issuer::new` bounds the issued ids (at most `ISSUED`, 4, each within `document.string_bytes`) and the terms' URLs (`uri_bytes`); the peer's `http.response` floor rises by `3 * document.string_bytes + 11` for the redirect's `&client_id=`.
     - 06.1: `scenario` in `tests/oauth/tests/fuzzy_world.rs` is 93 lines; its `token_plan` helper only if clippy's `too_many_lines` flags it.
     - 06.2: `OpenId`'s `issuer`, `key_set_url` (`uri_bytes`) and `subject` (`document.string_bytes`) bounded by `Issuer::new`; `exp` from `Wall::as_secs` and `Duration::as_nanos`; the reference test finds the dots with `skein_lib::bytes::{find, find_from}`, without `rposition` or `cut + 1`; the sketches' bodies are not `todo!()`.
     Not run: clippy on the sketches and the suites, the build lane being held by gates; the re-check at 06's start runs them.

102. **skein 08's Shapes against main** (ready check of skein-08.1).
     skein 08's ready check, against skein `887302f8`, corrected `skein/08-driver-accounts.md`:
     - 08.1 Files: `lib.rs`, whose doc says the crate "knows no account names"; Test shape: `fuzzy_private.rs` draws its story by `random.below(20)`, so `Story::NoIssuedId` is drawn only if it draws below the list's length; `accounts.rs` already imports `skein_tls::Name`.
     - 08.3 Shape: `Component::refreshable` tests only `refresh_token.is_some()`; `wanted` joins `held` at `Component::fire`'s refresh at the lead, `Component::next_deadline`'s `State::Record` arm and `fail_exchange`'s `held(slot)`.
     - 08.4 Files: `component.rs`'s `up` (carries `Listening`'s bound port) and `progress` (passes the bound URI to `Listener::progress`).
     - The `add` sketch's body is never `todo!()`.
     Ran: clippy on `skein-oauth-accounts` with the sketch (only its `todo!()` and unused parameters failed). Not run: the suites, the build lane being held by gates; the re-check at 08's start runs them.

103. **skein 02's Shapes against main** (ready check of skein-02.1).
     skein 02's ready check, against skein `887302f8`, corrected `skein/02-public-route.md`:
     - 02.1 Shape: the exhaustive `Provider` matches it missed, `tests/llm/tests/derive.rs` `archived_heads_fit_with_headroom_and_complete_with_derived_limits` and `fake.rs` `absent_affinity_sends_none_and_anthropic_ignores_it_on_the_actual_wire`, `peer_native_request` and the two of `growing_conversations_preserve_native_content_and_encode_twice_identically`; `translate.rs` `validate_codex` and `charge_replay_item` keep `OpenAiCodex` until 02.3.
     - 02.1 Test shape and Proof: the rows refusing one OpenAI route's replay on the other move to 02.3, since 02.1's transitional `Error::Unsupported` refuses every `OpenAi` call first.
     - 02.2 Shape: `openai::StreamDecoder::new` and `with_reasoning_drop` take the `Provider` first, kept in a field for `accept`'s `classify`.
     - 02.3 Shape: the public route's request id is the head's `x-request-id` (llm.md 4.8), and the unread `openai::identity::REQUEST_HEADER` goes; `measure_request` and `encode_request` take the route, and `charge_replay_item` counts reasoning on either OpenAI route; `public_call` and the reference test set `Some(...)` credentials, after 01.5.
     - Done when: `rg -oIw`, not `rg -ohw` (`-h` prints rg's help), in 02's and in 01's.
     Ran: the 02.2 reference test with the names on main, under clippy on skein-llm-world; skein-llm-world's nextest, 139 passed in the default profile and 7 in fuzzy.

104. **smith 02's Shapes against main** (ready check of smith-02.1).
     smith 02's ready check, against smith `7bd4b6a6` and temper `2bc7d825`, corrected `smith/02-notices.md`, and `smith/01-grants.md` with it:
     - Decision 3 keeps the model's own refusal at wire tag 3 as `Declined`, while 01.2 adds a `refused: RefusedFault` arm to `ModelFault`: in version 4's schema tag 3's arm is named `declined` (01.2's Shape says so now), and 02's Choices name the tag and the event word `refused` it keeps.
     - The 800-line baselines: `service.rs` 2968, `responses.rs` 1870, `sinks.rs` 1513, host-domain `domain.rs` 1496, local-service `process.rs` 1423, protocol-channel `translate.rs` 873.
     - 02.1: the reference test, rustfmt-clean, paths written whole as `a_model_failure_keeps_transport_evidence_and_cooldown` writes them, and `(7, &said)`; the story sits beside that one, which already writes a failure at the answer; `Report::none()` is `#[must_use]`; Removes names the bare `Duration`, not the variant; temper's `finish.rs` is expected to compile unchanged.
     - 02.3: `each_model_fault_has_its_exit_status` lives in smith-local-domain's `tests.rs` (`end.rs` has no test module); temper's `crates/temper-worker-domain/src/tests/entrance.rs` builds `ModelFault::Exhausted`.
     - 02.4: `account_failures_keep_their_own_words` exists and gains rows; the new arms append `run failed: ` themselves; smith-events publishes `clock(at: Wall) -> [u8; 5]`, appended through `Shown`, since `put` into a `List` panics past its room.
     Not run: the suites and the sketch, the build lane being held by gates; the re-check at 02's start runs them.

105. **smith 04's Shapes against main** (ready check of smith-04.1).
     smith 04's ready check, against smith `7bd4b6a6`, jig `83a00a2f` and temper `2bc7d825`, corrected `smith/04-values.md`:
     - 04.1 Files and Shape: `crates/smith-protocol-channel/src/limits.rs`, whose worst case names `credentials::Secret`, takes `GrantValue`; a further exhaustive match on `GrantError` takes `Scheme`; the install check reads each endpoint configured with the grant's account, two endpoints sharing one; `duplicate_names_fail_at_startup` keeps its `TooMany` row.
     - 04.2: a charter's `Llm.dialect` moves with its configuration's endpoint, since `Endpoints::name_of(number, dialect)` finds no name on a mismatch: `tests/local-process/src/process/configuration.rs`'s `run::charter::Llm` takes 1 with its JSON, and `tests/agent-process/src/lib.rs`'s pair moves together if either does. Downstream adds jig's `testing/jig-test-service/src/tests/fixtures.rs` and temper's `crates/temper-service/src/tests/fixtures.rs`, with the charters that meet them.
     - The 800-line list adds `tests/agent-process/src/world.rs` (1586), `tests/channel/src/lib.rs` (1131) and `tests/end-to-end/end_to_end.rs` (1000), and refreshes `service.rs` 2968, `config.rs` 1659, protocol-channel `component.rs` 1083 and `stories.rs` 1236.
     Not run: the suites and the sketch, the build lane being held by gates; the re-check at 04's start runs them.

106. **skein 05.2's record fixtures** (cross-session scope, skein 05's block of 2026-10-19T12:47:11Z).
     Yes, at 512. The session's question names its own case: "if a downstream
     `record_bytes` cannot hold a version 3 record at its downstream caps
     (decision 15), name the fixture and the size." Both 256-byte fixtures are
     skein's own, `crates/skein-oauth-accounts/src/tests.rs` `limits()` and
     `tests/oauth-accounts/tests/memory.rs`
     `every_account_holds_a_maximal_record_within_the_heap_bound`, each with
     `token_bytes` 64 and `scope_bytes` 64: a version 3 maximum is 379 bytes,
     387 with the zero-cap fields present empty, which 256 cannot hold. Each
     takes `record_bytes: 512`. Refusing the larger combinations instead would
     leave a fixture that cannot hold the record its own caps admit. The memory
     test's bound follows from `worst_case(&limits)`, so no ceiling literal
     moves with it. Every other `record_bytes` literal on skein, smith and jig
     is 1024 or more; a further `Limits` literal whose `record_bytes` cannot
     hold a version 3 record at its caps takes the next power of two that
     holds it, and the commit body lists it. 05.2's Shape now says so.

107. **smith 01.2's credential fields** (cross-session scope, smith 01's block of 2026-10-19T12:50:35Z).
     Neither: the generator has the mechanism, and smith does not wait. skein's
     codec.md, section 3, says it in its own words: "A credential is a
     `secret N`, never `bytes N`: a token, a key, a secret, a grant's value.
     Its bytes cross as `bytes N` would; its type, lib's `Secret` (lib.md,
     section 6), shows only its length in `Debug`, so a record that holds one
     is printed, logged or traced without it. Its owner reads the bytes with
     `expose`, to use or send them." 01.2's Shape wrote `bytes 8192` for both
     values, a plan defect. In `channel-v4.schema`, `CredentialValue` is
     `{ scheme: Scheme bytes: secret 8192 }` and `WithdrawnValue` is
     `{ id: text 1024 bearer: secret 8192 }`. The generator at smith's pinned
     skein (5c8ae967) parses `secret` and emits `skein_lib::Secret` for it,
     which derives `Clone`, `PartialEq`, `Eq` and `Hash` and writes its own
     `Debug` as its length, so the generated derives stand and the drift check
     holds. The bytes on the wire, and so the goldens, are those `bytes 8192`
     would give. The host builds each value with `skein_lib::Secret::new`, and
     the agent reads it with `expose` where it repacks the table's envelope.
     Version 3's `GrantValue { credential: bytes 8192 }` goes with `v3`. A
     further v4 field that holds a credential, a token or a key is a
     `secret N` too; no other field of the schema holds one.

108. **skein 02's Shapes against main** (re-check of skein-02.1).
     skein 02's re-check after 02.1's start, against skein 9fbda6b (02.1
     landed as 489df9e), corrects 02.3 in `skein/02-public-route.md`. Since
     01.2, `client.rs` `headers` and `head_length` choose `x-api-key` or
     `Authorization: Bearer` from `Credential.scheme`, and `measure_headers`
     refuses `Scheme::Key` on Codex alone: 02.3's "`Authorization: Bearer`
     for either scheme" on `OpenAi` goes through them as well as
     `provider_headers`. `tests/llm/tests/credentials.rs` `input` empties the
     account id only `if provider == Provider::Anthropic`, so a public call
     would carry "account-test": 02.3 makes it `provider !=
     Provider::OpenAiCodex`. The 800-line baseline of `client.rs` is 1744 at
     9fbda6b, not 1716. A further site that picks a header by scheme or
     route takes the same arm, listed in the commit body.

109. **skein 04's Shapes against main** (ready check of skein-04.1).
     skein 04's ready check of skein-04.1, against skein b481ad9 (02.1 since
     landed as 489df9e; read again at 9fbda6b), smith 9d4430e9, jig 149970b,
     hearth 5efba43 and temper efe462a, corrects `skein/04-fake-credentials.md`:
     the size baselines (`provider.rs` 1095, `tests/llm/src/fake.rs` 967,
     `tests/llm/tests/fake.rs` 1367); `Auth` and `Presented` derive `Clone,
     Copy, PartialEq, Eq, Debug` for the reference tests' `assert_eq!`; 04.1
     names `provider::Server`'s field that carries the matched index from
     `entrance` to `requested`'s `Query.credential`, and has
     `Exchange::new_configured` and the test module's `delayed` copy
     `input.credential.scheme` where they write `Scheme::Bearer`; 04.2's
     existing `observe_head` gains `presented`, its socket helper is
     `request::request(sim, server, client, peer, fields, body)` in its own
     `tests/llm/request.rs` (`support::call` neither pumps the peer nor
     returns the read buffer, and in `tests/support/mod.rs` it is dead code in
     the `oauth` and `forge` binaries), and its unauthorized rows reach no
     `Query`, as `requested` answers `server.refused` first; 04.4's callers
     are 5 of `extra_worst_case` in `bounded_peer_memory.rs`, 1 in
     `fake_memory.rs` and 2 of `prepared` in `fake.rs`, and
     `fake_credentials.rs` defines its own `completion`, since `fake.rs`'s
     helpers are private. A further site that hard-codes a scheme or a count
     the plan names is corrected the same way, and listed in the commit body.

110. **A test helper is written once** (steer of the check of 2026-10-19T13:13:32Z).
     The hourly check of 2026-10-19T13:13:32Z found `tests/llm/tests/credentials.rs`
     (skein 01.2) retyping `header` and `rejection` of `anthropic_admission.rs`
     and writing the `World::new` / `Request::Start` / `run` lines twice,
     against testing-strategy.md, section 10, "A setup written once". The
     README's "Rules of this plan" gains "A test helper is written once": a
     helper a second test file of a crate needs moves to the crate's test
     library (`tests/llm/src/lib.rs` for skein-llm's worlds) and both files
     call it; skein 01 moves the first copies at its next increment. A further
     retyped helper moves the same way, named in the commit body.

111. **skein 05.3's terms, meanwhile** (design gap, skein 05's block of 2026-10-19T13:32:52Z).
     Interim: whether a registration's terms refuse an empty added-parameter
     name and an empty sign-in-again error name (`Malformed`), or count each
     entry against `parameter_bytes` (`Limit`), is a design gap: oauth.md,
     section 1, gives each added parameter as "a name and a value" and section
     5 says `Limits` "also caps the added parameters", and neither says
     whether a name may be empty or how an entry's storage counts. It went to
     the designer at 2026-10-19T13:35:02Z, in the run's `question.md`. Meanwhile skein 05.3
     goes on with the work that does not rest on the answer: the move into
     `redirect.rs`, `Terms` and its types, the varying redirect and its bound
     port, the resource and the nonce, and their tests, as 05.3's Shape and
     Test shape give them. Leave the empty-name refusal and any per-entry term
     of `client_worst_case` for the answer's ruling, and do not land 05.3
     before it; commit locally meanwhile.

112. **skein 05.3's empty term names** (design gap, skein 05's block of 2026-10-19T13:52:37Z).
     The designer's answer, skein 0ccaf590 and smith d275fd98, settles what ruling 111 left open; 05.3 lands with it:

     An empty added parameter name and an empty sign-in-again name are refused
     by `validate_registration` as `Limit`, not `Malformed`, and not counted
     as a byte. An empty value is admitted. oauth.md, section 1, at `0ccaf590`:
     "parameters added to the authorization URL, each a name of at least one
     byte and a value, which may be empty"; "the error names that mean sign in
     again, each of at least one byte". Section 5: "A registration whose terms
     hold an empty parameter name or an empty error name is refused as `Limit`:
     each entry then holds at least one byte of the terms' sum, so that sum
     also bounds how many entries there are. `worst_case` counts the in-progress
     machine with its registration (each of the terms' entries with its own
     storage, beside its bytes)". Section 4 is unchanged: "`InvalidRedirect`
     and `Limit` identify local admission failures."

     05.3, as amended:
     - Shape: refuses as `Limit` "an empty added parameter name or
       sign-in-again name (as an empty client id is; an empty value is
       admitted)". `client_worst_case` counts "`parameter_bytes` as bytes and
       again as at most that many entries, each priced at
       `size_of::<Parameter>()`, the larger of the two entries".
     - Test shape: `terms_are_refused_where_the_design_refuses` gains rows "an
       empty parameter name and an empty sign-in-again name (`Limit`), beside
       an empty value (admitted, the URL carrying `name=`)"; and the bound's
       first measure, memory, `tests/oauth/tests/memory.rs`:
       `terms_at_their_most_entries_fit_the_worst_case` (256 parameters, each
       the name `a` and an empty value, built inside the meter, stepped to its
       `Visit`, `url_bytes` 2048, against `client_worst_case`).
     - Size: about 275 lines. 05.4 is unchanged.

     The driver needs nothing new: `limits::worst_case` already counts
     `client_worst_case` per account and per exchange, which now prices the
     terms; oauth.md 6.7 names each account's registration to match.

113. **smith 01.3's pin takes skein 02.3** (plan defect, smith 01's block of 2026-10-19T14:36:10Z).
     A plan defect: the README's "smith takes skein twice" pins smith 01.3 to
     the later of skein 01.5's and 02.1's landings (04c223a), but 02.1 leaves
     `Provider::OpenAi`'s measure and prepare `Err(Error::Unsupported)`, which
     02.3 fills. 01.3's Shape ("`framing.rs`'s `prompt_allowance` takes
     `Provider::OpenAi` with `Endpoint::openai()`") and its Test shape ("the
     prompt allowance's rows gain `OpenAi`") need the filled route, and smith
     04.1's agent-process world runs an `openai` endpoint on 01.3's pin. So the
     pin moves, not the tests: smith 01.3 repins to the later of skein 01.5's
     and 02.3's landings, `--grep='^Step: skein-0(1\.5|2\.3)$'`, today
     31bbf58, as the amended README rule gives it, still in a commit of its
     own whose body names `$c`. No interim `Unsupported` row, and nothing for
     smith-repin to flip.

     Between 04c223a and 31bbf58 only skein 05.3 (1a6e753, f634f9c) and 02.3
     landed. 02.3's Shape says "Downstream: none". 05.3's Files name its smith
     users, which the repin commit now repairs as decision 13 says ("those of
     skein 05 or 08 that landed first"), as 05.3's Files give them (decision
     15): `terms: None` in the `Registration` literals of smith-local-shell
     `src/credentials/sign_in.rs`, smith-local-service
     `src/credentials/tests/private.rs` and `tests/local-process/src/process/configuration.rs`;
     the three new `ClientLimits` fields in smith-local-service
     `src/credentials/tests.rs` and smith-local-shell
     `src/credentials/product.rs` (`client_limits`). A further smith user of a
     skein change at or before `$c` that `heavy cargo check --workspace
     --all-targets` finds is repaired in the same commit and listed in its
     body; past about 200 lines besides `Cargo.lock`, it is a question.

114. **skein 08's Shapes against main** (re-check of skein-08.1).
     The re-check of skein-08.1 against skein f634f9c, then main 31ae5ab, found
     08.1's Shapes short of the code; `skein/08-driver-accounts.md`, 08.1, now
     says, checked by `git grep` on skein origin/main 31ae5ab:
     - Decision 8 lives in `component/private.rs`'s `loaded`, before
       `has_record`: `private_terminal` builds `Unloaded::Unreadable` only for
       a decode error (`Err(_) => Some(Failure::Unloaded { at: Place::File,
       why: Unloaded::Unreadable })`) and hands `loaded` the decoded record
       (`self.loaded(env, account, Some(record), None, up, io)`); the owner's
       case is `Component::new`'s `Keeper::Owner { kept }` arm. Both ask one
       private helper over the terms and the record: skein-oauth's `issued_id`
       is `pub(crate) fn issued_id(registration: &Registration) -> bool` in
       `redirect.rs`. `boundary.rs`'s `Unreadable` doc ("Loaded bytes are
       damaged or belong to another version") names the new case; `settled`
       (`self.accounts.clear(); self.sources.clear(); self.keepers.clear();`)
       clears `names` too.
     - The world: `tests/oauth-accounts/src/private.rs`' `FileSystem::new`
       (`match story` at two places) and `run`'s `client.stop_at_store =
       match story` and `client.stop_at_load = match story` take
       `Story::NoIssuedId`; `world.rs`' `Client` gains `terms`, set by
       `Client::private(root, story, terms)` (now `fn private(root:
       kernel::Fd, story: Story)`), whose callers are `private::run` and
       `Client::recovery`. The version-2 bytes are `encode_record`'s of a
       version-3 record with no optional field, version byte 5 set to 2, the
       five presence bytes cut (`decode_record`: `if version == 2 { (None,
       None, None, None, None) }`).
     - The units: `refresh_account(account, address)` takes no terms (the test
       sets them on its `Account::SignIn`); `limits()` has
       `issued_client_bytes: 0`; the grant row of
       `a_name_no_live_account_holds_is_refused` is already
       `a_missing_account_and_missing_record_have_typed_terminals`'
       (`Request::Grant { account: 3 }` refused `Refusal::Account`); no unit
       test reaches `file::Request::Store` after a sign-in, so the digest's
       proof reads the keeper's `expected` after `Loaded`.
     - `fuzzy_private.rs`' draw (`random.below(20)`) takes the list's length
       through a local; the table's `Name` is `component.rs`'s; sizes are
       `world.rs` 1249 and `sign_in.rs` 858.
     A further user of what 08.1 changes is refreshed the same way and listed
     in the commit body.

115. **skein 08's Shapes against main, after skein-05.4** (re-check of skein-08.1).
     The re-check of skein-08.1 against skein 0b64ac7 (skein-05.4) found the
     driver's helper for decision 8 confirmed (`issued_id` is `pub(crate)` in
     skein-oauth's private `redirect` module) and one gap:
     `decode_record` admits an empty issued id, while the client reads one as
     missing (`crates/skein-oauth/src/machine.rs`: `let Some(client_id) =
     &prior.issued_client_id else { return Err(Failure::Refused) }; if
     client_id.is_empty() { return Err(Failure::Refused); }`). 08.1's helper
     counts an empty id as none, so such a record loads `Unreadable` (private)
     or is held as none (owner) under issued terms. The new `SignIn` and
     `Redirected` fields change nothing else in 08.1: `deliver`'s `client_id:
     None` ends an issued sign-in `Incomplete` until 08.4, which 08.1's Proof
     already leaves to it.

116. **skein 08.1 lands again past a timing flake** (process, skein 08's block of 2026-10-19T15:02:47Z).
     Land 08.1 again, once, without changing a test. skein's
     `tests/world/tests/real.rs`
     `a_keep_time_after_the_last_word_fails_the_world_naming_the_process`
     failed as "test did not panic as expected" in 0.014 s, in code no step
     of this run touches; it passed in the other 25 of this run's gate logs
     that ran it. Its deadline, `clock.now().now.saturating_add(Duration::from_millis(5))`,
     is taken before `real::World::new(Judge)` and `world.spawn`, so a setup
     slower than 5 ms leaves no keep time after the last word: a wall-clock
     bound, which implementer.md, 8.5, retries once ("If the gate fails only
     on a timeout (a suite's global timeout, or a test's wall-clock bound),
     in tests outside your session's files, `land` again once without
     asking"). The README's "Rules of this plan" now says so for this test
     (ruling 116), and `later.md` holds its fix. Before landing again,
     08.1 takes ruling 115: its decision-8 helper counts an empty issued id
     as none, as `Client::refresh` does.

117. **skein 06's Shapes against main** (re-check of skein-06.1).
     The re-check of skein-06.1 at its start (skein ceda164e, smith a34ac885,
     jig 8922bfa4) ran its sketches and suites, which its ready check had not;
     06.1 landed meanwhile (skein 11565f3), so its three 06.1-only mismatches
     (the fixtures' move, `fn redirected`, `scenario`'s length) are moot: its
     commit body says "The existing world library already owns response,
     time, wall, registration and error, so no fixture movement is needed".
     `skein/06-fake-issuer.md` now says:
     - 06.1's Downstream: the peer floor 11565f3 names ("256 + 6 * 256 +
       267") refuses two fixtures past skein, which the closing repins raise:
       smith `tests/local-process/src/process.rs` (`response: 49_152`) and
       jig `tests/protocol-accounts/src/fixtures.rs` (`response: 2048`, the
       `profile.server` that `world.rs`' `fn issuer` passes), both by `git
       grep -n 'response: '` on their origin/main.
     - 06.2's `key.rs`: `openssl genpkey -algorithm RSA -pkeyopt
       rsa_keygen_bits:2048 -outform DER` on this machine (OpenSSL 3.6.4)
       writes 1192 bytes whose `asn1parse` opens `SEQUENCE`, `INTEGER :00`,
       `INTEGER` (the modulus): PKCS#1, not PKCS#8. `PKCS8` comes from the
       PEM body or `openssl pkcs8 -topk8 -nocrypt -outform DER`.
     - 06.2's reference test: skein's `clippy.toml` disallows `{ path =
       "slice::split_at", reason = "split_at_checked" }`; it uses
       `split_at_checked(cut).expect("in range")`, which compiled clean.
     A further fixture the floor refuses rises the same way, named in the
     commit body.

118. **skein 07's Shapes against main** (ready check of skein-07.1).
     The ready check of skein-07.1 (skein c7f4949, smith a34ac88, jig 8922bfa,
     hearth 61ab3d1, temper 4121493) found 07's Shapes short of main;
     `skein/07-openid.md` now says, checked by `git grep` on skein origin/main
     11565f3:
     - 07.1 meets tests that read a `Visit` right after `SignIn`:
       `machine_tests/terms.rs` (`a_nonce_is_bounded_and_needs_openid`:231,
       `an_admitted_nonce_is_sent_in_the_visit_url`:252) answer the key GET
       first, and `machine_tests.rs`' `limits()` (`document_bytes: 1024`,
       `tokens: 64` today) takes the key caps, 4096 and 128, so that
       `terms_are_refused_where_the_design_refuses` still reaches its rows;
       `tests/oauth/tests/world.rs`' `an_openid_sign_in_fails_before_its_check_exists`
       (:396) answers the keys through `begin`, its `Malformed` kept to 07.2.
     - `begin` returns `Option<oauth::Request>`, built as `visit` builds its
       `SignIn` (`key: 7`, `now: time(1)` today), at `time(seconds)`.
     - 07.1's worlds rest on 06.2's `documents()` (main: `string_bytes: 256`,
       below a 2048-bit `n`, 342 bytes).
     - The key-holding memory proof is a new sign-in step:
       `client_and_rotating_issuer_steps_fit_declared_heap_bounds` meters
       `oauth::Event::Refresh` alone.
     - `machine.rs` is 759 lines and `machine_tests.rs` 732: the GET's handling
       goes to `machine/fetch.rs` (beside `machine/revoke.rs`) and the machine
       tests to `machine_tests/openid.rs` (beside `terms.rs`); the question to
       raise when reached is answered.
     - `component.rs` is unchanged: it already ends a `None` from `Web::new` as
       `Failure::Exchange(oauth::Failure::Malformed)`; `Web::new` matches on
       the method (clippy.toml: `core::matches`, "a catch-all match in
       disguise"). The in-skein driver literals take 0, 0, 0, 2048 and skew 0.
     - `issuer_endpoint` (`redirect.rs`:85, private) becomes `pub(crate)`;
       `Cargo.toml` denies `todo` and `unimplemented`, so the sketches'
       `todo!()` bodies never land.
     - RFC 7515 A.2 is copied from skein-tls's private test consts `N`,
       `INPUT`, `SIGNATURE`; its payload (`iss` "joe") ends at `Issuer`.
     A further user of what 07 changes is refreshed the same way and listed in
     the commit body.

119. **A table test's rows are data** (steer of 2026-10-19T15:24:42Z).
     The check of 2026-10-19T15:24:42Z saw, a second time after the steer of
     14:31Z, table tests that branch on a row's value and assert inside match
     arms or helpers: skein 05.4's `issued.rs`
     (`an_issued_client_id_is_bounded_by_its_cap` asserts `end` inside its arm;
     `every_later_request_carries_the_issued_id` branches twice on
     `refreshing`) and 05.5's `revoke.rs` (`answer_case` asserts on the way and
     branches on `retry` and `evidence`; `a_revocation_posts_its_form` computes
     its expected id with `if issued`). The README's "Rules of this plan" gains
     "A table test's rows are data" (testing-strategy.md, 10, "Rows carry
     data"): a row holds its inputs and its expected value, the body compares
     once after any match, a helper returns a value and asserts nothing, and
     rows that need different code are tests of their own. The landed tests
     stay as they are. A script cannot test the rule, so it adds no gate step.

120. **smith 03's Shapes against main** (ready check of smith-03.1).
     The ready check of smith-03.1 (skein 11565f3, smith f3857e50, jig
     bfc3a82f, hearth 61ab3d1, temper ec08ab265) found 03's Shapes short of
     main; `smith/03-records.md` now says, checked on smith origin/main
     09be081e with the Rules' own `rg` and `G`:
     - The Rules' counts: charter v2 20 (with `tests/channel/tests/live.rs`),
       `Kind::First`/`Window` 21 (with `tests/channel/src/lib.rs`),
       `Refusal::(Endpoint|Dialect)` 18 (with
       `smith-agent-service/src/answers.rs`, `smith-local-service/src/stream.rs`,
       `tests/agent/tests/feedback_table.rs`, now named in 03.3's Files).
     - 03.1: `guard.rs`'s `measure` and `check` take `credential: &Credential`
       and pass `Some(credential)` to skein's `client::measure`: they take the
       option; `domain.rs`'s `IssuedCompletion` has `pub(crate) grant:
       GrantName`: it becomes an option, its `failed()` arms handling none;
       the full `session::Spec` literals of `tests/session`
       (`tests/{memory,simulation}.rs`, `src/{world,recorded}.rs`) take
       `account`.
     - 03.1's world: `tests/agent/tests/models.rs` has a private `fn
       world(settings: &Settings, charter: run::Charter) -> World`, which
       moves into `tests/agent/src/world.rs` with the granted accounts a
       parameter (README, "A test helper is written once").
     - 03.3: `crates/smith-host-domain/src/channel.rs`' `pub enum
       TranscriptRefusal` has `Endpoint` and `Dialect`, which go too.
     - Downstream: jig's `crates/jig-protocol-charter/src/{lib,tests,result}.rs`,
       `examples/ops/domain/src/{tests,limits}.rs` and
       `tests/ops/src/conformance/{scripts,config}.rs` name
       `smith_charter::v2` (`git grep -l` on jig origin/main), and temper's
       `crates/temper-service/src/tests/workers.rs`: the closing repins take
       them; `crates/jig-protocol-transcript/tests/data/README.md` ("Smith
       transcript-v5 goldens copied from smith revision") is refreshed with
       its `.bin` files.
     - The sketch: `record::Turn.dialect` and `Transcript.dialect` are `pub
       dialect: u32` at the base (`crates/smith-domain-session/src/record.rs`);
       only the codec's is text.
     No suite ran in the check (the queue held its run); a further user of
     what 03 changes is refreshed the same way and listed in the commit body.

121. **skein 09's Shapes against main** (ready check of skein-09.1).
     The ready check of skein-09.1 against skein 11565f3, then main 1c58a52,
     found 09's Shapes short of the code; `skein/09-driver-shared.md` now
     says, checked by `git grep` on skein origin/main 1c58a52:
     - Rules: each story module's `run` takes its flags as plain `bool`s, as
       `tests/oauth-accounts/src/sign_in.rs`' `pub fn run(seed: u64, story:
       Story, faulted: bool, large: bool, memory: Memory)` does; 09.1's world
       already calls `sign_out::run(seed, story, private, Memory::Checked)`,
       and `src/private.rs`' is `pub fn run(seed: u64, story: Story)`.
     - 09.1 and 09.3 Files take `component/names.rs`: `named` matches every
       `Request` with no `..` (`Request::Grant { account } =>
       Request::Grant { account: self.lookup(account, Asked::Grant, up)? }`,
       `Request::Kept { account, generation, keeping } => Request::Kept {
       account: self.terminal_place(account)?, .. }`); `SignOut` maps by
       `lookup(account, Asked::SignOut, up)`, `Locked` as `Kept` is mapped.
     - 09.1: an access-only record (`pub fn refresh_state(&self) ->
       Option<RefreshState>`, `crates/skein-oauth/src/record.rs`) is not
       revoked, oauth.md 2.3: "An access-only record has no refresh token and
       is never revoked by the client": `revoked` false, no connection, the
       record forgotten, a row of
       `a_sign_out_says_whether_revocation_was_confirmed`.
       `tests/oauth-accounts/src/sign_in.rs`' `match lower` gains a `Remove`
       arm.
     - 09.2: `missing` is `Box<[u8]>` (`Tokens { record: SavedToken, missing:
       Box<[u8]> }`, `crates/skein-oauth/src/machine.rs`), so the question to
       raise goes. `component.rs` drops it (`oauth::Request::Tokens { record,
       .. } =>` … `exchange.stage = Stage::Keeping { candidate: record };`):
       `exchange.rs`' `Keeping { candidate: oauth::SavedToken }` holds it
       until `SignedIn`. `component/private.rs`' `let sign_in_allowed = match
       failure {` lists each `Failure` (`Some(Failure::Unloaded { .. } |
       Failure::Expired | Failure::Exchange(_) | Failure::NotKept) => false`)
       and takes `NotGranted`, as 09.3's Files already take it for `Busy`.
     A further user of what 09 changes is refreshed the same way and listed
     in the commit body.

122. **smith 02's Shapes against main** (re-check of smith-02.1).
     The re-check of smith-02.1 against skein 11565f3 and smith 09be081e,
     on what moved since its ready check, found `smith/02-notices.md` short
     of the code; it now says, checked by `git grep` on smith origin/main
     5bd8fab8:
     - 02.1 Files take `tests/run/src/child_final_referee.rs` (`judge`:
       `FactKind::Ended { conversation, end: End::Budget(Exhausted::Turns),
       .. } =>`), whose pattern takes `EndKind`, and `tests/agent/src/memory.rs`
       (`Event::Failed {`), whose literal 02.1 changes as the listed files'.
     - 02.2: `tests/channel/tests/stories.rs`' `fn
       account_notices_reach_the_host_without_values()` already drives the
       agent's side (`world.agent_reports_unauthorized(3, 4)`) and stops at
       host-protocol (`Observation::HostUnauthorized { account: 3, generation:
       4 }`); `tests/channel/tests/composed.rs` composes both domains (`fn
       the_host_start_enters_the_agent_domain_and_its_answer_returns_to_the_host_domain()`).
       The proof, that each notice "crosses both real halves into the host
       domain", is a new `account_notices_reach_the_host_domain_without_values`
       in `composed.rs`, which joins 02.2's Files; Size about 290.
     - 02.3: `crates/smith-host-protocol/src/answer.rs`' comment "// smith
       02.1 gives the two new classes their domain form." names 02.3, which
       gives them.
     A further user of what 02 changes is refreshed the same way and listed
     in the commit body.

123. **smith 04's Shapes against main** (re-check of smith-04.1).
     The re-check of smith-04.1 against smith 09be081e and temper e307ddea5,
     on what moved since its ready check, found `smith/04-values.md` short of
     the code; it now says, checked by `git grep` on smith origin/main
     f4b38800 and temper origin/main 9c01f3636:
     - 04.1's reference fails as written (the check ran it: 13 of 14 passed
       before it): `tests/agent-process/src/world.rs`' `World::observe` skips
       its admission and reply assertions only for `TraceCase::Refused`
       (`if matches!(self.tracing, Some((TraceCase::Refused, _)))`), and the
       referee's `Terminal` wants `answers == 1 && parent.status ==
       Some(kernel::Exit::Code(0))` unless hung up. `World::granting` of an
       uninstallable value expects the entrance refusal: those assertions
       yield to it, `Terminal` takes the refused start's settlement, and the
       reference's `world.exit()` states its exit.
     - `tests/agent-process/src/lib.rs` has no `grant_bytes`: its Start
       builds `smith_channel::GrantValue::Withdrawn(` inline, and `fn
       scenario_credential(anthropic: bool, private_grant: bool) ->
       skein_llm::Credential` builds only the fake LLM's: the helpers replace
       the inline value, and there is no `scenario_grant`.
     - The byte paths 04.1 ends are named: `pub(crate) fn keep_wire(`
       (protocol-channel `credentials.rs`), `fn grant_bytes(limits:
       &smith_channel::Limits) -> Option<u64>` (`limits.rs`), `pub(crate) fn
       withdrawn(credential: &[u8], limits: &wire::Limits)` (host-protocol
       `translate.rs`), local-service's `pub fn credential(&mut self, grant:
       agent::Grant, envelope: Box<[u8]>)` and `pub envelope_bytes: u64`
       (`service.rs`), and the byte fixtures: `pub const INITIAL_GRANT:
       &[u8] = b"\0\0secret";` and its two siblings and `agent_grant_value`
       (`tests/channel/src/lib.rs`), `const SYNTHETIC_GRANT` (`composed.rs`),
       `b"\0\x03acctoken"` (`tests/hosts/src/lib.rs`).
     - 04.2 Files take `tests/local-process/src/process.rs`: `fn
       configure_choices` pushes `Endpoint { name: b"main".as_slice().into(),
       number: 0, dialect: 0, account: 0 }`, which moves to 1 with
       `configuration.rs`' charter. temper's `tests/real-loop/src/fixtures.rs`
       (`"dialect":0,"account":1,"provider":"codex"`) joins temper's repin.
     - The Rules' sizes are main's (`wc -l`): service.rs 2980, config.rs
       1676, process.rs 1431, service.rs 1340, component.rs 1029, stories.rs
       1256, world.rs 1595, lib.rs 1165, end_to_end.rs 1000.
     A further user of what 04 changes is refreshed the same way and listed
     in the commit body.

124. **smith 04.1's key row waits for smith-repin** (plan defect, smith 04's block of 2026-10-19T16:22:18Z).
     Ruling 124: content.rs's `no_grant_value_reaches_a_turn_fact_or_trace`
     takes its bearer row alone in smith 04.1; its key row waits for
     `smith-repin`, which adds it. The fake at smith's pin (skein 31bbf586)
     reads only `Authorization: Bearer`, so a key gets 401s and no provider
     reply; skein 04 (`skein/04-fake-credentials.md`) gives the fake
     `x-api-key` against the keys, and only `smith-repin` moves smith's pin
     to it (README, "No other step of this plan but `smith-repin` moves the
     pin"). 04.1's Proof already rests on the unit
     `each_grant_value_installs_its_scheme` for the key's installation, as
     decision 13 says. Keep the world's provider-reply assertions for an
     installable value (ruling 123), and assert no 401 path in this
     increment. Go on with 04.1 to its landing.

125. **skein 08's test shapes and named outcomes** (steer of 2026-10-19T16:30:20Z).
     The hourly check of 2026-10-19T16:30:20Z found skein 08's reference
     tests short of testing-strategy.md, section 10, and 08.2's landed
     `crates/skein-oauth-accounts/src/accounts_tests.rs` short of the
     README's rules; checked by `git grep` on skein origin/main 8564196:
     - 08.2's `an_added_name_is_refused_while_a_live_account_holds_it` holds
       three behaviours and asserts on the way (`assert!(up.pop().is_none(),
       "an admitted add answers nothing");`), and
       `an_add_past_the_accounts_limit_is_refused_as_full` holds both sides of
       the bound (`assert!(up.is_empty(), "the account bound admits both free
       places");`), as 08.2's Test shape wrote them. `skein/08-driver-accounts.md`
       now says: the taken-name test adds `ADDED`, unasserted, and asserts
       only the second add's `Refusal::Taken`; beside it,
       `a_free_name_is_added_with_no_answer`,
       `an_add_at_the_accounts_limit_is_admitted` and
       `an_add_past_the_accounts_limit_is_refused_as_full` (`Full { bound:
       ROOMY_ACCOUNTS }` only).
     - 08.3: `no_refresh_starts_before_the_earliest_refresh_time` gains its
       other side, `a_refresh_starts_at_the_earliest_refresh_time`, in Test
       shape and Proof. 08.3 edits `accounts_tests.rs`, so it brings 08.2's
       tests there to the rules: the taken-name and limit tests as above;
       `Bad` and `unusable(row: Bad)` (a `match row` building each `Account`)
       give way to rows that hold each `Account`, built by a named
       constructor; `type Failed = (u32, Ends, Failure);` and `type Sent =
       (Option<Failed>, (bool, u32, bool));` become structs; `finish_sent`'s
       `RESPONSE` (`Content-Length: 61`, counted by hand) takes a named
       `BODY` and `BODY.len()`; the hand driver of
       `a_removal_lets_a_sent_refresh_and_its_keep_finish` (`fn sent`, `fn
       finish_sent`, `fn finish_socket`, `fn finish_file`, about 100 lines)
       moves into `private_tests.rs`' `pub(super) struct Owner` as one named
       responder per effect, or that test leaves the behaviour to
       `tests/oauth-accounts/tests/accounts.rs`' `Story::Refreshing` and keeps
       what the owner decides. Size about 290.
     - 08.4: `an_account_with_port_zero_is_configured_then_signed_in`, which
       asserted four things on the way, becomes
       `an_account_with_port_zero_listens_on_port_zero` and
       `an_account_with_port_zero_visits_at_its_bound_port` (also the
       Proof's); `a_redirects_client_id_reaches_the_machine` gains the row at
       its cap; `a_public_sign_in_visits_only_after_its_listener_listens`
       compares the effects up to the `Visit` once.
     - The README's Rules of this plan gain "A test's outcome is named, and
       its sizes derived" (testing-strategy.md, 10, "Named fixtures and
       sizes"): no tuple read by position, no size counted by hand.
     A further copy of these patterns in a file a step edits is brought to
     the rules the same way, and listed in the commit body; copies in files
     no step touches stay as they landed.

126. **skein 04's Shapes against main** (re-check of skein-04.1).
     The re-check of skein-04.1, on what moved since its ready check (skein
     b481ad9 to 8564196, temper efe462a to c896c4287), found
     `skein/04-fake-credentials.md` short of the code and of fake-llm.md 3;
     04.1 has since landed (skein fb4fc28). Checked by `git grep` on skein
     origin/main fb4fc28 and temper 859e9f166, it now says:
     - 04.2's `Presented` is what fake-llm.md 3 says in its own words: "an
       accepted credential, by its scheme and its index; a credential that
       matched none, by the scheme its field shows (a key in `x-api-key`,
       else a bearer) and no index; or no credential field at all, as the
       scheme none and no index". 04.4's reference test (an accepted key at
       index 0 on the public route, `Some(Scheme::Key)`) stands; its
       `a_credential_the_peer_does_not_hold_is_unauthorized` rows expect, on
       the public route, `Bearer` with no index for both a key and a bearer.
     - 04.2: the observation count is `run_configured`'s (`observations:
       calls * 5,` in `tests/llm-connection/src/simulated.rs`, which
       `run_with_shutdown` calls), not `run_with_shutdown`'s; `make_on`
       derives the `documents::Provider` and the path from `endpoint`.
     - Downstream of jig, its `tests/real-loop/src/llm.rs` builds a
       `provider::Config` literal in `scripted` and calls `Peer::new` in
       `peer` (04.1's entry change, for its repin), and takes
       `smith_agent_process_world::fake::limits()` directly (`limits:
       smith_agent_process_world::fake::limits(),`); its
       `tests/one-process` takes it through `fake::configured`: both join
       04.2's worlds counted against 42.
     A further user of what 04.2 to 04.4 change is refreshed the same way and
     listed in the commit body.

127. **skein 11's Shapes against main** (ready check of skein-11.1).
     The ready check of skein-11.1 against skein 8564196, smith 6c34c8d9, jig
     b5d523d3, hearth 61ab3d1 and temper c896c4287 found
     `skein/11-fake-loopback.md` short of the code and of its own Rule ("no
     token, code, ID token or secret in an observation"); checked by `git
     grep` on skein origin/main fb4fc28 and jig 1bc89fa7, it now says:
     - `Observation::Redirect { connection: Token, location: Box<[u8]> }`
       (testing/skein-fake-peers/src/oauth.rs) holds the code, as the check's
       test saw ("an observation holds 0000000000000001"): it becomes
       `Redirect { connection }`, in Shape, sketch, Removes and Done when. No
       user reads `location` in the five repositories; smith's
       `tests/local-process/src/oauth.rs` reads the head's `Location`.
     - `skein_fake_oauth::openid::document` matches a whole URL (`suffix ==
       b"/.well-known/openid-configuration"`; `endpoint ==
       openid.key_set_url.as_ref()`): `Peer` keeps the metadata and key-set
       URLs and the revocation endpoint, as it keeps `token_endpoint`;
       `Paths`' `revoke`, `metadata` and `key_set` are `Option`s, the
       metadata path taken from the whole metadata URL.
     - `fn post` clears `token_post` before `deliver` runs (`if
       !connection.token_post { return; } connection.token_post = false;`):
       `call` sets `post` for the four routes, `deliver`'s `Request::Http`
       arm reads and clears it, and a Metadata or KeySet answer observes no
       `Answered`, which jig's `tests/protocol-accounts/src/world.rs` waits
       on (`peer::Observation::Answered { status: 200, .. }`).
     - `route` takes no `_ =>` on `Method` (`wildcard_enum_match_arm =
       "deny"`, skein-fake-peers' `Cargo.toml`); helpers outside a `#[test]`
       use `expect` (`unwrap_used = "deny"`; clippy.toml's
       `allow-unwrap-in-tests = true` covers `#[test]` functions only).
     - The fake peers' tests: `carried: 1`, a public client's PKCE fields,
       `observations: 16` exactly four requests, `fetch` taking the peer's
       `Pid`; `route_names_each_endpoint`'s prose matches its rows (only
       the authorize target has a query). skein's
       `tests/oauth-accounts/src/world.rs` and `sign_in.rs` match `Post {
       .. }` and need no change. Size about 260.
     A further user of what 11.1 changes is refreshed the same way and listed
     in the commit body.

128. **skein 07's Shapes against main** (re-check of skein-07.1).
     The re-check of skein-07.1, from skein c7f4949 to 8564196 (06.2 landed),
     found `skein/07-openid.md` short of the code; checked by `git grep` on
     skein origin/main 4dc2ce3, it now says:
     - 07.1's `HttpRequest` literals add `testing/skein-fake-oauth/src/tests.rs`
       (`fn post`, `fn exchange`: `request: HttpRequest {`), each taking
       `method: Method::Post`; the one in `tests/oauth-accounts/src/world.rs`
       is in `pub(crate) fn protocol_client`; `tests/oauth/tests/memory.rs`'
       `ClientLimits` literal is in `fn check_client`.
     - The reference test `the_keys_are_fetched_again_at_each_sign_in` ends
       each round with `assert!(client_step(&mut client,
       oauth::Event::Reset).is_none());`: `client_step` is `#[must_use]` and
       `tests/oauth/Cargo.toml` has `unused_must_use = "deny"`.
     - `machine_tests/terms.rs`' `terms(openid, ..)` builds `OpenId { issuer:
       boxed(b"https://issuer.example"), key_set: None }`, whose first GET is
       the metadata: `terms(true, ..)` names a key set URL
       (`https://issuer.example/keys`), so that the nonce tests answer one
       GET with a one-key set literal.
     - `world.rs`' `an_openid_sign_in_fails_before_its_check_exists` builds
       `issuer_for(&registration)` (`configured_issuer(registration, None)`,
       a fake with no `OpenId`, which answers each GET 404): it takes 07.1's
       `openid_registration()` and `openid_issuer(None)` instead.
     - 06.2 left `documents()` at `document_bytes: 4096, string_bytes: 1024`,
       so "under main's 1024 and 256" goes.
     - 07.2: `jwt.rs`' module doc ("Payload metadata only: this decoder does
       not authenticate JWT signatures.") names `verify_id_token`'s check;
       the fake's `Fault::Expired(ago)` takes `ago` from the wall
       (`wall.as_secs().checked_sub(..)`, else `Error::Limits`), so each
       world row's token POST is at `wall(n)`, n at least 60.
     A further user of what 07.1 or 07.2 changes is refreshed the same way
     and listed in the commit body.

129. **The order toward the repins** (director's request of 2026-10-19T17:22Z).
     The director reports that forge-kit's temper 02.1 waits on this run's
     `smith-repin` and `jig-repin`, and asks that the steps on their path go
     first, the plan's dependencies unchanged. The README now says:
     - The closing repins gain `jig-repin.s1`, which the README already plans
       ("the supervisor adds `jig-repin.s1` once smith 01 and 04 have landed",
       decision 22): it starts once smith 01 (landed, 3 of 3) and smith 04
       (04.1 landed; 04.2 running) have, and moves jig's smith to
       `origin/main` and its skein with `--precise` to smith's `Cargo.lock`
       pin, repairing smith 01's and 04's breaks. Added now so that it starts
       at 04.2's landing; the closing `jig-repin` still runs.
     - "When slots are short" gains the order from here: `jig-repin.s1`,
       skein 09 (09.1 to 10.2, five steps, the longest chain left to
       `smith-repin`, which needs every skein step), skein 04 (04.2 to 04.4,
       then 11.1), skein 07, smith 02 (02.2 to 02.4, then smith 05, which
       `jig-repin` needs with every smith step), smith 03, smith 05, then
       skein 11.
     Checked: `plan status` at 17:18Z (smith 01 3 of 3 landed; smith 04 1 of
     2, 04.2 running); the kit's `plan set STEP KEY=VALUE…` adds a ledger row
     (`set) set_step "$@"`).

130. **jig's refused notice** (cross-session scope, jig repin.s1's block of 2026-10-19T17:52:15Z).
     No accessor. smith's `parent::Request::Refused` reaches no hub event in
     jig: its arm does nothing, and the run's failed answer, which smith sends
     as before, carries the refusal. Decision 16's bullet "the refused notice
     is handled as the rejected one was" was a plan defect, now amended to
     "reaches no event downstream":
     - Decision 16 itself says the repins "keep today's behaviour", and the
       director kept it as "map ... the refused notice onto today's outcomes"
       (decisions.md, For the user, C). Today a refused completion raises no
       notice at all: before smith 02.2 (92192967), `Failure::Refused` sat
       with `Overloaded` and `Unavailable` in the arm that pushes none.
     - smith's design, in its own words: a run that met a refused account
       "ends failed for its account, except an unauthorized credential, which
       a refreshed grant may answer" (host.md 7). jig's rejected is "a
       generation unauthorized by its provider" (llm.md 5.2), which starts a
       refresh. A refused notice routed as rejected would refresh a grant that
       no refresh answers, with a generation the notice does not carry.

     So in `testing/jig-test-worker/src/domain.rs`, `up`:
     `smith::parent::Request::Unauthorized { client, account, generation }`
     goes to `host::Event::Rejected` as `Rejected` did, `Limited { client,
     account, reset }` to `host::Event::Exhausted { retry_after: reset, .. }`,
     and `Refused { .. }` to nothing. jig-domain-host gains no public API.
     jig's own handling of the refused notice stays with the second part
     (later.md's "jig's part of LLM accounts" row).

     temper's repin meets the same match in
     `crates/temper-worker-domain/src/agents.rs`, `up`, and maps it the same
     way. Any further match on smith's `Refused` notice downstream takes the
     same arm, listed in its commit body.

## For the user

The choices below have long-term consequences; each is the planner's,
and the user, or the director, may veto it before the run:

- **A. The licence's copyright line** (11): `Copyright (c) 2026 Free
  Ekanayaka`, in skein and smith. A veto names the holder.
  Decision: keep, by director, 2026-10-19.
- **B. smith's fault names** (3): `Declined` for the model's own
  refusal, `Refused` and `Limited` for an account's. A veto keeps
  `Refused` for the model and names the account's `AccountRefused`.
  Decision: keep, by director, 2026-10-19.
- **C. The repins' defaults downstream** (16): jig's and the
  downstream repository's repins map the driver's new failures and the
  refused notice onto today's outcomes, and keep sending the withdrawn
  grant, until their own part of LLM accounts. A veto makes the mapping
  a question for jig's designer, and the jig repin waits for it.
  Decision: keep, by director, 2026-10-19.
- **D. No live check of this plan's routes** (9, 10): the public route
  and Anthropic's key route are proven against the fake only, until
  `accounts-sources`' live suite and a capture with an OpenAI key.
  Decision: keep, by director, 2026-10-19.
