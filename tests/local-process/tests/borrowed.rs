use smith_local_process_world::{
    Borrowing, Files, World,
    referee::{Ending, review},
};

#[global_allocator]
static HEAP: skein_world::domain::heap::Counting = skein_world::domain::heap::Counting;

#[test]
fn an_expired_borrowed_login_starts_no_run_and_names_its_program() {
    let files = Files::new();
    let mut world = World::borrowed(41, &files, Borrowing::Expired);
    world.settle();
    assert!(world.queries().is_empty(), "no provider query uses the expired login");
    assert!(
        world.shown().windows(16).any(|bytes| bytes == b"Run `codex` once"),
        "the notice names the source's program"
    );
    assert_eq!(world.login_writes(), 0, "the borrowed file is never written");
    review(&world.seen(), &world, Ending::Unavailable).assert_passed(41);
}

#[test]
fn a_borrowed_login_lends_its_grant_and_its_file_is_never_written() {
    let files = Files::new();
    let mut world = World::borrowed(42, &files, Borrowing::Valid);
    world.settle();
    assert!(!world.queries().is_empty(), "the access-only grant reaches the provider");
    assert_eq!(world.login_reads(), 2, "startup and the due grant read their source");
    assert_eq!(world.login_writes(), 0, "the borrowed file is never written");
    let login: serde_json::Value = serde_json::from_slice(&world.borrowed_login()).expect("unchanged fixture login");
    assert_eq!(
        login["tokens"]["refresh_token"], "UNISSUED_NEVER_COPY",
        "the source's refresh token stays in its fixture"
    );
    review(&world.seen(), &world, Ending::Report(b"First answer".to_vec())).assert_passed(42);
}

#[test]
fn a_login_refreshed_by_its_program_is_read_again_when_a_grant_is_due() {
    let files = Files::new();
    let mut world = World::borrowed(43, &files, Borrowing::Renewed);
    world.settle();
    assert!(!world.queries().is_empty(), "the login's program supplied a fresh grant");
    assert_eq!(world.login_reads(), 2, "the driver read the changed source for the grant");
    assert_eq!(world.login_writes(), 0, "only the fixture's external program renewed its file");
    review(&world.seen(), &world, Ending::Report(b"First answer".to_vec())).assert_passed(43);
}

#[test]
fn an_env_account_lends_its_variable() {
    let files = Files::new();
    let mut world = World::borrowed(44, &files, Borrowing::Env);
    world.settle();
    assert!(!world.queries().is_empty(), "the inherited variable's value reaches the provider");
    assert_eq!(world.login_reads(), 0, "the driver never opens a file for an env source");
    assert_eq!(world.login_writes(), 0, "env credentials are not kept in a login file");
    review(&world.seen(), &world, Ending::Report(b"First answer".to_vec())).assert_passed(44);
}

#[test]
fn a_key_account_lends_its_key_to_the_run() {
    let files = Files::new();
    let mut world = World::borrowed(45, &files, Borrowing::Key);
    world.settle();
    assert!(!world.queries().is_empty(), "the key grant reaches the provider");
    assert_eq!(world.login_reads(), 0, "a key reads no borrowed login");
    assert!(!world.shown().windows(b"key-for-work".len()).any(|bytes| bytes == b"key-for-work"), "no key in output");
    review(&world.seen(), &world, Ending::Report(b"First answer".to_vec())).assert_passed(45);
}
