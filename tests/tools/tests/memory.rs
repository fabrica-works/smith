//! Memory stays within the worst case (programming-model.md, section 6.3), measured by
//! a counting allocator: the tools child domain with every kit holding the
//! longest authority, knowing as many files as it may at the longest paths,
//! and running as many edits, writes and reads as it may. Driven at random,
//! it is checked in `fuzzy_memory.rs`.

use skein_lib::{Duration, Env, Queue, ReplyTo, Time, Token, Wall};
use skein_world::domain::heap::{self, Meter};
use smith_domain_tools::{Domain, Done, Event, Limits, Request, Version, max_out, worst_case};
use smith_tools_world::memory::{LIMITS, authority, edit, read, write};

#[global_allocator]
static HEAP: heap::Counting = heap::Counting;

/// Fills every kit of a domain under `limits` to its limits, checking the peak
/// of the heap in every step against the worst case: it knows twice as many
/// files as it may, the oldest forgotten, then runs as many calls as it may at
/// once, writes of the files it knows and reads.
fn fill(limits: Limits) {
    let bound = worst_case(&limits).expect("the test limits fit");
    let env = Env { now: Time::ZERO, wall: Wall::EPOCH, limits };
    let mut out = Queue::with_capacity(max_out(&limits));
    let meter = Meter::new();
    let mut domain = Domain::new(&limits);
    // Each step is an iteration of its own, ending at the reclaim point. The
    // requests are the session's and io's to hold and count: each is dropped,
    // keeping only the token it names, and the step's peak checked less them.
    let mut step = |event: Event| -> Vec<Token> {
        meter.start();
        while domain.pop_fact().is_some() {}
        smith_domain_tools::step(&mut domain, &env, event, &mut out);
        domain.reclaim();
        let measured = meter.end();
        let mut named = Vec::new();
        while let Some(request) = out.pop() {
            match request {
                Request::Opened { kit, .. } => named.push(kit),
                Request::Io { owner, .. } => named.push(owner),
                Request::Answer { .. } => {}
                request @ (Request::Invalid { .. }
                | Request::Refused { .. }
                | Request::Closed { .. }
                | Request::JobsCancelled { .. }
                | Request::Job { .. }
                | Request::CancelIo { .. }) => {
                    panic!("{limits:?}: unexpected {request:?}")
                }
            }
        }
        meter.check(measured, bound, &limits);
        named
    };
    let deadline = Time::ZERO.saturating_add(Duration::from_secs(60));
    let mut file = 0;
    for session in 0..limits.kits {
        let opened = step(Event::Open { session: Token::new(session.into()), authority: authority(&limits) });
        let [kit] = opened[..] else { panic!("{limits:?}: the authority fits") };
        let list = smith_domain_tools::Call::List {
            path: smith_domain_tools::Path { absolute: true, parts: Box::new([]) },
            depth: smith_domain_tools::Depth::Whole,
            glob: None,
        };
        assert!(
            step(Event::Call {
                kit,
                observation: None,
                reply_to: ReplyTo::new(Token::new(9000)),
                call: list,
                deadline
            })
            .is_empty(),
            "namespace listing has no IO owner"
        );
        for _ in 0..limits.known_files * 2 {
            file += 1;
            let reply_to = ReplyTo::new(Token::new(file));
            let [owner] =
                step(Event::Call { kit, observation: None, reply_to, call: read(&limits, file), deadline })[..]
            else {
                panic!("{limits:?}: the read runs");
            };
            let done = Done::Loaded { content: b"x\n"[..].into(), version: Version::new([file, 0, 0, 0]) };
            assert!(step(Event::Done { owner, done }).is_empty(), "a read answers");
        }
        for call in 0..u64::from(limits.calls) {
            let call = if call < u64::from(limits.known_files) {
                if call % 2 == 0 { edit(&limits, file - call) } else { write(&limits, file - call) }
            } else {
                file += 1;
                read(&limits, file)
            };
            let reply_to = ReplyTo::new(Token::new(file + 1000));
            assert_eq!(
                step(Event::Call { kit, observation: None, reply_to, call, deadline }).len(),
                1,
                "{limits:?}: the call runs"
            );
        }
    }
    let held = meter.held();
    let known = u64::from(limits.kits) * u64::from(limits.known_files) * u64::from(limits.path_bytes);
    assert!(held >= known, "{limits:?}: every kit knows as many files as it may, at the longest paths");
    drop(domain);
}

#[test]
fn a_domain_with_every_kit_full_stays_within_its_worst_case() {
    fill(LIMITS);
    fill(Limits { kits: 16, calls: 8, repos: 8, known_files: 64, path_bytes: 511, ..LIMITS });
    fill(Limits { kits: 64, calls: 1, repos: 1, known_files: 200, path_bytes: 127, ..LIMITS });
}

fn job_step(
    domain: &mut Domain,
    env: &Env<Limits>,
    out: &mut Queue<Request>,
    meter: &Meter,
    event: Event,
) -> Vec<Request> {
    meter.start();
    while domain.pop_fact().is_some() {}
    smith_domain_tools::step(domain, env, event, out);
    domain.reclaim();
    let measured = meter.end();
    meter.check(measured, worst_case(&env.limits).expect("bounded jobs fixture"), &"held jobs and queued reads");
    std::iter::from_fn(|| out.pop()).collect()
}

fn fill_jobs() {
    use smith_domain_tools::{Budget, Call, JobEnd, JobNumber, Timeout, Wait, Yield};
    let env = Env { now: Time::ZERO, wall: Wall::EPOCH, limits: LIMITS };
    let mut out = Queue::with_capacity(max_out(&LIMITS));
    let meter = Meter::new();
    let mut domain = Domain::new(&LIMITS);
    let deadline = Time::from_nanos(u64::MAX);
    let mut kits = Vec::new();
    for session in 0..LIMITS.kits {
        let requests = job_step(
            &mut domain,
            &env,
            &mut out,
            &meter,
            Event::Open { session: Token::new(u64::from(session)), authority: authority(&LIMITS) },
        );
        let Request::Opened { kit, .. } = requests[0] else { panic!("one admitted kit") };
        let mut owners = Vec::new();
        for number in 1..=LIMITS.jobs {
            let call = Call::Shell {
                command: b"build".as_slice().into(),
                timeout: Timeout::Default,
                output: Budget::Default,
                after: Yield::Asked(Duration::ZERO),
            };
            let requests = job_step(
                &mut domain,
                &env,
                &mut out,
                &meter,
                Event::Call {
                    kit,
                    reply_to: ReplyTo::new(Token::new(u64::from(number))),
                    observation: None,
                    call,
                    deadline,
                },
            );
            let Request::Io { owner, .. } = requests[0] else { panic!("one command owner") };
            owners.push(owner);
            drop(job_step(
                &mut domain,
                &env,
                &mut out,
                &meter,
                Event::Done { owner, done: Done::Held { head: Box::default(), tail: Box::default(), dropped: 0 } },
            ));
        }
        for reader in 0..LIMITS.calls {
            let call = Call::Job { job: JobNumber(1), wait: Wait::Default, output: Budget::Default, stop: false };
            drop(job_step(
                &mut domain,
                &env,
                &mut out,
                &meter,
                Event::Call {
                    kit,
                    reply_to: ReplyTo::new(Token::new(u64::from(reader))),
                    observation: None,
                    call,
                    deadline,
                },
            ));
        }
        kits.push((kit, owners));
    }
    for (kit, owners) in &kits {
        drop(job_step(&mut domain, &env, &mut out, &meter, Event::CancelJobs { kit: *kit }));
        for (number, owner) in owners.iter().enumerate() {
            drop(job_step(
                &mut domain,
                &env,
                &mut out,
                &meter,
                Event::Done {
                    owner: *owner,
                    done: Done::Job {
                        head: if number == 0 { Box::default() } else { vec![b'h'; LIMITS.shell_head as usize].into() },
                        tail: if number == 0 { Box::default() } else { vec![b't'; LIMITS.shell_tail as usize].into() },
                        dropped: 0,
                        end: JobEnd::Cancelled,
                    },
                },
            ));
        }
    }
    for (kit, _) in kits {
        drop(job_step(&mut domain, &env, &mut out, &meter, Event::Close { kit }));
    }
    assert_eq!(domain.kits(), 0);
    assert_eq!(domain.flights(), 0);
}

#[test]
fn held_jobs_and_queued_reads_fit_the_tools_bound() {
    fill_jobs();
}
