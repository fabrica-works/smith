//! Program budget admission and settlement (protocol/programs.md 5.7).
use super::*;
use crate::{CompletionPermit, Most};

fn running() -> (Harness, Token, Token) {
    let mut harness = Harness::new(LIMITS);
    let (run, main) = harness.running_on(1, 100, charter());
    (harness, run, main)
}

#[test]
fn an_unreserved_completion_reserves_no_spend() {
    let mut h = Harness::new(LIMITS);
    let (_, main) = h.running_on(1, 100, agents());
    let (child, _) = h.child(main, 7, families(true, false, false), 101);
    assert_eq!(crate::reserve(&mut h.domain, main, Token::new(500), Most::Unreserved), Ok(()));
    assert_eq!(crate::reserve(&mut h.domain, child, Token::new(501), Most::Bounded(BUDGET.spend)), Ok(()));
}

#[test]
fn an_unreserved_completion_settles_any_charge() {
    let (mut h, _, main) = running();
    let owner = Token::new(500);
    assert_eq!(crate::reserve(&mut h.domain, main, owner, Most::Unreserved), Ok(()));
    assert!(crate::settle_reservation(&mut h.domain, main, owner, u64::MAX));
}

#[test]
fn an_unreserved_completion_counts_its_turn() {
    let mut h = Harness::new(LIMITS);
    let mut selected = charter();
    selected.budget.turns = 1;
    let (_, main) = h.running_on(1, 100, selected);
    let owner = Token::new(500);
    assert_eq!(crate::reserve(&mut h.domain, main, owner, Most::Unreserved), Ok(()));
    assert!(crate::settle_reservation(&mut h.domain, main, owner, 1));
    drop(h.step(Event::Used { conversation: main, spend: Spend { turns: 1, ..Spend::ZERO } }));
    assert_eq!(
        crate::completion_permit(&mut h.domain, main, h.env.now, Most::Unreserved),
        CompletionPermit::Denied(Exhausted::Turns)
    );
}

#[test]
fn an_unreserved_permit_winds_down_without_spend_left() {
    let mut h = Harness::new(LIMITS);
    let mut selected = charter();
    selected.reserve = crate::charter::Reserve { turns: 1, time: Duration::from_secs(10) };
    let reserve = crate::budget::reserve_spend(selected.reserve, &selected.llm).unwrap();
    let ordinary = selected.budget.spend.checked_sub(reserve).unwrap();
    let (_, main) = h.running_on(1, 100, selected);
    drop(h.step(Event::Priced { conversation: main, own_spent: ordinary, subtree_spent: ordinary }));
    let CompletionPermit::Final { note: Some(_) } =
        crate::completion_permit(&mut h.domain, main, h.env.now, Most::Unreserved)
    else {
        panic!("ordinary spend starts finishing");
    };
}

#[test]
fn an_unreserved_permit_is_denied_past_the_budget() {
    let (mut h, _, main) = running();
    let over = BUDGET.spend.checked_add(1).unwrap();
    drop(h.step(Event::Priced { conversation: main, own_spent: over, subtree_spent: over }));
    assert_eq!(
        crate::completion_permit(&mut h.domain, main, h.env.now, Most::Unreserved),
        CompletionPermit::Denied(Exhausted::Spend)
    );
    assert!(crate::reserve(&mut h.domain, main, Token::new(500), Most::Unreserved).is_err());
}

#[test]
fn a_final_unreserved_completion_uses_the_reserve() {
    let mut h = Harness::new(LIMITS);
    let mut selected = charter();
    selected.budget.turns = 3;
    selected.reserve = crate::charter::Reserve { turns: 1, time: Duration::from_secs(10) };
    let (_, main) = h.running_on(1, 100, selected);
    drop(h.step(Event::Used { conversation: main, spend: Spend { turns: 2, ..Spend::ZERO } }));
    let CompletionPermit::Final { .. } = crate::completion_permit(&mut h.domain, main, h.env.now, Most::Unreserved)
    else {
        panic!("ordinary turns start finishing");
    };
    assert_eq!(crate::reserve(&mut h.domain, main, Token::new(500), Most::Unreserved), Ok(()));
    assert!(crate::settle_reservation(&mut h.domain, main, Token::new(500), 1));
    assert_eq!(
        crate::completion_permit(&mut h.domain, main, h.env.now, Most::Unreserved),
        CompletionPermit::Denied(Exhausted::Turns)
    );
}
