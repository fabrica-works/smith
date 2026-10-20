use super::*;
use crate::credentials::{key, logout, tests::Files};
use std::os::unix::fs::PermissionsExt;

#[test]
fn piped_keys_replace_privately() {
    let files = Files::new();
    let path = files.0.join("keys/work");
    keep(&path, &mut b"key-for-work\n".as_slice(), false).unwrap();
    assert!(key::read(&path).unwrap().as_ref() == b"key-for-work", "trailing newline removed");
    assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    keep(&path, &mut b"key-for-next".as_slice(), false).unwrap();
    assert!(key::read(&path).unwrap().as_ref() == b"key-for-next", "key replaced");
    logout::remove_key(&path).unwrap();
    assert!(!path.exists());
}

#[test]
fn terminal_input_is_refused() {
    let files = Files::new();
    assert!(keep(&files.0.join("work"), &mut b"key-for-work".as_slice(), true).is_err());
    assert!(!files.0.join("work").exists());
}

#[test]
fn empty_input_is_refused() {
    let files = Files::new();
    for input in [b"".as_slice(), b"\n", b"key with spaces", b"key\n\n"] {
        let result = keep(&files.0.join("work"), &mut &*input, false);
        assert!(result.is_err());
    }
    assert!(!files.0.join("work").exists());
}

#[test]
fn login_debug_holds_no_key_text() {
    let environment = crate::credentials::Environment { variables: vec![("WORK_KEY".into(), "key-for-work".into())] };
    let report = format!("{environment:?}");
    assert!(!report.contains("key-for-work"), "environment debug is redacted");
    let words = crate::credentials::credential_words("work", &crate::credentials::Credential::Usable { until: None });
    assert!(!words.contains("key-for-work"), "credential words hold no value");
}
