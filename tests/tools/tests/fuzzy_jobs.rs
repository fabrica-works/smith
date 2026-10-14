//! Jobs' polls, stops, cancellations and ends drawn among edits.
use skein_fake_checkout::Exit;
use skein_lib::{Duration, Time};
use smith_domain_tools::{Call, Grants};
use smith_tools_world::calls::{edit, job, shell_after};
use smith_tools_world::fixture::{Fixture, program};
use smith_tools_world::{Settings, Span, Step, World};

#[test]
fn jobs_polls_and_edits_interleave_with_one_terminal_each() {
    for seed in 0..100 {
        let mut settings = Settings::calm(seed);
        settings.think = Span::millis(0, 0);
        let mut fixture = Fixture::new();
        fixture.checkout.program(b"build", program(20_000, b"output", Exit::Code(0), &[]));
        let authority = fixture.authority(Grants { inspect: true, modify: true, shell: true });
        let mut script = Vec::new();
        for number in 1..=4 {
            script.push(Step::Calls(vec![shell_after(b"build", 1)]));
            script.push(Step::Calls(vec![edit(b"src/lib.rs", b"one", b"four", false)]));
            let mut poll = job(number, seed % 4);
            if let Call::Job { stop, .. } = &mut poll {
                *stop = (seed + u64::from(number)) % 3 == 0;
            }
            script.push(Step::Calls(vec![poll]));
            if seed % 2 == 0 {
                script.push(Step::Send(vec![job(number, 5)]));
                script.push(Step::CancelJobs);
            }
            script.push(Step::Sleep(Duration::from_secs(seed % 8)));
        }
        let mut world = World::new(settings, fixture.checkout);
        world.session(Time::ZERO, authority, script);
        world.run(10_000);
        assert_eq!(world.stats().commands, 4, "seed {seed}");
    }
}
