use super::*;
use crate::settings::{Origin, Sourced};

#[test]
fn the_setting_is_the_base() {
    let settings = Settings {
        paths: crate::settings::Paths::default(),
        workspace: "/unused".into(),
        values: [(
            "credentials".into(),
            Sourced { value: toml::Value::String("/chosen".into()), origin: Origin::BuiltIn },
        )]
        .into(),
    };
    let directory = Directory::of(&settings).expect("base");
    assert_eq!(directory.key("work"), PathBuf::from("/chosen/keys/work"), "setting wins");
}

#[test]
fn the_state_home_is_the_default_base() {
    let directory = Directory::default_in(Some(OsStr::new("/state")), Some(OsStr::new("/home"))).expect("base");
    assert_eq!(directory.key("work"), PathBuf::from("/state/smith/keys/work"), "state default");
}

#[test]
fn the_homes_local_state_is_the_next_base() {
    for state in [None, Some(OsStr::new("relative"))] {
        let directory = Directory::default_in(state, Some(OsStr::new("/home"))).expect("base");
        assert_eq!(directory.key("work"), PathBuf::from("/home/.local/state/smith/keys/work"), "home fallback");
    }
}

#[test]
fn a_relative_base_gives_no_directory() {
    assert!(Directory::from_base("relative".into()).is_none(), "absolute base required");
    assert!(
        Directory::default_in(Some(OsStr::new("relative")), Some(OsStr::new("relative"))).is_none(),
        "no absolute default"
    );
    assert!(Directory::default_in(None, None).is_none(), "no environment");
}

#[test]
fn keys_programs_tokens_live_under_the_base() {
    let directory = Directory::from_base("/base".into()).expect("base");
    assert_eq!(directory.key("work"), PathBuf::from("/base/keys/work"), "key");
    assert_eq!(directory.program("work"), PathBuf::from("/base/programs/work"), "program");
    assert_eq!(directory.token("work"), PathBuf::from("/base/tokens/work"), "token");
}
