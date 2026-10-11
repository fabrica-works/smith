//! Constrained output never removes failed or child trace facts.
//! Failed runs also retain the existing withheld-ACK window sweep; the two-turn
//! child uses the original eight-turn credit while its output slots vary.
use smith_agent_process_world::{TraceCase, World, charter};

fn vocabulary(world: &World) -> Vec<(Box<[u8]>, u64)> {
    let Some(smith_events::Record { event: smith_events::Event::SessionEnded(end), .. }) = world.records().last()
    else {
        panic!("end is last")
    };
    end.emitted.iter().map(|count| (count.record.clone(), count.count)).collect()
}

#[test]
fn seeded_output_and_acknowledgement_windows_count_projection_loss_without_changing_the_trace() {
    for seed in 0..32_u64 {
        for case in [TraceCase::Failed, TraceCase::ChildAnswered] {
            let mut roomy = World::new(seed, &charter());
            roomy.observe_facts(case, 8);
            assert!(roomy.settle());
            let mut constrained = World::new(seed, &charter());
            let frames = 1 + u32::try_from(seed % 8).expect("small window");
            constrained.observe_facts(case, frames);
            if case == TraceCase::Failed {
                constrained.acknowledgement_window(1 + u32::try_from(seed % 8).expect("small window"));
            }
            assert!(constrained.settle());
            constrained.assert_event_conformance();
            roomy.assert_event_conformance();
            assert_eq!(constrained.answer(), roomy.answer(), "seed {seed}, {case:?}");
            assert_eq!(vocabulary(&constrained), vocabulary(&roomy), "seed {seed}, {case:?}");
            assert_eq!(constrained.trace_loss(), 0, "seed {seed}, {case:?}");
            assert!(constrained.channel_loss() >= 1, "the post-answer terminal is counted, seed {seed}, {case:?}");
            constrained.assert_agent_clean();
            roomy.assert_agent_clean();
        }
    }
}
