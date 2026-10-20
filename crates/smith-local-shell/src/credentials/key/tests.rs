use super::*;
use crate::credentials::tests::Files;
use std::os::unix::fs::PermissionsExt;

pub(crate) fn file(files: &Files, bytes: &[u8], mode: u32) -> std::path::PathBuf {
    let path = files.0.join("key");
    std::fs::write(&path, bytes).expect("scratch key");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("key permissions");
    path
}

#[test]
fn a_key_file_reads_without_its_trailing_newline() {
    let files = Files::new();
    let bytes = read(&file(&files, b"key-for-work\n", 0o600)).expect("private key");
    assert!(bytes.as_ref() == b"key-for-work", "one newline removed");
}

#[test]
fn an_absent_key_is_not_signed_in() {
    let files = Files::new();
    assert!(matches!(inspect(&files.0.join("absent")), Credential::NotSignedIn), "missing key");
    assert!(matches!(inspect(Path::new("relative")), Credential::NotSignedIn), "no directory");
}

#[test]
fn a_key_open_to_others_is_unreadable() {
    let files = Files::new();
    assert!(
        matches!(inspect(&file(&files, b"key-for-work", 0o644)), Credential::KeyUnreadable { .. }),
        "private mode required"
    );
}

#[test]
fn an_empty_key_is_unreadable() {
    let files = Files::new();
    for bytes in [b"".as_slice(), b"\n"] {
        assert!(matches!(inspect(&file(&files, bytes, 0o600)), Credential::KeyUnreadable { .. }), "key is empty");
    }
}

#[test]
fn a_key_with_a_space_is_unreadable() {
    let files = Files::new();
    for bytes in [b"key for-work".as_slice(), b"key-for-work\n\n", b"key-for-work\r\n", b"key\x7f"] {
        assert!(
            matches!(inspect(&file(&files, bytes, 0o600)), Credential::KeyUnreadable { .. }),
            "printable one-line key required"
        );
    }
}

#[test]
fn a_key_at_the_token_bound_is_usable() {
    let files = Files::new();
    let bound = usize::try_from(crate::local_host::token_limits().token_bytes).expect("token bound");
    let bytes = vec![b'x'; bound];
    assert!(inspect(&file(&files, &bytes, 0o600)).usable(), "key at bound");
}

#[test]
fn a_key_past_the_token_bound_is_unreadable() {
    let files = Files::new();
    let bound = usize::try_from(crate::local_host::token_limits().token_bytes).expect("token bound");
    let bytes = vec![b'x'; bound + 1];
    assert!(matches!(inspect(&file(&files, &bytes, 0o600)), Credential::KeyUnreadable { .. }), "key past bound");
}

#[test]
fn a_usable_key_report_holds_no_value() {
    let files = Files::new();
    let credential = inspect(&file(&files, b"key-for-work", 0o600));
    assert!(matches!(credential, Credential::Usable { until: None }), "usable key required");
    for text in
        [credential.line("work"), format!("{credential:?}"), serde_json::to_string(&credential).expect("state JSON")]
    {
        assert!(!text.contains("key-for-work"), "report hides the key");
    }
}

#[test]
fn unusable_key_reports_name_the_problem_without_private_data() {
    let files = Files::new();
    let bound = usize::try_from(crate::local_host::token_limits().token_bytes).expect("token bound");
    let oversized = vec![b'x'; bound + 1];
    for (bytes, mode, expected, words) in [
        (b"key-for-work".as_slice(), 0o644, KeyProblem::Mode, "mode"),
        (b"".as_slice(), 0o600, KeyProblem::Size, "size"),
        (oversized.as_slice(), 0o600, KeyProblem::Size, "size"),
        (b"key for-work".as_slice(), 0o600, KeyProblem::Unreadable, "unreadable"),
    ] {
        let path = file(&files, bytes, mode);
        let credential = inspect(&path);
        assert!(
            matches!(credential, Credential::KeyUnreadable { problem } if problem == expected),
            "key failure class"
        );
        let line = credential.line("work");
        assert!(line.contains("work") && line.contains(words), "account and problem named");
        for text in [line, format!("{credential:?}"), serde_json::to_string(&credential).expect("state JSON")] {
            assert!(!text.contains("key-for-work") && !text.contains("key for-work"), "no key bytes");
            assert!(!text.contains(path.to_str().expect("scratch UTF-8")), "no private path");
        }
    }
}

#[test]
fn key_descriptors_hide_their_paths_in_debug() {
    let files = Files::new();
    let path = file(&files, b"key-for-work", 0o600);
    let directory = crate::credentials::Directory::from_base(files.0.clone()).expect("absolute base");
    let source = crate::credentials::Source::Key { file: path };
    for text in [format!("{directory:?}"), format!("{source:?}")] {
        assert!(!text.contains(files.0.to_str().expect("scratch UTF-8")), "no private path");
        assert!(!text.contains("key-for-work"), "no key bytes");
    }
}
