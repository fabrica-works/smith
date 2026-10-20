use super::*;

#[test]
fn logout_requires_an_account() {
    assert_eq!(parse(vec!["logout".into()]), Err(Usage("smith logout requires an account".into())));
}

#[test]
fn logout_takes_the_shared_options() {
    let command =
        parse(["logout", "work", "-C", "/work", "-c", "accounts.work.source='key'"].map(Into::into).into()).unwrap();
    let Command::Logout { shared, account } = command else { panic!("logout") };
    assert_eq!(account, "work");
    assert_eq!(shared.directory, Some("/work".into()));
    assert_eq!(shared.overrides, ["accounts.work.source='key'"]);
}
