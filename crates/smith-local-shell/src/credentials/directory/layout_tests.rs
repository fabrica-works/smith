use super::*;

#[test]
fn a_token_lives_beside_its_lock() {
    let directory = Directory::from_base("/base".into()).expect("absolute base");
    assert_eq!(directory.token("work"), PathBuf::from("/base/tokens/work"));
    assert_eq!(directory.lock("work"), PathBuf::from("/base/tokens/work.lock"));
    assert_eq!(directory.host_id(), PathBuf::from("/base/host-id"));
}
