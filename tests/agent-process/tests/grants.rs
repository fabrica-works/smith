use smith_agent_process_world::{World, bearer, charter, key, program, withdrawn};
use smith_channel::{InvalidStart, RunResult, StartRefusal};

fn refusal(world: &World) -> Option<InvalidStart> {
    world.answer().and_then(|answer| match answer.result() {
        RunResult::Refused(refused) => match refused.reason() {
            StartRefusal::Invalid(invalid) => Some(invalid.value().clone()),
            StartRefusal::Busy => None,
        },
        RunResult::Accepted(_) | RunResult::Parked | RunResult::Failed(_) => None,
    })
}

#[test]
fn an_uninstallable_grant_refuses_the_start_as_grants() {
    let charter = charter();
    let expected = (true, false, Some(InvalidStart::Grants), Some(skein_io::kernel::Exit::Code(0)));
    let rows = [
        (bearer(b""), false, expected.clone()),
        (key(b"not-for-codex"), false, expected.clone()),
        (withdrawn(b"account", b"value"), true, expected.clone()),
        (program(b"source"), false, expected),
    ];
    for (index, (value, openai, expected)) in rows.into_iter().enumerate() {
        let mut world = World::new(31, &charter);
        world.granting(value);
        if openai {
            world.openai();
        }
        let settled = world.settle();
        let admitted = world.observed().iter().any(|frame| frame.kind == 0x0106);
        assert_eq!((settled, admitted, refusal(&world), world.exit()), expected, "row {index}");
        world.assert_agent_clean();
    }
}

#[test]
fn a_later_uninstallable_grant_fails_the_run_as_credential() {
    for (value, openai) in [(key(b"not-for-codex"), false), (withdrawn(b"account", b"value"), true)] {
        let mut world = World::new(32, &charter());
        if openai {
            world.openai();
            world.granting(bearer(b"initial"));
        }
        world.refreshing(value);
        assert!(world.settle());
        assert!(world.answer().is_none());
        let errors = String::from_utf8(world.errors()).expect("stderr UTF-8");
        assert_eq!(
            errors.lines().last(),
            Some("smith: the run could not answer: a host credential could not be installed")
        );
        assert_eq!(world.exit(), Some(skein_io::kernel::Exit::Code(1)));
        world.assert_agent_clean();
    }
}
