//! Charter accounts are admitted before any provider effect (domain/run.md, section 4).
use smith_agent_world::{Job, Settings, scripted_charter, world};
use smith_domain::run;

#[test]
fn a_start_without_an_accounts_grant_is_invalid() {
    for (index, main, child, granted) in [(0, Some(9), Some(0), 0), (1, Some(0), Some(9), 0), (2, Some(8), Some(9), 8)]
    {
        let settings = Settings { job: Job::Reporting, ..Settings::calm(1501 + index) };
        let mut charter = scripted_charter(&settings);
        charter.llm.account = main;
        charter.models.get_mut(0).expect("child LLM").account = child;
        let mut world = world(&settings, charter, &[granted]);
        world.run(2000);
        let invalid = run::Refusal::Invalid(run::Invalid::Grants);
        assert_eq!(world.answer(), &run::Answer::Refused(invalid), "row {index}");
        assert!(world.prompts().is_empty() && world.turns().is_empty(), "row {index}");
    }
}

#[test]
fn llms_sharing_an_account_share_its_grant() {
    let settings = Settings { job: Job::Reporting, ..Settings::calm(1504) };
    let mut world = world(&settings, scripted_charter(&settings), &[0]);
    world.run(2000);
    assert!(matches!(world.answer(), run::Answer::Accepted { .. }));
    assert!(!world.prompts().is_empty());
}

#[test]
fn an_llm_with_no_account_is_admitted_with_no_grant() {
    let settings = Settings { job: Job::Reporting, ..Settings::calm(1505) };
    let mut charter = scripted_charter(&settings);
    charter.llm.account = None;
    for llm in &mut charter.models {
        llm.account = None;
    }
    let mut world = world(&settings, charter, &[]);
    world.run(2000);
    assert!(matches!(world.answer(), run::Answer::Accepted { .. }));
    assert!(!world.prompts().is_empty());
}

#[test]
fn a_held_grant_serves_a_later_start() {
    let settings = Settings { job: Job::Reporting, ..Settings::calm(1506) };
    let mut world = world(&settings, scripted_charter(&settings), &[]);
    world.hold_grant(0);
    world.run(2000);
    assert!(matches!(world.answer(), run::Answer::Accepted { .. }));
    assert!(!world.prompts().is_empty());
}
