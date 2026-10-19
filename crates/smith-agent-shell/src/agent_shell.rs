//! Shared agent startup and shell pass (protocol/agent.md, sections 4–6).
//! It keeps the service, startup header, inherited mount descriptors and error
//! output. It never keeps credentials outside the service or knows the driving
//! kernel, simulator or neighbours. `Agent::read` reads startup settings;
//! `Agent::new` adopts caller resources; `Host::iterate` runs the same pass in
//! the binary and worlds. `run` supplies the binary's kernel and clock.

use std::io::Write;
use std::path::Path;

use service::profile::{Layer, LimitName, Refusal};
use skein_io::kernel::{Complete, Exit, Fd, Op, Submit};
use skein_lib::{List, Queue, Time, Token, Wall};
use skein_shell::{Clock, Config as KernelConfig, Host, Kernel, drive};
use smith_agent_service as service;

use crate::config;

/// Channel and signal descriptors handed by the caller to one agent invocation.
pub struct Resources {
    pub input: Fd,
    pub output: Fd,
    pub signals: Fd,
    pub seed: u64,
    /// The process identity read by startup, or supplied by a simulated process.
    pub pid: u32,
    /// An already opened append descriptor for the configured trace, when the
    /// caller supplies startup effects. The invocation owns its close.
    pub trace: Option<Fd>,
    /// An inherited stderr descriptor whose close the invocation owns; the
    /// supplied writer owns the diagnostic bytes.
    pub error: Option<Fd>,
    /// When present, roots are adopted in Start order; unused roots close at
    /// the invocation's end instead of returning to the caller.
    pub roots: Option<Box<[Fd]>>,
}

/// The binary's agent and shell effects, hosted by a kernel or world until settlement.
pub struct Agent {
    service: service::Service,
    header: Option<smith_events::SessionStarted>,
    trace_root: Option<Fd>,
    roots: Option<std::vec::IntoIter<Fd>>,
    errors: Box<dyn Write>,
    error: Option<Fd>,
    completions: Queue<Complete>,
    submissions: Queue<Submit>,
    closing: u32,
    result: Option<Result<(), String>>,
    memory: u64,
    started: bool,
    reported: bool,
    worst: u64,
    operations: u32,
}

struct Prepared {
    service: service::Service,
    header: Option<smith_events::SessionStarted>,
    trace_root: Option<Fd>,
    worst: u64,
    operations: u32,
    queue: u32,
}

impl Agent {
    /// Read the binary's configuration and build the same invocation worlds host.
    /// Every startup refusal is written once to the supplied error output.
    pub fn read(path: &Path, resources: Resources, mut errors: Box<dyn Write>) -> Result<Self, String> {
        let configuration = read_configuration(path, errors.as_mut())?;
        Self::new(configuration, resources, errors)
    }

    /// Build a configured service and adopt its streams; `drain` announces startup.
    /// Worlds inject their clock through `iterate` and deterministic resource seeds.
    pub fn new(
        configuration: config::Configuration,
        resources: Resources,
        mut errors: Box<dyn Write>,
    ) -> Result<Self, String> {
        let memory = configuration.memory;
        let prepared = match prepare(configuration, &resources) {
            Ok(prepared) => prepared,
            Err(why) => {
                diagnostic(errors.as_mut(), &why);
                return Err(why);
            }
        };
        Ok(Self {
            service: prepared.service,
            header: prepared.header,
            trace_root: prepared.trace_root,
            roots: resources.roots.map(|roots| roots.into_vec().into_iter()),
            errors,
            error: resources.error,
            completions: Queue::with_capacity(prepared.queue),
            submissions: Queue::with_capacity(prepared.queue),
            closing: 0,
            result: None,
            memory,
            started: false,
            reported: false,
            worst: prepared.worst,
            operations: prepared.operations,
        })
    }

    /// The settled invocation's success or failure, already logged to error output.
    #[must_use]
    pub fn result(&self) -> Option<Result<(), &str>> {
        if self.is_empty() {
            self.result.as_ref().map(|result| result.as_ref().copied().map_err(String::as_str))
        } else {
            None
        }
    }

    fn close(&mut self, fd: Fd) {
        self.closing = self.closing.checked_add(1).expect("bounded inherited closes");
        self.submissions.push(Submit {
            op: Token::new(u64::MAX.checked_sub(u64::from(self.closing)).expect("bounded close token")),
            kind: Op::Close { fd },
        });
    }

    fn finish(&mut self, answered: bool) {
        self.result = Some(if answered {
            Ok(())
        } else {
            let why = match service::failure(&self.service).expect("an unanswered agent keeps its first failure") {
                service::Failure::ChannelEnded => "the channel ended before the answer",
                service::Failure::WriteFailed => "the channel's output failed",
                service::Failure::Credential => "a host credential could not be installed",
                service::Failure::Send => "a required host record could not be sent",
            };
            Err(format!("the run could not answer: {why}"))
        });
    }
}

fn effective_header(configuration: &config::Configuration) -> Result<List<smith_events::EffectiveLimit>, String> {
    let named = service::profile::effective(
        &configuration.profile,
        &configuration.declarations,
        &configuration.service.limits,
        &configuration.service.llm_endpoints,
    )
    .ok_or_else(|| derivation_refusal("effective"))?;
    let header_limits =
        service::profile::header(&named, &configuration.service.channel_endpoints, &configuration.declarations)
            .ok_or_else(|| derivation_refusal("effective_names"))?;
    Ok(header_limits)
}

fn prepare(configuration: config::Configuration, resources: &Resources) -> Result<Prepared, String> {
    let root_count = resources.roots.as_ref().map_or(0, |roots| roots.len());
    if root_count > usize::try_from(configuration.service.limits.domain.run.directories).expect("directory bound") {
        return Err("inherited agent roots exceed the directory limit".into());
    }
    let extra_operations = u32::try_from(root_count)
        .map_err(|_| derivation_refusal("inherited_roots"))?
        .checked_add(u32::from(resources.error.is_some()))
        .and_then(|count| count.checked_add(u32::from(configuration.trace.is_some() && resources.trace.is_none())))
        .ok_or_else(|| derivation_refusal("routes"))?;
    let queue =
        configuration.service.limits.queue.checked_add(extra_operations).ok_or_else(|| derivation_refusal("queue"))?;
    let operations = configuration
        .service
        .limits
        .routes
        .checked_add(extra_operations)
        .ok_or_else(|| derivation_refusal("routes"))?;
    let header_limits = effective_header(&configuration)?;
    let mut header_bytes = List::<smith_events::EffectiveLimit>::worst_case(header_limits.len())
        .ok_or_else(|| derivation_refusal("startup_header"))?;
    for entry in &header_limits {
        header_bytes = header_bytes
            .checked_add(u64::try_from(entry.name.len()).map_err(|_| derivation_refusal("startup_header"))?)
            .ok_or_else(|| derivation_refusal("startup_header"))?;
    }
    let worst = service::worst_case(&configuration.service.limits, &configuration.service.llm_endpoints)
        .and_then(|bytes| {
            bytes.checked_add(if configuration.trace.is_some() {
                // The retained startup header's four strings; its structure is in Agent.
                u64::try_from(
                    b"smith"
                        .len()
                        .checked_add(env!("CARGO_PKG_VERSION").len())?
                        .checked_add(env!("SMITH_BUILD").len())?
                        .checked_add(b"standard".len())?,
                )
                .ok()?
                .checked_add(header_bytes)?
            } else {
                0
            })
        })
        .and_then(|bytes| bytes.checked_add(Queue::<Complete>::worst_case(queue)?))
        .and_then(|bytes| bytes.checked_add(Queue::<Submit>::worst_case(queue)?))
        .and_then(|bytes| bytes.checked_add(u64::try_from(size_of::<Agent>()).ok()?))
        .and_then(|bytes| bytes.checked_add(u64::try_from(root_count.checked_mul(size_of::<Fd>())?).ok()?))
        .ok_or_else(|| derivation_refusal("worst_case"))?;
    service::profile::validate::memory(worst, configuration.memory).map_err(config::profile_refusal)?;
    let profile = match configuration.profile.name {
        service::profile::Name::Standard => b"standard".as_slice(),
    };
    let header = configuration.trace.as_ref().map(|trace| smith_events::SessionStarted {
        chat: None,
        wall_ms: 0,
        agent: smith_events::Agent {
            name: b"smith".as_slice().into(),
            version: env!("CARGO_PKG_VERSION").as_bytes().into(),
            build: env!("SMITH_BUILD").as_bytes().into(),
        },
        mode: smith_events::Mode::Agent,
        containment: smith_events::Containment::Groups,
        pid: u64::from(resources.pid),
        profile: profile.into(),
        capture: trace.capture.encoded(),
        delivery: trace.delivery.encoded(),
        versions: smith_events::Versions {
            events: smith_events::VERSION,
            channel: 3,
            charter: 2,
            transcript: 5,
            chat_store: None,
        },
        limits: header_limits,
    });
    let mut service = service::Service::new(configuration.service, resources.seed)
        .map_err(|error| format!("agent configuration cannot start: {error:?}"))?;
    service
        .adopt_streams(resources.input, resources.output, resources.signals)
        .map_err(|fd| format!("agent stream {} cannot be adopted", fd.raw()))?;
    let mut trace_root = None;
    if let Some(trace) = configuration.trace {
        let fd = if let Some(fd) = resources.trace {
            fd
        } else {
            let path = std::path::absolute(&trace.path).map_err(|error| format!("trace path: {error}"))?;
            let parent = path.parent().ok_or("trace path has no parent")?;
            let file = path.file_name().ok_or("trace path has no filename")?;
            let root = skein_shell::open_root(parent).map_err(|error| format!("trace root: {error:?}"))?;
            trace_root = Some(root);
            skein_shell::open_append(root, file.as_encoded_bytes(), 0o600)
                .map_err(|error| format!("trace append: {error:?}"))?
        };
        service
            .adopt_destination(fd, service::Destination::Trace, trace.delivery, true)
            .map_err(|fd| format!("trace descriptor {} cannot be adopted", fd.raw()))?;
    } else if resources.trace.is_some() {
        return Err("an inherited trace requires a configured destination".into());
    }
    Ok(Prepared { service, header, trace_root, worst, operations, queue })
}

impl Host for Agent {
    fn drain(&mut self) {
        if !self.started {
            self.started = true;
            diagnostic(self.errors.as_mut(), "commands run in process groups and are not contained");
            diagnostic(
                self.errors.as_mut(),
                &format!("agent started; worst case {} of {} configured bytes", self.worst, self.memory),
            );
        }
        if self.result.is_some() && !self.reported {
            self.reported = true;
            if let Some(Err(why)) = &self.result {
                diagnostic(self.errors.as_mut(), why);
            }
            match service::last(&self.service) {
                Some(service::Last::Failed { failure, detail }) => {
                    let words = smith_events::words(&failure, None, None);
                    let mut line = format!(
                        "run failed: {}",
                        String::from_utf8(words.into_vec()).expect("operator words are UTF-8")
                    );
                    let detail = smith_events::clip(&detail);
                    if !detail.is_empty() {
                        line.push_str(": ");
                        line.push_str(core::str::from_utf8(detail).expect("clipped detail is UTF-8"));
                    }
                    diagnostic(self.errors.as_mut(), &line);
                }
                Some(service::Last::Quiet | service::Last::Unanswered(_)) | None => {}
            }
        }
    }

    fn iterate(&mut self, now: Time, wall: Wall) {
        while let Some(complete) = self.completions.pop() {
            if complete.op.raw() >= u64::MAX.saturating_sub(u64::from(self.operations)) {
                assert!(complete.result.is_ok(), "inherited descriptor closes");
                self.closing = self.closing.checked_sub(1).expect("submitted inherited close");
            } else {
                self.service.completions().push(complete);
            }
        }
        if self.result.is_some() {
            return;
        }
        if let Some(mut header) = self.header.take() {
            header.wall_ms = wall.as_nanos() / 1_000_000;
            self.service.start_session(now, header);
        }
        if let Some(root) = self.trace_root.take() {
            self.close(root);
        }
        service::iterate(&mut self.service, now, wall);
        while let Some(destination) = self.service.pop_given_up() {
            let name = match destination {
                service::Destination::Trace => "trace file",
                service::Destination::Output => "standard output events",
            };
            diagnostic(
                self.errors.as_mut(),
                &format!("event destination {name} was given up; later records count as lost"),
            );
        }
        if let Some(overflow) = self.service.pop_accounting_overflow() {
            let line = match overflow {
                smith_domain::run::Overflow::Spend => "run accounting failed: spend overflow",
                smith_domain::run::Overflow::Usage => {
                    "run accounting failed: usage overflow; overflowing reported totals are unavailable"
                }
            };
            diagnostic(self.errors.as_mut(), line);
        }
        if self.service.pop_usage_unread() {
            diagnostic(self.errors.as_mut(), "process resource usage is unavailable");
        }
        while let Some(path) = self.service.root_to_open() {
            let host = std::str::from_utf8(path)
                .ok()
                .and_then(|path| std::path::absolute(Path::new(path)).ok())
                .and_then(|path| {
                    smith_protocol_channel::host_path(
                        path.as_os_str().as_encoded_bytes(),
                        self.service.host_path_bytes(),
                    )
                });
            let Some(host) = host else {
                self.service.root_opened(Err(()), Box::new([]));
                continue;
            };
            let root = match &mut self.roots {
                Some(roots) => roots.next().ok_or(()),
                None => match std::str::from_utf8(path) {
                    Ok(path) => skein_shell::open_root(Path::new(path)).map_err(|_| ()),
                    Err(_) => Err(()),
                },
            };
            self.service.root_opened(root, host);
        }
        if let Some(answered) = service::done(&self.service) {
            self.finish(answered);
            while let Some(fd) = self.roots.as_mut().and_then(Iterator::next) {
                self.close(fd);
            }
            if let Some(fd) = self.error.take() {
                self.close(fd);
            }
        }
        while let Some(submission) = self.service.submissions().pop() {
            self.submissions.push(submission);
        }
    }

    fn completions(&mut self) -> &mut Queue<Complete> {
        &mut self.completions
    }

    fn submissions(&mut self) -> &mut Queue<Submit> {
        &mut self.submissions
    }

    fn work_pending(&self, now: Time) -> bool {
        (self.result.is_none() && service::work_pending(&self.service, now))
            || !self.completions.is_empty()
            || !self.submissions.is_empty()
    }

    fn next_deadline(&self) -> Option<Time> {
        if self.result.is_none() { service::next_deadline(&self.service) } else { None }
    }

    fn next_policy_deadline(&self) -> Option<Time> {
        if self.result.is_none() { service::next_policy_deadline(&self.service) } else { None }
    }

    fn is_empty(&self) -> bool {
        self.result.is_some() && self.closing == 0 && self.completions.is_empty() && self.submissions.is_empty()
    }

    fn exit(&self) -> Option<Exit> {
        self.result().map(|result| Exit::Code(u8::from(result.is_err())))
    }

    fn worst_case(&self) -> u64 {
        self.worst
    }

    fn operations(&self) -> u32 {
        self.operations
    }
}

fn read_configuration(path: &Path, errors: &mut dyn Write) -> Result<config::Configuration, String> {
    config::read(path).inspect_err(|why| diagnostic(errors, why))
}

fn diagnostic(errors: &mut dyn Write, line: &str) {
    // Diagnostics are best effort, as trace records are; a broken operator
    // output cannot prevent the channel's answer or its lower settlement.
    drop(writeln!(errors, "smith: {line}"));
}

/// Run the shipped agent command; startup and final failures are already logged.
pub fn run(path: &Path, mut errors: Box<dyn Write>) -> Result<Exit, String> {
    let configuration = read_configuration(path, errors.as_mut())?;
    let seed = skein_shell::seed()
        .map_err(|errno| format!("kernel seed failed (errno {errno})"))
        .inspect_err(|why| diagnostic(errors.as_mut(), why))?;
    let signals = skein_shell::open_termination_signals()
        .map_err(|errno| format!("termination signals cannot open (errno {errno})"))
        .inspect_err(|why| diagnostic(errors.as_mut(), why))?;
    let mut agent = Agent::new(
        configuration,
        Resources {
            input: Fd::new(0),
            output: Fd::new(1),
            signals,
            seed,
            pid: skein_shell::process_id(),
            trace: None,
            error: None,
            roots: None,
        },
        errors,
    )?;
    let mut kernel = Kernel::open(KernelConfig { operations: agent.operations() })
        .map_err(|error| format!("agent kernel cannot open: {error}"))
        .inspect_err(|why| diagnostic(agent.errors.as_mut(), why))?;
    Ok(drive(&mut kernel, &Clock::new(), &mut agent))
}

fn derivation_refusal(field: &'static str) -> String {
    config::profile_refusal(Refusal::Derived { limit: LimitName::deployment(Layer::Service, field) })
}

/// The shipped agent identity, shared with the product's event header.
#[must_use]
pub fn identity() -> smith_events::Agent {
    smith_events::Agent {
        name: b"smith".as_slice().into(),
        version: env!("CARGO_PKG_VERSION").as_bytes().into(),
        build: env!("SMITH_BUILD").as_bytes().into(),
    }
}
