use crate::{Action, History, RecordName, Store, StoreFault, StoreLimits, StoreRequest};
use skein_io::{digest::digest, file, kernel};
use skein_lib::{Queue, Time, Token};
use smith_local_domain as local;

const OWNER: Token = Token::new(7);
fn store() -> Store {
    Store::new(
        OWNER,
        Token::new(8),
        kernel::Fd::new(9),
        StoreLimits { file_bytes: 4096, turns: 2, deliveries: 2, scan_bytes: 4096 },
        crate::store::tests::fixture().0,
    )
    .unwrap()
}
fn state() -> local::ChatState {
    local::ChatState {
        identity: smith_domain::run::Identity::new([42; 16]),
        activation: 1,
        next_message: 1,
        read: None,
    }
}
fn save() -> StoreRequest {
    StoreRequest::SaveState { state: state(), history: History::Kept }
}
fn io(out: &mut Queue<Action>) -> file::Request {
    let Action::Io { request, .. } = out.pop().unwrap() else { panic!("file request") };
    request
}
fn failed(committed: bool) -> file::Event {
    file::Event::Failed { owner: OWNER, error: kernel::Error::Other(5), committed, residue: None }
}
#[test]
fn a_turn_file_is_stored_expecting_its_name_absent() {
    let mut store = store();
    let mut out = Queue::with_capacity(3);
    let (_, turn) = crate::store::tests::fixture();
    store.request(StoreRequest::SaveTurn { number: 1, read: None, opens_window: true, turn }, Time::ZERO, &mut out);
    let file::Request::Store { expected: file::Expect::Absent, no_follow: true, .. } = io(&mut out) else {
        panic!("expected store terminal")
    };
}
#[test]
fn a_second_store_of_a_turn_still_expects_absence() {
    let mut store = store();
    let mut out = Queue::with_capacity(3);
    let (_, turn) = crate::store::tests::fixture();
    store.request(
        StoreRequest::SaveTurn { number: 1, read: None, opens_window: true, turn: turn.clone() },
        Time::ZERO,
        &mut out,
    );
    io(&mut out);
    store.from_io(file::Event::Stored { owner: OWNER, digest: digest(b"turn") }, Time::ZERO, &mut out);
    let Some(Action::Domain(local::Event::TurnSaved { number: 1 })) = out.pop() else { panic!("turn terminal") };
    store.request(StoreRequest::SaveTurn { number: 1, read: None, opens_window: true, turn }, Time::ZERO, &mut out);
    let file::Request::Store { expected: file::Expect::Absent, .. } = io(&mut out) else {
        panic!("a turn always expects its file absent")
    };
}

#[test]
fn a_store_of_unknown_durability_is_sent_again_once() {
    let mut store = store();
    let mut out = Queue::with_capacity(3);
    store.request(save(), Time::ZERO, &mut out);
    let file::Request::Store { bytes, .. } = io(&mut out) else { panic!("store") };
    store.from_io(failed(true), Time::ZERO, &mut out);
    let file::Request::Store { expected: file::Expect::Digest(version), .. } = io(&mut out) else {
        panic!("expected store terminal")
    };
    assert_eq!(version, digest(&bytes));
    store.from_io(failed(true), Time::ZERO, &mut out);
    let Some(Action::Fault(StoreFault::NotDurable { file: RecordName::State })) = out.pop() else {
        panic!("expected store terminal")
    };
    let Some(Action::Domain(local::Event::StoreFailed { reason: local::StoreFailure::Write, .. })) = out.pop() else {
        panic!("expected store terminal")
    };
    assert!(out.is_empty());
}
#[test]
fn an_empty_metadata_file_is_malformed_rather_than_an_absent_chat() {
    let mut store = store();
    let mut out = Queue::with_capacity(3);
    store.request(StoreRequest::Load, Time::ZERO, &mut out);
    io(&mut out);
    store.from_io(file::Event::Loaded { owner: OWNER, bytes: Box::new([]) }, Time::ZERO, &mut out);
    let Some(Action::Fault(StoreFault::Malformed { file: RecordName::Journal })) = out.pop() else {
        panic!("expected store terminal")
    };
    let Some(Action::Domain(local::Event::StoreFailed { reason: local::StoreFailure::Read, .. })) = out.pop() else {
        panic!("expected store terminal")
    };
}
#[test]
fn a_changed_metadata_version_fails_without_overwriting_it() {
    let mut store = store();
    let mut out = Queue::with_capacity(3);
    store.request(save(), Time::ZERO, &mut out);
    io(&mut out);
    store.from_io(file::Event::Conflict { owner: OWNER, now: Some(digest(b"other")) }, Time::ZERO, &mut out);
    let Some(Action::Fault(StoreFault::Conflict { file: RecordName::State })) = out.pop() else {
        panic!("expected store terminal")
    };
    let Some(Action::Domain(local::Event::StoreFailed { .. })) = out.pop() else { panic!("expected store terminal") };
    assert!(out.is_empty());
}
#[test]
fn a_close_failure_ends_the_close_without_an_extra_domain_terminal() {
    let mut store = store();
    let mut out = Queue::with_capacity(3);
    store.close(Time::ZERO, &mut out);
    io(&mut out);
    store.from_io(failed(false), Time::ZERO, &mut out);
    let Some(Action::Fault(_)) = out.pop() else { panic!("expected store terminal") };
    let Some(Action::Closed) = out.pop() else { panic!("expected store terminal") };
    assert!(out.is_empty());
}
#[test]
fn a_failure_while_closing_reserves_all_three_outputs() {
    let mut store = store();
    let mut out = Queue::with_capacity(3);
    store.request(save(), Time::ZERO, &mut out);
    io(&mut out);
    store.close(Time::ZERO, &mut out);
    store.from_io(failed(false), Time::ZERO, &mut out);
    let Some(Action::Fault(_)) = out.pop() else { panic!("expected store terminal") };
    let Some(Action::Domain(local::Event::StoreFailed { .. })) = out.pop() else { panic!("expected store terminal") };
    let file::Request::Close { .. } = io(&mut out) else { panic!("expected store terminal") };
}
#[test]
fn a_large_delivery_is_refused_before_io() {
    let mut store = store();
    let mut out = Queue::with_capacity(3);
    let name = smith_domain::run::CallName { activation: 1, completion: 1, position: 0 };
    let record = local::DeliveryRecord {
        name,
        state: local::DeliveryState::Intent(local::DeliveryIntent {
            directories: Box::new([local::IntentDirectory {
                directory: 0,
                changed: true,
                head: Some(Box::new([b'x'; 5000])),
                push: None,
            }]),
        }),
        landed: Box::new([]),
    };
    store.request(StoreRequest::SaveDelivery { record: Box::new(record) }, Time::ZERO, &mut out);
    let Some(Action::Fault(_)) = out.pop() else { panic!("expected store terminal") };
    let Some(Action::Domain(local::Event::StoreFailed { .. })) = out.pop() else { panic!("expected store terminal") };
    assert!(out.is_empty());
}

fn private() -> Store {
    let mut store = store();
    store.ephemeral(Token::new(12), kernel::Fd::new(13), b"private".as_slice().into());
    store
}

fn known_files(store: &mut Store, out: &mut Queue<Action>) {
    store.request(save(), Time::ZERO, out);
    io(out);
    store.from_io(file::Event::Stored { owner: OWNER, digest: digest(b"state") }, Time::ZERO, out);
    let Some(Action::Domain(local::Event::StateSaved)) = out.pop() else { panic!("saved private state") };
    let (_, turn) = crate::store::tests::fixture();
    store.request(StoreRequest::SaveTurn { number: 1, read: None, opens_window: true, turn }, Time::ZERO, out);
    io(out);
    store.from_io(file::Event::Stored { owner: OWNER, digest: digest(b"turn") }, Time::ZERO, out);
    let Some(Action::Domain(local::Event::TurnSaved { number: 1 })) = out.pop() else { panic!("saved private turn") };
}

#[test]
fn a_private_chat_starts_empty_without_listing_or_loading_a_record() {
    let mut store = private();
    let mut out = Queue::with_capacity(3);
    store.request(StoreRequest::Load, Time::ZERO, &mut out);
    let Some(Action::Domain(local::Event::Loaded { state: None, transcript: None, deliveries })) = out.pop() else {
        panic!("empty private chat")
    };
    assert!(deliveries.is_empty());
    assert!(out.is_empty());
    store.request(StoreRequest::SaveState { state: state(), history: History::Fresh }, Time::ZERO, &mut out);
    let file::Request::Store { path, .. } = io(&mut out) else { panic!("private state write") };
    assert_eq!(path.as_ref(), b"state");
}

#[test]
fn a_private_chat_removes_its_known_names_then_closes_both_roots() {
    let mut store = private();
    let mut out = Queue::with_capacity(3);
    known_files(&mut store, &mut out);
    store.close(Time::ZERO, &mut out);
    for name in [b"0000000001.turn".as_slice(), b"state".as_slice()] {
        expect_remove(&mut out, kernel::Fd::new(9), name, false);
        store.from_io(file::Event::Removed { owner: OWNER }, Time::ZERO, &mut out);
    }
    expect_close(&mut out, Token::new(8));
    store.from_io(file::Event::Closed { owner: OWNER }, Time::ZERO, &mut out);
    expect_remove(&mut out, kernel::Fd::new(13), b"private", true);
    store.from_io(file::Event::Removed { owner: OWNER }, Time::ZERO, &mut out);
    expect_close(&mut out, Token::new(12));
    store.from_io(file::Event::Closed { owner: OWNER }, Time::ZERO, &mut out);
    let Some(Action::Closed) = out.pop() else { panic!("private store closed") };
    assert!(out.is_empty());
}

#[test]
fn a_private_directory_not_empty_warns_without_a_domain_failure() {
    let mut store = private();
    let mut out = Queue::with_capacity(3);
    store.close(Time::ZERO, &mut out);
    io(&mut out);
    store.from_io(file::Event::Closed { owner: OWNER }, Time::ZERO, &mut out);
    expect_remove(&mut out, kernel::Fd::new(13), b"private", true);
    store.from_io(
        file::Event::Failed { owner: OWNER, error: kernel::Error::NotEmpty, committed: false, residue: None },
        Time::ZERO,
        &mut out,
    );
    expect_close(&mut out, Token::new(12));
    store.from_io(file::Event::Closed { owner: OWNER }, Time::ZERO, &mut out);
    expect_warning(&mut out, kernel::Error::NotEmpty);
    let Some(Action::Closed) = out.pop() else { panic!("private store closed") };
    assert!(out.is_empty());
}

#[test]
fn a_private_file_removal_failure_still_attempts_each_other_known_name() {
    let mut store = private();
    let mut out = Queue::with_capacity(3);
    known_files(&mut store, &mut out);
    store.close(Time::ZERO, &mut out);
    io(&mut out);
    store.from_io(failed(false), Time::ZERO, &mut out);
    expect_remove(&mut out, kernel::Fd::new(9), b"state", false);
    store.from_io(file::Event::Removed { owner: OWNER }, Time::ZERO, &mut out);
    io(&mut out);
    store.from_io(file::Event::Closed { owner: OWNER }, Time::ZERO, &mut out);
    expect_close(&mut out, Token::new(12));
    store.from_io(file::Event::Closed { owner: OWNER }, Time::ZERO, &mut out);
    expect_warning(&mut out, kernel::Error::Other(5));
    let Some(Action::Closed) = out.pop() else { panic!("private store closed") };
}

#[test]
fn private_written_names_refuse_beyond_the_stores_record_capacity_before_io() {
    let mut store = private();
    let mut out = Queue::with_capacity(3);
    let (_, turn) = crate::store::tests::fixture();
    for number in 1..=6 {
        let turn = numbered_turn(&turn, number);
        store.request(
            StoreRequest::SaveTurn { number, read: None, opens_window: true, turn: turn.clone() },
            Time::ZERO,
            &mut out,
        );
        io(&mut out);
        store.from_io(file::Event::Stored { owner: OWNER, digest: digest(b"turn") }, Time::ZERO, &mut out);
        out.pop();
    }
    let turn = numbered_turn(&turn, 7);
    store.request(StoreRequest::SaveTurn { number: 7, read: None, opens_window: true, turn }, Time::ZERO, &mut out);
    let Some(Action::Fault(StoreFault::TooLarge { file: RecordName::Turn(7) })) = out.pop() else {
        panic!("known name bound")
    };
    let Some(Action::Domain(local::Event::StoreFailed { .. })) = out.pop() else { panic!("failed bounded store") };
    assert!(out.is_empty());
}

fn expect_remove(out: &mut Queue<Action>, wanted_root: kernel::Fd, wanted_name: &[u8], wanted_directory: bool) {
    let file::Request::Remove { root, name, directory, .. } = io(out) else { panic!("known removal") };
    assert_eq!(root, wanted_root);
    assert_eq!(name.as_ref(), wanted_name);
    assert_eq!(directory, wanted_directory);
}

fn expect_close(out: &mut Queue<Action>, wanted: Token) {
    let file::Request::Close { file, .. } = io(out) else { panic!("owned root close") };
    assert_eq!(file, wanted);
}

fn expect_warning(out: &mut Queue<Action>, wanted: kernel::Error) {
    let Some(Action::CleanupWarning { error }) = out.pop() else { panic!("private cleanup warning") };
    assert_eq!(error, wanted);
}

fn numbered_turn(turn: &smith_domain::Turn, sequence: u32) -> smith_domain::Turn {
    smith_domain::Turn {
        sequence,
        kind: smith_domain::session::record::Kind::Window {
            affinity: smith_domain::llm::Affinity { key: [0; 16], thread: 0 },
        },
        ..turn.clone()
    }
}
