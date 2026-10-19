//! Both channel halves are measured under their derived deployment configuration with
//! Skein's shared counting allocator (protocol/channel.md, section 7;
//! programming-model.md, section 6.3; testing-strategy.md, section 6).

use skein_channel::StreamMode;
use skein_lib::{List, Queue};
use skein_world::domain::heap::{Counting, Meter};
use smith_host_protocol as host;
use smith_protocol_channel as agent;

#[global_allocator]
static HEAP: Counting = Counting;

#[test]
fn derived_limit_halves_and_their_opening_fit_their_combined_worst_case() {
    let agent_limits = smith_agent_process_world::limits().channel;
    let mut host_limits = host::Limits {
        bodies: agent_limits.bodies,
        start_bytes: agent_limits.start_bytes,
        charter: agent_limits.charter,
        transcript: agent_limits.transcript,
        endpoints: agent_limits.endpoints,
        channel: agent_limits.channel,
        calls: agent_limits.calls,
    };
    host_limits.channel.output_bytes = host::output_bytes(&host_limits).expect("host directional frame");
    let mut entries = List::with_capacity(1);
    entries.push(agent::Endpoint { name: Box::default(), number: 0, dialect: 0 }).expect("one endpoint");
    let endpoints = agent::Endpoints::new(entries);
    let bound = host::worst_case(&host_limits)
        .expect("checked host bound")
        .checked_add(agent::worst_case(&agent_limits).expect("checked agent bound"))
        .expect("combined bound");
    let meter = Meter::new();
    meter.start();
    let mut host = host::Component::new(&host_limits, StreamMode::Two, endpoints.clone()).expect("ceiling host");
    let agent = agent::Component::new(&agent_limits, StreamMode::Two, endpoints).expect("ceiling agent");
    let mut host_events = Queue::with_capacity(host::max_out(&host_limits).to_domain);
    let mut below = Queue::with_capacity(host::max_out(&host_limits).below);
    host.open(&mut host_events, &mut below);
    let measurement = meter.end();
    let peak = meter.check(measurement, bound, &"both derived channel halves and opening");
    assert!(peak > 0, "the allocator saw the real channel construction");
    drop((host, agent, host_events, below));
}
