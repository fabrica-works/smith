//! The agent's session child domain at random: many noisy worlds, each settled
//! with every session ended, every way a session ends reached among them.

use std::collections::BTreeSet;

use smith_domain_session::End;
use smith_domain_session::llm::Failure;
use smith_domain_tools as tools;
use smith_session_world::{Ended, World, noisy, submit_noisily};

const ITERATIONS: u32 = 100_000;

/// Every original scheduled ending class remains mandatory across the full sweep.
/// World contract: domain/session.md, sections 4, 6, 10 and 12.
const EXPECTED_ENDS: [&str; 11] =
    ["busy", "invalid", "closed", "failed", "timed out", "Turns", "Input", "Output", "CacheRead", "CacheWrite", "Time"];

/// A thousand worlds with random limits, faults, schedules and openers: each
/// settles, with every session ended once and nothing left alive or in flight
/// (checked by `World::run`), and between them they reach every way a session
/// can yield and end.
#[test]
fn random_worlds_settle_with_every_session_ended() {
    let mut ends = BTreeSet::new();
    let mut stops = BTreeSet::new();
    let (mut stale, mut invalid, mut not_run, mut parallel, mut op_timeouts) = (0, 0, 0, 0, 0);
    let mut most_runs = 0;
    let mut concrete_turns = 0;
    let mut failures = BTreeSet::new();
    let mut races = [0; 4];
    let mut served = [0; 6];
    for seed in 0..1000 {
        let settings = noisy(seed);
        let mut world = World::new(settings);
        submit_noisily(&mut world, &settings, seed);
        world.run(ITERATIONS);
        stale += world.stats().stale;
        parallel = parallel.max(world.stats().most_parallel);
        most_runs = most_runs.max(world.stats().most_runs);
        op_timeouts += world.stats().op_timeouts;
        invalid += world.told().invalid_calls;
        not_run += world.stats().not_run;
        let stats = world.stats();
        let delegated = [
            stats.delegates,
            stats.withdraws,
            stats.answered_after_withdraw,
            stats.delegate_timeouts,
            stats.finishes_refused,
            stats.finishes_accepted,
        ];
        for (count, more) in served.iter_mut().zip(delegated) {
            *count += more;
        }
        let won = [
            stats.answered_after_cancel,
            stats.failed_after_cancel,
            stats.done_after_cancel,
            stats.closed_while_closing,
        ];
        for (race, count) in races.iter_mut().zip(won) {
            *race += count;
        }
        for (_, session) in world.sessions() {
            concrete_turns += session.records.len();
            if let Some(Ended { end: End::Failed { failure, .. }, .. }) = session.ended {
                failures.insert(format!("{failure:?}"));
            }
            for (stop, _) in &session.yields {
                stops.insert(format!("{stop:?}"));
            }
            let kind = match &session.ended.as_ref().expect("every session ends").end {
                End::Busy => "busy".into(),
                End::Invalid => "invalid".into(),
                End::Closed => "closed".into(),
                End::Failed { failure: Failure::TimedOut { .. }, .. } => "timed out".into(),
                End::Failed { .. } | End::NoSummary { .. } => "failed".into(),
                End::Budget { spent } => format!("{spent:?}"),
                End::TranscriptRefused { .. } | End::PriceOverflow | End::UsageOverflow => {
                    unreachable!("bounded scheduled peers neither restore nor overflow")
                }
            };
            ends.insert(kind);
        }
    }
    assert_eq!(ends, EXPECTED_ENDS.into_iter().map(String::from).collect());
    assert!(concrete_turns > 0, "accepted provider completions emitted concrete checked Turns");
    println!(
        "canonical scheduled hits: concrete_turns={concrete_turns}, stale={stale}, invalid={invalid}, not_run={not_run}, parallel={parallel}, most_runs={most_runs}, op_timeouts={op_timeouts}, delegated={served:?}, races={races:?}, ends={ends:?}, stops={stops:?}"
    );
    assert_eq!(stops, ["Done", "Malformed", "Refused", "Truncated"].into_iter().map(String::from).collect());
    assert!(stale > 0, "some continues and closes reached sessions that had ended");
    assert!(parallel > 1, "some reads ran side by side");
    assert!(most_runs > parallel, "an iteration held more runs, ended ones among them, than a batch");
    assert!(
        served.iter().all(|&count| count > 0),
        "the opener served calls, some withdrawn, some answered all the same, some late, finishes refused and accepted: {served:?}"
    );
    assert!(
        races.iter().all(|&count| count > 0),
        "calls, failures and runs won races with cancels, and closes came twice: {races:?}"
    );
    for failure in ["Overloaded", "Unavailable", "Limit { which: Window, bound: 65536 }", "Unauthorized"] {
        assert!(failures.contains(failure), "some session failed as {failure}: {failures:?}");
    }
    assert!(op_timeouts > 0, "some operations of the tools ran out of time");
    assert!(
        invalid > 0 && not_run > 0,
        "some calls were malformed, and some were cut short and not run: {invalid} {not_run}"
    );
}

#[test]
fn drawn_small_windows_compact_within_them() {
    use smith_domain_session::{Event, llm, record};
    use smith_session_world::recorded::{World, opening};
    let mut rng = skein_lib::Rng::new(4712);
    for seed in 0..100 {
        let mut world = World::new(seed, 256);
        world.env.limits.spend = 10_000;
        let mut start = opening(None, world.env.limits.spend);
        start.spec.window = u32::try_from(rng.between(2_000, 3_000)).expect("small window");
        start.spec.output = 512;
        let window = start.spec.window;
        world.open(start);
        let text =
            |bytes: &[u8]| Box::new([llm::Block::Text { text: bytes.into(), replay: None }]) as Box<[llm::Block]>;
        let usage =
            llm::Usage { input_tokens: Some(u64::from(window) * 4 / 5), output_tokens: Some(32), ..llm::Usage::ZERO };
        world.complete(text(b"answer"), llm::Stop::EndTurn, usage);
        world.step(Event::Continue { session: world.session.expect("opened"), content: b"continue".as_slice().into() });
        world.complete(text(b"summary"), llm::Stop::EndTurn, llm::Usage { output_tokens: Some(8), ..usage });
        assert!(matches!(world.turns.last().expect("window turn").kind, record::Kind::Window(_)));
        let prompt = world.prompts.last().expect("new window prompt");
        smith_session_world::assert_prompt_bound(prompt.input_bytes().expect("bounded") - 7 + 8, window);
        world.close();
    }
}

#[test]
fn no_drawn_session_ends_because_its_history_grew() {
    use smith_domain_session::{Event, llm, record};
    use smith_session_world::recorded::{World, opening, written_calls};
    let mut rng = skein_lib::Rng::new(4717);
    for seed in 0..100 {
        let mut world = World::new(seed, 256);
        world.env.limits.history_bytes = rng.between(24_576, 32_768);
        world.open(opening(None, world.env.limits.spend));
        world.complete(written_calls(16), llm::Stop::ToolUse, llm::Usage::ZERO);
        for _ in 0..16 {
            let owner = *world.delegated.last().expect("every call runs");
            let count = usize::try_from(rng.between(15_000, 16_384)).expect("bounded text");
            world.step(Event::Answered { owner, text: vec![b'r'; count].into_boxed_slice(), error: false, spent: 0 });
        }
        assert!(world.end.is_none());
        world.complete(
            Box::new([llm::Block::Text { text: b"summary".as_slice().into(), replay: None }]),
            llm::Stop::EndTurn,
            llm::Usage::ZERO,
        );
        assert!(matches!(world.turns.last().expect("window turn").kind, record::Kind::Window(_)));
        world.close();
    }
}

/// The parent holds a step until its complete fact reserve is available.
#[test]
fn a_fact_queue_at_its_bound_holds_the_step_and_loses_nothing_over_many_seeds() {
    let mut holds = 0;
    for seed in 0..100 {
        let run = |minimum| {
            let mut settings = noisy(seed);
            if minimum {
                settings.agent.tools.facts =
                    (settings.agent.parallel_tools + 2) * tools::max_facts(&settings.agent.tools);
                settings.agent.facts = smith_domain_session::max_facts(&settings.agent);
            }
            let mut world = World::new(settings);
            submit_noisily(&mut world, &settings, seed);
            world.run(ITERATIONS);
            (world.fact_holds(), world.stats(), world.told())
        };
        let (_, stats, told) = run(false);
        assert!(told.ended > 0 && stats.calls > 0, "seed {seed}: actual calls settled");
        let (held, same, all) = run(true);
        holds += held;
        assert!(all.ended > 0 && same.calls > 0, "seed {seed}: every pressured call settled");
    }
    assert!(holds > 0, "actual pending work was held for native facts");
}

#[test]
fn drawn_ratios_and_rendered_sizes_send_nothing_past_the_line() {
    use skein_lib::{Rng, Time};
    use smith_session_world::{Count, Settings, spec};
    const SEEDS: u64 = 1_000;
    const LINE_MIN: u64 = 512;
    const LINE_MAX: u64 = 2_048;
    const TASK_BYTES: u64 = 32;
    let mut ratios = BTreeSet::new();
    let (mut guarded, mut refused, mut expanded) = (0, 0, 0);
    let mut rng = Rng::new(4102);
    for seed in 0..SEEDS {
        let calm = Settings::calm(seed);
        let settings = Settings { nudges: Count { min: 0, max: 2 }, ..calm };
        let mut world = World::new(settings);
        let line = rng.between(LINE_MIN, LINE_MAX);
        let bytes = usize::try_from(rng.between(1, TASK_BYTES)).expect("small task");
        let opener = world.submit(Time::ZERO, smith_domain_session::Spec { line, ..spec(&vec![b'x'; bytes]) });
        world.run(ITERATIONS);
        for prompt in &world.session(opener).measures {
            guarded += u32::from(prompt.guard.is_some());
            refused += u32::from(!prompt.sent);
            if prompt.sent && prompt.ordinary {
                assert!(prompt.input <= line, "seed {seed}: {prompt:?}");
            }
            for block in &prompt.blocks {
                ratios.insert(block.ratio);
                assert!((block.counted..=block.counted * 6).contains(&block.rendered), "seed {seed}: {block:?}");
                expanded += u32::from(block.rendered > block.counted);
            }
        }
    }
    assert_eq!(ratios, (1..=u64::from(skein_llm::TOKEN_BYTES)).collect());
    assert!(guarded > 0 && refused > 0 && expanded > 0, "the sweep exercised drawn additions and unsent guards");
}
