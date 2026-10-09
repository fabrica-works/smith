//! Positive-first mutations of actual child and landing observations.
//! Contract: domain/run.md, sections 5.3, 8.1 and 13.

use skein_world::domain::Verdict;
use smith_run_world::{
    Settings, World,
    lease_referee::{Seen, judge},
};

#[test]
fn the_lease_referee_rejects_overlapping_holders_and_unowned_checks_or_delivery() {
    let mut settings = Settings::calm(734);
    settings.host.jobs = 2;
    settings.host.agents = 1000;
    settings.host.changes = 1000;
    settings.host.turns_min = 30;
    settings.host.turns_max = 30;
    settings.partner.asks = 300;
    settings.partner.finishes = 150;
    settings.partner.yields = 100;
    settings.partner.changes = 1000;
    settings.partner.parallel = 3;
    settings.checkout.checks = 1000;
    let mut world = World::new(settings);
    world.run(1_000_000);
    let good = world.lease_seen();
    assert!(judge(good).iter().all(|verdict| *verdict == Verdict::Passed));
    let child = good
        .iter()
        .position(|(_, _, seen)| matches!(seen, Seen::ChildOpened { writing: true, .. }))
        .expect("an actual writing child");
    let workspace = good
        .iter()
        .position(|(_, _, seen)| matches!(seen, Seen::WorkspaceOpened { .. }))
        .expect("an actual workspace operation");
    let check = good.iter().position(|(_, _, seen)| matches!(seen, Seen::Check { .. })).expect("an actual check");
    let delivery =
        good.iter().position(|(_, _, seen)| matches!(seen, Seen::Delivery { .. })).expect("an actual delivery");
    for change in 0..5 {
        let mut bad = good.to_vec();
        match change {
            0 => {
                bad.insert(child + 1, bad[child]);
            }
            1 => {
                bad.insert(
                    child + 1,
                    (bad[child].0, bad[child].1, Seen::WorkspaceOpened { owner: skein_lib::Token::new(u64::MAX) }),
                );
            }
            2 => {
                bad.remove(workspace);
            }
            3 => {
                bad[check].2 = Seen::Check { owner: skein_lib::Token::new(u64::MAX) };
            }
            4 => {
                bad[delivery].2 = Seen::Delivery { owner: skein_lib::Token::new(u64::MAX) };
            }
            _ => unreachable!(),
        }
        assert!(judge(&bad).iter().any(|verdict| matches!(verdict, Verdict::Failed(_))), "mutation {change}");
    }
}
