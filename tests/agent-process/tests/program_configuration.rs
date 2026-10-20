//! Focused controls for the real probes' private state, without a program.
use std::os::unix::fs::PermissionsExt;
#[path = "live/probe.rs"]
mod probe;
use probe::Probe;

#[test]
fn live_probes_have_distinct_private_state() {
    let first = Probe::new();
    let second = Probe::new();
    assert_ne!(first.home, second.home);
    assert_ne!(first.session, second.session);
    assert_eq!(first.session.len(), 36);
    assert_eq!(second.session.len(), 36);
    for home in [&first.home, &second.home] {
        assert!(home.is_dir());
        assert_eq!(std::fs::metadata(home).expect("private home").permissions().mode() & 0o777, 0o700);
    }
    let removed = first.home.clone();
    drop(first);
    assert!(!removed.exists());
    assert!(second.home.is_dir(), "one probe does not remove another's home");
}
