use smith_local_process_world::{
    Files, Lending, World,
    referee::{Ending, review},
};

#[global_allocator]
static HEAP: skein_world::domain::heap::Counting = skein_world::domain::heap::Counting;

#[test]
fn an_env_account_lends_its_variable() {
    let files = Files::new();
    let mut world = World::lending(44, &files, Lending::Env);
    world.settle();
    assert!(!world.queries().is_empty(), "the inherited variable's value reaches the provider");
    review(&world.seen(), &world, Ending::Report(b"First answer".to_vec())).assert_passed(44);
}

#[test]
fn a_key_account_lends_its_key_to_the_run() {
    let files = Files::new();
    let mut world = World::lending(45, &files, Lending::Key);
    world.settle();
    assert!(!world.queries().is_empty(), "the key grant reaches the provider");
    assert!(!world.shown().windows(b"key-for-work".len()).any(|bytes| bytes == b"key-for-work"), "no key in output");
    review(&world.seen(), &world, Ending::Report(b"First answer".to_vec())).assert_passed(45);
}
