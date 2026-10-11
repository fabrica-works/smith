//! Two slow event destinations retain maximal escaped call identities while
//! every owning step is checked by Skein's allocator (events.md, section 5.3).
#[cfg(test)]
mod tests {
    #![allow(
        clippy::disallowed_macros,
        clippy::disallowed_methods,
        clippy::wildcard_enum_match_arm,
        reason = "ordinary Rust test fixtures and boundary assertions follow programming-model.md section 10.2"
    )]
    use skein_heap::{Counting, Meter};
    use skein_io as io;
    use skein_lib::{Queue, Token, stream};
    use smith_agent_service::sinks::{Capture, Delivery, Destination, SinkLimits, Sinks, sinks_worst_case};
    use smith_events::{Effect, Event, Record, Source, ToolStarted};

    #[global_allocator]
    static HEAP: Counting = Counting;

    #[test]
    #[expect(
        clippy::print_stdout,
        reason = "record independently measured peak and boundary bytes for the proof receipt"
    )]
    fn retained_escaped_calls_and_partial_sends_stay_within_destination_ownership() {
        let limits = SinkLimits {
            destinations: 2,
            output: 8 * 1024 * 1024,
            records: smith_events::Limits { string: 65536, content: 16384, items: 32 },
            answer_bytes: 16384,
            metadata_string: 64,
            conversations: 6,
        };
        let record = Record {
            t_ms: 1,
            event: Event::ToolStarted(ToolStarted {
                run: 1,
                conversation: 1,
                call: vec![0; 65536].into_boxed_slice(),
                tool: vec![0; 65536].into_boxed_slice(),
                source: Source::Workspace,
                effect: Effect::Read,
                deadline_ms: None,
                input_bytes: 0,
                input: None,
            }),
        };
        let bound = sinks_worst_case(&limits, Capture::Everything).expect("source bound");
        let tokens = [Token::new(1), Token::new(2)];
        let mut requests = Queue::with_capacity(2);
        let meter = Meter::new();
        meter.start();
        let mut sinks = Sinks::new(&limits, Capture::Everything, 4096);
        sinks.add(Destination::Trace, tokens[0], Delivery::Complete).expect("trace");
        sinks.add(Destination::Output, tokens[1], Delivery::Complete).expect("output");
        assert!(sinks.takes());
        // One gathered fact can project two records before its next credit check.
        sinks.record(&record);
        sinks.record(&record);
        assert!(!sinks.takes(), "slow destinations hold the next fact");
        sinks.finish();
        let measured = meter.end();
        let mut peak = meter.check(measured, bound, &"retain two owning lines per destination");
        let mut sent = [0_usize; 2];
        for _ in 0_u32..2000 {
            meter.start();
            sinks.down(&mut requests);
            let measured = meter.end();
            while let Some(request) = requests.pop() {
                match request {
                    io::Request::Stream { stream: owner, down: stream::Down::Demand { room, .. } } => {
                        assert_eq!(room, 4096);
                        sinks.up(io::Event::Stream { owner, up: stream::Up::Room });
                    }
                    io::Request::Stream { stream: owner, down: stream::Down::Send(bytes) } => {
                        assert!(bytes.len() <= 4096);
                        let index = tokens.iter().position(|token| *token == owner).expect("actual writer");
                        sent[index] += bytes.len();
                        drop(bytes);
                    }
                    io::Request::Close { entity } => sinks.up(io::Event::Closed { owner: entity }),
                    other => panic!("unexpected request {other:?}"),
                }
            }
            peak = peak.max(meter.check(measured, bound, &"bounded partial send"));
            if sinks.closed() {
                break;
            }
        }
        assert!(sinks.closed());
        assert_eq!(sent[0], sent[1]);
        assert!(sent[0] > 2 * 65536 * 6, "both escaped identities crossed both boundaries");
        assert!(peak > 2 * 65536 * 6, "meter reached retained encoded output");
        assert!(sinks.pop_given_up().is_none());
        println!("SINK_MEMORY peak={peak} bound={bound} sent_per_destination={}", sent[0]);
        drop(sinks);
        assert_eq!(meter.held(), 0, "all sink ownership released");
    }
}
