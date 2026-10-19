use skein_world::domain::assert_replays;
use smith_local_domain::ExitStatus;
use smith_local_world::{Cut, StoreFault, World};

#[test]
fn a_chat_granted_sub_agents_delegates_a_reading_and_answers() {
    let mut world = World::with_delegation(702);
    world.line(b"Read the answer with a child");
    world.drive(400);
    assert_eq!(world.child_prompts().len(), 2, "child reads, then answers its caller");
    for (grants, served) in world.child_prompts() {
        assert!(grants.inspect && !grants.modify && !grants.shell);
        assert!(served.is_empty(), "the child cannot delegate or use main's served tools");
    }
    assert!(
        world.tool_results().iter().any(|text| text
            .windows(b"The answer is in src/lib.rs".len())
            .any(|part| part == b"The answer is in src/lib.rs")),
        "main receives the child's reading"
    );
    assert!(
        world
            .tool_results()
            .iter()
            .any(|text| text.windows(b"pub fn answer()".len()).any(|part| part == b"pub fn answer()")),
        "the child receives the actual file read"
    );
    assert!(world.shown().iter().any(|text| text.as_ref() == b"The delegated reading found the answer."));
    assert_eq!(world.saved_turns(), 2, "main's delegate and final answer are durable");
    let (source, _) = world.disk().load(1, b"src/lib.rs", 4096).expect("source kept");
    assert_eq!(source, b"pub fn answer() -> u32 { 42 }\n");
    assert_replays(702, 703, |seed| {
        let mut world = World::with_delegation(seed);
        world.line(b"Read the answer with a child");
        world.drive(400);
        (world.trace().to_vec(), world.shown().to_vec())
    });
}

#[test]
fn a_question_is_answered_and_the_chat_waits_for_the_next() {
    let mut world = World::new(17);
    world.line(b"What can you do?");
    assert!(world.drive(200), "the chat reaches a saved turn and a wait");
    assert_eq!(world.saved_turns(), 1, "the answered exchange is durable while the next wait remains withheld");
    assert!(world.waiting(), "the run asked to wait for another person message");
    assert!(world.completions() >= 2, "the fake provider served the reply and wait");
    assert!(
        world
            .shown()
            .iter()
            .any(|text| text.windows(b"Hello from the agent.".len()).any(|bytes| bytes == b"Hello from the agent."))
    );
}

#[test]
fn a_chat_reads_and_edits_files_in_a_workspace_directory() {
    let mut world = World::with_workspace(18);
    world.line(b"Change the answer in work");
    world.drive(400);
    let (content, _) = world.disk().load(1, b"src/lib.rs", 4096).expect("workspace source remains present");
    assert_eq!(content, b"pub fn answer() -> u32 { 43 }\n");
    assert!(world.shown().iter().any(|text| text.as_ref() == b"The answer is 43."));
}

#[test]
fn a_change_to_a_plain_directory_keeps_its_files() {
    let mut world = World::with_plain_change(19);
    world.line(b"Make the answer 43");
    world.drive(400);
    let (content, _) = world.disk().load(1, b"src/lib.rs", 4096).expect("plain file is kept");
    assert_eq!(content, b"pub fn answer() -> u32 { 43 }\n");
    assert!(world.shown().iter().any(|text| text.as_ref() == b"Change delivered"));
}

#[test]
fn a_delivery_with_no_changes_answers_nothing_and_the_llm_is_told() {
    let mut world = World::with_plain_nothing(192);
    world.line(b"Deliver the current state");
    world.drive(400);
    assert!(matches!(world.delivery(), Some(smith_domain::run::Delivery::Nothing)));
    assert!(world.tool_results().iter().any(|text| text.windows(b"nothing".len()).any(|part| part == b"nothing")));
}

#[test]
fn a_marker_left_in_a_conflicted_file_refuses_the_delivery_naming_it() {
    let mut world = World::with_git_marker(193);
    world.line(b"Deliver the merge");
    world.drive(400);
    let Some(smith_domain::run::Delivery::Refused(refusal)) = world.delivery() else {
        panic!("marker refusal recorded")
    };
    assert_eq!(refusal.marker().expect("named marker").path(), b"src/lib.rs");
    assert!(
        world.tool_results().iter().any(|text| text.windows(b"src/lib.rs".len()).any(|part| part == b"src/lib.rs"))
    );
}

#[test]
fn a_commit_failing_in_the_second_directory_names_it_and_stops() {
    let mut world = World::with_second_commit_failure(194);
    world.line(b"Deliver both repositories");
    world.drive(500);
    let Some(smith_domain::run::Delivery::Failed(failure)) = world.delivery() else { panic!("failure recorded") };
    assert_eq!(failure.directory, 1);
    assert_eq!(failure.reason, smith_domain::run::DeliveryReason::Broken);
    assert_eq!(failure.diagnostic.output(), b"simulated git commit failure");
    assert!(world.commit_message().is_some(), "first directory committed before the second failed");
}

#[test]
fn a_cancel_during_a_delivery_waits_for_it_and_still_answers_cancelled() {
    let mut world = World::with_mid_report(195);
    world.slow_git();
    world.line(b"Make the answer 43");
    assert!(!world.drive(500), "delivery pauses at its first git terminal");
    world.interrupt();
    assert!(!world.drive_to_cancelled(100), "cancel waits for the delivery terminal");
    for _ in 0..4 {
        if !world.release_git() {
            break;
        }
        if world.drive_to_cancelled(500) {
            break;
        }
    }
    assert!(world.commit_message().is_some(), "the in-flight delivery completed");
    assert!(
        world.shown().iter().any(|text| text.as_ref() == b"interrupted: the run was cancelled\n"),
        "shown: {:?}, delivery: {:?}",
        world.shown(),
        world.delivery()
    );
}

#[test]
fn a_change_is_checked_and_committed_in_place() {
    let mut world = World::with_git_change(20);
    world.line(b"Make the answer 43");
    world.drive(500);
    let (content, _) = world.disk().load(1, b"src/lib.rs", 4096).expect("committed source remains present");
    assert_eq!(content, b"pub fn answer() -> u32 { 43 }\n");
    assert_eq!(
        world.commit_message(),
        Some(b"Make the answer 43\n\nThe answer is 43 now.\n\nSmith-Delivery: 1/3/0".as_slice())
    );
    assert!(world.shown().iter().any(|text| text.as_ref() == b"Change delivered"));
}

#[test]
fn a_configured_push_lands_after_the_local_commit() {
    let mut world = World::with_push(205);
    world.line(b"Make the answer 43 and push");
    world.drive(500);
    assert_eq!(world.remote_head(), Some(2), "the remote branch names the committed change");
    assert!(matches!(world.delivery(), Some(smith_domain::run::Delivery::Delivered(_))));
    assert!(world.shown().iter().any(|text| text.as_ref() == b"Change delivered"));
}

#[test]
fn a_moved_remote_makes_a_configured_push_stale() {
    let mut world = World::with_moved_remote(206);
    world.line(b"Make the answer 43 and push");
    world.drive(500);
    assert_eq!(world.remote_head(), Some(2), "the remote stayed on its outside commit");
    assert!(matches!(world.delivery(), Some(smith_domain::run::Delivery::Stale)));
    assert!(world.shown().iter().any(|text| text.as_ref() == b"run failed: stale: workspace changed\n"));
}

#[test]
fn a_commit_made_meanwhile_in_the_repository_makes_the_delivery_stale() {
    let mut world = World::with_git_change(234);
    world.commit_meanwhile();
    world.line(b"Make the answer 43");
    world.drive(600);
    assert!(
        matches!(world.delivery(), Some(smith_domain::run::Delivery::Stale)),
        "delivery: {:?}; trace: {:?}",
        world.delivery(),
        world.trace()
    );
    assert_eq!(world.delivery_commits(), 0, "the host made no delivery commit");
}

#[test]
fn a_second_delivery_after_the_hosts_own_commit_is_not_stale() {
    let mut world = World::with_two_deliveries(235);
    world.line(b"Deliver twice");
    world.drive(600);
    assert_eq!(world.delivery_commits(), 1);
    assert!(matches!(world.delivery(), Some(smith_domain::run::Delivery::Nothing)));
}

#[test]
fn a_commit_before_push_is_recovered_to_the_recorded_target() {
    let mut first = World::with_push(224);
    first.line(b"Make the answer 43 and push");
    assert!(first.drive_to_cut(600, Cut::AfterCommit(0)));
    assert_eq!(first.remote_head(), Some(1));
    let mut second = World::with_git_change_store(225, first.into_store());
    second.line(b"What landed?");
    assert!(second.drive_to_cut(600, Cut::AfterSaveDelivery));
    assert_eq!(second.remote_head(), Some(2), "recovery uses the saved target despite changed configuration");
    assert!(matches!(second.delivery(), Some(smith_domain::run::Delivery::Delivered(_))));
    assert_eq!(second.delivery_commits(), 1);
}

#[test]
fn a_push_effect_before_its_terminal_is_recovered_as_delivered() {
    let mut first = World::with_push(226);
    first.line(b"Make the answer 43 and push");
    assert!(first.drive_to_cut(600, Cut::BeforePushTerminal));
    assert_eq!(first.remote_head(), Some(2));
    let mut second = World::with_push_store(227, first.into_store());
    second.line(b"What landed?");
    assert!(second.drive_to_cut(600, Cut::AfterSaveDelivery));
    assert_eq!(second.remote_head(), Some(2));
    assert!(matches!(second.delivery(), Some(smith_domain::run::Delivery::Delivered(_))));
    assert_eq!(second.delivery_commits(), 1);
}

#[test]
fn a_push_terminal_before_the_saved_answer_is_recovered_as_delivered() {
    let mut first = World::with_push(228);
    first.line(b"Make the answer 43 and push");
    assert!(first.drive_to_cut(600, Cut::AfterPushTerminal));
    assert_eq!(first.remote_head(), Some(2));
    let mut second = World::with_push_store(229, first.into_store());
    second.line(b"What landed?");
    assert!(second.drive_to_cut(600, Cut::AfterSaveDelivery));
    assert_eq!(second.remote_head(), Some(2));
    assert!(matches!(second.delivery(), Some(smith_domain::run::Delivery::Delivered(_))));
    assert_eq!(second.delivery_commits(), 1);
}

#[test]
fn a_recovered_push_whose_remote_moved_is_stale() {
    let mut first = World::with_moved_remote(230);
    first.line(b"Make the answer 43 and push");
    assert!(first.drive_to_cut(600, Cut::AfterCommit(0)));
    let mut second = World::with_moved_remote_store(231, first.into_store());
    second.line(b"What landed?");
    assert!(second.drive_to_cut(600, Cut::AfterSaveDelivery));
    assert!(matches!(second.delivery(), Some(smith_domain::run::Delivery::Stale)));
    assert_eq!(second.delivery_commits(), 1);
}

#[test]
fn a_recovered_push_failure_keeps_its_local_commit_receipt() {
    let mut first = World::with_push(232);
    first.line(b"Make the answer 43 and push");
    assert!(first.drive_to_cut(600, Cut::AfterCommit(0)));
    let mut second = World::with_push_store(233, first.into_store());
    second.fail_next_push();
    second.line(b"What landed?");
    assert!(second.drive_to_cut(600, Cut::AfterSaveDelivery));
    let Some(smith_domain::run::Delivery::Failed(failure)) = second.delivery() else { panic!("push failed") };
    assert_eq!(failure.directory, 0);
    assert_eq!(failure.reason, smith_domain::run::DeliveryReason::Broken);
    assert_eq!(failure.diagnostic.output(), b"simulated push failure");
    assert_eq!(second.delivery_commits(), 1);
}

#[test]
fn a_crash_after_a_commit_tells_the_resumed_run_what_was_committed() {
    let mut first = World::with_git_change(201);
    first.line(b"Make the answer 43");
    assert!(first.drive_to_cut(600, Cut::AfterSaveDelivery), "the commit and its decision become durable");
    assert_eq!(
        first.commit_message(),
        Some(b"Make the answer 43\n\nThe answer is 43 now.\n\nSmith-Delivery: 1/3/0".as_slice())
    );
    let store = first.into_store();
    let mut second = World::with_git_change_store(202, store);
    second.line(b"What happened before the crash?");
    second.drive(600);
    assert!(
        second.prompt_texts().iter().any(|text| text.windows(b"commit 2".len()).any(|part| part == b"commit 2")),
        "prompts: {:?}, trace: {:?}, shown: {:?}",
        second.prompt_texts(),
        second.trace(),
        second.shown()
    );
}

#[test]
fn two_deliveries_before_a_saved_turn_are_both_told_after_a_crash() {
    let mut first = World::with_two_deliveries(216);
    first.line(b"Deliver twice");
    assert!(first.drive_to_cut(600, Cut::AfterSecondSaveDelivery), "both delivery answers are durable");
    let store = first.into_store();
    assert_eq!(store.turns(), 1, "the call turn is not saved yet");
    assert_eq!(store.delivery_answers(), 2);
    let mut second = World::with_two_deliveries_store(217, store);
    second.line(b"What was delivered?");
    second.drive(600);
    assert_eq!(second.delivery_commits(), 1, "the first delivery is not repeated");
    let prompts = second.prompt_texts();
    for answer in [b"position=0 tool=deliver result: delivered".as_slice(), b"position=1 tool=deliver error: nothing"] {
        assert!(
            prompts.iter().any(|text| text.windows(answer.len()).any(|part| part == answer)),
            "missing: {answer:?}"
        );
    }
}

#[test]
fn a_saved_stale_answer_is_told_after_a_crash() {
    let mut first = World::with_moved_remote(218);
    first.line(b"Make the answer 43 and push");
    assert!(first.drive_to_cut(600, Cut::AfterSaveDelivery));
    assert!(matches!(first.delivery(), Some(smith_domain::run::Delivery::Stale)));
    let mut second = World::with_moved_remote_store(219, first.into_store());
    second.line(b"What happened?");
    second.drive(600);
    assert!(
        second.prompt_texts().iter().any(|text| text
            .windows(b"tool=deliver error: stale".len())
            .any(|part| part == b"tool=deliver error: stale"))
    );
    assert_eq!(second.delivery_commits(), 1);
}

#[test]
fn a_saved_failure_after_a_commit_is_told_after_a_crash() {
    let mut first = World::with_second_commit_failure(220);
    first.line(b"Deliver both repositories");
    assert!(first.drive_to_cut(600, Cut::AfterSaveDelivery));
    assert_eq!(first.delivery_commits(), 1);
    let mut second = World::with_second_commit_failure_store(221, first.into_store());
    second.line(b"What happened?");
    second.drive(600);
    let outcome = b"tool=deliver error: delivery-failed directory=1 reason=broken";
    assert!(second.prompt_texts().iter().any(|text| text.windows(outcome.len()).any(|part| part == outcome)));
    assert_eq!(second.delivery_commits(), 1);
}

#[test]
fn a_maximum_length_person_line_reaches_the_run_whole_after_a_crash() {
    let mut first = World::with_git_change(222);
    first.line(b"Make the answer 43");
    assert!(first.drive_to_cut(600, Cut::AfterSaveDelivery));
    let mut second = World::with_git_change_store(223, first.into_store());
    let line = vec![b'Q'; 1024];
    second.line(&line);
    second.drive(600);
    assert!(second.prompt_texts().iter().any(|text| text.windows(line.len()).any(|part| part == line)));
}

#[test]
fn an_intent_saved_before_the_first_commit_reconciles_as_interrupted() {
    let mut first = World::with_mid_report(207);
    first.line(b"Make the answer 43");
    assert!(first.drive_to_cut(600, Cut::AfterIntent));
    assert_eq!(first.delivery_commits(), 0);
    let mut second = World::with_mid_report_store(208, first.into_store());
    second.drive(100);
    let Some(smith_domain::run::Delivery::Failed(failure)) = second.delivery() else { panic!("interruption saved") };
    assert_eq!(failure.directory, 0);
    assert_eq!(failure.reason, smith_domain::run::DeliveryReason::Broken);
    assert_eq!(failure.diagnostic.output(), b"delivery interrupted; committed: none");
    assert_eq!(second.delivery_commits(), 0);
    second.line(b"What happened before the crash?");
    second.drive(600);
    assert!(second.trace().iter().any(|line| line.contains("Message")), "the next run received its waking message");
}

#[test]
fn an_intent_after_the_first_of_two_commits_reports_only_that_commit() {
    let mut first = World::with_second_commit_failure(209);
    first.line(b"Deliver both repositories");
    assert!(first.drive_to_cut(600, Cut::AfterCommit(0)));
    assert_eq!(first.delivery_commits(), 1);
    let mut second = World::with_second_commit_failure_store(210, first.into_store());
    second.drive(100);
    let Some(smith_domain::run::Delivery::Failed(failure)) = second.delivery() else {
        panic!("partial delivery saved")
    };
    assert_eq!(failure.directory, 1);
    assert_eq!(failure.reason, smith_domain::run::DeliveryReason::Broken);
    assert!(failure.diagnostic.output().windows(b"commit 2".len()).any(|part| part == b"commit 2"));
    assert_eq!(second.delivery_commits(), 1, "recovery did not commit again");
    second.line(b"What landed?");
    second.drive(600);
    assert!(
        second.prompt_texts().iter().any(|text| text.windows(b"commit 2".len()).any(|part| part == b"commit 2")),
        "prompts: {:?}, shown: {:?}",
        second.prompt_texts(),
        second.shown()
    );
}

#[test]
fn an_intent_after_every_commit_reconciles_to_delivered() {
    let mut first = World::with_git_change(211);
    first.line(b"Make the answer 43");
    assert!(first.drive_to_cut(600, Cut::AfterCommit(0)));
    assert_eq!(first.delivery_commits(), 1);
    let mut second = World::with_git_change_store(212, first.into_store());
    second.drive(100);
    assert!(matches!(second.delivery(), Some(smith_domain::run::Delivery::Delivered(_))));
    assert_eq!(second.delivery_commits(), 1);
    second.line(b"What landed?");
    second.drive(600);
    assert!(
        second.prompt_texts().iter().any(|text| text.windows(b"commit 2".len()).any(|part| part == b"commit 2")),
        "prompts: {:?}, shown: {:?}",
        second.prompt_texts(),
        second.shown()
    );
}

#[test]
fn a_delivery_asked_again_under_its_name_gets_its_first_answer() {
    let mut world = World::with_git_change(203);
    world.line(b"Make the answer 43");
    assert!(world.drive_to_cut(500, Cut::AfterSaveDelivery));
    let commit = world.commit_message().expect("one real commit").to_vec();
    let store = world.into_store();
    let name = store.delivery_name().expect("one saved delivery name");
    let first = store.delivery_answer(name).expect("first answer remains saved");
    assert!(matches!(first, smith_domain::run::Delivery::Delivered(_)));
    assert_eq!(store.delivery_answer(name), Some(first));
    assert_eq!(store.delivery_answer(smith_domain::run::CallName { activation: name.activation + 1, ..name }), None);
    let resumed = World::with_git_change_store(204, store);
    assert_eq!(resumed.commit_message(), Some(commit.as_slice()), "replay did not make a second commit");
}

#[test]
fn a_chat_with_no_workspace_waits_parks_and_resumes_on_the_next_invocation() {
    let mut first = World::new(21);
    first.line(b"Hello");
    assert!(first.drive_to_park(300), "first activation parks after waiting");
    assert_eq!(first.activation(), 1);
    let stored = first.into_store();
    let prior_turns = stored.turns();
    assert!(prior_turns > 0, "the parked chat kept its turns");

    let mut second = World::with_store(22, stored);
    second.line(b"I am back");
    assert!(!second.drive(300), "a fresh domain resumes from {prior_turns} stored turns: {:?}", second.trace());
    assert_eq!(second.activation(), 2, "the next activation is durable before its agent runs");
    assert_eq!(second.prompt_assistants().first(), Some(&prior_turns), "provider sees the saved assistant history");
    assert!(
        second.shown().iter().any(|text| text.windows(b"Welcome back.".len()).any(|bytes| bytes == b"Welcome back."))
    );
}

#[test]
fn a_chat_keeps_its_identity_across_activations_and_invocations() {
    let mut first = World::new(8084);
    first.line(b"Hello");
    assert!(first.drive_to_park(300), "first activation parks");
    let identity = first.identity().expect("identity durable before the child starts");
    assert_eq!(first.activation(), 1);
    first.line(b"Another activation in this invocation");
    assert!(first.drive_to_park(300), "a later activation actually reaches its own park");
    assert_eq!(first.activation(), 2);
    assert_eq!(first.identity(), Some(identity));
    assert!(!first.prompt_affinities().is_empty());
    assert!(first.prompt_affinities().iter().all(|affinity| affinity.key == identity.bytes() && affinity.thread == 0));

    let mut second = World::with_store(8085, first.into_store());
    second.line(b"Resume in another invocation");
    assert!(second.drive_to_park(300), "resumed activation parks");
    assert_eq!(second.activation(), 3);
    assert_eq!(second.identity(), Some(identity), "the new invocation's seed does not replace the saved key");
    assert!(!second.prompt_affinities().is_empty());
    assert!(second.prompt_affinities().iter().all(|affinity| affinity.key == identity.bytes() && affinity.thread == 0));

    let mut fresh = World::with_store_fresh(8086, second.into_store());
    fresh.line(b"Begin afresh");
    assert!(fresh.drive_to_park(300), "resume off starts a fresh conversation");
    let replacement = fresh.identity().expect("fresh identity durable");
    assert_ne!(replacement, identity);
    assert!(!fresh.prompt_affinities().is_empty());
    assert!(
        fresh.prompt_affinities().iter().all(|affinity| affinity.key == replacement.bytes() && affinity.thread == 0)
    );
    let mut resumed = World::with_store(8087, fresh.into_store());
    resumed.line(b"Resume the replacement chat");
    assert!(!resumed.drive(300));
    assert_eq!(resumed.identity(), Some(replacement));
    assert!(resumed.prompt_affinities().iter().all(|affinity| affinity.key == replacement.bytes()));
}

#[test]
fn a_crash_after_saving_an_activation_never_reuses_it() {
    let mut first = World::new(23);
    first.line(b"First attempt");
    assert!(first.drive_to_cut(100, Cut::AfterSaveState));
    let stored = first.into_store();
    assert_eq!(stored.activation(), 1);
    let identity = stored.identity().expect("metadata names the chat before any provider work");
    assert_eq!(stored.turns(), 0, "the child was never started");

    let mut second = World::with_store(24, stored);
    second.line(b"Retry");
    assert!(second.drive_to_cut(100, Cut::AfterSaveState));
    assert_eq!(second.activation(), 2, "the crashed activation is never reused");
    assert_eq!(second.identity(), Some(identity), "the crash did not replace its durable chat identity");
}

#[test]
fn a_crash_before_a_turn_is_saved_resumes_from_the_turn_before_with_a_new_activation() {
    let mut first = World::new(25);
    first.line(b"Hello");
    assert!(first.drive_to_cut(300, Cut::BeforeSaveTurn(2)));
    let stored = first.into_store();
    assert_eq!(stored.turns(), 1, "the second turn did not reach durable storage");
    let identity = stored.identity().expect("first activation identity durable");
    assert_eq!(stored.read(), Some(skein_lib::Token::new(1)), "the first turn read the carried person line");

    let mut second = World::with_store(26, stored);
    second.line(b"Continue");
    assert!(!second.drive(300), "the next activation resumes the saved prefix");
    assert_eq!(second.activation(), 2, "the replacement activation has a new name");
    assert_eq!(second.identity(), Some(identity));
    assert!(second.prompt_affinities().iter().all(|affinity| affinity.key == identity.bytes()));
    assert_eq!(second.prompt_assistants().first(), Some(&1), "only the turn before the cut is replayable");
}

#[test]
fn a_crash_after_turn_saved_replays_that_turn() {
    let mut first = World::new(27);
    first.line(b"Hello");
    assert!(first.drive_to_cut(300, Cut::AfterTurnSaved(2)));
    let stored = first.into_store();
    assert_eq!(stored.turns(), 2);
    assert!(stored.read().is_some(), "the acknowledged turn's read fence is durable");
    let mut second = World::with_store(28, stored);
    second.line(b"Continue");
    assert!(!second.drive(300), "the acknowledged turn is replayed");
    assert_eq!(second.activation(), 2);
    assert_eq!(second.prompt_assistants().first(), Some(&2), "the acknowledged turns are in the next prompt");
}

#[test]
fn a_transcript_the_agent_refuses_is_reported_and_nothing_starts() {
    let mut first = World::new(29);
    first.line(b"Hello");
    assert!(first.drive_to_park(300));
    let mut stored = first.into_store();
    stored.refuse_version();
    let mut second = World::with_store(30, stored);
    second.line(b"Continue");
    assert!(!second.drive(300), "a refused transcript cannot reach a wait");
    assert_eq!(second.completions(), 0, "refusal precedes provider effects");
    assert!(second.shown().iter().any(|text| text.as_ref() == b"run failed: chat cannot be resumed: version\n"));

    let stored = second.into_store();
    let mut fresh = World::with_store_fresh(31, stored);
    fresh.line(b"Start a fresh chat");
    assert!(fresh.drive(300), "resume false ignores the refused history");
    assert!(fresh.completions() > 0, "the fresh run reaches the provider");

    let stored = fresh.into_store();
    let mut resumed = World::with_store(32, stored);
    resumed.line(b"Continue after a fresh start");
    assert!(!resumed.drive(300), "the replacement history is resumable");
}

#[test]
fn an_expired_credential_is_refreshed_and_the_completion_retried() {
    let mut world = World::new(41);
    world.reject_first_credential();
    world.line(b"Hello");
    assert!(world.drive(300), "the provider retry reaches the next wait");
    assert_eq!(world.credential_requests(), 2, "the rejected generation triggers one refresh");
    assert!(world.completions() >= 3, "the rejected completion was retried");
    assert!(
        world.shown().iter().any(|text| {
            text.windows(b"Hello from the agent.".len()).any(|bytes| bytes == b"Hello from the agent.")
        })
    );
}

#[test]
fn a_limited_account_is_shown_with_its_reset() {
    let mut world = World::new(44);
    world.exhaust_first_account();
    world.line(b"Hello");
    world.drive(300);
    assert!(world.notices().contains(&smith_local_domain::Notice::Failure(smith_local_domain::Failure::Account(
        smith_local_domain::AccountFailure::Limited {
            account: 0,
            reset_at: Some(skein_lib::Wall::from_nanos(60_000_000_000))
        }
    ))));
    assert!(
        world
            .shown()
            .iter()
            .any(|text| text.as_ref() == b"run failed: account 0 reached its usage limit, usable again at 00:01\n")
    );
}

#[test]
fn a_refused_account_is_shown_with_its_code_and_reason() {
    let mut world = World::headless(44, b"Hello");
    world.refuse_first_account();
    assert!(world.drive_to_exit(300));
    assert!(
        world
            .shown()
            .iter()
            .any(|text| text.as_ref() == b"run failed: account 0: the provider refused it: billing: contact support\n")
    );
    assert_eq!(world.exit(), Some(smith_local_domain::ExitStatus::Credentials { account: 0 }));
}

#[test]
fn a_run_cancelled_at_the_terminal_answers_cancelled_after_its_last_turn_is_saved() {
    let mut world = World::new(42);
    world.slow_store();
    world.line(b"Hello");
    assert!(!world.drive(300), "the slow store holds a turn acknowledgement");
    assert!(world.delayed_turns() > 0);
    world.interrupt();
    assert!(!world.drive_to_cancelled(300), "the cancelled answer waits for the store");
    assert!(!world.shown().iter().any(|text| text.as_ref() == b"interrupted: the run was cancelled\n"));
    for _ in 0..4 {
        assert!(world.release_turn(), "a stalled turn must be acknowledged");
        if world.drive_to_cancelled(300) {
            break;
        }
    }
    assert!(
        world.shown().iter().any(|text| text.as_ref() == b"interrupted: the run was cancelled\n"),
        "the answer follows the last saved turn"
    );
    assert!(world.saved_turns() > 0);
}

#[test]
fn a_slow_store_pauses_the_agent_and_loses_no_turn() {
    let mut world = World::tight_unsaved(43);
    world.slow_store();
    world.line(b"Hello");
    assert!(!world.drive(300), "the full unsaved window pauses the chat");
    assert_eq!(world.saved_turns(), 1, "the second completion cannot tell another turn yet");
    assert_eq!(world.delayed_turns(), 1);

    let mut parked = false;
    for _ in 0..8 {
        assert!(world.release_turn(), "each stalled turn has an acknowledgement to release");
        if world.drive_to_park(300) {
            parked = true;
            break;
        }
    }
    assert!(parked, "the paused agent parks after its actual store acknowledgements");
    assert!(world.waiting(), "the run asked to wait before parking");
    assert_eq!(world.saved_turns(), 2, "the exchange and withheld wait each reach the store once");
}

#[test]
fn a_chat_replays_the_same_boundary_trace_from_its_seed() {
    let trace = assert_replays(71, 72, |seed| {
        let mut world = World::new(seed);
        world.line(b"Hello");
        assert!(world.drive(300));
        (world.trace().to_vec(), (world.saved_turns(), world.shown().to_vec()))
    });
    assert!(trace.iter().any(|line| line.contains("TurnSaved")));
}

#[test]
fn an_interrupted_delivery_replays_the_same_reconciliation() {
    let trace = assert_replays(213, 214, |seed| {
        let mut first = World::with_git_change(seed);
        first.line(b"Make the answer 43");
        assert!(first.drive_to_cut(600, Cut::AfterCommit(0)));
        let mut trace = first.trace().to_vec();
        let mut second = World::with_git_change_store(seed + 1, first.into_store());
        second.drive(100);
        trace.extend(second.trace().iter().cloned());
        (trace, (second.delivery().cloned(), second.delivery_commits()))
    });
    assert!(trace.iter().any(|line| line.contains("DeliveryRecorded")));
}

#[test]
fn facts_do_not_change_the_chat_when_their_queue_is_full() {
    let mut observed = World::new(73);
    observed.line(b"Hello");
    assert!(observed.drive(300));
    let mut dropped = World::without_facts(73);
    dropped.line(b"Hello");
    assert!(dropped.drive(300));
    assert_eq!(observed.saved_turns(), dropped.saved_turns());
    assert_eq!(observed.shown(), dropped.shown());
    assert!(dropped.facts_lost() > 0);
    assert!(observed.judged().0 > 0);
}

#[test]
fn a_store_failure_stops_the_chat_and_tells_the_person() {
    for fault in [StoreFault::Load, StoreFault::State, StoreFault::Turn] {
        let mut world = World::new(75);
        world.fail_store(fault);
        world.line(b"Hello");
        world.drive(300);
        assert_eq!(world.exit(), Some(ExitStatus::RunFailed), "{fault:?} failure exits");
        assert!(world.notices().iter().any(|notice| matches!(
            notice,
            smith_local_domain::Notice::Failure(smith_local_domain::Failure::Store { .. })
        )));
    }
}

#[test]
fn a_deadline_racing_a_landed_commit_is_answered_with_its_receipt() {
    let mut world = World::with_git_change(871);
    world.uncertain_git(smith_local_world::GitFault::Deadline);
    world.line(b"Make the answer 43");
    world.drive(600);
    assert_eq!(world.delivery_commits(), 1);
    assert!(matches!(world.delivery(), Some(smith_domain::run::Delivery::Delivered(_))));
}

#[test]
fn an_unreadable_head_after_commit_is_settled_by_the_delivery_trailer() {
    let mut world = World::with_git_change(872);
    world.uncertain_git(smith_local_world::GitFault::HeadUnreadable);
    world.line(b"Make the answer 43");
    world.drive(600);
    assert_eq!(world.delivery_commits(), 1);
    assert!(matches!(world.delivery(), Some(smith_domain::run::Delivery::Delivered(_))));
}

#[test]
fn an_unknown_commit_keeps_its_intent_until_a_restart_finds_it() {
    let mut first = World::with_git_change(873);
    first.uncertain_git(smith_local_world::GitFault::InspectionUnreadable);
    first.line(b"Make the answer 43");
    first.drive(600);
    assert_eq!(first.delivery_commits(), 1);
    let store = first.into_store();
    assert_eq!(store.delivery_answers(), 0, "unknown did not replace the saved intent");
    assert!(store.delivery_name().is_some(), "the intent survives a later saved turn");
    let mut second = World::with_git_change_store(874, store);
    second.line(b"What landed?");
    second.drive(600);
    assert_eq!(second.delivery_commits(), 1, "reconciliation never committed again");
    assert!(second.prompt_texts().iter().any(|text| text.windows(b"delivered".len()).any(|part| part == b"delivered")));
}

#[test]
fn an_absent_delivery_trailer_and_unchanged_head_prove_no_effect() {
    let mut world = World::with_git_change(875);
    world.uncertain_git(smith_local_world::GitFault::NoEffect);
    world.line(b"Make the answer 43");
    world.drive(600);
    assert_eq!(world.delivery_commits(), 0);
    let Some(smith_domain::run::Delivery::Failed(failure)) = world.delivery() else { panic!("original failure") };
    assert_eq!(failure.reason, smith_domain::run::DeliveryReason::TimedOut);
}

#[test]
fn a_headless_run_that_would_wait_parks_at_once_and_needs_input() {
    let mut world = World::headless(710, b"What can you do?");
    assert!(world.drive_to_park(400));
    assert_eq!(world.exit(), Some(ExitStatus::NeedsInput));
    assert_eq!(world.saved_turns(), 1);
    assert_eq!(world.completions(), 1, "zero waiting admits no follow-up completion");
}

#[test]
fn a_headless_prompt_opens_the_run_and_its_answer_ends_the_host() {
    let mut world = World::headless_workspace(711, b"Change the answer in work");
    world.line(b"This follow-up must not be relayed");
    world.drive(400);
    assert_eq!(world.exit(), Some(ExitStatus::Accepted));
    assert_eq!(world.activation(), 1);
    let prompt = String::from_utf8_lossy(&world.prompt_texts()[0]);
    assert!(prompt.contains("person") && prompt.contains("Change the answer in work"));
    assert!(
        !world
            .prompt_texts()
            .iter()
            .any(|text| text.windows(b"This follow-up".len()).any(|bytes| bytes == b"This follow-up"))
    );
}

#[test]
fn a_headless_run_out_of_turns_ends_with_the_budget_status() {
    let mut world = World::headless_budget(712, b"Change the answer in work");
    world.drive(500);
    assert_eq!(world.exit(), Some(ExitStatus::Budget));
}

#[test]
fn a_headless_run_whose_model_fails_ends_run_failed() {
    let mut world = World::headless(713, b"What can you do?");
    world.fail_model();
    world.drive(500);
    assert_eq!(world.exit(), Some(ExitStatus::RunFailed));
}

#[test]
fn a_headless_run_without_a_credential_starts_nothing() {
    let mut world = World::headless(714, b"What can you do?");
    world.without_credential();
    world.drive(100);
    assert_eq!(world.exit(), Some(ExitStatus::Credentials { account: 0 }));
    assert_eq!(world.completions(), 0);
    assert_eq!(world.credential_requests(), 1);
}

#[test]
fn an_interrupt_ends_a_headless_run_interrupted() {
    let mut world = World::headless(715, b"What can you do?");
    world.interrupt();
    world.drive(100);
    assert_eq!(world.exit(), Some(ExitStatus::Interrupted));
    assert_eq!(world.completions(), 0);
}

#[test]
fn a_headless_delivery_commits_in_place() {
    let mut world = World::headless_delivery(716, b"Make the answer 43");
    world.drive(500);
    assert_eq!(world.exit(), Some(ExitStatus::Accepted));
    assert_eq!(world.delivery_commits(), 1);
    assert!(world.commit_message().is_some_and(|message| message.starts_with(b"Make the answer 43")));
    let (content, _) = world.disk().load(1, b"src/lib.rs", 4096).expect("committed source");
    assert_eq!(content, b"pub fn answer() -> u32 { 43 }\n");
}

#[test]
fn a_budget_failure_is_shown_naming_what_ran_out() {
    let mut world = World::headless_budget(47, b"Update the source");
    world.drive_to_exit(300);
    assert!(world.shown().iter().any(|text| text.as_ref() == b"run failed: out of turns: used 3 of 3\n"));
}

#[test]
fn a_provider_refusal_is_shown_with_its_class_and_text() {
    let mut world = World::new(48);
    world.refuse_first_model();
    world.line(b"Hello");
    world.drive(300);
    let shown = world.shown().iter().find(|text| text.starts_with(b"run failed: ")).expect("failure line");
    assert!(shown.windows(18).any(|part| part == b"invalid (HTTP 400)"));
    assert!(shown.ends_with(b"after 1 attempts: model does not exist\n"));
    assert!(!shown.windows(11).any(|part| part == b"second line"));
}
