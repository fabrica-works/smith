//! Model quantities are checked at the root entrance before provider work.
//! Contract: domain/run.md, section 4; protocol/charter.md, sections 2 and 5.

use smith_agent_world::{Job, Settings, scripted_charter, world};
use smith_domain::run;

#[test]
fn a_charter_naming_an_undeclared_model_is_refused_at_the_entrance() {
    let settings = Settings { job: Job::Reporting, ..Settings::calm(1401) };
    let mut requested = scripted_charter(&settings);
    requested.llm.model = Box::from(&b"not-served"[..]);
    let mut world = world(&settings, requested, &[0]);
    world.run(2000);
    assert_eq!(world.answer(), &run::Answer::Refused(run::Refusal::Invalid(run::Invalid::Llm)));
    assert!(world.prompts().is_empty());
    assert!(world.turns().is_empty());
}

#[test]
fn a_window_above_its_declaration_is_refused() {
    let settings = Settings { job: Job::Reporting, ..Settings::calm(1402) };
    let mut requested = scripted_charter(&settings);
    requested.llm.window = 65_537;
    let mut world = world(&settings, requested, &[0]);
    world.run(2000);
    assert_eq!(world.answer(), &run::Answer::Refused(run::Refusal::Invalid(run::Invalid::Llm)));
    assert!(world.prompts().is_empty());
    assert!(world.turns().is_empty());
}

#[test]
fn the_fake_provider_receives_the_charters_effective_output_cap() {
    let settings = Settings { job: Job::Reporting, ..Settings::calm(1403) };
    let mut requested = scripted_charter(&settings);
    requested.llm.window = 4096;
    requested.llm.output = 256;
    let mut world = world(&settings, requested, &[0]);
    world.run(2000);
    assert!(matches!(world.answer(), run::Answer::Accepted { .. }));
    assert!(!world.prompts().is_empty());
    assert!(world.prompts().iter().all(|query| query.max_tokens == 256));
}
