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

#[test]
fn actual_facts_tell_child_shares_and_budget_without_deciding_work() {
    use smith_domain_run::facts::{FactKind, Kind};
    use smith_run_world::facts_referee::{Seen, judge};

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
    let children: Vec<_> = world
        .facts_seen()
        .iter()
        .filter_map(|fact| {
            let FactKind::Opened { kind: Kind::Child { asked, granted, .. }, .. } = fact.kind else { return None };
            Some((fact.at, asked, granted))
        })
        .collect();
    assert_eq!(children.len(), world.share_seen().len());
    assert!(!children.is_empty());
    for ((at, asked, granted), (opened_at, seen)) in children.iter().zip(world.share_seen()) {
        assert_eq!(at, opened_at);
        assert_eq!(*asked, seen.asked);
        assert_eq!(
            smith_domain_run::Budget { turns: granted.turns, time: granted.time, spend: granted.spend },
            seen.effective
        );
    }
    let budgets: Vec<_> = world
        .facts_seen()
        .iter()
        .filter_map(|fact| {
            let FactKind::Budget { run, standing, .. } = fact.kind else { return None };
            Some((fact.at, run, standing))
        })
        .collect();
    assert!(!budgets.is_empty());
    assert!(budgets.len() >= world.charges_seen().len(), "stage changes also tell standing");
    let observed: Vec<_> = world
        .charges_seen()
        .iter()
        .map(|(charged_at, charged_run, charged)| {
            let (_, _, standing) = budgets
                .iter()
                .find(|(at, run, standing)| {
                    at == charged_at
                        && run == charged_run
                        && standing.turns == charged.spent.turns
                        && standing.spend == charged.spent.units
                })
                .expect("every actual charge has a standing fact");
            (*charged_at, Seen { charged: *charged, standing: *standing })
        })
        .collect();
    assert_eq!(judge(&observed), Verdict::Passed);
    for part in 0..4 {
        let mut bad = observed.clone();
        match part {
            0 => bad[0].1.standing.turns += 1,
            1 => bad[0].1.standing.spend += 1,
            2 => bad[0].1.standing.time = Duration::from_nanos(bad[0].1.standing.time.as_nanos() + 1),
            3 => bad[0].1.standing.reserve_spend += 1,
            _ => unreachable!(),
        }
        assert!(matches!(judge(&bad), Verdict::Failed(_)), "budget part {part}");
    }
    check_child_returns(&world);
    let mut tight = World::new(Settings {
        run: smith_domain_run::Limits { facts: smith_domain_run::max_facts(&settings.run), ..settings.run },
        ..settings
    });
    tight.run(1_000_000);
    // The iterator batches input before submitting output. Drain pressure may
    // change their same-clock interleaving, but never either boundary's sequence.
    for face in ["run <-", "run ->", "run alarm"] {
        let tight_face: Vec<_> = tight.trace().iter().filter(|line| line.contains(face)).collect();
        let roomy_face: Vec<_> = world.trace().iter().filter(|line| line.contains(face)).collect();
        if tight_face != roomy_face {
            let index = tight_face
                .iter()
                .zip(&roomy_face)
                .position(|(a, b)| a != b)
                .unwrap_or(tight_face.len().min(roomy_face.len()));
            let a: String = tight_face.get(index).map_or("missing", |s| s.as_str()).chars().take(240).collect();
            let b: String = roomy_face.get(index).map_or("missing", |s| s.as_str()).chars().take(240).collect();
            panic!(
                "fact drains preserve {face}: difference at {index} of {}/{}: tight={a}, roomy={b}",
                tight_face.len(),
                roomy_face.len()
            );
        }
    }
    assert_eq!(tight.facts_seen(), world.facts_seen(), "every larger fact is retained in order");
}

fn check_child_returns(world: &World) {
    use smith_domain_run::facts::{FactKind, Kind};
    let mut returns = 0_u32;
    for fact in world.facts_seen() {
        let FactKind::Returned { call, result, .. } = fact.kind else { continue };
        let (smith_domain_run::facts::Return::Answered { used, .. }
        | smith_domain_run::facts::Return::Unanswered { used }) = result
        else {
            continue;
        };
        let child = world
            .facts_seen()
            .iter()
            .find_map(|opened| {
                let FactKind::Opened { conversation, kind: Kind::Child { call: opened_call, .. }, .. } = opened.kind
                else {
                    return None;
                };
                (opened_call == call).then_some(conversation)
            })
            .expect("the child's enclosing call opened it");
        let (ended_at, expected) = world.child_usage_seen(child).expect("actual child terminal callback");
        assert_eq!(fact.at, ended_at);
        assert_eq!(used, expected, "child usage follows its own callbacks and injected elapsed time");
        returns += 1;
    }
    assert!(returns > 0, "the story actually settles a child result");
}
