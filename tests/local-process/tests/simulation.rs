use skein_io::kernel;
use smith_events::{AnswerClass, Event};

#[test]
fn an_interrupt_mid_turn_and_a_second_drop_the_run() {
    for placement in [Placement::Spawned, Placement::InProcess] {
        let files = Files::new();
        let mut world = World::dropped(12, &files, placement);
        world.settle();
        assert_eq!(world.exit(), Some(kernel::Exit::Code(130)));
        let errors = String::from_utf8_lossy(world.errors());
        assert!(errors.ends_with("smith: interrupted: the run was dropped before it answered\n"));
    }
}

#[test]
fn a_dropped_inline_run_completes_as_cancelled_and_dropped() {
    let files = Files::new();
    let mut world = World::dropped(12, &files, Placement::InProcess);
    world.scenario.launch.traced = true;
    world.settle();
    let records = world.records();
    let completed = records
        .iter()
        .filter_map(|record| match &record.event {
            Event::RunCompleted(completed) => Some(completed),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].status, smith_events::Status::Failed);
    let failure = completed[0].failure.as_ref().expect("dropped run");
    assert_eq!(failure.class, AnswerClass::Cancelled);
    assert_eq!(failure.dropped, Some(true));
    assert!(!completed[0].wind_down);
    assert!(completed[0].result.is_none());
    assert_eq!(completed[0].turns, 0);
    assert_eq!(completed[0].spent, 0);
}
use smith_local_process_world::{
    Files, Placement, World,
    referee::{CheckoutRead, Ending, review},
};

#[test]
fn a_store_failure_completes_as_host_store_with_its_turn() {
    let files = Files::new();
    let mut first = World::new(6, &files, Placement::InProcess, b"First answer");
    first.settle();
    let mut world = World::failed_store_before_answer(7, &files, Placement::InProcess);
    world.scenario.launch.traced = true;
    world.settle();
    let records = world.records();
    let completed = records
        .iter()
        .filter_map(|record| match &record.event {
            Event::RunCompleted(completed) => Some(completed),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(completed.len(), 1);
    let failure = completed[0].failure.as_ref().expect("host failure");
    assert_eq!(failure.class, AnswerClass::Host);
    assert_eq!(failure.reason.as_deref(), Some(b"store".as_slice()));
    assert_eq!(failure.turn, Some(2));
    assert_eq!(completed[0].status, smith_events::Status::Failed);
    assert!(completed[0].result.is_none());
    assert_eq!(completed[0].turns, 1);
    assert_eq!(completed[0].read, Some(2));
    assert!(!completed[0].wind_down);
}

#[test]
fn a_cancelled_answer_completes_with_dropped_false() {
    let files = Files::new();
    let mut world = World::interrupted(4, &files, Placement::InProcess);
    world.scenario.launch.traced = true;
    world.settle();
    let records = world.records();
    let completed = records
        .iter()
        .filter_map(|record| match &record.event {
            Event::RunCompleted(completed) => Some(completed),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(completed.len(), 1);
    let failure = completed[0].failure.as_ref().expect("cancelled answer");
    assert_eq!(failure.class, AnswerClass::Cancelled);
    assert_eq!(failure.dropped, Some(false));
}

#[test]
fn a_chat_resumes_from_its_files_across_runs_of_the_binary() {
    let files = Files::new();
    let mut first = World::new(1, &files, Placement::Spawned, b"First answer");
    first.settle();
    assert_eq!(first.exit(), Some(kernel::Exit::Code(0)));
    assert_eq!(first.turns(), 1);
    let first_messages = first.queries()[0].messages.len();
    drop(first);
    let mut second = World::new(2, &files, Placement::Spawned, b"Second answer");
    second.settle();
    assert_eq!(second.exit(), Some(kernel::Exit::Code(0)));
    assert_eq!(second.turns(), 2);
    assert!(second.queries()[0].messages.len() > first_messages, "saved conversation reached the provider");
    assert!(
        second.queries()[0].messages.iter().flat_map(|message| message.parts.iter()).any(|part| {
            matches!(part, skein_fake_llm_domain::api::Part::ToolCall { name, arguments, .. }
            if name.as_ref() == b"finish" && arguments.windows(12).any(|part| part == b"First answer"))
        }),
        "the provider saw the previous finish result"
    );
}

#[test]
fn a_chat_runs_the_same_with_its_agent_in_process() {
    let first_files = Files::new();
    let second_files = Files::new();
    let mut spawned = World::new(3, &first_files, Placement::Spawned, b"First answer");
    let mut colocated = World::new(3, &second_files, Placement::InProcess, b"First answer");
    spawned.settle();
    colocated.settle();
    assert_eq!(spawned.exit(), colocated.exit());
    assert_eq!(spawned.shown(), colocated.shown());
    let mut spawned_queries = spawned.queries().to_vec();
    let mut colocated_queries = colocated.queries().to_vec();
    // Until the local host supplies its durable chat identity (08.4), each
    // child kind mints from its own RNG path. Each actual activation still
    // has one scope; every other native provider field must remain equal.
    for queries in [&mut spawned_queries, &mut colocated_queries] {
        let key = match &queries.first().expect("actual native request").caching {
            skein_fake_llm_domain::api::Caching::Scope(key) => *key,
            skein_fake_llm_domain::api::Caching::Unscoped | skein_fake_llm_domain::api::Caching::Marks(_) => {
                panic!("Codex run affinity is rendered")
            }
        };
        for query in queries {
            assert_eq!(query.caching, skein_fake_llm_domain::api::Caching::Scope(key));
            query.caching = skein_fake_llm_domain::api::Caching::Scope([0; 16]);
        }
    }
    assert_eq!(spawned_queries, colocated_queries);
    assert_eq!(spawned.turns(), colocated.turns());
}

#[test]
fn an_interrupt_in_the_middle_of_a_turn_cancels_the_run() {
    for placement in [Placement::Spawned, Placement::InProcess] {
        let files = Files::new();
        let mut world = World::interrupted(4, &files, placement);
        world.settle();
        assert!(world.shown().windows(13).any(|text| text == b"Run cancelled"));
        assert_eq!(world.exit(), Some(kernel::Exit::Code(0)));
        assert_eq!(world.queries().len(), 1);
    }
}

#[test]
fn a_store_failure_stops_the_chat_and_tells_the_person() {
    for placement in [Placement::Spawned, Placement::InProcess] {
        let files = Files::new();
        let mut first = World::new(6, &files, placement, b"First answer");
        first.settle();
        assert_eq!(first.exit(), Some(kernel::Exit::Code(0)));
        assert_eq!(first.turns(), 1);
        let kept = files.path().join("chat/0000000001.turn");
        let original = std::fs::read(&kept).expect("first durable turn");
        let mut failed = World::failed_store(7, &files, placement);
        failed.settle();
        assert_eq!(failed.exit(), Some(kernel::Exit::Code(1)));
        let shown = String::from_utf8_lossy(failed.shown());
        assert!(shown.contains("Chat main turn 2 could not be kept: write failed"), "{shown}");
        assert!(!failed.queries().is_empty());
        let fault = failed.scenario.launch.state_directory.join("0000000002.turn");
        assert!(std::fs::metadata(&fault).expect("failed turn target").is_dir());
        std::fs::remove_dir(fault).expect("remove only the test's rename fault");
        assert_eq!(failed.turns(), 1, "failed turn was never made durable");
        let mut resumed = World::new(8, &files, placement, b"Second answer");
        resumed.settle();
        assert_eq!(resumed.exit(), Some(kernel::Exit::Code(0)));
        assert_eq!(resumed.turns(), 2, "resume starts from the last durable transcript");
        assert_eq!(std::fs::read(kept).expect("kept turn after recovery"), original);
    }
}

#[test]
fn a_change_is_committed_in_place_with_its_fields_as_the_message() {
    for placement in [Placement::Spawned, Placement::InProcess] {
        let files = Files::new();
        let mut world = World::changed(5, &files, placement);
        let before = world.head().expect("initial head");
        world.settle();
        review(&world.seen(), &world, Ending::Change { before }).assert_passed(5);
        assert_eq!(world.exit(), Some(kernel::Exit::Code(0)));
        let message = std::str::from_utf8(world.commit_message()).expect("commit UTF-8");
        assert!(message.starts_with("Updated result\n\nCreated the result file"), "{message}");
        assert!(message.contains("Smith-Delivery:"), "{message}");
        assert_eq!(world.commit_files().get(b"result.txt".as_slice()).expect("committed file"), b"new\n");
        assert!(!world.git_commands().iter().any(|args| args[0].as_ref() == b"push"));
    }
}

#[test]
fn a_follow_up_sent_as_the_run_finishes_is_read_once_in_both_placements() {
    use skein_lib::Token;
    use smith_local_domain::Fact;
    for placement in [Placement::Spawned, Placement::InProcess] {
        let files = Files::new();
        let mut world = World::finishing(19, &files, placement);
        world.settle();
        review(&world.seen(), &world, Ending::Report(b"Second answer".to_vec())).assert_passed(19);
        let reads = world
            .facts()
            .iter()
            .filter(|fact| matches!(fact, Fact::Answered { read: Some(name), .. } if *name == Token::new(2)))
            .count();
        assert_eq!(reads, 1, "the follow-up ends read once");
        assert_eq!(world.exit(), Some(kernel::Exit::Code(0)));
    }
}

#[test]
fn an_interrupt_with_a_line_unread_starts_nothing_in_both_placements() {
    use smith_local_domain::Fact;
    use smith_local_process_world::terminal::Action;
    for placement in [Placement::Spawned, Placement::InProcess] {
        let files = Files::new();
        let mut world = World::interrupted(21, &files, placement);
        world.scenario.commands = vec![
            Action::Send(b"hello\nfollow-up\n".as_slice().into()),
            Action::Wait(b"Run cancelled".as_slice().into()),
            Action::Eof,
        ];
        world.settle();
        review(&world.seen(), &world, Ending::Cancelled).assert_passed(21);
        assert_eq!(world.facts().iter().filter(|fact| matches!(fact, Fact::Started { .. })).count(), 1);
    }
}

#[test]
fn no_waiting_notice_shows_while_a_line_is_unread_in_both_placements() {
    for placement in [Placement::Spawned, Placement::InProcess] {
        let files = Files::new();
        let mut world = World::waiting(23, &files, placement);
        world.settle();
        review(&world.seen(), &world, Ending::Report(b"Read follow-up".to_vec())).assert_passed(23);
        let read = world.shown().windows(14).position(|text| text == b"Read follow-up").expect("follow-up read");
        assert!(!world.shown()[..read].windows(21).any(|text| text == b"Waiting for a message"));
        assert!(world.facts().iter().any(|fact| {
            matches!(fact, smith_local_domain::Fact::Answered { read: Some(name), .. } if name.raw() == 2)
        }));
    }
}

fn assert_crossing_wait_records(records: &[smith_events::Record]) {
    let position = |predicate: fn(&Event) -> bool| records.iter().position(|record| predicate(&record.event)).unwrap();
    let wait = position(|event| matches!(event, Event::ToolStarted(tool) if tool.tool.as_ref() == b"wait"));
    let received = position(|event| matches!(event, Event::MessageReceived(message) if message.name == 2));
    let read = position(|event| matches!(event, Event::MessageRead(message) if message.name == 1));
    let resumed = position(|event| matches!(event, Event::ResponseStarted(response) if response.response == 2));
    assert!(wait < received && received < read && read < resumed);
    assert!(records.iter().any(|record| matches!(&record.event,
        Event::MessageRead(message) if message.name == 2)));
}

#[test]
fn a_turn_saved_while_a_line_crosses_a_wait_releases_the_next_response() {
    use smith_local_process_world::terminal::Action;
    for placement in [Placement::Spawned, Placement::InProcess] {
        let files = Files::new();
        let mut world = World::waiting(37, &files, placement);
        world.scenario.launch.traced = true;
        world.scenario.ending = Ending::Report(b"Crossed wait".to_vec());
        world.scenario.commands = vec![
            Action::Send(b"hello\n".as_slice().into()),
            Action::Wait(b"Crossed wait".as_slice().into()),
            Action::Eof,
        ];
        world.settle();
        review(&world.seen(), &world, Ending::Report(b"Crossed wait".to_vec())).assert_passed(37);
        assert_eq!(world.queries().len(), 2);
        assert_eq!(world.exit(), Some(kernel::Exit::Code(0)));
        assert_eq!(world.turns(), 2);
        let records = match placement {
            Placement::Spawned => world.agent_records(),
            Placement::InProcess => world.records(),
        };
        assert_crossing_wait_records(&records);
    }
}
