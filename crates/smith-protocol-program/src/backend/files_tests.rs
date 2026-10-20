#![expect(clippy::disallowed_methods, reason = "ordinary Rust fault harness, programming-model.md 10.2")]
use super::*;

fn failed(committed: bool, residue: bool) -> file::Event {
    file::Event::Failed {
        owner: Token::new(15),
        error: kernel::Error::Permission,
        committed,
        residue: residue
            .then(|| file::Residue { name: b".store-test".as_slice().into(), error: kernel::Error::Permission }),
    }
}
fn prepared() -> Backend {
    let mut backend = super::super::lifecycle_tests::backend();
    backend.phase = Phase::Ending;
    backend.settings.directory = Some(super::super::Root { file: Token::new(40), descriptor: kernel::Fd::new(41) });
    backend.files.run = Some(Token::new(42));
    backend.files.run_descriptor = Some(kernel::Fd::new(43));
    backend.files.runs = Some(Token::new(44));
    backend.files.runs_descriptor = Some(kernel::Fd::new(45));
    backend.files.remove_run = true;
    backend
}
#[test]
fn a_committed_store_failure_removes_its_target() {
    let mut backend = prepared();
    let fault = failed(true, false);
    assert!(!backend.files.accept(Step::Mcp, fault, None), "the committed fault was injected");
    assert!(backend.files.mcp, "the committed secret is still owned");
    let mut below = Queue::with_capacity(4);
    backend.cleanup(Time::ZERO, &mut below);
    let Some(Below::File { request: file::Request::Remove { name, .. }, .. }) = below.pop() else {
        panic!("cleanup remove");
    };
    assert_eq!(name.as_ref(), b"mcp.json");
    assert!(backend.files.mcp, "submission does not relinquish ownership");
    assert!(backend.files.accept(Step::RemoveMcp, file::Event::Removed { owner: Token::new(15) }, None));
    assert!(!backend.files.mcp);
}
#[test]
fn a_failed_store_residue_is_removed_before_the_target() {
    let mut backend = prepared();
    assert!(!backend.files.accept(Step::Mcp, failed(true, true), None), "the residue fault was injected");
    let mut below = Queue::with_capacity(4);
    backend.cleanup(Time::ZERO, &mut below);
    let Some(Below::File { request: file::Request::Remove { name, .. }, .. }) = below.pop() else {
        panic!("cleanup remove");
    };
    assert_eq!(name.as_ref(), b".store-test");
    assert!(backend.files.mcp);
}
#[test]
fn a_failed_removal_retains_ownership_for_retry() {
    let mut backend = prepared();
    backend.files.mcp = true;
    let mut below = Queue::with_capacity(4);
    backend.cleanup(Time::ZERO, &mut below);
    drop(below.pop());
    let step = backend.files.pending.take().expect("bounded test fixture");
    assert!(!backend.files.accept(step, failed(false, false), None), "the unlink fault was injected");
    assert!(backend.files.mcp);
    backend.cleanup(Time::ZERO, &mut below);
    let Some(Below::File { request: file::Request::Remove { name, .. }, .. }) = below.pop() else {
        panic!("cleanup remove");
    };
    assert_eq!(name.as_ref(), b"mcp.json");
    assert!(backend.files.accept(step, file::Event::Removed { owner: Token::new(15) }, None));
    assert!(!backend.files.mcp);
}
#[test]
fn an_absent_cleanup_target_relinquishes_ownership() {
    let mut files = Files::new();
    files.mcp = true;
    assert!(files.accept(
        Step::RemoveMcp,
        file::Event::Failed { owner: Token::new(15), error: kernel::Error::NotFound, committed: false, residue: None },
        None
    ));
    assert!(!files.mcp);
}
