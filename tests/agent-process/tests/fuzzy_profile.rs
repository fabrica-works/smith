//! Declared quantities either refuse by name or satisfy the startup relationships.

use skein_lib::{Duration, Rng};
use smith_agent_service::profile::{self, Configuration, Endpoint, Model, OversizedReasoning};

#[test]
fn seeded_profiles_refuse_without_wrapping_or_keep_the_checked_relationships() {
    for seed in [19_405, 31_007, 90_451, 130_003] {
        let mut rng = Rng::new(seed);
        for case in 0_u32..512 {
            let mut profile = profile::standard();
            profile.declared.memory = 2_147_483_648;
            profile.declared.tool_payload = u32::try_from(rng.next_u64() % 65536 + 1).expect("payload");
            profile.declared.read_window = u32::try_from(rng.next_u64() % 65536 + 1).expect("read");
            profile.declared.search_bytes = u32::try_from(rng.next_u64() % 8192 + 1).expect("search");
            profile.policy.fields = u32::try_from(rng.next_u64() % 64 + 1).expect("fields");
            profile.policy.inbox = u32::try_from(rng.next_u64() % 16 + 1).expect("inbox");
            profile.policy.shell_timeout = Duration::from_secs(rng.next_u64() % 180 + 1);
            profile.policy.tool_deadline = Duration::from_secs(rng.next_u64() % 180 + 1);
            let window = u32::try_from(rng.next_u64() % 8192 + 1).expect("window");
            let output = u32::try_from(rng.next_u64() % 8192 + 1).expect("output");
            let configuration = Configuration {
                role_bound: None,
                environment_bytes: u32::try_from(rng.next_u64() % 128).expect("environment"),
                endpoints: Box::new([Endpoint {
                    provider: skein_llm::Provider::OpenAiCodex,
                    option_bytes: 0,
                    number: 7,
                    connect: Duration::from_secs(2),
                    handshake: Duration::from_secs(3),
                    models: Box::new([Model {
                        efforts: Box::new([]),
                        name: Box::from(b"one".as_slice()),
                        window,
                        output,
                        reasoning_item: 2048,
                        oversized_reasoning: OversizedReasoning::Fail,
                        head: Duration::from_secs(5),
                        idle: Duration::from_secs(11),
                    }]),
                }]),
            };
            match profile::derive(&profile, &configuration) {
                Ok(limits) => {
                    assert!(output < window, "seed={seed}, case={case}: output fits window");
                    assert!(
                        profile.declared.read_window <= profile.declared.tool_payload,
                        "seed={seed}, case={case}: default read fits payload"
                    );
                    assert!(
                        profile.policy.shell_timeout <= profile.policy.tool_deadline,
                        "seed={seed}, case={case}: shell fits tool deadline"
                    );
                    assert!(
                        profile.policy.fields <= smith_charter::CEILINGS.run_result_fields,
                        "seed={seed}, case={case}: fields fit vocabulary"
                    );
                    profile::validate::machine(&limits.machine, &limits.domain).expect("machine holds tools");
                }
                Err(refusal) => {
                    assert!(
                        refusal.check().is_some() || matches!(refusal, profile::Refusal::Llm { endpoint: 7, .. }),
                        "seed={seed}, case={case}: named Smith check or native derivation violation"
                    );
                    let words = profile::words(&refusal);
                    assert!(words.len() <= 512, "operator line bound");
                }
            }
        }
    }
}
