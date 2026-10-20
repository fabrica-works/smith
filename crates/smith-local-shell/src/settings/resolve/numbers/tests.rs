use super::*;
use crate::settings::Layer;
use crate::settings::tests::{settings, source};

#[test]
fn a_number_is_the_first_four_sha256_bytes_big_endian() {
    for (name, expected) in [
        ("a", 3_398_926_610),
        ("test", 2_676_412_545),
        ("codex", 1_474_186_484),
        ("work", 14_761_687),
        ("personal", 1_242_182_555),
        ("fake", 3_050_654_777),
        ("openai", 2_100_401_399),
        ("anthropic", 3_339_635_307),
        ("local", 633_310_746),
    ] {
        assert_eq!(number(name), expected, "{name}");
    }
}

#[test]
fn two_names_with_one_number_are_refused_naming_both() {
    let settings = settings(&[source(Layer::User, "[accounts.z]\nsource='none'\n")]).expect("settings");
    let refusal = refuse_collisions(&settings, &[("a".into(), 7), ("z".into(), 7)]).expect_err("collision");
    assert_eq!(refusal.key, "accounts.z", "later account");
    assert!(refusal.line().contains("accounts.a and accounts.z share number 7"), "both names");
    assert!(refusal.line().contains("fixture.toml:2"), "source origin");
}

fn checked(name: &str) -> Result<(), Refusal> {
    let settings = settings(&[]).expect("settings");
    refuse_collisions(&settings, &[(name.into(), 7)])
}

// shell.md, section 5.2 bounds account names at 32 bytes.
const NAME_BYTES: usize = 32;

#[test]
fn a_name_of_thirty_two_characters_is_accepted() {
    assert!(checked(&"a".repeat(NAME_BYTES)).is_ok(), "name at bound");
}

#[test]
fn a_name_of_thirty_three_characters_is_refused() {
    assert!(checked(&"a".repeat(NAME_BYTES + 1)).is_err(), "name past bound");
}

#[test]
fn an_uppercase_underscore_or_empty_name_is_refused() {
    for name in ["A", "a_b", ""] {
        assert!(checked(name).is_err(), "invalid {name:?}");
    }
}

#[test]
fn an_invalid_name_refusal_uses_the_first_declaration() {
    let settings =
        settings(&[source(Layer::User, "[accounts.BAD]\nsource='none'\nfile='unused'\n")]).expect("settings");
    let refusal = check(&settings).expect_err("invalid account name");
    assert!(refusal.line().contains("fixture.toml:2"), "source declared before file");
}

#[test]
fn a_collision_refusal_uses_the_first_declaration() {
    let settings = settings(&[source(Layer::User, "[accounts.z]\nsource='none'\nfile='unused'\n")]).expect("settings");
    let refusal = refuse_collisions(&settings, &[("a".into(), 7), ("z".into(), 7)]).expect_err("collision");
    assert!(refusal.line().contains("fixture.toml:2"), "source declared before file");
}
