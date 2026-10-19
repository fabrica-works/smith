#[cfg(test)]
mod tests {
    use skein_fake_llm_domain::api;
    use skein_lib::Duration;
    use smith_host_domain::End;
    use smith_hosts_world::{Program, World};

    #[test]
    fn an_agent_that_starts_runs_and_exits_after_its_last_word() {
        let mut world = World::new(7, Program::Service);
        world.settle();
        let seen = world.seen();
        assert!(seen.started);
        assert!(seen.admitted);
        assert!(seen.answered.is_some(), "the actual agent service supplied its last word");
        assert_eq!(
            seen.gone,
            Some(End::Exited { exit: Some(smith_host_domain::Exit::Code(0)), forced: smith_host_domain::Forced::No })
        );
        assert_eq!(seen.exits, 1);
        assert_eq!(seen.reaps, 1);
        assert!(world.peer_replied(), "the fake LLM served the child");
    }

    #[test]
    fn an_answered_agent_exits_within_its_grace_and_is_never_signalled() {
        let mut world = World::new(19, Program::Service);
        world.settle();
        let seen = world.seen();
        assert!(seen.answered.is_some());
        assert!(seen.signals.is_empty());
        assert_eq!(
            seen.gone,
            Some(End::Exited { exit: Some(smith_host_domain::Exit::Code(0)), forced: smith_host_domain::Forced::No }),
        );
    }

    #[test]
    fn an_agent_that_fails_to_start_is_reported_with_its_errors_tail() {
        let mut world = World::new(7, Program::ErrorTail);
        world.settle();
        let seen = world.seen();
        assert_eq!(seen.gone, Some(End::Unspawned));
        assert!(!seen.started);
        assert!(seen.answered.is_none());
        assert_eq!(seen.detail.as_deref(), Some(&b"agent configuration failed"[..]));
    }

    #[test]
    fn a_refused_spawn_reports_an_unstarted_agent() {
        let mut world = World::new(7, Program::Refused);
        world.settle();
        assert_eq!(world.seen().gone, Some(End::Unspawned));
    }

    #[test]
    fn an_agent_that_misses_the_openings_deadline_is_stopped_and_reported() {
        let mut world = World::new(7, Program::Silent);
        world.settle();
        let seen = world.seen();
        assert_eq!(seen.gone, Some(End::Unspawned));
        assert!(!seen.started);
        assert!(seen.answered.is_none());
    }

    #[test]
    fn an_agent_that_ignores_the_cancel_is_killed_after_the_grace() {
        let mut world = World::new(11, Program::Service);
        world.ignore_cancel();
        world.settle();
        let seen = world.seen();
        assert_eq!(
            seen.gone,
            Some(End::Exited {
                exit: Some(smith_host_domain::Exit::Signal(9)),
                forced: smith_host_domain::Forced::Killed
            })
        );
        assert_eq!(seen.signals, [smith_host_domain::Signal::Terminate, smith_host_domain::Signal::Kill]);
        assert_eq!(seen.exits, 1);
        assert_eq!(seen.reaps, 1);
    }

    #[test]
    fn a_child_killed_during_start_settles_the_write_and_the_independent_read_once() {
        let mut world = World::new(7, Program::Service);
        world.crash_after(0);
        world.settle();
        assert!(world.crashed());
        assert!(world.seen().gone.is_some());
        assert_eq!(world.seen().exits, 1);
        assert_eq!(world.seen().reaps, 1);
    }

    #[test]
    fn a_host_asking_for_text_hears_the_reply_in_pieces_before_its_turn() {
        const SEED: u64 = 37;
        const HOLD: &[u8] = b"host-reply";
        const FIRST: &[u8] = b"Hel";
        const LAST: &[u8] = b"lo";
        // The prefix's three bytes, released after one query and its live interval.
        const FIRST_BYTES: u64 = 3;
        const QUERY: usize = 1;
        const RELEASE_AFTER: Duration = Duration::from_secs(1);
        let mut world = World::new(SEED, Program::Service);
        world.ask(true, false, false);
        world.script(Box::new([api::Turn {
            lines: Box::new([api::Line::Pieces {
                pieces: Box::new([
                    api::Piece { text: FIRST.into(), hold: None },
                    api::Piece { text: LAST.into(), hold: Some(HOLD.into()) },
                ]),
            }]),
            finish: api::Finish::Stop,
            tokens: 1,
        }]));
        world.release_after(HOLD, QUERY, RELEASE_AFTER);
        world.settle();
        let pieces: Vec<_> = world
            .seen()
            .pieces
            .iter()
            .map(|(told, piece)| (*told, piece.turn(), piece.retry(), piece.offset(), piece.bytes()))
            .collect();
        assert_eq!(pieces, [(0, 1, 0, 0, FIRST), (0, 1, 0, FIRST_BYTES, LAST)]);
        let joined: Vec<_> = world.seen().pieces.iter().flat_map(|(_, piece)| piece.bytes().iter().copied()).collect();
        assert_eq!(joined, b"Hello");
    }
}
