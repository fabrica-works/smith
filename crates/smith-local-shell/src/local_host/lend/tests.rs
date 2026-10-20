use super::*;
use crate::credentials::{
    Credential, prepare,
    program_check::ProgramStates,
    tests::{Files, private_key, resolved_source},
};
use skein_lib::Wall;

// shell.md, section 7.4: the name work hashes to this number.
const WORK: u32 = 14_761_687;

fn lent(value: GrantValue, generation: u64) -> Lent {
    let mut accounts = Map::with_capacity(1);
    accounts.insert(WORK, (generation, value)).expect("one account");
    Lent::new(accounts)
}

fn key_value() -> GrantValue {
    GrantValue::Key(b"key-for-work".as_slice().into())
}

fn assert_lend(answer: Answer, account: u32, generation: u64, expected: &GrantValue) {
    match answer {
        Answer::Lend(grant, value) => {
            assert_eq!(grant.name.account, account, "account number");
            assert_eq!(grant.name.generation, generation, "fresh generation");
            assert_eq!(grant.valid, Duration::from_nanos(u64::MAX), "unrefreshed validity");
            assert!(value == *expected, "same startup value");
        }
        Answer::Missing => panic!("grant expected"),
    }
}

#[test]
fn every_key_request_lends_a_fresh_generation() {
    let mut lent = lent(key_value(), 0);
    for generation in [1, 2, 3] {
        assert_lend(lent.answer(WORK), WORK, generation, &key_value());
    }
}

#[test]
fn every_program_request_lends_a_fresh_generation() {
    let files = Files::new();
    let prepared = prepare(
        &resolved_source(&files, "program"),
        Wall::EPOCH,
        false,
        &ProgramStates::from([("codex".into(), Credential::Usable { until: None })]),
    )
    .expect("program startup");
    let mut lent = Lent::new(prepared.lendable);
    for generation in [1, 2, 3] {
        assert_lend(lent.answer(0), 0, generation, &GrantValue::Program(b"codex".as_slice().into()));
    }
}

#[test]
fn generations_advance_per_account() {
    // One key account and one program account.
    const ACCOUNTS: u32 = 2;
    const PROGRAM: u32 = 7;
    let program = GrantValue::Program(b"review".as_slice().into());
    let mut accounts = Map::with_capacity(ACCOUNTS);
    accounts.insert(WORK, (0, key_value())).expect("key account");
    accounts.insert(PROGRAM, (0, program.clone())).expect("program account");
    let mut lent = Lent::new(accounts);
    for (account, generation, value) in [
        (WORK, 1, key_value()),
        (WORK, 2, key_value()),
        (PROGRAM, 1, program.clone()),
        (WORK, 3, key_value()),
        (PROGRAM, 2, program),
    ] {
        assert_lend(lent.answer(account), account, generation, &value);
    }
}

#[test]
fn an_account_without_a_value_answers_missing() {
    let files = Files::new();
    let prepared =
        prepare(&resolved_source(&files, "none"), Wall::EPOCH, false, &ProgramStates::new()).expect("nothing startup");
    let mut lent = Lent::new(prepared.lendable);
    for account in [0, WORK] {
        assert!(matches!(lent.answer(account), Answer::Missing), "no grant for a Nothing or unknown account");
    }
}

#[test]
fn the_last_generation_lends_before_overflow_answers_missing() {
    let mut lent = lent(key_value(), u64::MAX - 1);
    assert_lend(lent.answer(WORK), WORK, u64::MAX, &key_value());
    for _ in 0..2 {
        assert!(matches!(lent.answer(WORK), Answer::Missing), "generation never wraps");
    }
}

#[test]
fn a_key_removed_after_start_still_lends_its_first_bytes() {
    let files = Files::new();
    private_key(&files);
    let prepared =
        prepare(&resolved_source(&files, "key"), Wall::EPOCH, false, &ProgramStates::new()).expect("key startup");
    std::fs::remove_file(files.0.join("keys/codex")).expect("removed after startup");
    let mut lent = Lent::new(prepared.lendable);
    for generation in [1, 2] {
        assert_lend(lent.answer(0), 0, generation, &key_value());
    }
}

#[test]
fn a_lent_answers_debug_holds_no_value() {
    let mut lent = lent(key_value(), 0);
    assert!(!format!("{lent:?}").contains("key-for-work"), "redacted stored value");
    let answer = lent.answer(WORK);
    assert!(matches!(answer, Answer::Lend(_, GrantValue::Key(_))), "lent key required");
    assert!(!format!("{answer:?}").contains("key-for-work"), "redacted grant value");
}
