//! Credential commands on scratch state, the fake issuer and fake program.
use super::support;
use smith_real_world::Scratch;
use std::os::unix::fs::DirBuilderExt;

const FULL_SCOPE: &[u8] = b"openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";

fn settings(scratch: &Scratch, extra: &str) {
    let path = scratch.path().join("config/smith");
    std::fs::create_dir_all(&path).expect("credential fixture");
    let host = smith_real_world::loopback();
    std::fs::write(path.join("config.toml"), format!(
        "[accounts.test]\nprovider='openai'\nsource='sign_in'\nissuer='http://{host}:34444'\nauthorization_url='http://{host}:34444/authorize'\ntoken_url='http://{host}:34444/token'\nkeys_url='http://{host}:34444/keys'\nrevocation_url='http://{host}:34444/revoke'\nredirect_port={}\n{extra}", smith_real_world::redirect_port())).expect("credential fixture");
}

fn limits() -> skein_oauth::Limits {
    let mut limits = smith_local_shell::local_host::token_limits();
    limits.issued_client_bytes = 1024;
    limits.subject_bytes = 1024;
    limits
}

fn path(scratch: &Scratch, name: &str) -> std::path::PathBuf {
    scratch.path().join("state/smith/tokens").join(name)
}

fn record(scratch: &Scratch, name: &str) -> skein_oauth::SavedToken {
    let bytes =
        skein_shell::read_private_file(&path(scratch, name), limits().record_bytes, skein_shell::effective_user())
            .expect("credential fixture");
    skein_oauth::decode_record(bytes.expose(), &limits()).expect("credential fixture")
}

fn seed_bytes(subject: &[u8]) -> Vec<u8> {
    let record = skein_oauth::SavedToken {
        issued_client_id: Some(b"oaiapp_for_work".as_slice().into()),
        granted_scopes: Some(FULL_SCOPE.into()),
        subject: Some(subject.into()),
        id_token: Some(b"prior-id-for-work".as_slice().into()),
        earliest_refresh_at: None,
        key: 0,
        generation: 1,
        access_token: b"access-for-work".as_slice().into(),
        refresh_token: Some(b"refresh-for-work".as_slice().into()),
        metadata: None,
        expires_at: skein_lib::Wall::from_nanos(u64::MAX),
    };
    skein_oauth::encode_record(&record, &limits()).expect("credential fixture").into_vec()
}

fn seed(scratch: &Scratch, name: &str, subject: &[u8]) {
    let path = path(scratch, name);
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path.parent().expect("credential fixture"))
        .expect("credential fixture");
    support::provider_login::keep_record(&path, &seed_bytes(subject)).expect("credential fixture");
}

fn no_values(ran: &support::Ran) {
    for bytes in [&ran.stdout, &ran.stderr] {
        let words = String::from_utf8_lossy(bytes);
        assert!(!words.contains("access-for-work"), "credential value absent from output");
        assert!(!words.contains("refresh-for-work"), "credential value absent from output");
        assert!(!words.contains("key-for-work"), "credential value absent from output");
    }
}

#[test]
fn a_first_sign_in_registers() {
    let scratch = Scratch::new();
    settings(&scratch, "");
    let (ran, urls, _) = support::provider_login::run(&scratch, "test", false, b"person-for-work", true, false, None);
    assert_eq!(ran.code, 0);
    assert!(
        record(&scratch, "test").issued_client_id.as_deref() == Some(b"oaiapp_for_work".as_slice()),
        "issued registration kept"
    );
    assert!(scratch.path().join("state/smith/host-id").exists());
    assert!(
        record(&scratch, "test").subject.as_deref() == Some(b"person-for-work".as_slice()),
        "validated subject kept"
    );
    assert!(String::from_utf8_lossy(&ran.stderr).contains("ssh -L "));
    assert!(String::from_utf8_lossy(&ran.stderr).contains(" 192.0.2.1"));
    assert!(path(&scratch, "test.lock").is_dir());
    assert_eq!(String::from_utf8_lossy(&ran.stderr).matches("You're using your ChatGPT plan.").count(), 1);
    assert!(urls[0].windows(b"agent_name_hint=smith".len()).any(|part| part == b"agent_name_hint=smith"));
    no_values(&ran);
}

#[test]
fn signing_in_again_uses_the_issued_id() {
    let scratch = Scratch::new();
    settings(&scratch, "");
    seed(&scratch, "test", b"person-for-work");
    let (ran, urls, _) = support::provider_login::run(&scratch, "test", false, b"person-for-work", true, true, None);
    assert_eq!(ran.code, 0);
    let url = String::from_utf8_lossy(&urls[0]);
    assert!(url.contains("client_id=oaiapp_for_work"));
    assert!(!url.contains("agent_name_hint"));
    assert!(url.contains("ext_agent_host_id="));
    assert!(url.contains("id_token_hint"));
    assert!(!String::from_utf8_lossy(&ran.stderr).contains("You're using your ChatGPT plan."));
    no_values(&ran);
}

#[test]
fn another_subject_preserves_the_record() {
    let scratch = Scratch::new();
    settings(&scratch, "");
    seed(&scratch, "test", b"person-for-work");
    let before = std::fs::read(path(&scratch, "test")).expect("credential fixture");
    let (ran, urls, _) = support::provider_login::run(&scratch, "test", false, b"person-for-home", true, true, None);
    assert_eq!(ran.code, 2);
    assert_eq!(urls.len(), 1);
    assert!(String::from_utf8_lossy(&ran.stderr).contains("another subject was refused"));
    assert!(before == std::fs::read(path(&scratch, "test")).expect("credential fixture"), "record stays byte-for-byte");
    no_values(&ran);
}

#[test]
fn a_declined_plan_scope_keeps_the_record() {
    let scratch = Scratch::new();
    settings(&scratch, "");
    let (ran, _, _) = support::provider_login::run(&scratch, "test", false, b"person-for-work", false, false, None);
    assert_eq!(ran.code, 3);
    assert!(path(&scratch, "test").exists());
    assert!(
        String::from_utf8_lossy(&ran.stderr)
            .contains("Signed in without plan usage: run `smith login test` again and allow it, or use a key")
    );
    no_values(&ran);
}

#[test]
fn a_duplicate_subject_names_its_account() {
    let scratch = Scratch::new();
    settings(&scratch, "[accounts.first]\nprovider='openai'\nsource='sign_in'\n");
    seed(&scratch, "first", b"person-for-work");
    let (ran, _, revoked) =
        support::provider_login::run(&scratch, "test", false, b"person-for-work", true, false, None);
    assert_eq!(ran.code, 2);
    assert!(String::from_utf8_lossy(&ran.stderr).contains("account first"));
    assert!(!path(&scratch, "test").exists());
    assert!(path(&scratch, "first").exists());
    assert_eq!(revoked, 1);
    no_values(&ran);
}

#[test]
fn logout_revokes_the_record() {
    let scratch = Scratch::new();
    settings(&scratch, "");
    seed(&scratch, "test", b"person-for-work");
    let (ran, _, revoked) =
        support::provider_login::run(&scratch, "test", true, b"person-for-work", true, true, Some(true));
    assert_eq!(ran.code, 0);
    assert_eq!(revoked, 1);
    assert!(!path(&scratch, "test").exists());
    assert!(!scratch.path().join("state/smith/host-id").exists());
    no_values(&ran);
}

#[test]
fn an_unconfirmed_revocation_says_where_to_disconnect() {
    let scratch = Scratch::new();
    settings(&scratch, "");
    seed(&scratch, "test", b"person-for-work");
    let (ran, _, revoked) =
        support::provider_login::run(&scratch, "test", true, b"person-for-work", true, true, Some(false));
    assert_eq!(ran.code, 0);
    assert_eq!(revoked, 1);
    assert!(!path(&scratch, "test").exists());
    assert!(String::from_utf8_lossy(&ran.stderr).contains("disconnect smith at chatgpt.com/settings"));
    no_values(&ran);
}

#[test]
fn key_commands_keep_values_private() {
    let scratch = Scratch::new();
    let _file = support::user_layer(&scratch, b"[accounts.work]\nsource='key'\n[accounts.empty]\nsource='none'\n[accounts.variable]\nsource='env'\nvariable='WORK_KEY'\n");
    let login = support::run_product(&scratch, &["login", "work"], b"key-for-work\n", None);
    assert_eq!(login.code, 0);
    assert!(scratch.path().join("state/smith/keys/work").is_file());
    no_values(&login);
    let terminal = support::provider_login::terminal_key(&scratch, "work");
    assert_eq!(terminal.code, 2);
    no_values(&terminal);
    for (account, code) in [("work", 0), ("empty", 0), ("variable", 0)] {
        let logout = support::run_product(&scratch, &["logout", account], b"", None);
        assert_eq!(logout.code, code);
        no_values(&logout);
    }
    assert!(!scratch.path().join("state/smith/keys/work").exists());
}

#[test]
fn a_racing_first_login_preserves_the_winner() {
    let scratch = Scratch::new();
    settings(&scratch, "");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path(&scratch, "test").parent().expect("tokens parent"))
        .expect("tokens fixture");
    let winner = seed_bytes(b"person-for-home");
    let (ran, urls, _) = support::provider_login::run_arriving(
        &scratch,
        "test",
        false,
        b"person-for-work",
        true,
        false,
        (None, Some((path(&scratch, "test"), winner.clone()))),
    );
    assert_eq!(ran.code, 2);
    assert_eq!(urls.len(), 1);
    assert!(String::from_utf8_lossy(&ran.stderr).contains("account test: another subject was refused"));
    assert!(std::fs::read(path(&scratch, "test")).expect("winner record") == winner, "winner stays byte-for-byte");
    no_values(&ran);
}

#[test]
fn a_seed_refuses_an_existing_public_file() {
    use std::os::unix::fs::PermissionsExt;
    let scratch = Scratch::new();
    let path = scratch.path().join("record");
    std::fs::write(&path, b"previous").expect("public fixture");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("public fixture mode");
    support::provider_login::keep_record(&path, b"record-for-work").expect_err("never write into a public file");
    assert!(std::fs::read(&path).expect("untouched fixture") == b"previous", "existing file stays untouched");
}
