//! The agent's run child domain at random: many noisy worlds, each settled with
//! every start answered once, with broad ending coverage among them.

use std::collections::{BTreeMap, BTreeSet};

use skein_lib::{Duration, Rng};
use smith_domain_run as run;
use smith_domain_run::{Answer, Exhausted, Failure, Fault, Invalid, Limits, Policy, Refusal};
use smith_run_world::host;
use smith_run_world::partner::Script;
use smith_run_world::{Checkouts, Settings, Span, World, noisy};

const ITERATIONS: u32 = 1_000_000;

fn settled(settings: &Settings) -> World {
    let mut world = World::new(*settings);
    world.run(ITERATIONS);
    world
}

fn answers(world: &World) -> Vec<&Answer> {
    world.answers().map(|answer| answer.expect("every start is answered")).collect()
}

fn time_pressure() -> Settings {
    let mut settings = Settings::calm(34);
    settings.host.jobs = 1;
    settings.host.agents = 1000;
    settings.host.writable = 0;
    settings.host.reports = 1000;
    settings.host.verdicts = 0;
    settings.host.host_tools = 0;
    settings.host.turns_min = 20;
    settings.host.turns_max = 20;
    settings.host.time = smith_run_world::Span::millis(3000, 3000);
    settings.host.reserve = smith_domain_run::charter::Reserve { turns: 2, time: Duration::from_secs(1) };
    settings.partner.turn = smith_run_world::Span::millis(1800, 1800);
    settings.partner.asks = 1000;
    settings.partner.yields = 0;
    settings.partner.changes = 0;
    settings
}

/// Hundreds of worlds with random limits, scripts, faults and schedules: each
/// settles with its invariants holding (checked by `World::run`) and reaches
/// the listed endings. A dedicated focused story covers stale as a final answer.
#[test]
fn random_worlds_settle_with_every_start_answered_once() {
    let mut seen = BTreeSet::new();
    let (mut cancels, mut races, mut stale) = (0, 0, 0);
    let (mut waited, mut delivered, mut host_answered) = (0, 0, 0);
    let (mut busy, mut lost, mut answered) = (0, 0, 0);
    // Ordinary expiry now starts wind-down or parks. Retain the noisy corpus
    // and add a long child turn that actually crosses the full run deadline.
    for seed in 0..304 {
        let settings = match seed {
            300 => time_pressure(),
            301 => {
                let mut settings = Settings::calm(3);
                settings.run.nudges = 0;
                settings.partner.yields = 1000;
                settings.partner.odd_stops = 1000;
                settings
            }
            302 => {
                let mut settings = Settings::calm(5);
                settings.host.tokens_min = 80_000;
                settings.host.tokens_max = 100_000;
                settings.host.turns_min = 100;
                settings.host.turns_max = 100;
                settings.partner.yields = 0;
                settings
            }
            303 => {
                let mut settings = Settings::calm(seed);
                settings.host.jobs = 1;
                settings.host.host_tools = 1000;
                settings.host.relay_lost = 1000;
                settings.host.relay_busy = 0;
                settings.partner.host_calls = 1000;
                settings.partner.yields = 0;
                settings
            }
            _ => noisy(seed),
        };
        let world = settled(&settings);
        assert_eq!(
            smith_run_world::child_final_referee::judge(world.facts_seen()),
            skein_world::domain::Verdict::Passed,
            "child quota seed {seed}"
        );
        let stats = world.stats();
        cancels += stats.cancels;
        races += stats.partner.races;
        stale += stats.partner.stale;
        waited += stats.partner.waiting;
        delivered += stats.partner.delivered;
        host_answered += stats.partner.host_answered;
        busy += stats.relay_busy;
        lost += stats.relay_lost;
        answered += stats.relay_answered;
        for answer in answers(&world) {
            let kind = answer_kind(answer);
            seen.insert(kind);
        }
    }
    let expected = [
        "accepted",
        "busy",
        "cancelled",
        "conversation invalid",
        "fault",
        "invalid",
        "parked",
        "stopped",
        "time",
        "spend",
        "turns",
        "unfinished",
    ];
    assert!(expected.iter().all(|kind| seen.contains(kind)), "missing sampled ending: {seen:?}");
    // And the races: turns spent after a close, a close or nudge crossing a
    // conversation's own end, conversations out of time, cancels.
    assert!(cancels > 0 && races > 0 && stale > 0, "{cancels} {races} {stale}");
    assert!(
        waited > 0 && delivered > 0 && host_answered > 0 && busy > 0 && lost > 0 && answered > 0,
        "wait {waited}, delivered {delivered}, host answered {host_answered}, relays {busy}/{lost}/{answered}"
    );
}

#[test]
fn random_reserves_keep_parallel_children_above_the_finishing_pool() {
    let mut children = 0;
    let mut main_in_reserve = 0;
    for seed in 900..960 {
        let mut rng = Rng::new(seed);
        let calm = Settings::calm(seed);
        let settings = Settings {
            host: smith_run_world::host::Script {
                jobs: 1,
                agents: 1000,
                writable: 0,
                reports: 1000,
                verdicts: 0,
                turns_min: u32::try_from(rng.between(4, 10)).expect("small drawn turns"),
                turns_max: 12,
                reserve: smith_domain_run::charter::Reserve {
                    turns: u32::try_from(rng.between(1, 3)).expect("small drawn reserve"),
                    time: Duration::from_secs(rng.between(1, 30)),
                },
                ..calm.host
            },
            partner: smith_run_world::partner::Script {
                asks: 800,
                finishes: 200,
                reports: 1000,
                yields: 0,
                shares: u32::try_from(rng.between(0, 1000)).expect("per-mille draw"),
                parallel: 3,
                ..calm.partner
            },
            ..calm
        };
        let world = settled(&settings);
        let observed = world.reserve_progress();
        children += observed.0;
        main_in_reserve += observed.1;
    }
    assert!(children > 0 && main_in_reserve > 0, "the sweep actually admits children and reserved main turns");
}

#[test]
fn drawn_prices_reserve_main_and_children_without_spending_past_the_ceiling() {
    let mut rng = Rng::new(0x0712);
    let mut charged = 0;
    for seed in 0..48 {
        let mut settings = Settings::calm(seed);
        settings.host.jobs = 1;
        settings.host.writable = 0;
        settings.host.verdicts = 0;
        settings.host.reports = 1000;
        settings.host.agents = 1000;
        settings.host.host_tools = 0;
        settings.host.reserve = smith_domain_run::charter::Reserve { turns: 2, time: Duration::from_secs(1) };
        settings.partner.prices = Some(smith_domain_run::Prices {
            input: rng.between(1, 5),
            cached: rng.between(1, 9),
            cache_write: Some(rng.between(1, 7)),
            output: rng.between(1, 8),
            unit: u32::try_from(rng.between(1, 100)).expect("small denominator"),
        });
        settings.partner.asks = 600;
        settings.partner.finishes = 300;
        settings.partner.changes = 0;
        settings.partner.parallel = 3;
        settings.partner.yields = 0;
        let world = settled(&settings);
        for (_, _, charge) in world.charges_seen() {
            assert!(charge.spent.units <= charge.budget.spend, "price seed {seed}: {charge:?}");
            charged += 1;
        }
    }
    assert!(charged > 48, "priced main and child callbacks actually enter the shared pool");
}

/// A world where the LLM finishes, with outcomes that fit, and changes land
/// after their checks fail a time or two.
fn finishing(seed: u64) -> Settings {
    let calm = Settings::calm(seed);
    Settings {
        host: host::Script { writable: 1000, changes: 1000, verdicts: 1000, ..calm.host },
        partner: Script { finishes: 300, yields: 0, ..calm.partner },
        checkout: Checkouts { checks: 1000, check_failures: 2, ..calm.checkout },
        ..calm
    }
}

fn wind_down_pressure(seed: u64, cancel: bool) -> Settings {
    let mut settings = Settings::calm(seed);
    settings.host.jobs = 1;
    settings.host.window = Duration::ZERO;
    settings.host.writable = 0;
    settings.host.verdicts = 0;
    settings.host.reports = 1000;
    settings.host.host_tools = 0;
    settings.host.time = Span::millis(6000, 6000);
    settings.host.reserve = run::charter::Reserve { turns: 2, time: Duration::from_secs(2) };
    settings.host.cancels = if cancel { 1000 } else { 0 };
    settings.host.cancel = Span::millis(5000, 5000);
    settings.partner.turn = Span::millis(3000, 3000);
    settings.partner.yields = 0;
    settings.partner.changes = 0;
    settings.partner.finishes = 0;
    settings
}

/// Adds what `world` counted of the states cancels and deadlines found runs in.
fn tally(world: &World, cancels: &mut BTreeMap<&'static str, u32>, deadlines: &mut BTreeMap<&'static str, u32>) {
    for (cell, count) in world.cancel_cells() {
        *cancels.entry(*cell).or_insert(0) += count;
    }
    for (cell, count) in world.deadline_cells() {
        *deadlines.entry(*cell).or_insert(0) += count;
    }
}

/// Cancels at random moments, from the host (once, twice, late) and handed
/// to the run right before or after any of its events, find runs in every
/// state, and each answers once (checked by `World::run`).
fn cancels_find_runs_in_every_state_over_many_seeds_cells() -> BTreeMap<&'static str, u32> {
    let (mut cancels, mut deadlines) = (BTreeMap::new(), BTreeMap::new());
    for seed in 0..250 {
        tally(&settled(&noisy(seed)), &mut cancels, &mut deadlines);
    }
    // Cancelled twice while the run reads a slow checkout.
    let calm = Settings::calm(20);
    let host = host::Script { cancels: 1000, cancel: Span::millis(0, 1_000), recancels: 1000, ..calm.host };
    let checkout = Checkouts { io: Span::millis(3_000, 5_000), ..calm.checkout };
    tally(&settled(&Settings { host, checkout, ..calm }), &mut cancels, &mut deadlines);
    // Cancelled as changes are checked and pushed, and as the LLM spends past
    // its budget.
    for seed in 21..24 {
        let landing = finishing(seed);
        let run = Limits {
            runs: 16,
            conversations: 16,
            calls: 16,
            check_timeout: Duration::from_millis(1_999),
            wrap_up: Duration::from_secs(60),
            ..landing.run
        };
        let host = host::Script {
            jobs: 16,
            cancels: 1000,
            cancel: Span::millis(2_000, 30_000),
            tokens_min: 80_000,
            tokens_max: 100_000,
            ..landing.host
        };
        let partner = Script { changes: 1000, ..landing.partner };
        let checkout = Checkouts { check: Span::millis(2_000, 8_000), check_failures: 3, ..landing.checkout };
        let settings = Settings { run, host, partner, checkout, inject: 20, ..landing };
        tally(&settled(&settings), &mut cancels, &mut deadlines);
    }
    // Grant one new operation at a time, so cancellation can find its retained
    // phase rather than an unrelated completion or a denied charter.
    for seed in 1000..1020 {
        let calm = Settings::calm(seed);
        let host = host::Script {
            cancels: 1000,
            cancel: Span::millis(100, 2_000),
            time: Span::millis(60_000, 60_000),
            ..calm.host
        };
        let partner = Script { waits: 1000, yields: 0, turn: Span::millis(10, 50), ..calm.partner };
        tally(&settled(&Settings { host, partner, ..calm }), &mut cancels, &mut deadlines);

        let host = host::Script {
            cancels: 1000,
            cancel: Span::millis(100, 6_000),
            host_tools: 1000,
            relay: Span::millis(8_000, 8_000),
            relay_busy: 500,
            relay_lost: 500,
            time: Span::millis(60_000, 60_000),
            ..calm.host
        };
        let partner = Script { host_calls: 1000, yields: 0, turn: Span::millis(10, 50), ..calm.partner };
        tally(&settled(&Settings { host, partner, ..calm }), &mut cancels, &mut deadlines);

        let host = host::Script {
            cancels: 1000,
            cancel: Span::millis(30, 600),
            host_tools: 1000,
            relay: Span::millis(1, 1),
            relay_busy: 1000,
            relay_lost: 0,
            time: Span::millis(60_000, 60_000),
            ..calm.host
        };
        tally(&settled(&Settings { host, partner, ..calm }), &mut cancels, &mut deadlines);
    }
    tally(&settled(&wind_down_pressure(901, true)), &mut cancels, &mut deadlines);
    cancels
}

#[test]
fn cancels_find_runs_in_every_state_over_many_seeds() {
    let cancels = cancels_find_runs_in_every_state_over_many_seeds_cells();
    let cells = [
        "preparing",
        "stopping",
        "opening",
        "working",
        "waiting",
        "relay sending",
        "relay withdrawing",
        "relay backoff",
        "landing",
        "wind-down",
        "winding",
        "answered",
        "gone",
    ];
    for cell in cells {
        assert!(cancels.get(cell).is_some_and(|count| *count > 0), "no cancel found a run {cell}: {cancels:?}");
    }
}

/// Deadlines short against a slow checkout and slow conversations fall due in
/// every state a run's deadline runs in.
fn deadlines_find_runs_in_every_state_they_run_in_over_many_seeds_cells() -> BTreeMap<&'static str, u32> {
    let (mut cancels, mut deadlines) = (BTreeMap::new(), BTreeMap::new());
    for seed in 0..50 {
        let calm = Settings::calm(seed);
        let host = host::Script { time: Span::millis(1_000, 6_000), writable: 0, verdicts: 1000, ..calm.host };
        let checkout = Checkouts { io: Span::millis(0, 2_000), ..calm.checkout };
        let partner = Script { turn: Span::millis(100, 2_000), ..calm.partner };
        let settings = Settings { host, checkout, partner, hop: Span::millis(0, 1_500), ..calm };
        tally(&settled(&settings), &mut cancels, &mut deadlines);
    }
    // And as changes are checked and pushed, and as the LLM spends past its
    // budget.
    for seed in 50..56 {
        let landing = finishing(seed);
        let run = Limits {
            runs: 16,
            conversations: 16,
            calls: 16,
            check_timeout: Duration::from_millis(1_999),
            wrap_up: Duration::from_secs(60),
            ..landing.run
        };
        let host = host::Script {
            jobs: 16,
            time: Span::millis(2_000, 15_000),
            tokens_min: 80_000,
            tokens_max: 100_000,
            ..landing.host
        };
        let partner = Script { changes: 1000, turn: Span::millis(100, 2_000), ..landing.partner };
        let checkout = Checkouts { check: Span::millis(2_000, 8_000), check_failures: 3, ..landing.checkout };
        let settings = Settings { run, host, partner, checkout, hop: Span::millis(0, 1_500), ..landing };
        tally(&settled(&settings), &mut cancels, &mut deadlines);
    }
    for seed in 2000..2020 {
        let calm = Settings::calm(seed);
        let partner = Script { waits: 1000, yields: 0, turn: Span::millis(10, 50), ..calm.partner };
        let host = host::Script { time: Span::millis(100, 500), ..calm.host };
        tally(&settled(&Settings { host, partner, ..calm }), &mut cancels, &mut deadlines);

        let partner = Script { host_calls: 1000, yields: 0, turn: Span::millis(10, 50), ..calm.partner };
        let host = host::Script {
            host_tools: 1000,
            relay: Span::millis(8_000, 8_000),
            time: Span::millis(100, 6_000),
            ..calm.host
        };
        tally(&settled(&Settings { host, partner, ..calm }), &mut cancels, &mut deadlines);
        let host = host::Script {
            host_tools: 1000,
            relay: Span::millis(1, 1),
            relay_busy: 1000,
            time: Span::millis(30, 300),
            ..calm.host
        };
        tally(&settled(&Settings { host, partner, ..calm }), &mut cancels, &mut deadlines);
    }
    tally(&settled(&wind_down_pressure(902, false)), &mut cancels, &mut deadlines);
    deadlines
}

#[test]
fn deadlines_find_runs_in_every_state_they_run_in_over_many_seeds() {
    let deadlines = deadlines_find_runs_in_every_state_they_run_in_over_many_seeds_cells();
    for cell in [
        "preparing",
        "opening",
        "working",
        "waiting",
        "relay sending",
        "relay withdrawing",
        "relay backoff",
        "landing",
        "wind-down",
    ] {
        assert!(deadlines.get(cell).is_some_and(|count| *count > 0), "no deadline found a run {cell}: {deadlines:?}");
    }
}

fn answer_kind(answer: &Answer) -> &'static str {
    match answer {
        Answer::Parked { .. } => "parked",
        Answer::Accepted { .. } => "accepted",
        Answer::Refused(Refusal::Busy) => "busy",
        Answer::Refused(Refusal::Invalid(Invalid::Conversation)) => "conversation invalid",
        Answer::Refused(Refusal::Invalid(_)) => "invalid",
        Answer::Failed { failure, .. } => match failure {
            Failure::Model(
                Fault::Completion { .. }
                | Fault::Provider
                | Fault::Unauthorized { .. }
                | Fault::Program { .. }
                | Fault::Limited { .. }
                | Fault::Refused { .. },
            ) => "fault",
            Failure::Model(Fault::Truncated | Fault::Declined | Fault::Malformed) => "stopped",
            Failure::Budget { exhausted: Exhausted::Turns, .. } => "turns",
            Failure::Budget { exhausted: Exhausted::Time, .. } => "time",
            Failure::Budget { exhausted: Exhausted::Spend, .. } => "spend",
            Failure::Budget { exhausted: Exhausted::Overflow(_), .. } => {
                panic!("bounded random partner usage and prices remain representable")
            }
            Failure::Budget { exhausted: Exhausted::Tokens(_), .. } => {
                panic!("source random partner has no receiving refusal")
            }
            Failure::Policy(Policy::Unfinished { .. }) => "unfinished",
            Failure::Cancelled => "cancelled",
            Failure::Stale => "stale",
            Failure::Transcript(_) => panic!("source random partner does not restore history"),
        },
    }
}
