use super::*;
use crate::service::ending::Ending;
use crate::{Service, iterate};
use skein_lib::Wall;

fn loading() -> Service {
    let mut service = service();
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    let Some(smith_local_protocol::StoreRequest::Load) = service.store_requests().pop() else { panic!("load") };
    service
}

fn loaded() -> local::Event {
    local::Event::Loaded { state: None, transcript: None, deliveries: Box::new([]) }
}

#[test]
fn a_line_before_interrupt_never_spawns_its_silent_check() {
    let mut service = loading();
    service.local_event(loaded());
    service.local_event(local::Event::Line { text: b"hello".as_slice().into() });
    service.local_event(local::Event::Interrupt);
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    assert!(service.below.process_mut().programs.as_ref().unwrap().active.is_none(), "no silent check starts");
    let Some(smith_local_protocol::StoreRequest::SaveState { .. }) = service.store_requests().pop() else {
        panic!("save")
    };
    service.local_event(local::Event::StateSaved);
    for _ in 0_u32..8 {
        iterate(&mut service, Time::ZERO, Wall::EPOCH);
        if service.closing() {
            service.shell_closed();
        }
        while let Some(submit) = service.submissions().pop() {
            let kernel::Op::Close { .. } = &submit.kind else { panic!("no child or status process") };
            service.completions().push(kernel::Complete {
                op: submit.op,
                kind: submit.kind,
                result: Ok(kernel::Done::Nothing),
            });
        }
    }
    let Ending::Closed { exit, .. } = service.ending else { panic!("chat settled before five seconds") };
    assert_eq!(exit, local::ExitStatus::Interrupted);
}

#[test]
fn a_line_after_interrupt_runs_its_check_in_the_same_pass() {
    let mut service = loading();
    // Loading can retain the line after Interrupt; Idle's Interrupt already exits.
    service.local_event(local::Event::Interrupt);
    service.local_event(local::Event::Line { text: b"hello".as_slice().into() });
    service.local_event(loaded());
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    let adapter = service.below.process_mut();
    let checks = adapter.programs.as_ref().unwrap();
    let active = checks.active.as_ref().expect("the later request starts");
    assert!(!active.cancelled);
    let Some(submit) = service.submissions().pop() else { panic!("status spawn") };
    let kernel::Op::Spawn { .. } = &submit.kind else { panic!("status spawn") };
    service.completions().push(kernel::Complete {
        op: submit.op,
        kind: submit.kind,
        result: Err(kernel::Error::Permission),
    });
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    let Some(smith_local_protocol::StoreRequest::SaveState { .. }) = service.store_requests().pop() else {
        panic!("save")
    };
    service.local_event(local::Event::StateSaved);
    for _ in 0_u32..4 {
        iterate(&mut service, Time::ZERO, Wall::EPOCH);
    }
    let Some(local::Request::Notice {
        notice: local::Notice::Failure(local::Failure::Credential { account: 7, reason }),
    }) = service.shell.pop()
    else {
        panic!("the check answer reaches the chat")
    };
    let local::CredentialFailure::ProgramFailed(_) = reason else { panic!("spawn failure, not interruption") };
}

#[test]
fn drop_cancels_a_credential_left_in_the_previous_pass() {
    let mut service = loading();
    service.local_event(loaded());
    for _ in 0..service.shell.capacity() {
        service.shell.push(local::Request::Credential { account: 99 });
    }
    service.local_event(local::Event::Line { text: b"hello".as_slice().into() });
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    assert!(!service.local_requests.is_empty());
    while service.shell.pop().is_some() {}
    service.local_event(local::Event::Drop);
    iterate(&mut service, Time::ZERO, Wall::EPOCH);
    assert!(service.below.process_mut().programs.as_ref().unwrap().active.is_none());
    let Some(local::Event::NoCredential { account: 7, reason: local::CredentialFailure::ProgramInterrupted }) =
        service.local_events.pop()
    else {
        panic!("queued credential interrupted")
    };
}
