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
fn selecting_a_new_chat_makes_a_private_directory_and_keeps_its_record_layout() {
    let mut store = store();
    let mut out = Queue::with_capacity(3);
    check_empty(&mut store, &mut out);
    store.request(StoreRequest::SelectChat { name: b"new".as_slice().into(), create: true }, Time::ZERO, &mut out);
    assert!(!store.takes());
    let file::Request::MakeDirectory { root, path, mode, .. } = io(&mut out) else { panic!("private directory") };
    assert_eq!(root, Token::new(8));
    assert_eq!(path.as_ref(), b"new");
    assert_eq!(mode, 0o700);
    store.from_io(file::Event::Made { owner: OWNER }, Time::ZERO, &mut out);
    let file::Request::OpenDirectory { root, name, no_follow, .. } = io(&mut out) else { panic!("selected directory") };
    assert_eq!(root, kernel::Fd::new(9));
    assert_eq!(name.as_ref(), b"new");
    assert!(no_follow);
    store.opened_directory_descriptor(kernel::Fd::new(19));
    store.from_io(file::Event::Opened { owner: OWNER, file: Token::new(10), len: 0 }, Time::ZERO, &mut out);
    let Some(Action::Domain(local::Event::ChatSelected { name, result: Ok(()) })) = out.pop() else {
        panic!("selection")
    };
    assert_eq!(name.as_ref(), b"new");
    assert!(store.takes());
    check_empty(&mut store, &mut out);
    store.request(save(), Time::ZERO, &mut out);
    let file::Request::Store { root, path, .. } = io(&mut out) else { panic!("same metadata layout") };
    assert_eq!(root, Token::new(10));
    assert_eq!(path.as_ref(), b"state");
}

#[test]
fn resuming_an_absent_chat_names_it_and_keeps_the_current_store() {
    let mut store = store();
    let mut out = Queue::with_capacity(3);
    check_empty(&mut store, &mut out);
    store.request(save(), Time::ZERO, &mut out);
    io(&mut out);
    store.from_io(file::Event::Stored { owner: OWNER, digest: digest(b"old state") }, Time::ZERO, &mut out);
    out.pop();
    store.request(StoreRequest::SelectChat { name: b"absent".as_slice().into(), create: false }, Time::ZERO, &mut out);
    let file::Request::OpenDirectory { name, .. } = io(&mut out) else { panic!("resume only opens") };
    assert_eq!(name.as_ref(), b"absent");
    store.from_io(
        file::Event::Failed { owner: OWNER, error: kernel::Error::NotFound, committed: false, residue: None },
        Time::ZERO,
        &mut out,
    );
    let Some(Action::Domain(local::Event::ChatSelected { name, result: Err(local::ChatRefusal::Missing) })) = out.pop()
    else {
        panic!("named missing chat")
    };
    assert_eq!(name.as_ref(), b"absent");
    store.request(save(), Time::ZERO, &mut out);
    let file::Request::Store { root, expected, .. } = io(&mut out) else { panic!("old store still selected") };
    assert_eq!(root, Token::new(8));
    assert_eq!(expected, file::Expect::Digest(digest(b"old state")));
}

#[test]
fn bad_chat_names_and_ephemeral_selection_open_nothing() {
    for name in [b"../other".as_slice(), b".", b"", b"a/b"] {
        let mut store = store();
        let mut out = Queue::with_capacity(3);
        check_empty(&mut store, &mut out);
        store.request(StoreRequest::SelectChat { name: name.into(), create: false }, Time::ZERO, &mut out);
        let Some(Action::Domain(local::Event::ChatSelected { name: refused, result: Err(local::ChatRefusal::Name) })) =
            out.pop()
        else {
            panic!("bad name")
        };
        assert_eq!(refused.as_ref(), name);
        assert!(out.is_empty());
        assert!(store.takes());
    }
    let mut store = store();
    store.ephemeral(Token::new(11), kernel::Fd::new(12), b"private".as_slice().into());
    let mut out = Queue::with_capacity(3);
    check_empty(&mut store, &mut out);
    store.request(StoreRequest::SelectChat { name: b"other".as_slice().into(), create: true }, Time::ZERO, &mut out);
    let Some(Action::Domain(local::Event::ChatSelected { result: Err(local::ChatRefusal::Ephemeral), .. })) = out.pop()
    else {
        panic!("typed terminal")
    };
    assert!(out.is_empty());
}

#[test]
fn selecting_another_chat_retires_the_previous_root_before_loading_and_closes_both_roots() {
    let (mut store, mut out) = selected_first_chat();
    store.request(StoreRequest::SelectChat { name: b"second".as_slice().into(), create: false }, Time::ZERO, &mut out);
    let file::Request::OpenDirectory { root, .. } = io(&mut out) else { panic!("workspace chats root") };
    assert_eq!(root, kernel::Fd::new(9));
    store.opened_directory_descriptor(kernel::Fd::new(29));
    store.from_io(file::Event::Opened { owner: OWNER, file: Token::new(20), len: 0 }, Time::ZERO, &mut out);
    closed_root(&mut out, Token::new(10));
    assert!(!store.takes());
    store.from_io(file::Event::Closed { owner: OWNER }, Time::ZERO, &mut out);
    let Some(Action::Domain(local::Event::ChatSelected { result: Ok(()), .. })) = out.pop() else {
        panic!("typed terminal")
    };
    check_empty(&mut store, &mut out);
    store.request(save(), Time::ZERO, &mut out);
    let file::Request::Store { root, expected, .. } = io(&mut out) else {
        panic!("new chat starts without old versions")
    };
    assert_eq!(root, Token::new(20));
    assert_eq!(expected, file::Expect::Absent);
    store.from_io(file::Event::Stored { owner: OWNER, digest: digest(b"second state") }, Time::ZERO, &mut out);
    out.pop().unwrap();
    store.close(Time::ZERO, &mut out);
    closed_root(&mut out, Token::new(20));
    store.from_io(file::Event::Closed { owner: OWNER }, Time::ZERO, &mut out);
    closed_root(&mut out, Token::new(8));
    store.from_io(file::Event::Closed { owner: OWNER }, Time::ZERO, &mut out);
    let Some(Action::Closed) = out.pop() else { panic!("typed terminal") };
    assert!(out.is_empty());
}
#[test]
fn a_turn_file_is_stored_expecting_its_name_absent() {
    let mut store = store();
    let mut out = Queue::with_capacity(3);
    check_empty(&mut store, &mut out);
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
    check_empty(&mut store, &mut out);
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
    check_empty(&mut store, &mut out);
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
    check_empty(&mut store, &mut out);
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
    check_empty(&mut store, &mut out);
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
    check_empty(&mut store, &mut out);
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
    check_empty(&mut store, &mut out);
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
    check_empty(&mut store, &mut out);
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
        kind: smith_domain::session::record::Kind::Window(Box::new(smith_domain::session::record::Opener {
            affinity: smith_domain::llm::Affinity { key: [0; 16], thread: 0 },
            account: None,
            backend: smith_domain::session::record::Backend::Direct,
        })),
        ..turn.clone()
    }
}

fn selected_first_chat() -> (Store, Queue<Action>) {
    let mut store = store();
    let mut out = Queue::with_capacity(3);
    check_empty(&mut store, &mut out);
    store.request(StoreRequest::SelectChat { name: b"first".as_slice().into(), create: false }, Time::ZERO, &mut out);
    io(&mut out);
    store.opened_directory_descriptor(kernel::Fd::new(19));
    store.from_io(file::Event::Opened { owner: OWNER, file: Token::new(10), len: 0 }, Time::ZERO, &mut out);
    out.pop().unwrap();
    check_empty(&mut store, &mut out);
    store.request(save(), Time::ZERO, &mut out);
    io(&mut out);
    store.from_io(file::Event::Stored { owner: OWNER, digest: digest(b"first state") }, Time::ZERO, &mut out);
    out.pop().unwrap();
    (store, out)
}

fn closed_root(out: &mut Queue<Action>, expected: Token) {
    let file::Request::Close { file, .. } = io(out) else { panic!("close request") };
    assert_eq!(file, expected);
}

fn check_empty(store: &mut Store, out: &mut Queue<Action>) {
    store.request(StoreRequest::Load, Time::ZERO, out);
    if let Some(Action::Domain(local::Event::Loaded { .. })) = out.iter().next() {
        out.pop().unwrap();
        return;
    }
    let file::Request::Load { path, .. } = io(out) else {
        panic!("marker first");
    };
    assert_eq!(path.as_ref(), b"format");
    store.from_io(
        file::Event::Failed { owner: OWNER, error: kernel::Error::NotFound, committed: false, residue: None },
        Time::ZERO,
        out,
    );
    let file::Request::Scan { .. } = io(out) else {
        panic!("expected Store terminal");
    };
    store.from_io(file::Event::Scanned { owner: OWNER, entries: Box::new([]), more: 0 }, Time::ZERO, out);
    let file::Request::Store { path, expected: file::Expect::Absent, .. } = io(out) else {
        panic!("no-replace marker");
    };
    assert_eq!(path.as_ref(), b"format");
    store.from_io(file::Event::Stored { owner: OWNER, digest: digest(&crate::format::file().bytes) }, Time::ZERO, out);
    let Some(Action::Domain(local::Event::Loaded { state: None, .. })) = out.pop() else {
        panic!("expected Store terminal");
    };
    assert!(out.is_empty());
}

#[test]
#[should_panic(expected = "Load must check the selected root before any save")]
fn saving_before_load_is_a_contract_violation() {
    store().request(save(), Time::ZERO, &mut Queue::with_capacity(3));
}

#[test]
#[should_panic(expected = "Load must check the selected root before any save")]
fn saving_a_turn_before_load_is_a_contract_violation() {
    let (_, turn) = crate::store::tests::fixture();
    store().request(
        StoreRequest::SaveTurn { number: 1, read: None, opens_window: true, turn },
        Time::ZERO,
        &mut Queue::with_capacity(3),
    );
}

#[test]
#[should_panic(expected = "Load must check the selected root before any save")]
fn saving_a_delivery_before_load_is_a_contract_violation() {
    let record = local::DeliveryRecord {
        name: smith_domain::run::CallName { activation: 1, completion: 1, position: 0 },
        state: local::DeliveryState::Answer(smith_domain::run::Delivery::Nothing),
        landed: Box::new([]),
    };
    store().request(StoreRequest::SaveDelivery { record: Box::new(record) }, Time::ZERO, &mut Queue::with_capacity(3));
}

#[test]
fn marker_creation_conflicts_and_failures_have_one_read_terminal() {
    for conflict in [false, true] {
        let mut store = store();
        let mut out = Queue::with_capacity(3);
        store.request(StoreRequest::Load, Time::ZERO, &mut out);
        io(&mut out);
        store.from_io(
            file::Event::Failed { owner: OWNER, error: kernel::Error::NotFound, committed: false, residue: None },
            Time::ZERO,
            &mut out,
        );
        io(&mut out);
        store.from_io(file::Event::Scanned { owner: OWNER, entries: Box::new([]), more: 0 }, Time::ZERO, &mut out);
        let file::Request::Store { expected: file::Expect::Absent, .. } = io(&mut out) else {
            panic!("expected Store terminal");
        };
        let event =
            if conflict { file::Event::Conflict { owner: OWNER, now: Some(digest(b"winner")) } } else { failed(false) };
        store.from_io(event, Time::ZERO, &mut out);
        let Some(Action::Fault(_)) = out.pop() else {
            panic!("expected Store terminal");
        };
        let Some(Action::Domain(local::Event::StoreFailed { reason: local::StoreFailure::Read, turn: None })) =
            out.pop()
        else {
            panic!("expected Store terminal");
        };
        assert!(out.is_empty());
    }
}

#[test]
fn an_unknown_or_failed_absence_scan_refuses_as_unrecorded_without_a_write() {
    for event in [
        file::Event::Scanned {
            owner: OWNER,
            entries: Box::new([file::Entry {
                name: b"format.skein-0123456789abcdef".as_slice().into(),
                kind: kernel::Kind::Other,
            }]),
            more: 0,
        },
        file::Event::Failed { owner: OWNER, error: kernel::Error::Permission, committed: false, residue: None },
        file::Event::TooLarge { owner: OWNER, size: 4097 },
    ] {
        let mut store = store();
        let mut out = Queue::with_capacity(3);
        store.request(StoreRequest::Load, Time::ZERO, &mut out);
        io(&mut out);
        store.from_io(
            file::Event::Failed { owner: OWNER, error: kernel::Error::NotFound, committed: false, residue: None },
            Time::ZERO,
            &mut out,
        );
        let file::Request::Scan { .. } = io(&mut out) else {
            panic!("expected Store terminal");
        };
        store.from_io(event, Time::ZERO, &mut out);
        let Some(Action::Domain(local::Event::StoreFailed {
            reason: local::StoreFailure::OlderBuild { why: local::Older::Unrecorded },
            ..
        })) = out.pop()
        else {
            panic!("expected Store terminal");
        };
        assert!(out.is_empty());
    }
}

#[test]
fn a_failed_previous_root_close_leaves_the_opened_candidate_unchecked() {
    let (mut store, mut out) = selected_first_chat();
    store.request(
        StoreRequest::SelectChat { name: b"candidate".as_slice().into(), create: false },
        Time::ZERO,
        &mut out,
    );
    let file::Request::OpenDirectory { .. } = io(&mut out) else {
        panic!("expected Store terminal");
    };
    store.opened_directory_descriptor(kernel::Fd::new(29));
    store.from_io(file::Event::Opened { owner: OWNER, file: Token::new(20), len: 0 }, Time::ZERO, &mut out);
    closed_root(&mut out, Token::new(10));
    store.from_io(failed(false), Time::ZERO, &mut out);
    let Some(Action::Domain(local::Event::StoreFailed { .. })) = out.pop() else {
        panic!("expected Store terminal");
    };
    let Some(Action::Domain(local::Event::ChatSelected { result: Err(local::ChatRefusal::Io), .. })) = out.pop() else {
        panic!("expected Store terminal");
    };
    assert!(out.is_empty());
    store.request(StoreRequest::Load, Time::ZERO, &mut out);
    let file::Request::Load { root, path, .. } = io(&mut out) else {
        panic!("candidate compatibility check");
    };
    assert_eq!(root, Token::new(20));
    assert_eq!(path.as_ref(), b"format");
}
