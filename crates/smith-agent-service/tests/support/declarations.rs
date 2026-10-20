//! One native model and its environment declaration for profile stories.

pub fn declarations(environment_bytes: u32) -> super::profile_service::profile::Configuration {
    use super::profile_service::profile::{Configuration, Endpoint, Model, OversizedReasoning};
    Configuration {
        role_bound: None,
        environment_bytes,
        endpoints: Box::new([Endpoint {
            number: 0,
            provider: skein_llm::Provider::OpenAiCodex,
            option_bytes: 0,
            connect: skein_lib::Duration::from_secs(1),
            handshake: skein_lib::Duration::from_secs(1),
            models: Box::new([Model {
                efforts: Box::new([]),
                name: Box::from(b"fake".as_slice()),
                window: 2_097_152,
                output: 65_536,
                reasoning_item: 2048,
                oversized_reasoning: OversizedReasoning::Fail,
                head: skein_lib::Duration::from_secs(1),
                idle: skein_lib::Duration::from_secs(1),
            }]),
        }]),
    }
}
