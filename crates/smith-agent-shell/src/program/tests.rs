use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt, symlink},
};

use super::*;
use std::os::unix::ffi::OsStringExt;

use skein_scratch::Scratch;

#[test]
fn preparing_makes_both_directories_private() {
    let scratch = Scratch::new("program-prepare");
    let programs = scratch.path().join("missing/programs");
    assert!(!private_directory(&programs).expect("absent directory"));
    let configuration = prepare(&programs, "source-1").expect("private directories");
    assert_eq!(configuration, programs.join("source-1"));
    for path in [&programs, &configuration] {
        let metadata = fs::symlink_metadata(path).expect("created directory");
        assert!(metadata.is_dir(), "created a directory");
        assert_eq!(metadata.mode() & 0o777, 0o700);
        assert_eq!(metadata.uid(), skein_shell::effective_user());
    }
}

#[test]
fn preparing_keeps_an_existing_private_directory() {
    let scratch = Scratch::new("program-existing");
    let programs = scratch.path().join("programs");
    let configuration = prepare(&programs, "source").expect("prepare once");
    fs::write(configuration.join("kept"), b"kept").expect("existing source data");
    let inode = fs::symlink_metadata(&configuration).expect("source metadata").ino();
    assert_eq!(prepare(&programs, "source").expect("prepare again"), configuration);
    assert_eq!(fs::symlink_metadata(&configuration).expect("source metadata").ino(), inode);
    assert_eq!(fs::read(configuration.join("kept")).expect("source data"), b"kept");
}

#[test]
fn preparing_refuses_a_directory_open_to_others() {
    let scratch = Scratch::new("program-mode");
    let programs = scratch.path().join("programs");
    fs::create_dir(&programs).expect("programs directory");
    fs::set_permissions(&programs, fs::Permissions::from_mode(0o755)).expect("open mode");
    match prepare(&programs, "source") {
        Err(Refusal::Unsafe { path, found: Unsafe::Mode(0o055) }) => assert_eq!(path, programs),
        other => panic!("expected mode refusal, got {other:?}"),
    }
    assert_eq!(fs::read_dir(&programs).expect("programs contents").count(), 0);
}

#[test]
fn preparing_refuses_a_link() {
    let scratch = Scratch::new("program-link");
    let target = prepare(&scratch.path().join("target"), "source").expect("private target");
    let programs = scratch.path().join("programs");
    symlink(&target, &programs).expect("linked programs");
    for path in [&programs, &programs.join(""), &programs.join(".")] {
        match prepare(path, "source") {
            Err(Refusal::Unsafe { path: refused, found: Unsafe::Link }) => assert_eq!(refused, *path),
            other => panic!("expected link refusal, got {other:?}"),
        }
    }
    assert_eq!(fs::read_dir(target).expect("target contents").count(), 0);
}

#[test]
fn preparing_refuses_a_file() {
    let scratch = Scratch::new("program-file");
    let programs = scratch.path().join("programs");
    fs::write(&programs, b"file").expect("file fixture");
    match prepare(&programs, "source") {
        Err(Refusal::Unsafe { path, found: Unsafe::NotADirectory }) => assert_eq!(path, programs),
        other => panic!("expected file refusal, got {other:?}"),
    }
    assert_eq!(fs::read(programs).expect("untouched file"), b"file");
}

#[test]
fn preparing_refuses_a_bad_name() {
    let scratch = Scratch::new("program-name");
    let programs = scratch.path().join("programs");
    for name in ["", &"a".repeat(33), "A", "..", "a/b"] {
        assert!(matches!(prepare(&programs, name), Err(Refusal::Name)), "bad source {name:?}");
        assert!(!programs.exists(), "bad name makes nothing");
    }
    let longest = "a".repeat(32);
    assert_eq!(prepare(&programs, &longest).expect("maximum source name"), programs.join(longest));
}

#[test]
fn preparing_refuses_a_relative_directory() {
    assert!(
        matches!(prepare(Path::new("relative"), "source"), Err(Refusal::Relative(_))),
        "relative directory refused"
    );
    assert!(matches!(prepare(Path::new("relative"), ".."), Err(Refusal::Name)), "name checked first");
}

#[test]
fn preparing_checks_the_existing_source_directory() {
    let scratch = Scratch::new("program-source");
    let programs = scratch.path().join("programs");
    let configuration = prepare(&programs, "source").expect("source directory");
    fs::set_permissions(&configuration, fs::Permissions::from_mode(0o755)).expect("open source");
    match prepare(&programs, "source") {
        Err(Refusal::Unsafe { path, found: Unsafe::Mode(0o055) }) => assert_eq!(path, configuration),
        other => panic!("expected source mode refusal, got {other:?}"),
    }
    fs::remove_dir(&configuration).expect("remove open source");
    symlink(scratch.path(), &configuration).expect("source link");
    match prepare(&programs, "source") {
        Err(Refusal::Unsafe { path, found: Unsafe::Link }) => assert_eq!(path, configuration),
        other => panic!("expected source link refusal, got {other:?}"),
    }
}

#[test]
fn the_sign_in_runs_auth_login_in_the_source_s_environment() {
    let program = Path::new("/usr/bin/claude");
    let configuration = Path::new("/private/programs/source");
    let base = [
        ("HOME", "/private/home"),
        ("ANTHROPIC_API_KEY", "key"),
        ("ANTHROPIC_AUTH_TOKEN", "token"),
        ("CLAUDE_CODE_OAUTH_TOKEN", "token"),
        ("CLAUDE_CODE_USE_BEDROCK", "1"),
        ("AWS_BEARER_TOKEN_BEDROCK", "token"),
        ("CLAUDE_CONFIG_DIR", "/person/config"),
    ];
    let command = sign_in(program, configuration, base.map(|(name, value)| (name.into(), value.into())));
    assert_eq!(command.get_program(), program);
    assert_eq!(command.get_args().collect::<Vec<_>>(), ["auth", "login"]);
    assert_eq!(
        command.get_envs().collect::<Vec<_>>(),
        [
            (OsStr::new("CLAUDE_CONFIG_DIR"), Some(configuration.as_os_str())),
            (OsStr::new("HOME"), Some(OsStr::new("/private/home"))),
        ]
    );
}

#[test]
fn the_terminal_environment_has_no_hosted_spawn_bound() {
    let mut base: Vec<_> = (0..100).map(|index| (format!("VARIABLE_{index}").into(), "x".repeat(100).into())).collect();
    base.push((OsString::from_vec(vec![b'N', 0xff]), OsString::from_vec(vec![0xfe])));
    let command = sign_in(Path::new("/claude"), Path::new("/programs/source"), base);
    assert_eq!(command.get_envs().count(), 102);
}
