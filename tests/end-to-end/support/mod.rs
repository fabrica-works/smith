//! Smith's shipped-binary stories: the shared person, browser, peers and scratch
//! over Skein's real loop (testing.md, sections 2.3 and 5). Only outside
//! observations are kept; no local or agent state is available.

use skein_io::kernel::{Complete, Exit, Submit};
use skein_lib::{Duration, Queue, Time, Wall};
use skein_shell::{Clock, Host};
use skein_world::{
    Referee,
    end_to_end::{Binary, Command, Mode, Streams},
    real,
};
use smith_local_process_world::{
    process::{self, Proc},
    referee::{self, Seen},
    terminal::Terminal,
    world::Scenario,
};
use smith_real_world::{
    Scratch,
    settled::{Before, assert_no_children},
};
use std::path::Path;

pub enum Process {
    Binary(Box<Binary>),
    Script(Proc),
}
impl Process {
    fn host(&self) -> &dyn Host {
        match self {
            Self::Binary(p) => p.as_ref(),
            Self::Script(p) => p,
        }
    }
    fn host_mut(&mut self) -> &mut dyn Host {
        match self {
            Self::Binary(p) => p.as_mut(),
            Self::Script(p) => p,
        }
    }
}
impl Host for Process {
    fn drain(&mut self) {
        self.host_mut().drain();
    }

    fn iterate(&mut self, now: Time, wall: Wall) {
        self.host_mut().iterate(now, wall);
    }
    fn completions(&mut self) -> &mut Queue<Complete> {
        self.host_mut().completions()
    }
    fn submissions(&mut self) -> &mut Queue<Submit> {
        self.host_mut().submissions()
    }
    fn work_pending(&self, now: Time) -> bool {
        self.host().work_pending(now)
    }
    fn next_deadline(&self) -> Option<Time> {
        self.host().next_deadline()
    }
    fn next_policy_deadline(&self) -> Option<Time> {
        self.host().next_policy_deadline()
    }
    fn is_empty(&self) -> bool {
        self.host().is_empty()
    }
    fn worst_case(&self) -> u64 {
        self.host().worst_case()
    }
    fn operations(&self) -> u32 {
        self.host().operations()
    }
}
struct Judge {
    now: Time,
    deadline: Time,
    saved: bool,
    tokens: std::path::PathBuf,
    checkout: smith_real_world::Checkout,
    ending: referee::Ending,
    seed: u64,
    reviewed: bool,
}
impl Referee<Process> for Judge {
    fn act(&mut self, _now: Time, procs: &mut [Process]) {
        let exit = procs.iter().find_map(|p| if let Process::Binary(p) = p { p.exit_status() } else { None });
        let url = procs.iter().find_map(|p| {
            if let Process::Script(Proc::Terminal(p)) = p {
                let shown = p.shown();
                shown.windows(9).position(|s| s == b"Sign in: ").and_then(|at| {
                    let rest = shown.get(at.checked_add(9).expect("sign-in prefix position")..)?;
                    rest.iter()
                        .position(|b| *b == b'\n')
                        .map(|end| rest.get(..end).expect("newline in sign-in suffix").trim_ascii_end().to_vec())
                })
            } else {
                None
            }
        });
        for proc in procs {
            match proc {
                Process::Script(Proc::Terminal(p)) => {
                    if let Some(exit) = exit {
                        p.finished(exit);
                    }
                }
                Process::Script(Proc::Peer(p)) if exit.is_some() => p.shutdown(),
                Process::Script(Proc::Issuer(p)) if exit.is_some() => p.shutdown(),
                Process::Script(Proc::Browser(p)) => {
                    if p.pages() == 0
                        && let Some(url) = &url
                    {
                        p.visit(url);
                    }
                    if exit.is_some() {
                        p.shutdown();
                    }
                }
                Process::Binary(_) | Process::Script(_) => {}
            }
        }
    }
    fn observe(&mut self, now: Time, procs: &[Process]) {
        self.now = now;
        if now >= self.deadline {
            for proc in procs {
                match proc {
                    Process::Binary(p) => eprintln!(
                        "binary exit={:?} empty={} stderr={}",
                        p.exit_status(),
                        p.is_empty(),
                        String::from_utf8_lossy(p.stderr())
                    ),
                    Process::Script(Proc::Terminal(p)) => {
                        eprintln!("person empty={} shown={}", p.is_empty(), String::from_utf8_lossy(p.shown()));
                    }
                    Process::Script(Proc::Peer(p)) => {
                        eprintln!("queries={:?}", smith_local_process_world::llm::queries(p).count());
                    }
                    Process::Script(_) => {}
                }
            }
        }
        let authenticated = procs.iter().any(|p| matches!(p, Process::Script(Proc::Issuer(_))));
        if authenticated && !self.saved && procs.iter().any(|p| matches!(p, Process::Script(Proc::Peer(p)) if smith_local_process_world::llm::queries(p).next().is_some())) {
            let store = smith_local_shell::local_tokens::Tokens::new(&self.tokens, smith_local_shell::local_host::token_limits()).expect("private tokens");
            assert_eq!(store.load(0).expect("token read").expect("saved before grant use").access_token.as_ref(), b"access-new", "token persists before provider query");
            self.saved = true;
        }
        let shown = procs
            .iter()
            .find_map(|proc| match proc {
                Process::Script(Proc::Terminal(person)) => Some(person.shown()),
                Process::Binary(_) | Process::Script(_) => None,
            })
            .unwrap_or_default();
        for proc in procs {
            if let Process::Binary(p) = proc
                && let Some(exit) = p.exit_status()
            {
                assert_eq!(
                    exit,
                    Exit::Code(0),
                    "shipped local failed: {}\nterminal: {}",
                    String::from_utf8_lossy(p.stderr()),
                    String::from_utf8_lossy(shown)
                );
            }
        }
        if !self.reviewed && procs.iter().all(Host::is_empty) {
            let seen = observations(procs, self.saved);
            referee::review_binary(&seen, &self.checkout, self.ending.clone()).assert_passed(self.seed);
            self.reviewed = true;
        }
    }
    fn next_deadline(&self) -> Option<Time> {
        Some(self.now.saturating_add(Duration::from_millis(1)).min(self.deadline))
    }
    fn overdue(&self, now: Time) -> Option<String> {
        (now >= self.deadline).then(|| "shipped Smith binary story did not settle".into())
    }
    fn passed(&self) -> bool {
        self.reviewed
    }
}

fn observations(procs: &[Process], saved: bool) -> Seen {
    let person = procs
        .iter()
        .find_map(|p| if let Process::Script(Proc::Terminal(p)) = p { Some(p) } else { None })
        .expect("person");
    let peer = procs
        .iter()
        .find_map(|p| if let Process::Script(Proc::Peer(p)) = p { Some(p) } else { None })
        .expect("provider");
    let browser = procs.iter().find_map(|p| if let Process::Script(Proc::Browser(p)) = p { Some(p) } else { None });
    let issuer = procs.iter().find_map(|p| if let Process::Script(Proc::Issuer(p)) = p { Some(p) } else { None });
    Seen {
        facts: vec![],
        shown: person.shown().to_vec(),
        errors: person.errors().to_vec(),
        exit: person.exit(),
        pushed: false,
        queries: smith_local_process_world::llm::queries(peer).cloned().collect(),
        oauth: issuer.map(|issuer| referee::OAuthSeen {
            posts: smith_local_process_world::oauth::posts(issuer.peer()),
            pages: browser.map_or(0, |b| b.pages()),
            browser_replied: browser.is_some_and(|b| b.replied()),
            saved_before_query: saved,
        }),
    }
}

#[must_use]
pub fn start(arguments: Vec<std::ffi::OsString>, directory: &Path, mode: Mode) -> Binary {
    start_program(Path::new(env!("CARGO_BIN_EXE_smith")), arguments, directory, mode)
}

fn start_program(program: &Path, arguments: Vec<std::ffi::OsString>, directory: &Path, mode: Mode) -> Binary {
    Binary::start(
        Command { program: program.into(), arguments, environment: vec![], directory: directory.into() },
        mode,
        65_536,
    )
    .unwrap_or_else(|why| panic!("shipped Smith end to end requires usable io_uring and binary startup: {why:?}"))
}

pub fn settings(scratch: &Scratch, scenario: &Scenario) {
    let trust = scratch.path().join("root.der");
    if !scenario.launch.authenticated {
        let tokens = smith_local_shell::local_tokens::Tokens::new(
            &scenario.launch.token_directory,
            smith_local_shell::local_host::token_limits(),
        )
        .expect("private tokens");
        let mut saved = tokens.load(0).expect("token load").expect("fixture token");
        let now = Clock::new().now().wall;
        if saved.expires_at <= Wall::from_nanos(now.as_nanos().saturating_add(Duration::from_secs(60).as_nanos())) {
            saved.expires_at = Wall::from_nanos(
                Clock::new().now().wall.as_nanos().saturating_add(Duration::from_secs(7200).as_nanos()),
            );
            tokens
                .save(
                    0,
                    &skein_oauth::encode_record(&saved, &smith_local_shell::local_host::token_limits())
                        .expect("token encoding"),
                )
                .expect("fresh fixture grant");
        }
    }
    let accounts = process::accounts_with(scenario.launch.authenticated, true, Some(&trust));
    let accounts: Vec<_> = accounts.iter().map(|account| {
        let oauth = account.oauth.as_ref().map(|o| serde_json::json!({"authorization_url":o.authorization_url,"token_endpoint":o.token_endpoint,"client_id":o.client_id,"redirect_uri":o.redirect_uri,"scope":o.scope,"address":o.address,"server_name":o.server_name,"trust_der":o.trust_der,"json":o.json}));
        serde_json::json!({"number":account.number,"account_id":account.account_id,"oauth":oauth})
    }).collect();
    let mut source = serde_json::json!({
        "agent":{"profile":{"name":"standard","declared":{"memory":2_147_483_648_u64,"llm_pool":134_217_728_u64,"conversations":2},"policy":{"group_stop":10}},
            "endpoints":[{"name":"fake","number":0,"dialect":0,"account":0,"provider":"codex","connect":10000,"handshake":10000,"address":"127.0.0.1:34443","server_name":"skein.test","trust_der":trust,"identity":"plain"}],
            "environment":[],"trace":{"path":scratch.path().join("agent-trace.jsonl"),"capture":"calls"}},
        "chat":"chat","instructions":"@local-shell Assist",
        "models":[{"endpoint":"fake","name":"fake","window":2_097_152,"output":65536,"reasoning_item":2048,"head":60000,"idle":30000,"input_price":1_250_000,"cached_price":125_000,"output_price":10_000_000,"price_unit":1_000_000}],
        "budget":{"turns":512,"spend":25_000_000,"seconds":3600},"waiting_seconds":30,
        "contract":{"form":"report","max":128},"token_directory":scenario.launch.token_directory,
        "accounts":accounts
    });
    if scenario.launch.change {
        source.as_object_mut().expect("settings object").insert(
            "directories".into(),
            serde_json::json!([{"name":"repo","path":scratch.path().join("repo"),"writable":true,"git":true}]),
        );
        source.as_object_mut().expect("settings object").insert("contract".into(), serde_json::json!({"form":"change","checks_must_pass":scenario.launch.tools,"fields":[{"name":"title","max":256},{"name":"body","max":1024}]}));
    }
    if scenario.launch.tools {
        source.as_object_mut().expect("settings object").insert(
            "conventions".into(),
            serde_json::json!({"guide":".smith-test/guide","checks":".smith-test/check"}),
        );
    }
    std::fs::write(scratch.path().join("settings.json"), serde_json::to_vec(&source).expect("settings"))
        .expect("settings fixture");
}

#[must_use]
pub fn run(scratch: &Scratch, scenario: Scenario) -> Seen {
    run_with_provider(scratch, scenario, None)
}

pub fn run_with_provider(
    scratch: &Scratch,
    mut scenario: Scenario,
    provider: Option<skein_fake_peers::llm::Peer>,
) -> Seen {
    assert_no_children();
    scenario.launch.tls = true;
    settings(scratch, &scenario);
    let path = trace_path(scratch.path(), scenario.launch.in_process);
    let old_trace = std::fs::read(&path).unwrap_or_default();
    let before = Before::new(scratch.path());
    let clock = Clock::new();
    let now = clock.now().now;
    let mut binary = start(
        vec!["local".into(), scratch.path().join("settings.json").into(), scratch.path().into()],
        scratch.path(),
        Mode::Terminal,
    );
    let streams = binary.take_streams().expect("scripted terminal");
    let descriptors = streams.descriptors();
    let Streams::Terminal { stream } = streams else { unreachable!("person on terminal") };
    let mut world = real::World::new(Judge {
        now,
        deadline: now.saturating_add(Duration::from_secs(5)),
        saved: false,
        tokens: scenario.launch.token_directory.clone(),
        checkout: if scenario.launch.change { scratch.checkout() } else { smith_real_world::Checkout::default() },
        ending: scenario.ending.clone(),
        seed: scenario.launch.seed,
        reviewed: false,
    });
    world.spawn_with_fds(binary.descriptors(), || Process::Binary(Box::new(binary)));
    world.spawn_with_fds(descriptors, || {
        Process::Script(Proc::Terminal(Box::new(Terminal::attached(stream, scenario.commands.clone()))))
    });
    world.spawn(|| Process::Script(Proc::Peer(Box::new(provider.unwrap_or_else(|| scenario.provider())))));
    if scenario.launch.authenticated {
        world.spawn(|| {
            Process::Script(Proc::Issuer(Box::new(process::IssuerProcess::configured(
                skein_fake_peers::Transport::Tls,
            ))))
        });
        world.spawn(|| Process::Script(Proc::Browser(Box::default())));
    }
    let outcome = world.run(&clock, Duration::from_secs(6));
    assert!(outcome.procs.iter().all(Host::is_empty), "binary and scripted processes drained");
    let seen = observations(&outcome.procs, scenario.launch.authenticated);
    before.assert_binary_expected(&scenario, &seen);
    let records = read_trace(scratch.path(), &old_trace);
    assert!(
        records.iter().any(|record| matches!(&record.event,
            smith_events::Event::ToolStarted(started) if started.tool.as_ref() == b"finish")),
        "agent actually called finish"
    );
    let trace = std::fs::read(path).expect("agent trace");
    for credential in [b"access-new".as_slice(), b"refresh-new".as_slice()] {
        assert!(!trace.windows(credential.len()).any(|bytes| bytes == credential), "trace excludes credentials");
    }
    assert_no_children();
    seen
}

/// Decode actual append bytes within the producer's configured receiving bounds.
/// Each appended invocation is checked independently; a prior prefix stays untouched.
#[must_use]
pub fn read_trace(directory: &Path, prefix: &[u8]) -> Vec<smith_events::Record> {
    let document = std::fs::read(directory.join("agent.json")).expect("actual agent configuration");
    let configuration = smith_agent_shell::config::parse(&document).expect("actual producer configuration");
    let events = configuration.service.limits.events.expect("configured event producer");
    let limits = events.sinks.records;
    let ceiling = events.sinks.line_ceiling(events.capture).expect("actual writer-derived line ceiling");
    let bytes =
        std::fs::read(configuration.trace.expect("configured agent destination").path).expect("actual append trace");
    assert!(bytes.starts_with(prefix), "durable trace prefix stays unchanged");
    let mut reader = smith_events::Reader::with_ceiling(&limits, ceiling)
        .unwrap_or_else(|why| panic!("configured receiving bounds {limits:?}: {why:?}"));
    let mut records = Vec::new();
    for chunk in bytes.get(prefix.len()..).expect("prefix retained").chunks(7) {
        let mut remaining = chunk;
        while !remaining.is_empty() {
            let line = reader.feed(remaining).expect("typed producer line");
            assert!(line.consumed > 0, "bounded reader makes progress");
            remaining = remaining.get(line.consumed..).expect("consumed input");
            if line.complete {
                records.push(line.record.expect("the writing version names a known record"));
            }
        }
    }
    reader.finish().expect("the append file ends between complete records");
    let mut beginning = 0_usize;
    for (index, record) in records.iter().enumerate() {
        if matches!(&record.event, smith_events::Event::SessionEnded(_)) {
            smith_agent_process_world::assert_event_conformance(
                records.get(beginning..=index).expect("one actual process stream"),
            );
            beginning = index.checked_add(1).expect("bounded trace inventory");
        }
    }
    assert_eq!(
        beginning,
        records.len(),
        "each native invocation has its authoritative end; record types {:?}",
        records.iter().map(|record| std::mem::discriminant(&record.event)).collect::<Vec<_>>()
    );
    assert!(!records.is_empty(), "the shipped process wrote its configured trace");
    records
}

struct Refusal {
    now: Time,
    deadline: Time,
}
impl Referee<Process> for Refusal {
    fn act(&mut self, _now: Time, procs: &mut [Process]) {
        let exit = procs.iter().find_map(|p| if let Process::Binary(p) = p { p.exit_status() } else { None });
        if let Some(exit) = exit {
            for p in procs {
                if let Process::Script(Proc::Terminal(p)) = p {
                    p.finished(exit);
                }
            }
        }
    }
    fn observe(&mut self, now: Time, procs: &[Process]) {
        self.now = now;
        if now >= self.deadline {
            for process in procs {
                match process {
                    Process::Binary(binary) => eprintln!(
                        "binary exit={:?} empty={} stderr={}",
                        binary.exit_status(),
                        binary.is_empty(),
                        String::from_utf8_lossy(binary.stderr())
                    ),
                    Process::Script(Proc::Terminal(person)) => eprintln!(
                        "terminal empty={} shown={}",
                        person.is_empty(),
                        String::from_utf8_lossy(person.shown())
                    ),
                    Process::Script(_) => {}
                }
            }
        }
    }
    fn next_deadline(&self) -> Option<Time> {
        Some(self.now.saturating_add(Duration::from_millis(1)).min(self.deadline))
    }
    fn overdue(&self, now: Time) -> Option<String> {
        (now >= self.deadline).then(|| "startup refusal did not exit and drain".into())
    }
    fn passed(&self) -> bool {
        true
    }
}

#[must_use]
pub fn refusal(scratch: &Scratch, arguments: Vec<std::ffi::OsString>) -> Vec<u8> {
    refusal_program(Path::new(env!("CARGO_BIN_EXE_smith")), scratch, arguments)
}

#[must_use]
pub fn refusal_program(program: &Path, scratch: &Scratch, arguments: Vec<std::ffi::OsString>) -> Vec<u8> {
    let refusal_code = if arguments.first().is_some_and(|command| command == "local") { 2 } else { 1 };
    refusal_program_code(program, scratch, arguments, refusal_code)
}

#[must_use]
pub fn refusal_code(scratch: &Scratch, arguments: Vec<std::ffi::OsString>, code: u8) -> Vec<u8> {
    refusal_program_code(Path::new(env!("CARGO_BIN_EXE_smith")), scratch, arguments, code)
}

fn refusal_program_code(
    program: &Path,
    scratch: &Scratch,
    arguments: Vec<std::ffi::OsString>,
    refusal_code: u8,
) -> Vec<u8> {
    assert_no_children();
    let clock = Clock::new();
    let now = clock.now().now;
    let mut binary = start_program(program, arguments, scratch.path(), Mode::Terminal);
    let streams = binary.take_streams().expect("startup terminal");
    let descriptors = streams.descriptors();
    let Streams::Terminal { stream } = streams else { unreachable!("startup on terminal") };
    let mut world = real::World::new(Refusal { now, deadline: now.saturating_add(Duration::from_secs(1)) });
    world.spawn_with_fds(binary.descriptors(), || Process::Binary(Box::new(binary)));
    world.spawn_with_fds(descriptors, || {
        Process::Script(Proc::Terminal(Box::new(Terminal::attached(
            stream,
            vec![smith_local_process_world::terminal::Action::Eof],
        ))))
    });
    let outcome = world.run(&clock, Duration::from_secs(2));
    assert!(outcome.procs.iter().all(Host::is_empty), "startup refusal drains all processes");
    let Process::Binary(binary) = outcome.procs.first().expect("binary process") else {
        unreachable!("binary observer")
    };
    assert_eq!(binary.exit_status(), Some(Exit::Code(refusal_code)), "startup refusal keeps its command category");
    let Process::Script(Proc::Terminal(person)) = outcome.procs.get(1).expect("terminal process") else {
        unreachable!("person")
    };
    assert!(person.shown().is_empty(), "startup refusal opens no channel or chat");
    assert_no_children();
    binary.stderr().to_vec()
}

pub mod live_run;

/// Drive the actual local child to a refused run, without any provider process.
#[must_use]
pub fn refused_run(scratch: &Scratch) -> Vec<smith_events::Record> {
    assert_no_children();
    let clock = Clock::new();
    let now = clock.now().now;
    let mut binary = start(
        vec!["local".into(), scratch.path().join("settings.json").into(), scratch.path().into()],
        scratch.path(),
        Mode::Terminal,
    );
    let streams = binary.take_streams().expect("actual local terminal");
    let descriptors = streams.descriptors();
    let Streams::Terminal { stream } = streams else { unreachable!("local terminal") };
    let mut world = real::World::new(Refusal { now, deadline: now.saturating_add(Duration::from_secs(3)) });
    world.spawn_with_fds(binary.descriptors(), || Process::Binary(Box::new(binary)));
    world.spawn_with_fds(descriptors, || {
        Process::Script(Proc::Terminal(Box::new(Terminal::attached(
            stream,
            vec![
                smith_local_process_world::terminal::Action::Send(Box::from(&b"A refused run\n"[..])),
                smith_local_process_world::terminal::Action::Eof,
            ],
        ))))
    });
    let outcome = world.run(&clock, Duration::from_secs(4));
    assert!(outcome.procs.iter().all(Host::is_empty), "local and actual agent drain");
    let Process::Binary(binary) = outcome.procs.first().expect("binary process") else {
        unreachable!("binary observer")
    };
    assert_eq!(
        binary.exit_status(),
        Some(Exit::Code(0)),
        "input EOF exits the interactive host: {}",
        String::from_utf8_lossy(binary.stderr())
    );
    let records = read_trace(scratch.path(), &[]);
    assert_no_children();
    records
}

/// The agent's expected destination from the story's selected placement.
#[must_use]
pub fn trace_path(directory: &Path, in_process: bool) -> std::path::PathBuf {
    directory.join(if in_process { "agent-trace.jsonl" } else { "agent-trace.jsonl.agent" })
}
