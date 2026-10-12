use smith_agent_process_world::events;
use smith_events::{Event, Mode};
use smith_local_process_world::world::{Files, Placement, World};

#[test]
fn the_products_header_carries_its_local_limits() {
    for placement in [Placement::InProcess, Placement::Spawned] {
        let files = Files::new();
        let mut world = World::traced(23, &files, placement, b"First answer");
        world.settle();
        let records = world.records();
        let Event::SessionStarted(header) = &records[0].event else { panic!("header first") };
        let local = header.limits.iter().find(|limit| limit.name.as_ref() == b"local.line_bytes");
        let memory = header.limits.iter().find(|limit| limit.name.as_ref() == b"declared.memory");
        assert_eq!(local.map(|limit| limit.value), Some(smith_events::LimitValue::Integer(248)));
        assert_eq!(memory.map(|limit| limit.value), Some(smith_events::LimitValue::Integer(2_147_483_648)));
        let inline = header.limits.iter().any(|limit| limit.name.as_ref() == b"inline.window.bytes");
        let host = header.limits.iter().any(|limit| limit.name.as_ref() == b"host.process.io.sockets");
        assert_eq!(inline, matches!(placement, Placement::InProcess));
        assert_eq!(host, matches!(placement, Placement::Spawned));
    }
}

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

#[test]
fn a_spawned_chat_writes_its_answer_and_end_to_the_product_stream() {
    let files = Files::new();
    let mut world = World::traced(17, &files, Placement::Spawned, b"First answer");
    world.settle();
    let records = world.records();
    assert!(matches!(records.first().map(|r| &r.event), Some(Event::SessionStarted(_))));
    assert!(matches!(records.last().map(|r| &r.event), Some(Event::SessionEnded(_))));
    assert_eq!(records.iter().filter(|r| matches!(r.event, Event::RunCompleted(_))).count(), 1);
    assert_eq!(records.iter().filter(|r| matches!(r.event, Event::SessionEnded(_))).count(), 1);
}

#[test]
fn the_spawned_agents_records_stay_in_its_own_trace() {
    let files = Files::new();
    let mut world = World::traced(19, &files, Placement::Spawned, b"First answer");
    world.settle();
    let product = world.records();
    let agent = world.agent_records();
    events::check(&agent);
    assert!(agent.iter().any(|r| matches!(r.event, Event::RunStarted(_))));
    assert!(agent.iter().any(|r| matches!(r.event, Event::ResponseCompleted(_))));
    assert!(!product.iter().any(|r| matches!(r.event, Event::RunStarted(_) | Event::ResponseCompleted(_))));
}
