use skein_io::kernel;
use smith_local_process_world::{
    Files, Placement, World,
    referee::{CheckoutRead, Ending, review},
};

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
    assert_eq!(spawned.queries(), colocated.queries());
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
fn an_interrupt_mid_turn_and_a_second_drop_the_run() {
    for placement in [Placement::Spawned, Placement::InProcess] {
        let files = Files::new();
        let mut world = World::dropped(405, &files, placement);
        world.settle();
        assert_eq!(world.exit(), Some(kernel::Exit::Code(130)));
        assert_eq!(world.queries().len(), 1, "no provider response before the drop");
        assert_eq!(world.turns(), 0);
        assert!(world.shown().windows(11).any(|bytes| bytes == b"Run dropped"));
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
