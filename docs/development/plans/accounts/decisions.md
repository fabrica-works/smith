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
    - the refused notice is handled as the rejected one was;
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
