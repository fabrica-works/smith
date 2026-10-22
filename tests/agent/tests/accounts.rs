//! Charter accounts are admitted before any provider effect (domain/run.md, section 4).
use smith_agent_world::{Job, Settings, scripted_charter, world};
use smith_domain::run;

#[test]
fn a_start_without_an_accounts_grant_is_invalid() {
    for (index, main, child, granted) in [(0, Some(9), Some(0), 0), (1, Some(0), Some(9), 0), (2, Some(8), Some(9), 8)]
    {
        let settings = Settings { job: Job::Reporting, ..Settings::calm(1501 + index) };
        let mut charter = scripted_charter(&settings);
        charter.llm.account = main;
        charter.models.get_mut(0).expect("child LLM").account = child;
        let mut world = world(&settings, charter, &[granted]);
        world.run(2000);
        let invalid = run::Refusal::Invalid(run::Invalid::Grants);
        assert_eq!(world.answer(), &run::Answer::Refused(invalid), "row {index}");
        assert!(world.prompts().is_empty() && world.turns().is_empty(), "row {index}");
    }
}

#[test]
fn llms_sharing_an_account_share_its_grant() {
    let settings = Settings { job: Job::Reporting, ..Settings::calm(1504) };
    let mut world = world(&settings, scripted_charter(&settings), &[0]);
    world.run(2000);
    assert!(matches!(world.answer(), run::Answer::Accepted { .. }));
    assert!(!world.prompts().is_empty());
}

#[test]
fn an_llm_with_no_account_is_admitted_with_no_grant() {
    let settings = Settings { job: Job::Reporting, ..Settings::calm(1505) };
    let mut charter = scripted_charter(&settings);
    charter.llm.account = None;
    for llm in &mut charter.models {
        llm.account = None;
    }
    let mut world = world(&settings, charter, &[]);
    world.run(2000);
    assert!(matches!(world.answer(), run::Answer::Accepted { .. }));
    assert!(!world.prompts().is_empty());
}

#[test]
fn a_held_grant_serves_a_later_start() {
    let settings = Settings { job: Job::Reporting, ..Settings::calm(1506) };
    let mut world = world(&settings, scripted_charter(&settings), &[]);
    world.hold_grant(0);
    world.run(2000);
    assert!(matches!(world.answer(), run::Answer::Accepted { .. }));
    assert!(!world.prompts().is_empty());
}

use skein_fake_checkout::Checkout;
use skein_fake_llm_domain::api::{Finish, Line, Script, Turn};
use skein_lib::{Duration, ReplyTo, Time, Token};
use smith_agent_world::{World, messages_referee::Seen};
use smith_domain::{Event, Fact, Grant, GrantName, Window, llm, session};

// The ordinary allowance is three retries and an hour; no backoff/time cap is shortened.
const RETRIES: u32 = 3;
const GRANT_LIFETIME: Duration = Duration::from_secs(7200);
const RESPONSE_LATENCY: Duration = Duration::from_millis(40);
const GENERATION_TWO_AT: Time = Time::from_nanos(1_000_000);
const RESPONSE_401: llm::Evidence = llm::Evidence::Response { status: Some(401) };

fn grant(generation: u64, renewable: bool, valid: Duration) -> Grant {
    Grant { name: GrantName { account: 0, generation }, valid, renewable }
}

fn account_world(account: Option<u32>, held: Option<Grant>, failures: u32, failure: llm::Failure) -> World {
    let mut settings = Settings { job: Job::Reporting, ..Settings::calm(1510) };
    settings.limits.session.retries = RETRIES;
    settings.provider.latency_min = RESPONSE_LATENCY;
    settings.provider.latency_max = RESPONSE_LATENCY;
    let mut charter = scripted_charter(&settings);
    charter.llm.account = account;
    charter.models = Box::new([]);
    let start = Event::Start {
        reply_to: ReplyTo::new(Token::new(1)),
        host_run: Token::new(1),
        activation: 1,
        identity: None,
        window: Window { turns: u32::MAX, bytes: u64::MAX },
        messages: Box::new([]),
        charter,
        workspace: None,
        transcript: None,
        answered: Box::new([]),
        grants: held.into_iter().collect(),
    };
    let scripts = Box::new([Script {
        cue: b"@report".as_slice().into(),
        turns: Box::new([Turn {
            lines: Box::new([Line::Call {
                name: b"finish".as_slice().into(),
                arguments: br#"{"report":"Account answered.","source":"provider"}"#.as_slice().into(),
            }]),
            finish: Finish::ToolCalls,
            tokens: 8,
        }]),
    }]);
    let mut world = World::with_workspace_scripts_start(settings, Checkout::new(), scripts, start);
    world.fail_calls(failures, failure, RESPONSE_401);
    world.failure_report(llm::Report {
        code: b"invalid-credential".as_slice().into(),
        request: b"request-401".as_slice().into(),
        field: Box::default(),
    });
    world
}

fn counts(world: &World) -> (usize, usize, usize, usize) {
    let retries = world
        .facts()
        .iter()
        .filter(|fact| {
            matches!(
                fact,
                Fact::Session { fact: session::Fact { kind: session::FactKind::CompletionRetried { .. }, .. } }
            )
        })
        .count();
    let terminals = world.messages_seen().iter().filter(|(_, seen)| matches!(seen, Seen::Answer { .. })).count();
    (world.completions().len(), retries, terminals, world.unauthorized().len())
}

#[test]
fn nonrenewable_401_count_proof() {
    // Attempts/retries/activation terminals/notices, observed at actual boundaries.
    // The first row is the regression: former policy produces (4, 3, 1, 1).
    for (label, account, held, replacement, expected, generations) in [
        ("static key", Some(0), Some(grant(1, false, GRANT_LIFETIME)), None, (1, 0, 1, 1), vec![1]),
        ("no account", None, None, None, (1, 0, 1, 0), vec![]),
        (
            "old key/new key",
            Some(0),
            Some(grant(1, false, GRANT_LIFETIME)),
            Some(grant(2, false, GRANT_LIFETIME)),
            (2, 1, 1, 2),
            vec![1, 2],
        ),
    ] {
        let mut world = account_world(account, held, RETRIES + 1, llm::Failure::Unauthorized);
        if let Some(replacement) = replacement {
            world.grant_at(GENERATION_TWO_AT, replacement);
        }
        world.run(2000);
        let observed = counts(&world);
        eprintln!("{label}: observed attempts/retries/terminals/notices = {observed:?}");
        assert_eq!(observed, expected, "{label}");
        assert_eq!(
            world.unauthorized().iter().map(|(_, grant)| grant.generation).collect::<Vec<_>>(),
            generations,
            "{label}: every rejected generation keeps its notice"
        );
        assert_eq!(
            world.completions().iter().filter_map(|call| call.grant.map(|grant| grant.generation)).collect::<Vec<_>>(),
            generations,
            "{label}: the retry dispatches the replacement"
        );
        assert!(
            world
                .completions()
                .iter()
                .all(|call| matches!(call.terminal, Some((_, smith_agent_world::CompletionTerminal::Failed)))),
            "{label}"
        );
        assert_eq!(
            world.answer(),
            &run::Answer::Failed {
                failure: run::Failure::Model(run::Fault::Completion {
                    failure: run::CompletionFailure::Unauthorized,
                    evidence: run::CompletionEvidence::Response { status: Some(401) },
                    model: 0,
                    attempts: u32::try_from(expected.0).expect("bounded attempts"),
                    report: run::Report {
                        code: b"invalid-credential".as_slice().into(),
                        request: b"request-401".as_slice().into(),
                        field: Box::default(),
                    },
                }),
                spent: run::Spend::ZERO,
                turns: 0,
            },
            "{label}: original failure, evidence, report, model and zero accepted spend/turns"
        );
        assert!(
            world
                .facts()
                .iter()
                .filter(|fact| matches!(
                    fact,
                    Fact::Session { fact: session::Fact { kind: session::FactKind::CompletionFailed { .. }, .. } }
                ))
                .all(|fact| matches!(
                    fact,
                    Fact::Session {
                        fact: session::Fact {
                            kind: session::FactKind::CompletionFailed {
                                failure: llm::Failure::Unauthorized,
                                evidence: RESPONSE_401,
                                ..
                            },
                            ..
                        }
                    }
                )),
            "{label}: failure facts preserve their class and response evidence"
        );
        assert!(world.next_deadline().is_none(), "{label}: no retry deadline survives the terminal");
        if expected.1 == 0 {
            assert_eq!(
                world.answered_at(),
                world.completions()[0].terminal.expect("actual failure").0,
                "{label}: terminal does not wait for backoff"
            );
        }
    }
}

#[test]
fn recoverable_failures_keep_their_attempt_counts() {
    for (label, renewable, lifetime, failures, failure, replacement, expected) in [
        (
            "bearer refreshed",
            true,
            GRANT_LIFETIME,
            1,
            llm::Failure::Unauthorized,
            Some(grant(2, true, GRANT_LIFETIME)),
            (2, 1, 1, 1),
        ),
        ("overloaded key", false, GRANT_LIFETIME, 1, llm::Failure::Overloaded, None, (2, 1, 1, 0)),
        ("unavailable key", false, GRANT_LIFETIME, 1, llm::Failure::Unavailable, None, (2, 1, 1, 0)),
        (
            "lapsed key",
            false,
            Duration::ZERO,
            0,
            llm::Failure::Unauthorized,
            Some(grant(2, false, GRANT_LIFETIME)),
            (1, 1, 1, 0),
        ),
    ] {
        let mut world = account_world(Some(0), Some(grant(1, renewable, lifetime)), failures, failure);
        if let Some(replacement) = replacement {
            world.grant_at(GENERATION_TWO_AT, replacement);
        }
        world.run(2000);
        eprintln!("{label}: observed attempts/retries/terminals/notices = {:?}", counts(&world));
        assert_eq!(counts(&world), expected, "{label}");
        assert!(matches!(world.answer(), run::Answer::Accepted { .. }), "{label}");
        assert_eq!(
            world
                .completions()
                .iter()
                .filter(|call| matches!(call.terminal, Some((_, smith_agent_world::CompletionTerminal::Completed(_)))))
                .count(),
            1,
            "{label}"
        );
        assert!(world.next_deadline().is_none(), "{label}: retry and reservation settle");
        let last = world.completions().last().expect("accepted provider response");
        assert_eq!(
            last.grant.expect("account retained").generation,
            replacement.map_or(1, |grant| grant.name.generation),
            "{label}"
        );
    }
}
