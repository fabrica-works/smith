use super::*;
use crate::service::ending::Ending;
use smith_agent_service::{Close, Step};

#[test]
fn service_ending_waits_for_the_check_root() {
    let mut service = crate::Service::new(crate::tests::config(), 7).expect("bounded service");
    let mut sources = List::with_capacity(1);
    sources.push((7, SourceName::new(b"work").expect("source"))).expect("source bound");
    service
        .handed_programs(
            sources,
            Programs { directory: b"/programs".as_slice().into(), claude: b"/claude".as_slice().into() },
            Box::new([]),
            kernel::Fd::new(3),
            1024,
            Duration::from_secs(300),
        )
        .expect("program binding");
    service.account_request(7);
    service.shell.pop().expect("covered request");
    service.account_request(7);
    let adapter = service.below.process_mut();
    let Some(io::Request::Spawn { owner, .. }) = adapter.io_requests().pop() else { panic!("status spawn") };
    adapter.close();
    service.ending = Ending::Closing {
        at: Time::ZERO,
        exit: local::ExitStatus::Accepted,
        close: Close::Graceful,
        step: Step::Components,
        shell_closed: true,
    };
    assert!(!service.end_work());
    let adapter = service.below.process_mut();
    adapter.programs.as_mut().expect("check").receive(io::Event::Closed { owner }, &mut adapter.io_requests);
    service.below.process_mut().programs_pass(Time::ZERO, &mut service.local_events, &mut service.shell, false);
    assert!(service.local_events.is_empty(), "the local domain already said its last word");
    assert!(!service.end_work());
    let checks = service.below.process_mut().programs.as_mut().expect("program root");
    assert_eq!(checks.closing_root(), Some(kernel::Fd::new(3)));
    checks.root_closed();
    assert!(service.end_work());
}
