//! Guide reads use the space left after admitting the charter's rendered material.

use super::{
    Answer, Charter, Event, Harness, Invalid, LIMITS, Limits, Read, Refusal, Request, Workspace, charter,
    workspace_directory,
};
use alloc::boxed::Box;

fn opening() -> Charter {
    let mut selected = charter();
    selected.grants.host_tools = Box::default();
    selected
}

fn workspace() -> Workspace {
    Workspace {
        directories: Box::new([
            workspace_directory(b"left", 900, false, false, &[]),
            workspace_directory(b"right", 901, false, false, &[]),
        ]),
    }
}

fn host_part(selected: &Charter, mounted: &Workspace, guide: u32) -> u64 {
    crate::prompt::host_bytes(selected, Some(mounted), guide)
        .expect("rendered host")
        .checked_add(LIMITS.schema_costs.finish(&selected.outcome).expect("finish schema"))
        .expect("small schema and host")
        .checked_add(LIMITS.schema_costs.finish_text)
        .expect("small schema text")
}

#[test]
fn each_guide_is_read_and_retained_within_its_share_of_the_host_part() {
    let selected = opening();
    let mounted = workspace();
    let host_part = host_part(&selected, &mounted, 16);
    let mut harness = Harness::new(Limits { host_part, ..LIMITS });
    let requests = harness.start_workspace(1, selected, Some(mounted));
    let [Request::Admitted { .. }, Request::Read { owner, max, .. }] = requests.as_ref() else {
        panic!("admitted guide read: {requests:?}")
    };
    assert_eq!(*max, 16);
    let owner = *owner;
    let requests = harness.step(Event::Read { owner, read: Read::Text { text: Box::new([b'g'; 16]), whole: false } });
    let [Request::Read { max, .. }] = requests.as_ref() else { panic!("next directory's guide") };
    assert_eq!(*max, 16);
    let requests = harness.step(Event::Read { owner, read: Read::Text { text: Box::new([b'g'; 16]), whole: false } });
    let [Request::Open { opening, .. }] = requests.as_ref() else { panic!("prepared main") };
    let mut guides = 0_u32;
    for piece in opening.system.windows(16) {
        if piece == [b'g'; 16] {
            guides = guides.checked_add(1).expect("two guide payloads");
        }
    }
    assert_eq!(guides, 2);
}

#[test]
fn a_charter_that_leaves_no_guide_bytes_is_admitted() {
    let selected = opening();
    let mounted = workspace();
    let host_part = host_part(&selected, &mounted, 0);
    let mut harness = Harness::new(Limits { host_part, ..LIMITS });
    let requests = harness.start_workspace(1, selected, Some(mounted));
    let [Request::Admitted { .. }, Request::Read { max, .. }] = requests.as_ref() else {
        panic!("admitted without guide bytes: {requests:?}")
    };
    assert_eq!(*max, 0);
}

#[test]
fn a_charter_that_cannot_fit_without_guide_bytes_is_refused_before_io() {
    let selected = opening();
    let mounted = workspace();
    let host_part = host_part(&selected, &mounted, 0) - 1;
    let mut harness = Harness::new(Limits { host_part, ..LIMITS });
    let requests = harness.start_workspace(1, selected, Some(mounted));
    let [Request::Answer { answer: Answer::Refused(Refusal::Invalid(Invalid::HostPart { bound })), .. }] =
        requests.as_ref()
    else {
        panic!("host part refused before preparation: {requests:?}")
    };
    assert_eq!(*bound, host_part);
}
