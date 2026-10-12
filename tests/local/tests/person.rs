use skein_lib::Token;
use smith_local_domain::{ExitStatus, Notice};
use smith_local_world::{
    World,
    person::{Act, Cue, Kind, Person, Shown, Trigger},
};

#[test]
fn a_person_typing_on_the_first_turn_has_both_lines_read() {
    let mut world = World::new(23);
    world.person(Person::new(vec![
        Cue { on: Trigger::Steps(0), act: Act::Type(b"What can you do?".to_vec()) },
        Cue { on: Trigger::Turn, act: Act::Type(b"And then?".to_vec()) },
        Cue { on: Trigger::Answer, act: Act::Close },
    ]));
    assert!(world.drive_to_exit(2_000), "the chat parks and exits");
    assert_eq!(world.exit(), Some(ExitStatus::Accepted));
    assert_eq!(world.into_store().read(), Some(Token::new(2)), "the second line was read");
}

#[test]
fn a_turn_releases_only_the_next_turn_cue() {
    let mut person = Person::new(vec![Cue { on: Trigger::Turn, act: Act::Type(b"hello".to_vec()) }]);
    assert_eq!(person.saw(Shown::Answer), None);
    assert_eq!(person.saw(Shown::Turn), Some(Act::Type(b"hello".to_vec())));
    assert_eq!(person.saw(Shown::Turn), None);
}

#[test]
fn an_answer_releases_the_persons_interrupt() {
    let mut person = Person::new(vec![Cue { on: Trigger::Answer, act: Act::Interrupt }]);
    assert_eq!(person.saw(Shown::Turn), None);
    assert_eq!(person.saw(Shown::Answer), Some(Act::Interrupt));
}

#[test]
fn a_notice_releases_the_person_by_kind() {
    let mut person = Person::new(vec![Cue { on: Trigger::Notice(Kind::Waiting), act: Act::Close }]);
    assert_eq!(person.saw(Shown::Notice(Notice::Checking(smith_local_domain::Checks::Started))), None);
    assert_eq!(person.saw(Shown::Notice(Notice::Waiting)), Some(Act::Close));
}

#[test]
fn a_step_cue_fires_once_after_its_countdown() {
    let mut person = Person::new(vec![Cue { on: Trigger::Steps(2), act: Act::Close }]);
    assert_eq!(person.step(), None);
    assert_eq!(person.step(), None);
    assert_eq!(person.step(), Some(Act::Close));
    assert_eq!(person.step(), None);
}

#[test]
fn closed_input_without_a_line_exits_the_chat() {
    let mut world = World::new(29);
    world.person(Person::new(vec![Cue { on: Trigger::Steps(0), act: Act::Close }]));
    assert!(world.drive_to_exit(200));
    assert_eq!(world.exit(), Some(ExitStatus::Accepted));
    assert_eq!(world.completions(), 0);
}
