use super::*;
use crate::credentials::tests::Files;
use std::os::unix::fs::PermissionsExt;

#[test]
fn host_id_is_drawn_once() {
    let files = Files::new();
    let directory = Directory::from_base(files.0.join("installation")).unwrap();
    let first = read_or_draw(&directory, &[3; 32]).unwrap();
    let second = read_or_draw(&directory, &[4; 32]).unwrap();
    assert_eq!(first, second);
    assert_eq!(first, "urn:uuid:03030303-0303-4303-8303-030303030303");
    assert_eq!(std::fs::metadata(directory.host_id()).unwrap().permissions().mode() & 0o777, 0o600);
}

#[test]
fn a_malformed_host_id_is_kept() {
    let files = Files::new();
    let directory = Directory::from_base(files.0.join("installation")).unwrap();
    std::fs::create_dir(directory.host_id().parent().unwrap()).unwrap();
    std::fs::write(directory.host_id(), b"not-a-uuid").unwrap();
    std::fs::set_permissions(directory.host_id(), std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(read_or_draw(&directory, &[4; 32]).is_err());
    assert_eq!(std::fs::read(directory.host_id()).unwrap(), b"not-a-uuid");
}
