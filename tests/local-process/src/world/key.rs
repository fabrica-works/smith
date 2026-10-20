//! A key-account byte peer using Skein's provider and the existing chat script.
use skein_fake_llm_domain::Domain;
use skein_fake_llm_protocol::{documents, provider};
use skein_fake_peers::{Transport, llm::Peer};

/// The inline chat script on the API route with one explicit test key.
#[must_use]
pub fn provider(value: Box<[u8]>) -> Peer {
    let config = skein_llm_world::fake::config();
    let mut protocol = skein_llm_world::fake::limits(&skein_llm_world::limits_for(skein_llm::Provider::OpenAi));
    protocol.documents.openai.request = 32_768;
    protocol.documents.openai.strings = 16_384;
    protocol.documents.openai.tokens = 4096;
    protocol.documents.openai.output_items = 256;
    protocol.http.body = protocol.http.body.max(32_768);
    Peer::new(
        (smith_agent_process_world::fake::loopback(), 34_443).into(),
        Transport::Tls,
        smith_agent_process_world::fake::limits(),
        provider::Config {
            echo: skein_llm::openai::Echo::NONE,
            usage_fields: documents::UsageFields::ALL,
            provider: documents::Provider::OpenAi,
            path: b"/v1/responses".as_slice().into(),
            headers: Box::new([]),
            without_credential: false,
        },
        skein_llm::Credential::key(value),
        protocol,
        Domain::try_scripted(&config, 17, super::scripts()).expect("bounded chat scripts"),
        config,
    )
    .expect("key provider peer")
}
