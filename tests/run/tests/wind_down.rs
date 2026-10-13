//! Actual reserve transitions, child terminals and final requests at run boundaries.
//! Contract: domain/run.md, sections 9.1 and 13.

use skein_lib::Duration;
use skein_world::domain::Verdict;
use smith_domain_run::{Answer, Exhausted, Failure, charter::Reserve};
use smith_run_world::{
    Settings, Span, World,
    wind_down_referee::{Seen, judge},
};

fn settings(seed: u64) -> Settings {
    let mut settings = Settings::calm(seed);
    settings.host.jobs = 1;
    settings.host.writable = 0;
    settings.host.reports = 1000;
    settings.host.verdicts = 0;
    settings.host.host_tools = 0;
    settings.host.turns_min = 5;
    settings.host.turns_max = 5;
    settings.host.reserve = Reserve { turns: 3, time: Duration::from_secs(5) };
    settings.host.time = Span::millis(10_000, 10_000);
    settings.partner.turn = Span::millis(100, 100);
    settings.partner.yields = 0;
    settings.partner.changes = 0;
    settings.partner.reports = 1000;
    settings.partner.parallel = 2;
    settings
}

#[test]
fn a_sub_agent_that_spends_the_ordinary_pool_is_closed_then_main_finishes_in_its_final_turns() {
    let mut accepted = 0;
    // Retain the prior seed; the child now answers on its final turn, which
    // changes the random draw sequence seen by main's subsequent completions.
    for seed in [3, 4, 5, 6, 7, 8] {
        let mut settings = settings(seed);
        settings.host.agents = 1000;
        settings.partner.asks = 800;
        settings.partner.finishes = 200;
        let mut world = World::new(settings);
        world.run(1_000_000);
        if world.stats().children > 0
            && world.wind_down_seen().iter().any(|(_, _, seen)| matches!(seen, Seen::Final { .. }))
        {
            accepted += world.answers().filter(|answer| matches!(answer, Some(Answer::Accepted { .. }))).count();
        }
    }
    assert!(accepted > 0, "a final result is accepted after an actual child spent the ordinary pool");
}

#[test]
fn the_ordinary_time_running_out_during_a_sub_agent_winds_the_run_down() {
    let mut settings = settings(34);
    settings.host.agents = 1000;
    settings.host.turns_min = 20;
    settings.host.turns_max = 20;
    settings.host.time = Span::millis(3000, 3000);
    settings.host.reserve = Reserve { turns: 2, time: Duration::from_secs(1) };
    settings.partner.turn = Span::millis(1800, 1800);
    settings.partner.asks = 1000;
    let mut world = World::new(settings);
    world.run(1_000_000);
    assert!(world.stats().children > 0);
    assert!(world.facts_seen().iter().any(|fact| matches!(fact.kind, smith_domain_run::facts::FactKind::Budget { standing, .. } if standing.stage == smith_domain_run::Stage::WindDown)));
    assert!(world.answers().any(|answer| matches!(
        answer,
        Some(Answer::Failed { failure: Failure::Budget { exhausted: Exhausted::Time, .. }, .. })
    )));
}

#[test]
fn the_ordinary_time_running_out_while_main_waits_parks_the_run() {
    let mut settings = settings(34);
    settings.host.turns_min = 20;
    settings.host.turns_max = 20;
    settings.host.time = Span::millis(40_000, 40_000);
    settings.run.waiting = Duration::from_secs(600);
    settings.partner.waits = 1000;
    let mut world = World::new(settings);
    world.waiting_time(Duration::from_secs(600));
    world.run(1_000_000);
    assert!(world.answers().any(|answer| matches!(answer, Some(Answer::Parked { .. }))));
    assert!(
        !world.wind_down_seen().iter().any(|(_, _, seen)| matches!(seen, Seen::Final { .. })),
        "a parked wait makes no final provider request"
    );
}

#[test]
fn final_turns_ignored_fail_once_for_the_budget() {
    let mut settings = settings(34);
    settings.host.agents = 0;
    settings.partner.asks = 0;
    settings.partner.finishes = 0;
    let mut world = World::new(settings);
    world.run(1_000_000);
    assert_eq!(world.answers().count(), 1);
    assert!(world.answers().any(|answer| matches!(
        answer,
        Some(Answer::Failed { failure: Failure::Budget { exhausted: Exhausted::Turns, .. }, .. })
    )));
    assert_eq!(world.wind_down_seen().iter().filter(|(_, _, seen)| matches!(seen, Seen::Final { .. })).count(), 3);
    assert!(judge(world.wind_down_seen()).iter().all(|verdict| *verdict == Verdict::Passed));
    for violation in 0..3 {
        let mut bad = world.wind_down_seen().to_vec();
        let first =
            bad.iter().position(|(_, _, seen)| matches!(seen, Seen::Final { .. })).expect("actual final request");
        match violation {
            0 => bad[first].2 = Seen::Final { main: true, live_children: 1 },
            1 => {
                let extra = bad[first];
                bad.insert(first, extra);
            }
            2 => bad[first].2 = Seen::Answer { turns_failure: true, time_failure: false, time_expired: false },
            _ => unreachable!(),
        }
        assert!(
            judge(&bad).iter().any(|verdict| matches!(verdict, Verdict::Failed(_))),
            "positive-first final rule {violation}"
        );
    }
}

#[test]
fn a_run_whose_spend_runs_out_first_winds_down_and_is_accepted() {
    let mut witnessed = 0;
    for seed in 0..16 {
        let mut settings = settings(seed);
        settings.host.turns_min = 100;
        settings.host.turns_max = 100;
        settings.partner.prices =
            Some(smith_domain_run::Prices { input: 1, cached: 1, cache_write: Some(1), output: 1, unit: 1 });
        settings.host.tokens_min = 2 * 69_632 + 9_001;
        settings.host.tokens_max = settings.host.tokens_min;
        settings.host.reserve.turns = 2;
        settings.host.agents = 0;
        settings.partner.finishes = 200;
        let mut world = World::new(settings);
        world.run(1_000_000);
        assert!(world.charges_seen().iter().all(|(_, _, charge)| charge.spent.units <= charge.budget.spend));
        if world.facts_seen().iter().any(|fact| {
            matches!(fact.kind,
            smith_domain_run::facts::FactKind::Budget { standing, .. }
                if standing.stage == smith_domain_run::Stage::WindDown
                    && standing.turns < standing.turns_limit - standing.reserve.turns
                    && standing.time.as_nanos() < standing.time_limit.as_nanos() - standing.reserve.time.as_nanos())
        }) && world.answers().any(|answer| matches!(answer, Some(Answer::Accepted { .. })))
        {
            witnessed += 1;
        }
    }
    assert!(witnessed > 0, "actual spend starts final work before turns or time, then accepts");
}
