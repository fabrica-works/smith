use super::*;
use crate::credentials::tests::Files;
use std::os::unix::fs::PermissionsExt;

// shell.md, section 7.4: the name work hashes to this number.
const WORK: u32 = 14_761_687;

fn locations(source: Source) -> List<Location> {
    let mut locations = List::with_capacity(1);
    locations.push(Location { account: WORK, name: "work".into(), source }).expect("one account");
    locations
}

fn key(files: &Files) -> std::path::PathBuf {
    let file = files.0.join("key");
    std::fs::write(&file, b"key-for-work").expect("scratch key");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("private key");
    file
}

#[test]
fn the_first_request_for_a_key_account_lends_its_key() {
    let files = Files::new();
    let locations = locations(Source::Key { file: key(&files) });
    let mut lent = Lent::with_capacity(1);
    match lent.answer(&locations, WORK) {
        Answer::Lend(grant, value) => {
            assert_eq!(grant.name.account, WORK, "account number");
            assert_eq!(grant.name.generation, 1, "only generation");
            assert!(grant.valid > Duration::from_secs(365 * 24 * 60 * 60), "valid past a year");
            match value {
                GrantValue::Key(bytes) => assert!(bytes.as_ref() == b"key-for-work", "private key bytes"),
                GrantValue::Bearer(_) | GrantValue::Program(_) | GrantValue::Withdrawn { .. } => {
                    panic!("key grant expected")
                }
            }
        }
        Answer::Missing => panic!("key grant expected"),
    }
}

#[test]
fn the_first_request_for_a_program_account_lends_its_name() {
    let locations = locations(Source::Program { name: "work".into() });
    match Lent::with_capacity(1).answer(&locations, WORK) {
        Answer::Lend(grant, value) => {
            assert_eq!(grant.name.account, WORK, "account number");
            assert_eq!(grant.name.generation, 1, "only generation");
            match value {
                GrantValue::Program(name) => assert_eq!(name.as_ref(), b"work", "source name"),
                GrantValue::Bearer(_) | GrantValue::Key(_) | GrantValue::Withdrawn { .. } => {
                    panic!("program grant expected")
                }
            }
        }
        Answer::Missing => panic!("program grant expected"),
    }
}

#[test]
fn a_second_request_is_answered_missing() {
    let locations = locations(Source::Program { name: "work".into() });
    let mut lent = Lent::with_capacity(1);
    drop(lent.answer(&locations, WORK));
    assert!(matches!(lent.answer(&locations, WORK), Answer::Missing), "never refresh");
}

#[test]
fn a_request_for_an_unknown_account_is_answered_missing() {
    let locations = locations(Source::Nothing);
    let mut lent = Lent::with_capacity(1);
    for account in [WORK, 7] {
        assert!(matches!(lent.answer(&locations, account), Answer::Missing), "no grant");
    }
}

#[test]
fn a_key_removed_after_start_is_answered_missing() {
    let files = Files::new();
    let file = key(&files);
    let locations = locations(Source::Key { file: file.clone() });
    std::fs::remove_file(file).expect("removed after startup");
    assert!(matches!(Lent::with_capacity(1).answer(&locations, WORK), Answer::Missing), "reread fails");
}

#[test]
fn a_lent_answers_debug_holds_no_value() {
    let files = Files::new();
    let locations = locations(Source::Key { file: key(&files) });
    let answer = Lent::with_capacity(1).answer(&locations, WORK);
    assert!(matches!(answer, Answer::Lend(_, GrantValue::Key(_))), "lent key required");
    assert!(!format!("{answer:?}").contains("key-for-work"), "redacted grant value");
}
