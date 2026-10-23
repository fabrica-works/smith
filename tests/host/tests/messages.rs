//! Actual message ingress, read fences and one terminal unread report on both faces.
use skein_lib::{Duration, Token};
use smith_agent_world::Job;
use smith_host_domain::{Answer, End, Fault, Input, Message, MessageRefusal, RunResult, Turn, Up, parent, process};
use smith_host_world::{World, inline, limits, start, turn_value};

fn message(world: &mut World, name: u64) {
    world.event(Input::Parent(parent::Event::Message {
        finish_requested: false,
        agent: world.agent(),
        name: Token::new(name),
        label: b"person".as_slice().into(),
        text: b"hello".as_slice().into(),
    }));
}

fn answer(world: &mut World, turns: u32) {
    world.up(Up::Answer { answer: Answer { read: world.seen.read, turns, spent: 0, result: RunResult::Parked } });
}

#[test]
fn an_answer_reports_sent_and_queued_names_and_later_ingress_has_its_own_refusal() {
    let mut world = World::new(501, limits());
    world.live();
    message(&mut world, 1);
    message(&mut world, 2);
    answer(&mut world, 0);
    assert_eq!(world.seen.unread, Some(vec![Token::new(1), Token::new(2)]));
    assert!(world.seen.refused_messages.is_empty());
    message(&mut world, 3);
    assert_eq!(world.seen.refused_messages, [(Token::new(3), MessageRefusal::Ending)]);
    assert_eq!(world.seen.fault, None);
    world.cleanup(Some(smith_host_domain::Exit::Code(0)));
    world.settled();
}

#[test]
fn an_actual_turn_fence_covers_a_prefix_and_the_report_includes_a_queued_suffix() {
    let mut world = World::new(502, limits());
    world.live();
    message(&mut world, 1);
    world.sent();
    message(&mut world, 2);
    world.sent();
    world.up(Up::Turn {
        turn: Turn { opens_window: true, number: 1, spent: 0, read: Some(Token::new(1)), body: turn_value(0) },
    });
    world.event(Input::Parent(parent::Event::Acknowledge { agent: world.agent(), turn: 1 }));
    message(&mut world, 3);
    answer(&mut world, 1);
    assert_eq!(world.seen.read_messages, [Token::new(1)]);
    assert_eq!(world.seen.unread, Some(vec![Token::new(2), Token::new(3)]));
    world.cleanup(Some(smith_host_domain::Exit::Code(0)));
    world.settled();
}

#[test]
fn an_actual_waiting_fence_covers_only_the_names_before_it() {
    let mut world = World::new(503, limits());
    world.live();
    message(&mut world, 1);
    world.sent();
    world.up(Up::Waiting { read: Some(Token::new(1)) });
    message(&mut world, 2);
    answer(&mut world, 0);
    assert_eq!(world.seen.read_messages, [Token::new(1)]);
    assert_eq!(world.seen.unread, Some(vec![Token::new(2)]));
    world.cleanup(Some(smith_host_domain::Exit::Code(0)));
    world.settled();
}

#[test]
fn malformed_channel_reports_all_accepted_names_before_fault_and_retains_lower_terminals() {
    let mut world = World::new(504, limits());
    world.live();
    message(&mut world, 1);
    message(&mut world, 2);
    world.event(Input::Process(process::Event::Malformed { owner: world.owner() }));
    assert_eq!(world.seen.unread, Some(vec![Token::new(1), Token::new(2)]));
    assert_eq!(world.seen.fault, Some(Fault::Rules));
    world.cleanup(Some(smith_host_domain::Exit::Code(0)));
    world.settled();
}

#[test]
fn failed_process_admission_reports_the_initial_messages_before_gone() {
    let mut world = World::new(505, limits());
    let mut source = start();
    source.messages = Box::from([Message {
        finish_requested: false,
        name: Token::new(1),
        label: b"person".as_slice().into(),
        text: b"hi".as_slice().into(),
    }]);
    world.spawn(source);
    world.event(Input::Process(process::Event::Unspawned { owner: world.owner(), detail: Box::default() }));
    assert_eq!(world.seen.unread, Some(vec![Token::new(1)]));
    assert_eq!(world.seen.gone, Some(End::Unspawned));
    world.settled();
}

#[test]
fn inline_stop_and_drop_report_the_queued_message_and_settle_the_actual_provider() {
    for drop_now in [false, true] {
        let mut world = inline::World::new(506, inline::limits());
        world.provider_latency(Duration::from_secs(20));
        let client = Token::new(1);
        world.event(parent::Event::Spawn { client, start: inline::start(506, Job::Waiting) });
        let agent = world.handle(client);
        world.event(parent::Event::Message {
            finish_requested: false,
            agent,
            name: Token::new(99),
            label: b"person".as_slice().into(),
            text: b"hi".as_slice().into(),
        });
        world.event(if drop_now { parent::Event::Drop { agent } } else { parent::Event::Stop { agent } });
        assert!(world.seen[&client].gone.is_none(), "actual provider terminal is still owed");
        if drop_now {
            assert_eq!(world.seen[&client].unread, Some(vec![Token::new(99)]));
            assert_eq!(world.seen[&client].fault, Some(Fault::Exited));
        }
        world.event(parent::Event::Message {
            finish_requested: false,
            agent,
            name: Token::new(100),
            label: b"person".as_slice().into(),
            text: b"late".as_slice().into(),
        });
        world.run();
        assert_eq!(world.seen[&client].unread, Some(vec![Token::new(99)]));
        assert_eq!(world.seen[&client].refused_messages, [(Token::new(100), MessageRefusal::Ending)]);
        assert!(world.seen[&client].gone.is_some());
    }
}
