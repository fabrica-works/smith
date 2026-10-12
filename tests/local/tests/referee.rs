//! Deliberately bad boundary histories must fail the local-world referee.

use skein_lib::{Time, Token};
use skein_world::domain::{Referee, Verdict};
use smith_domain::run::CallName;
use smith_local_world::referee::{Meeting, Seen};

fn observe(referee: &mut Referee<Meeting>, seen: Seen) {
    referee.observe(Time::ZERO, seen, &mut Vec::new());
}

fn broken(history: Vec<Seen>, reason: &str) {
    broken_with(Meeting::default(), history, reason);
}

fn broken_with(meeting: Meeting, mut history: Vec<Seen>, reason: &str) {
    let mut referee = Referee::new(meeting);
    for seen in history.drain(..) {
        observe(&mut referee, seen);
    }
    let Verdict::Failed(failure) = referee.verdict() else { panic!("bad history must fail") };
    assert_eq!(failure.why, reason);
}

#[test]
fn a_delivery_returned_before_its_record_is_durable_is_rejected() {
    let name = CallName { activation: 1, completion: 1, position: 0 };
    broken(
        vec![Seen::DeliveryRecorded { name, intent: false, receipts: Vec::new() }, Seen::DeliveryReturned { name }],
        "delivery is recorded before the child hears it",
    );
}

#[test]
fn two_decisions_for_one_delivery_name_are_rejected() {
    let name = CallName { activation: 1, completion: 1, position: 0 };
    broken(
        vec![
            Seen::DeliveryRecorded { name, intent: false, receipts: Vec::new() },
            Seen::DeliveryRecorded { name, intent: false, receipts: Vec::new() },
        ],
        "one durable decision per delivery name",
    );
}

#[test]
fn a_named_commit_missing_from_the_saved_answer_is_rejected() {
    let name = CallName { activation: 1, completion: 1, position: 0 };
    broken_with(
        Meeting::default().writable(&[0], &[1]),
        vec![
            Seen::DeliveryRecorded { name, intent: true, receipts: Vec::new() },
            Seen::DeliverySaved { name, intent: true },
            Seen::Committed { name, directory: 0, tree: std::collections::BTreeMap::default() },
            Seen::DeliveryRecorded { name, intent: false, receipts: Vec::new() },
        ],
        "every named commit is in the saved delivery answer",
    );
}

#[test]
fn a_delivered_target_without_its_remote_commit_is_rejected() {
    broken(
        vec![Seen::DeliveredTarget { remote_matches: false }],
        "delivered target names its commit on the fake remote",
    );
}

#[test]
fn a_commit_of_a_tree_changed_after_checks_is_rejected() {
    let checked = std::collections::BTreeMap::from([(b"answer".to_vec(), b"43".to_vec())]);
    let changed = std::collections::BTreeMap::from([(b"answer".to_vec(), b"44".to_vec())]);
    broken_with(
        Meeting::default().writable(&[0], &[1]),
        vec![
            Seen::DeliveryRecorded {
                name: CallName { activation: 1, completion: 1, position: 0 },
                intent: true,
                receipts: Vec::new(),
            },
            Seen::DeliverySaved { name: CallName { activation: 1, completion: 1, position: 0 }, intent: true },
            Seen::Checked { directory: 0, tree: checked },
            Seen::Committed {
                name: CallName { activation: 1, completion: 1, position: 0 },
                directory: 0,
                tree: changed,
            },
        ],
        "delivery commits exactly the checked tree",
    );
}

#[test]
fn a_write_outside_the_writable_roots_is_rejected() {
    broken_with(
        Meeting::default().writable(&[0], &[1]),
        vec![Seen::Wrote { root: 2 }],
        "agent writes only writable roots",
    );
}

#[test]
fn a_reused_activation_is_rejected() {
    broken(vec![Seen::Activation { number: 0, message: 1 }], "activations strictly increase across invocations");
}

#[test]
fn an_out_of_order_turn_save_is_rejected() {
    broken(
        vec![
            Seen::Activation { number: 1, message: 1 },
            Seen::Turn { number: 1 },
            Seen::SaveTurn { number: 1, read: None },
            Seen::Turn { number: 2 },
            Seen::SaveTurn { number: 2, read: None },
            Seen::TurnSaved { number: 2 },
        ],
        "turns become durable in order",
    );
}

#[test]
fn an_answer_shown_before_its_turn_is_durable_is_rejected() {
    broken(
        vec![
            Seen::Activation { number: 1, message: 1 },
            Seen::Turn { number: 1 },
            Seen::SaveTurn { number: 1, read: None },
            Seen::Answered { activation: 1, read: None },
            Seen::Shown { activation: 1 },
        ],
        "the answer is shown after all turns are saved",
    );
}

#[test]
fn a_read_fence_without_a_relayed_line_is_rejected() {
    broken(
        vec![
            Seen::Activation { number: 1, message: 1 },
            Seen::Turn { number: 1 },
            Seen::SaveTurn { number: 1, read: Some(Token::new(1)) },
        ],
        "a read fence names a relayed person line",
    );
}

#[test]
fn a_duplicate_provider_terminal_is_rejected() {
    broken(
        vec![
            Seen::Complete { owner: Token::new(2) },
            Seen::Completed { owner: Token::new(2) },
            Seen::Completed { owner: Token::new(2) },
        ],
        "one terminal per completion request",
    );
}

#[test]
fn a_duplicate_answer_is_rejected() {
    broken(
        vec![
            Seen::Activation { number: 1, message: 1 },
            Seen::Answered { activation: 1, read: None },
            Seen::Answered { activation: 1, read: None },
        ],
        "one answer per run",
    );
}

#[test]
fn a_turn_saved_after_the_result_was_shown_is_rejected() {
    broken(
        vec![
            Seen::Activation { number: 1, message: 1 },
            Seen::Answered { activation: 1, read: None },
            Seen::Shown { activation: 1 },
            Seen::Turn { number: 1 },
        ],
        "nothing is told after the shown answer",
    );
}

#[test]
fn a_line_relayed_twice_in_one_activation_is_rejected() {
    broken(
        vec![
            Seen::Activation { number: 1, message: 1 },
            Seen::Message { name: Token::new(1) },
            Seen::Message { name: Token::new(1) },
        ],
        "a line is relayed at most once in an activation",
    );
}

#[test]
fn a_read_line_relayed_again_is_rejected() {
    broken(
        vec![
            Seen::Activation { number: 1, message: 1 },
            Seen::Message { name: Token::new(1) },
            Seen::Turn { number: 1 },
            Seen::SaveTurn { number: 1, read: Some(Token::new(1)) },
            Seen::TurnSaved { number: 1 },
            Seen::Activation { number: 2, message: 1 },
            Seen::Message { name: Token::new(1) },
        ],
        "a read line is never relayed again",
    );
}

#[test]
fn a_fence_that_goes_back_is_rejected() {
    broken(
        vec![
            Seen::Activation { number: 1, message: 2 },
            Seen::Message { name: Token::new(1) },
            Seen::Message { name: Token::new(2) },
            Seen::Turn { number: 1 },
            Seen::SaveTurn { number: 1, read: Some(Token::new(2)) },
            Seen::TurnSaved { number: 1 },
            Seen::Turn { number: 2 },
            Seen::SaveTurn { number: 2, read: Some(Token::new(1)) },
        ],
        "the read fence never moves backwards",
    );
}

#[test]
fn an_unread_line_can_be_relayed_in_the_next_activation() {
    let mut referee = Referee::new(Meeting::default());
    for seen in [
        Seen::Activation { number: 1, message: 1 },
        Seen::Message { name: Token::new(1) },
        Seen::Answered { activation: 1, read: None },
        Seen::Shown { activation: 1 },
        Seen::Activation { number: 2, message: 1 },
        Seen::Message { name: Token::new(1) },
    ] {
        observe(&mut referee, seen);
    }
    assert!(!matches!(referee.verdict(), Verdict::Failed(_)));
}

#[test]
fn a_duplicate_unread_report_is_rejected() {
    let name = Token::new(1);
    broken(
        vec![
            Seen::Activation { number: 1, message: 1 },
            Seen::Message { name },
            Seen::Unread { name },
            Seen::Unread { name },
        ],
        "the unread report comes once",
    );
}

#[test]
fn a_named_line_with_no_terminal_is_rejected_when_its_deadline_expires() {
    let mut referee = Referee::new(Meeting::default());
    observe(&mut referee, Seen::Activation { number: 1, message: 1 });
    referee.fire(Time::from_nanos(skein_lib::Duration::from_secs(3601).as_nanos()), &mut Vec::new());
    assert!(matches!(referee.verdict(), Verdict::Failed(_)));
}

#[test]
fn a_waiting_notice_while_a_relayed_line_is_unread_is_rejected() {
    broken(
        vec![Seen::Activation { number: 1, message: 1 }, Seen::Message { name: Token::new(1) }, Seen::Waiting],
        "waiting is shown only when every relayed line is read",
    );
}

#[test]
fn no_failure_words_enter_a_turn_or_a_prompt() {
    let mut world = smith_local_world::World::new(49);
    world.refuse_first_model();
    world.line(b"Hello");
    world.drive(300);
    assert!(!world.diagnostic_in_history(b"model does not exist"));
    assert!(!world.diagnostic_in_history(b"run failed:"));
}
