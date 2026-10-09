//! Positive-first mutations of actual ordinary-pool child openings.
//! Contract: domain/run.md, sections 5.3, 9.1 and 13.

use skein_lib::Duration;
use skein_world::domain::Verdict;
use smith_run_world::{Settings, World, share_referee::judge};

#[test]
fn a_child_share_crossing_any_ordinary_part_is_rejected() {
    let mut settings = Settings::calm(34);
    settings.host.agents = 1000;
    settings.host.reserve = smith_domain_run::charter::Reserve { turns: 2, time: Duration::from_secs(600) };
    settings.partner.asks = 300;
    settings.partner.finishes = 50;
    settings.partner.parallel = 3;
    let mut world = World::new(settings);
    world.run(1_000_000);
    let good = world.share_seen();
    assert!(!good.is_empty(), "the real run actually opens children");
    assert_eq!(judge(good), Verdict::Passed);
    for part in 0..3 {
        let mut bad = good.to_vec();
        let opening = &mut bad[0].1;
        match part {
            0 => opening.effective.turns = opening.ordinary.turns + 1,
            1 => opening.effective.spend = opening.ordinary.spend + 1,
            2 => opening.effective.time = Duration::from_nanos(opening.ordinary.time.as_nanos() + 1),
            _ => unreachable!(),
        }
        assert!(matches!(judge(&bad), Verdict::Failed(_)), "ordinary part {part}");
    }
}

#[test]
fn a_sub_agent_asking_more_than_is_left_above_the_reserve_is_clamped() {
    let mut settings = Settings::calm(34);
    settings.host.agents = 1000;
    settings.host.writable = 0;
    settings.host.reports = 1000;
    settings.host.turns_min = 5;
    settings.host.turns_max = 5;
    settings.host.time = skein_world::domain::Span::millis(10_000, 10_000);
    settings.host.reserve = smith_domain_run::charter::Reserve { turns: 3, time: Duration::from_secs(5) };
    settings.partner.asks = 500;
    settings.partner.finishes = 50;
    settings.partner.shares = 1000;
    settings.partner.parallel = 3;
    let mut world = World::new(settings);
    world.run(1_000_000);
    let openings = world.share_seen();
    assert!(!openings.is_empty(), "real child openings witness the requested share");
    assert_eq!(judge(openings), Verdict::Passed);
    assert!(openings.iter().any(|(_, seen)| seen.asked.turns.is_some_and(|turns| turns > seen.ordinary.turns)));
    assert!(openings.iter().any(|(_, seen)| seen.asked.time.is_some_and(|time| time > seen.ordinary.time)));
    let mut bad = openings.to_vec();
    let short = &mut bad[0].1;
    short.effective.time = Duration::ZERO;
    assert!(matches!(judge(&bad), Verdict::Failed(_)), "the oracle also rejects a short grant that fits the pool");
}
