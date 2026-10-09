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
