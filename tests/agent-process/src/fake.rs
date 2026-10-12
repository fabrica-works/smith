//! Scenario configuration and observations of Skein's independent LLM peer.
//! It keeps no transport state or service state; the peer owns both byte
//! routing and its script. Contract: testing.md, sections 4 and 5.
use skein_fake_llm_domain::{self as domain, api};
use skein_fake_llm_protocol::{documents, provider};
use skein_fake_peers::{Limits, Transport, llm};
use skein_lib::Duration;

/// Bounded loopback transport and observation settings for Smith's peers.
#[must_use]
pub fn limits() -> Limits {
    Limits {
        io: skein_io::Limits {
            sockets: 8,
            intake: 32_768,
            output: 65_536,
            close_timeout: Duration::from_millis(50),
            ..super::limits().io
        },
        connections: 4,
        queue: 256,
        plaintext: 32_768,
        ciphertext: 65_536,
        observations: 256,
        observation_bytes: 4_194_304,
    }
}

/// The one-turn agent fixture's provider response, sent on plaintext loopback.
#[must_use]
pub fn peer() -> llm::Peer {
    configured(
        Box::new([api::Script {
            cue: Box::new([]),
            turns: Box::new([api::Turn {
                lines: Box::new([api::Line::Text { text: b"Hello".as_slice().into() }]),
                finish: api::Finish::Stop,
                tokens: 1,
            }]),
        }]),
        skein_llm::Credential { access_token: b"token".as_slice().into(), account_id: b"acc".as_slice().into() },
        Duration::ZERO,
    )
}

/// Build a seeded plaintext loopback peer for a local scenario.
#[must_use]
pub fn configured(scripts: Box<[api::Script]>, credential: skein_llm::Credential, latency: Duration) -> llm::Peer {
    configured_transport(scripts, credential, latency, Transport::Plaintext)
}

/// Configure the same fake domain for plaintext replay or TLS in a real loop.
#[must_use]
pub fn configured_transport(
    scripts: Box<[api::Script]>,
    credential: skein_llm::Credential,
    latency: Duration,
    transport: Transport,
) -> llm::Peer {
    configured_dialect(scripts, credential, latency, transport, false, false)
}

/// Select Anthropic's real byte codec for the native live-text scenario.
#[must_use]
pub fn configured_anthropic(scripts: Box<[api::Script]>, credential: skein_llm::Credential) -> llm::Peer {
    configured_dialect(scripts, credential, Duration::ZERO, Transport::Plaintext, true, false)
}

/// An actual independent HTTP provider rejection, with no application decoder substitute.
#[must_use]
pub fn rejected() -> llm::Peer {
    configured_dialect(
        Box::new([]),
        skein_llm::Credential { access_token: b"token".as_slice().into(), account_id: b"acc".as_slice().into() },
        Duration::ZERO,
        Transport::Plaintext,
        false,
        true,
    )
}

fn configured_dialect(
    scripts: Box<[api::Script]>,
    credential: skein_llm::Credential,
    latency: Duration,
    transport: Transport,
    anthropic: bool,
    rejected: bool,
) -> llm::Peer {
    configured_bounds(scripts, credential, latency, transport, anthropic, rejected, (32_768, 16_384))
}

fn configured_bounds(
    scripts: Box<[api::Script]>,
    credential: skein_llm::Credential,
    latency: Duration,
    transport: Transport,
    anthropic: bool,
    rejected: bool,
    bounds: (u32, u32),
) -> llm::Peer {
    let mut config = skein_llm_world::fake::config();
    config.latency_min = latency;
    config.latency_max = latency;
    config.script_bytes = config.script_bytes.max(bounds.1);
    config.answer_bytes = config.answer_bytes.max(bounds.1);
    config.query_bytes = config.query_bytes.max(bounds.1);
    if rejected {
        config.too_long = 1000;
    }
    let mut protocol = skein_llm_world::fake::limits(&skein_llm_world::limits());
    protocol.documents.openai.request = bounds.0;
    protocol.documents.openai.strings = bounds.1;
    protocol.documents.openai.answer = protocol.documents.openai.answer.max(bounds.1);
    protocol.http.body = protocol.http.body.max(u64::from(bounds.0));
    protocol.sse.event = protocol.sse.event.max(bounds.1);
    protocol.documents.openai.tokens = 4096;
    protocol.documents.openai.output_items = 256;
    protocol.documents.anthropic.output_items = 256;
    llm::Peer::new(
        (std::net::Ipv4Addr::LOCALHOST, if transport == Transport::Tls { 34_443 } else { 443 }).into(),
        transport,
        limits(),
        provider::Config {
            echo: skein_llm::openai::Echo::NONE,
            usage_fields: documents::UsageFields::ALL,
            provider: if anthropic { documents::Provider::Anthropic } else { documents::Provider::OpenAi },
            path: if anthropic {
                skein_llm::Endpoint::anthropic().target
            } else {
                skein_llm_world::call(1).endpoint.target
            },
            headers: Box::new([]),
        },
        credential,
        protocol,
        domain::Domain::try_scripted(&config, 17, scripts).expect("bounded scenario scripts"),
        config,
    )
    .expect("configured independent peer")
}

/// Decoded outside requests emitted by the independent byte peer.
pub fn queries(peer: &llm::Peer) -> impl Iterator<Item = &api::Query> {
    peer.observations().iter().filter_map(|observation| match observation {
        llm::Observation::Query { query, .. } => Some(query),
        llm::Observation::Head { .. }
        | llm::Observation::Accepted { .. }
        | llm::Observation::Answered { .. }
        | llm::Observation::Closed { .. } => None,
    })
}

/// Whether the actual script domain emitted a successful terminal.
#[must_use]
pub fn replied(peer: &llm::Peer) -> bool {
    peer.observations()
        .iter()
        .any(|observation| matches!(observation, llm::Observation::Answered { result: Ok(()), .. }))
}

/// Whether the independent provider actually terminated with the configured context rejection.
#[must_use]
pub fn context_rejected(peer: &llm::Peer) -> bool {
    peer.observations().iter().any(|observation| {
        matches!(observation, llm::Observation::Answered { result: Err(api::Error::ContextTooLong), .. })
    })
}

/// Admit the observed reasoning turn when it is replayed in the next request.
pub(crate) fn reasoning(scripts: Box<[api::Script]>) -> llm::Peer {
    configured_bounds(
        scripts,
        skein_llm::Credential { access_token: b"token".as_slice().into(), account_id: b"acc".as_slice().into() },
        Duration::ZERO,
        Transport::Plaintext,
        false,
        false,
        (131_072, 131_072),
    )
}
