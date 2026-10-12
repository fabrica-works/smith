use smith_agent_process_world::events;
use smith_events::{Event, Mode};
use smith_local_process_world::world::{Files, Placement, World};

#[test]
fn an_inline_chat_writes_a_complete_stream_through_its_answer() {
    let files = Files::new();
    let mut world = World::traced(11, &files, Placement::InProcess, b"First answer");
    world.settle();
    let records = world.records();
    events::check(&records);
    let Event::SessionStarted(header) = &records[0].event else { panic!("header first") };
    assert_eq!(header.mode, Mode::Chat);
    assert_eq!(header.chat.as_deref(), Some(b"main".as_slice()));
}

#[test]
fn a_slowed_destination_loses_no_record_under_complete() {
    let files = Files::new();
    let mut world = World::traced(111, &files, Placement::InProcess, b"First answer");
    world.settle();
    events::check(&world.records());
}

#[test]
fn one_response_completed_per_provider_response_of_an_inline_chat() {
    let files = Files::new();
    let mut world = World::traced(13, &files, Placement::InProcess, b"First answer");
    world.settle();
    let responses = world.records().iter().filter(|record| matches!(record.event, Event::ResponseCompleted(_))).count();
    assert_eq!(responses, world.queries().len());
    assert!(responses > 0);
}
