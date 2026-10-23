//! Seeded message and cancellation crossings on both actual inline slots.
use skein_lib::{Duration, Rng, Time, Token};
use smith_agent_world::Job;
use smith_host_domain::parent;
use smith_host_world::inline::{self, World};

fn story(seed: u64) -> (Vec<String>, Vec<Option<smith_host_domain::RunResult>>) {
    let mut rng = Rng::new(seed);
    let mut bounds = inline::limits();
    bounds.cancel_grace = Duration::from_millis(rng.between(0, 8));
    let mut world = World::new(seed, bounds);
    world.provider_latency(Duration::from_millis(rng.between(1, 40)));
    for client in 1..=2 {
        world.event(parent::Event::Spawn { client: Token::new(client), start: inline::start(seed, Job::HostTools) });
        let at = Time::ZERO.saturating_add(Duration::from_millis(rng.between(0, 8)));
        world.schedule.send(
            at,
            parent::Event::Message {
                finish_requested: false,
                agent: world.handle(Token::new(client)),
                name: Token::new(90 + client),
                label: b"person".as_slice().into(),
                text: b"Continue".as_slice().into(),
            },
        );
        if rng.between(0, 1) == 1 {
            world.schedule.send(at, parent::Event::Stop { agent: world.handle(Token::new(client)) });
            if rng.between(0, 1) == 1 {
                world.schedule.send(
                    at.saturating_add(Duration::from_millis(rng.between(0, 8))),
                    parent::Event::Drop { agent: world.handle(Token::new(client)) },
                );
            }
        }
    }
    world.run();
    world.settled();
    let outcomes = world.seen.into_values().map(|seen| seen.answer.map(|answer| answer.result)).collect();
    (world.trace.lines().to_vec(), outcomes)
}

#[test]
fn inline_message_cancel_crossings_replay_for_every_seed() {
    for seed in 320..384 {
        skein_world::domain::assert_replays(seed, seed + 1000, story);
    }
}

fn workspace_story(seed: u64) -> (Vec<String>, Vec<Option<smith_host_domain::RunResult>>) {
    let mut rng = Rng::new(seed);
    let mut bounds = inline::limits();
    bounds.cancel_grace = Duration::from_millis(rng.between(0, 8));
    let mut world = World::new(seed, bounds);
    world.provider_latency(Duration::from_millis(rng.between(1, 40)));
    world.auto_machine = false;
    world.event(parent::Event::Spawn { client: Token::new(1), start: inline::workspace_start(seed, Job::Reporting) });
    let crossing = Time::ZERO.saturating_add(Duration::from_millis(rng.between(0, 25)));
    let event = if rng.between(0, 1) == 1 {
        parent::Event::Stop { agent: world.handle(Token::new(1)) }
    } else {
        parent::Event::Drop { agent: world.handle(Token::new(1)) }
    };
    world.schedule.send(crossing, event);
    world.at(Time::ZERO.saturating_add(Duration::from_millis(rng.between(0, 25))));
    world.auto_machine = true;
    world.machine_replies();
    world.run();
    world.settled();
    let outcomes = world.seen.into_values().map(|seen| seen.answer.map(|answer| answer.result)).collect();
    (world.trace.lines().to_vec(), outcomes)
}

#[test]
fn inline_workspace_cancel_crossings_replay_for_every_seed() {
    for seed in 400..416 {
        skein_world::domain::assert_replays(seed, seed + 1000, workspace_story);
    }
}
