//! The retained one-turn admission story, expressed in product settings.
use smith_local_process_world::world::Scenario;
use smith_real_world::Scratch;

pub fn configure(scratch: &Scratch, scenario: &Scenario) {
    let _token = super::settings(scratch, scenario);
    let path = scratch.path().join("config/smith/config.toml");
    let mut settings: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).expect("actor refusal fixture")).expect("actor refusal fixture");
    let deployment = settings.get_mut("deployment").expect("deployment").as_table_mut().expect("actor refusal fixture");
    for (name, value) in [
        ("memory", 4_294_967_296),
        ("conversations", 6),
        ("tool_payload", 40_960),
        ("file_bytes", 1_048_576),
        ("read_window", 32_768),
    ] {
        deployment.insert(name.into(), toml::Value::Integer(value));
    }
    let model = settings
        .get_mut("models")
        .expect("models")
        .get_mut("fake/fake")
        .expect("model")
        .as_table_mut()
        .expect("actor refusal fixture");
    for (name, value) in [("window", 272_000), ("output", 32_000), ("reasoning_item", 65_536), ("idle", 300)] {
        model.insert(name.into(), toml::Value::Integer(value));
    }
    let budget = settings.get_mut("budget").expect("budget").as_table_mut().expect("actor refusal fixture");
    budget.insert("turns".into(), toml::Value::Integer(1));
    budget.insert("spend".into(), toml::Value::Float(1000.0));
    budget.insert("seconds".into(), toml::Value::Integer(600));
    std::fs::write(&path, toml::to_string(&settings).expect("actor refusal fixture")).expect("actor refusal fixture");
}

#[must_use]
pub fn completed(records: &[smith_events::Record]) -> Vec<&smith_events::RunCompleted> {
    records
        .iter()
        .filter_map(|record| match &record.event {
            smith_events::Event::RunCompleted(completed) => Some(completed),
            smith_events::Event::Budget(_)
            | smith_events::Event::SessionStarted(_)
            | smith_events::Event::SessionEnded(_)
            | smith_events::Event::RunStarted(_)
            | smith_events::Event::ConversationOpened(_)
            | smith_events::Event::ConversationClosed(_)
            | smith_events::Event::ResponseStarted(_)
            | smith_events::Event::ResponseCompleted(_)
            | smith_events::Event::TextDelta(_)
            | smith_events::Event::ToolStarted(_)
            | smith_events::Event::JobEnded(_)
            | smith_events::Event::ToolCompleted(_)
            | smith_events::Event::CheckStarted(_)
            | smith_events::Event::CheckCompleted(_)
            | smith_events::Event::MessageReceived(_)
            | smith_events::Event::MessageRead(_)
            | smith_events::Event::MessageUnread(_)
            | smith_events::Event::Notice(_) => None,
        })
        .collect()
}
