use skein_lib::Token;
use smith_local_domain::{ExitStatus, Notice, Reading, Undelivered};
use smith_local_world::{
    World,
    person::{Act, Cue, Person, Trigger},
};

#[test]
fn lines_then_the_end_of_input_are_each_read_before_the_host_exits() {
    let mut world = World::new(19);
    world.line(b"What can you do?");
    world.line(b"And in this directory?");
    world.close();
    assert!(world.drive_to_exit(2_000), "the chat parks and exits");
    assert_eq!(world.exit(), Some(ExitStatus::Accepted));
    assert_eq!(world.into_store().read(), Some(Token::new(2)), "both lines were read");
}

fn follow_up(world: &mut World) {
    world.line(b"First question");
    world.person(Person::new(vec![
        Cue { on: Trigger::Turn, act: Act::Type(b"And another question".to_vec()) },
        Cue { on: Trigger::Answer, act: Act::Close },
    ]));
}

#[test]
fn a_follow_up_sent_as_the_run_finishes_is_read_once() {
    let mut world = World::finishing(21);
    follow_up(&mut world);
    assert!(world.drive_to_exit(2_000), "the unread follow-up opens another activation");
    assert_eq!(world.activation(), 2);
    assert_eq!(world.into_store().read(), Some(Token::new(2)));
}

#[test]
fn a_line_at_the_moment_of_parking_opens_the_next_activation() {
    let mut world = World::new(23);
    world.line(b"First question");
    world.person(Person::new(vec![
        Cue { on: Trigger::Answer, act: Act::Type(b"A question after parking".to_vec()) },
        Cue { on: Trigger::Steps(1), act: Act::Close },
    ]));
    assert!(world.drive_to_exit(2_000));
    assert_eq!(world.activation(), 2);
    assert_eq!(world.into_store().read(), Some(Token::new(2)));
}

#[test]
fn an_interrupt_with_a_line_unread_shows_it_not_delivered_and_starts_nothing() {
    let mut world = World::finishing(25);
    world.line(b"First question");
    world.line(b"Unread question");
    world.person(Person::new(vec![
        Cue { on: Trigger::Steps(1), act: Act::Interrupt },
        Cue { on: Trigger::Steps(1), act: Act::Close },
    ]));
    assert!(world.drive_to_exit(2_000));
    assert!(
        world.notices().iter().any(|notice| matches!(notice, Notice::NotDelivered { why: Undelivered::Dropped, .. }))
    );
    assert_eq!(world.activation(), 1);
}

#[test]
fn a_brief_with_an_early_finish_carries_the_line_to_the_next_activation() {
    let mut world = World::finishing(27);
    follow_up(&mut world);
    assert!(world.drive_to_exit(2_000));
    assert!(world.prompt_texts().iter().any(|text| text.windows(20).any(|part| part == b"And another question")));
    assert_eq!(world.into_store().read(), Some(Token::new(2)));
}

#[test]
fn a_slow_store_racing_the_answer_loses_no_line() {
    let mut world = World::finishing(29);
    world.slow_store();
    follow_up(&mut world);
    assert!(world.drive_to_exit(2_000));
    assert_eq!(world.activation(), 2);
    assert_eq!(world.into_store().read(), Some(Token::new(2)));
}

#[test]
fn the_held_input_reopens_when_a_fence_reads_a_line() {
    let mut world = World::new(31);
    let inbox = smith_agent_world::LIMITS.run.messages;
    for _ in 0..inbox {
        world.line(b"One queued question");
    }
    world.close();
    assert!(world.drive_to_exit(2_000));
    assert_eq!(world.readings().first(), Some(&Reading::Held));
    assert_eq!(world.readings().last(), Some(&Reading::Open));
}

#[test]
fn no_waiting_notice_shows_while_a_sent_line_is_unread() {
    let mut world = World::new(33);
    world.line(b"First question");
    world.person(Person::new(vec![Cue { on: Trigger::Turn, act: Act::Type(b"Second question".to_vec()) }]));
    assert!(world.drive_to_park(2_000), "the crossing wait parks after reading both lines");
    assert!(!world.notices().contains(&Notice::Waiting));
    world.close();
    assert!(world.drive_to_exit(2_000));
    assert_eq!(world.exit(), Some(ExitStatus::Accepted));
    assert_eq!(world.into_store().read(), Some(Token::new(2)));
}

#[test]
fn no_waiting_notice_shows_before_the_first_turn() {
    let mut world = World::new(35);
    assert!(!world.drive(20));
    assert!(!world.notices().contains(&Notice::Waiting));
    world.close();
    assert!(world.drive_to_exit(2_000));
}
