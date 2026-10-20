//! Shared account stories reuse the private driver's simulated world.
use super::*;
use crate::Settled;

#[test]
fn a_refresh_takes_the_record_lock() {
    let mut world = Private::configured(Story::Refresh, true);
    world.until(Until::Ready);
    assert_eq!(world.saved().generation, 2);
    let lock = world.files.open(world.root, b"tokens/record.lock", How::Directory).unwrap();
    let ceiling = world.sim.now().saturating_add(Duration::from_millis(1));
    for _ in 0..4000_u32 {
        if world.owner.files.open_files() == 2 {
            break;
        }
        world.step_before(Some(ceiling));
    }
    world.files.lock(lock).expect("refresh released its lock root");
    world.files.close(lock);
    world.close();
}

#[test]
fn a_private_account_opens_no_lock() {
    let mut world = Private::new(Story::Refresh);
    world.until(Until::Ready);
    world
        .files
        .open(world.root, b"tokens/record.lock", How::Directory)
        .expect_err("private account has no lock directory");
    world.close();
}

#[test]
fn a_sign_out_removes_an_unconfirmed_record() {
    let mut world = Private::configured(Story::Load, true);
    world.until(Until::Ready);
    world.owner.release_all(world.sim.now(), world.sim.wall());
    world.owner.sign_out(7);
    let mut terminal = None;
    for _ in 0..4000_u32 {
        world.step();
        if let Some((_, settled)) = world.owner.settled() {
            terminal = Some(settled);
            break;
        }
    }
    match terminal {
        Some(Settled::SignedOut { revoked: false }) => {}
        Some(Settled::Failed { ends, failure }) => panic!("sign-out failed {ends:?} {failure:?}"),
        Some(Settled::SignedIn { .. } | Settled::SignedOut { revoked: true }) | None => panic!("missing sign-out"),
    }
    world.files.open(world.root, b"tokens/record", How::Read).expect_err("sign-out removed its record");
    world.close();
}

#[test]
fn cancel_ends_a_waiting_sign_in() {
    let mut world = Private::configured(Story::Cancel, true);
    world.until(Until::Visit);
    world.owner.cancel(7);
    world.until(Until::Failed);
    match world.owner.settled() {
        Some((number, Settled::Failed { ends, .. })) => {
            assert_eq!(number, 7);
            assert_eq!(ends, oauth::Ends::SignIn);
        }
        Some((_, Settled::SignedIn { .. } | Settled::SignedOut { .. })) | None => panic!("cancel ends sign-in"),
    }
    world.close();
}

#[test]
fn settled_keeps_the_sign_in_scope_terminal() {
    let mut world = Private::configured(Story::SignIn, true);
    world.until(Until::SignedIn);
    match world.owner.settled() {
        Some((number, Settled::SignedIn { subject, missing })) => {
            assert_eq!(number, 7);
            assert!(subject.is_none(), "explicit fixture has no subject");
            assert!(missing.is_empty(), "explicit fixture requires no scope");
        }
        Some((_, Settled::Failed { .. } | Settled::SignedOut { .. })) | None => panic!("sign-in terminal"),
    }
    world.close();
}

#[test]
fn a_busy_lock_preserves_the_record() {
    let mut world = Private::configured(Story::Refresh, true);
    let tokens = world.files.open(world.root, b"tokens", How::Directory).unwrap();
    world.files.make_directory(tokens, b"record.lock", 0o700).unwrap();
    world.files.close(tokens);
    let held = world.files.open(world.root, b"tokens/record.lock", How::Directory).unwrap();
    world.files.lock(held).unwrap();
    let before = world.sim.now();
    world.until(Until::Failed);
    assert!(world.sim.now() >= before.saturating_add(native_limits().lock_wait), "busy waits for the lock budget");
    match world.owner.settled() {
        Some((number, Settled::Failed { failure, .. })) => {
            assert_eq!(number, 7);
            assert_eq!(failure, oauth::Failure::Busy);
        }
        Some((_, Settled::SignedIn { .. } | Settled::SignedOut { .. })) | None => panic!("lock refusal"),
    }
    assert_eq!(world.saved().generation, 1);
    assert!(world.saved().refresh_token.as_deref() == Some(b"refresh-old".as_slice()));
    world.close();
    world.files.close(held);
}

#[test]
fn a_lock_freed_within_the_wait_allows_refresh() {
    let mut world = Private::configured(Story::Refresh, true);
    let tokens = world.files.open(world.root, b"tokens", How::Directory).unwrap();
    world.files.make_directory(tokens, b"record.lock", 0o700).unwrap();
    world.files.close(tokens);
    let held = world.files.open(world.root, b"tokens/record.lock", How::Directory).unwrap();
    world.files.lock(held).unwrap();
    let release = world.sim.now().saturating_add(Duration::from_millis(100));
    for _ in 0..4000_u32 {
        world.step();
        if world.sim.now() >= release {
            break;
        }
    }
    assert_eq!(world.count(Until::Failed), 0, "contention is retried before its deadline");
    assert!(world.sim.now() < release.saturating_add(native_limits().lock_wait), "retry has a near timer");
    world.files.close(held);
    world.until(Until::Ready);
    assert_eq!(world.saved().generation, 2);
    world.close();
}
