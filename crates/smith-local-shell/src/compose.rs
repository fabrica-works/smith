//! Product startup and the inline local composition (shell.md, sections 4 and 9).
//! Resolution owns model/account selection; the shell opens named resources and
//! carries one kernel from the stdin phase into the local host's run.
use crate::{
    command::{Command, Exec, Prompt, Usage},
    credentials, local_shell, settings,
};
mod files;
pub(crate) use files::{Failure as ResultFileFailure, Files, ResultFile};

use skein_lib::Wall;
use skein_shell::{Clock, Config, Kernel};

/// The product's child placement; exec composes the inline kind.
#[derive(Clone, Copy)]
pub enum AgentKind {
    /// The local domain owns its agent and the native effects in this process.
    Inline,
}

/// A typed startup refusal, rendered once by the product binary.
pub enum Refused {
    /// The invocation cannot supply one bounded prompt.
    Usage(Usage),
    /// A settings value or relationship refused.
    Settings(settings::Refusal),
    /// A selected source has no usable credential.
    Credentials(credentials::Refusal),
    /// Resource opening or complete memory admission refused.
    Local(local_shell::Failure),
}

impl Refused {
    /// The product's code for this startup terminal.
    #[must_use]
    pub const fn code(&self) -> u8 {
        match self {
            Self::Usage(_) | Self::Settings(_) => 2,
            Self::Credentials(why) => why.exit_code(),
            Self::Local(why) => why.exit_code(),
        }
    }

    /// One clipped operator line, with settings and source origins preserved.
    #[must_use]
    pub fn line(&self) -> String {
        let line = match self {
            Self::Usage(why) => format!("smith: usage: {}", why.0),
            Self::Settings(why) => why.line(),
            Self::Credentials(why) => why.line(),
            Self::Local(why) => format!("smith: {}", why.detail()),
        };
        String::from_utf8_lossy(smith_events::clip(line.as_bytes())).into_owned()
    }
}

pub(crate) struct Opened {
    pub resolved: settings::resolve::Resolved,
    pub invocation: Exec,
    pub seed: u64,
    pub wall: Wall,
    pub input_closed: bool,
    pub interactive: Option<bool>,
}

/// Read the product layers and one complete prompt before any credentials are inspected.
pub(crate) fn open(invocation: Exec, wall: Wall, clock: &Clock) -> Result<(Opened, Kernel), Refused> {
    open_mode(invocation, wall, clock, None)
}

fn open_mode(
    invocation: Exec,
    wall: Wall,
    clock: &Clock,
    interactive: Option<bool>,
) -> Result<(Opened, Kernel), Refused> {
    let settings = settings::read(&invocation.shared).map_err(Refused::Settings)?;
    let command = Command::Exec(invocation);
    let resolved = settings::resolve::resolve(&settings, &command).map_err(Refused::Settings)?;
    let mut invocation = match command {
        Command::Exec(invocation) => invocation,
        Command::Check { .. } | Command::Legacy(_) | Command::Login { .. } | Command::Interactive { .. } => {
            unreachable!("one exec invocation")
        }
    };
    let operations =
        crate::Local::kernel_operations(&resolved.agent.service.limits.io, resolved.agent.service.limits.routes, true)
            .ok_or_else(|| {
                Refused::Local(local_shell::Failure::Configuration { detail: "local operation bound overflows".into() })
            })?;
    let operations = operations
        .checked_add(if invocation.output.is_some() { 4 } else { 0 })
        .ok_or_else(|| Refused::Usage(Usage("the output operation bound overflows".into())))?;
    let mut kernel = Kernel::open(Config { operations })
        .map_err(|why| Refused::Local(local_shell::Failure::Host { detail: format!("local kernel: {why}") }))?;
    let maximum = resolved.limits.line_bytes;
    let input_closed = match &invocation.prompt {
        Prompt::Stdin => true,
        Prompt::Argument(_) => false,
    };
    let prompt = match invocation.prompt {
        Prompt::Argument(bytes) => bytes,
        Prompt::Stdin => crate::front::headless::read_prompt(
            &mut kernel,
            clock,
            resolved.agent.service.limits.io,
            maximum,
            resolved.agent.memory,
        )
        .map_err(Refused::Usage)?,
    };
    if prompt.len() > usize::try_from(maximum).expect("message bound fits usize") {
        return Err(Refused::Usage(Usage("the prompt exceeds the message bound".into())));
    }
    if interactive.is_none() && prompt.trim_ascii().is_empty() {
        return Err(Refused::Usage(Usage("the prompt is empty".into())));
    }
    if std::str::from_utf8(&prompt).is_err() {
        return Err(Refused::Usage(Usage("the prompt is not UTF-8".into())));
    }
    invocation.prompt = Prompt::Argument(prompt);
    let seed = skein_shell::seed().map_err(|why| {
        Refused::Local(local_shell::Failure::Host { detail: format!("local seed failed (errno {why})") })
    })?;
    Ok((Opened { resolved, invocation, seed, wall, input_closed, interactive }, kernel))
}

struct Composed {
    local: crate::Local,
}

/// Compose the selected native values with their roots and source driver, without a configuration file.
fn compose(opened: Opened, kind: AgentKind, now: skein_lib::Time, files: &mut Files) -> Result<Composed, Refused> {
    let Opened { mut resolved, invocation, seed, wall, input_closed, interactive } = opened;
    match kind {
        AgentKind::Inline => {}
    }
    files.prepare_output(seed)?;
    files.create_temporary()?;
    if interactive.is_none()
        && let Some(chat) = invocation_chat(&invocation.shared, seed)
    {
        resolved.local.chat = chat;
    }
    let (workspace, chat_path) =
        chat_paths(&resolved.settings.workspace, &resolved.local.chat, files.temporary.as_deref())?;
    let prompt = match invocation.prompt {
        Prompt::Argument(prompt) => prompt,
        Prompt::Stdin => unreachable!("stdin phase ended"),
    };
    if interactive.is_none() {
        resolved.local.mode = smith_local_domain::Mode::Headless { prompt };
    } else {
        resolved.local.resume = true;
    }
    let prepared = credentials::prepare(&resolved, wall, interactive.is_some()).map_err(Refused::Credentials)?;
    deliver_contract(&mut resolved.local, invocation.deliver);
    let WorkspaceRoots { paths, delivery_roots, effect_roots } = workspace_roots(&resolved.local)?;
    let host_limits =
        smith_local_service::profile::host_limits(&resolved.agent.profile, &resolved.agent.service.limits).map_err(
            |why| {
                Refused::Local(local_shell::Failure::Configuration {
                    detail: String::from_utf8_lossy(&smith_agent_service::profile::words(&why)).into_owned(),
                })
            },
        )?;
    let mut limits = local_shell::service_limits(resolved.limits, host_limits, &resolved.agent.service)
        .map_err(|detail| Refused::Local(local_shell::Failure::Configuration { detail }))?;
    let (trace, header, available) = files.prepare_stream(
        &mut resolved.agent,
        &resolved.local.chat,
        &mut limits,
        invocation.json,
        wall,
        interactive.is_some(),
    )?;
    let (front, available) =
        prepare_front(interactive, input_closed, invocation.json, &resolved.agent, &limits, files, available)?;
    let environment = resolved.agent.service.environment.command_entries();
    let config = smith_local_service::Config {
        local: resolved.local,
        limits,
        endpoints: resolved.agent.service.channel_endpoints.clone(),
        paths,
        launch: smith_local_service::Launch {
            program: Box::new([]),
            arguments: Box::new([]),
            environment: Box::new([]),
            root: root(&workspace)?,
            directory: b".".as_slice().into(),
        },
        agent: smith_local_service::AgentConfig::Spawned,
    };
    let resources = crate::Resources {
        state_root: files.store_root(if interactive.is_some() {
            chat_path.parent().expect("workspace chats parent")
        } else {
            &chat_path
        })?,
        token_directory: chat_path.join("unused-tokens"),
        input: skein_io::kernel::Fd::new(0),
        output: skein_io::kernel::Fd::new(1),
        signals: skein_shell::open_termination_signals().map_err(|errno| {
            Refused::Local(local_shell::Failure::Host { detail: format!("termination signals failed (errno {errno})") })
        })?,
        delivery_roots,
        effect_roots,
        delivery_environment: environment,
        accounts: Box::new([]),
        seed,
        oauth_entropy: local_shell::oauth_entropy()
            .map_err(|detail| Refused::Local(local_shell::Failure::Host { detail }))?,
    };
    let mut local = crate::Local::within_memory(
        config,
        Some(resolved.agent.service),
        resources,
        Box::new(std::io::stderr()),
        available,
        Some(prepared),
        Some(front),
    )
    .map_err(Refused::Local)?;
    files.attach_private(&mut local)?;
    start_stream(&mut local, trace, header, now)?;
    local.result_file(files.output.take());
    files.composed();
    Ok(Composed { local })
}

fn deliver_contract(local: &mut smith_local_domain::Config, deliver: bool) {
    if deliver && let smith_local_domain::Contract::Report(_) = local.contract {
        local.contract = smith_local_domain::Contract::Change(smith_domain::run::outcome::ChangeSpec {
            checks_must_pass: true,
            fields: Box::new([
                smith_domain::run::outcome::FieldRule { name: b"title".as_slice().into(), max: 256 },
                smith_domain::run::outcome::FieldRule { name: b"body".as_slice().into(), max: 1024 },
            ]),
        });
    }
}

fn prepare_front(
    interactive: Option<bool>,
    input_closed: bool,
    json: bool,
    agent: &smith_agent_shell::config::Configuration,
    limits: &smith_local_service::Limits,
    files: &Files,
    available: u64,
) -> Result<(crate::front::Front, u64), Refused> {
    if let Some(plain) = interactive {
        let front = crate::front::interactive::Interactive::new(plain, &agent.service.limits)
            .ok_or_else(|| Refused::Usage(Usage("interactive footprint overflows".into())))?;
        return Ok((crate::front::Front::Interactive(front), files.reserve_available(available)?));
    }
    let mut front = crate::front::headless::Headless::new(
        json,
        skein_shell::prepare_output(skein_io::kernel::Fd::new(1)).map_err(|why| {
            Refused::Local(local_shell::Failure::Host { detail: format!("standard output: {why:?}") })
        })?,
        limits.local.show_bytes,
        limits.local.smith.run.outcome_bytes,
    );
    front.input_closed = input_closed;
    let available = files.reserve(&mut front, available)?;
    Ok((crate::front::Front::Headless(front), available))
}

fn start_stream(
    local: &mut crate::Local,
    trace: Option<smith_agent_shell::trace::TraceConfig>,
    mut header: Option<smith_events::SessionStarted>,
    now: skein_lib::Time,
) -> Result<(), Refused> {
    if let Some(trace) = trace {
        let parent =
            trace.path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));
        let name =
            trace.path.file_name().ok_or_else(|| Refused::Usage(Usage("the trace requires a file name".into())))?;
        let descriptor = root(parent)?;
        let fd = skein_shell::open_append(descriptor, name.as_encoded_bytes(), 0o600)
            .map_err(|why| Refused::Local(local_shell::Failure::Host { detail: format!("trace append: {why:?}") }))?;
        local
            .adopt_trace(fd, Some(descriptor), now, header.take().expect("configured header"), trace.delivery)
            .map_err(|why| {
                Refused::Local(local_shell::Failure::Host {
                    detail: format!("trace descriptor {} cannot be adopted", why.raw()),
                })
            })?;
    } else if let Some(header) = header {
        local.start_session(now, header);
    }
    Ok(())
}

struct WorkspaceRoots {
    paths: Box<[Box<[u8]>]>,
    delivery_roots: Box<[skein_io::kernel::Fd]>,
    effect_roots: Box<[skein_io::kernel::Fd]>,
}

fn workspace_roots(local: &smith_local_domain::Config) -> Result<WorkspaceRoots, Refused> {
    let directories = local.workspace.as_ref().expect("resolved workspace").directories.as_ref();
    let paths = directories.iter().map(|directory| directory.host.clone()).collect();
    let roots = || {
        directories
            .iter()
            .map(|directory| {
                skein_shell::open_root(std::path::Path::new(
                    std::str::from_utf8(&directory.host).expect("resolved host path"),
                ))
                .map_err(|errno| {
                    Refused::Local(local_shell::Failure::Host {
                        detail: format!("workspace root failed (errno {errno})"),
                    })
                })
            })
            .collect::<Result<Vec<_>, _>>()
    };
    let delivery_roots = roots()?.into_boxed_slice();
    let effect_roots = roots()?.into_boxed_slice();
    Ok(WorkspaceRoots { paths, delivery_roots, effect_roots })
}

fn chat_paths(
    workspace: &std::path::Path,
    chat_name: &[u8],
    temporary: Option<&std::path::Path>,
) -> Result<(std::path::PathBuf, std::path::PathBuf), Refused> {
    use std::{fmt::Write, os::unix::fs::DirBuilderExt, path::PathBuf};
    let chat = std::str::from_utf8(chat_name).expect("decoded chat");
    if chat == "." || chat == ".." || chat.contains(['/', '\\', '\0']) {
        return Err(Refused::Usage(Usage("the chat must be one directory name".into())));
    }
    let workspace = workspace.canonicalize().map_err(|why| host(&why))?;
    if let Some(path) = temporary {
        return Ok((workspace, path.into()));
    }
    let digest = skein_io::digest::digest(workspace.as_os_str().as_encoded_bytes());
    let mut key = String::new();
    for part in digest.0 {
        write!(&mut key, "{part:016x}").expect("writing a string");
    }
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .ok_or_else(|| Refused::Usage(Usage("HOME or XDG_STATE_HOME is required for chats".into())))?;
    let chat_path = base.join("smith/chats").join(key).join(chat);
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&chat_path).map_err(|why| host(&why))?;
    Ok((workspace, chat_path))
}

pub(crate) fn invocation_chat(shared: &crate::command::Shared, seed: u64) -> Option<Box<[u8]>> {
    shared.chat.is_none().then(|| format!("exec-{seed:016x}").into_bytes().into())
}

pub(crate) fn prepare_stream(
    agent: &mut smith_agent_shell::config::Configuration,
    chat: &[u8],
    limits: &mut smith_local_service::Limits,
    json: bool,
    wall: Wall,
    result_file: skein_lib::Duration,
    front: bool,
) -> Result<(Option<smith_agent_shell::trace::TraceConfig>, Option<smith_events::SessionStarted>, u64), Refused> {
    use std::path::PathBuf;
    let trace = agent.trace.take();
    let capture = if front && !json {
        smith_agent_service::sinks::Capture::None
    } else {
        smith_agent_service::sinks::Capture::Everything
    };
    let specification = smith_agent_shell::trace::TraceConfig {
        path: trace.as_ref().map_or_else(|| PathBuf::from("stdout"), |trace| trace.path.clone()),
        capture: trace.as_ref().map_or(capture, |trace| trace.capture),
        delivery: trace.as_ref().map_or(smith_agent_service::sinks::Delivery::Complete, |trace| trace.delivery),
    };
    let destinations = u32::from(trace.is_some()).checked_add(u32::from(json)).expect("two destinations");
    let (mut header, available) = crate::Local::prepare_header(
        agent,
        limits,
        (destinations != 0 || front).then_some(&specification),
        chat,
        u64::from(std::process::id()),
        crate::local_host::View { destinations, front },
        result_file,
    )
    .map_err(Refused::Local)?;
    if let Some(header) = &mut header {
        header.mode = if front { smith_events::Mode::Chat } else { smith_events::Mode::Exec };
        header.wall_ms = wall.as_nanos() / 1_000_000;
    }
    Ok((trace, header, available))
}

fn host(why: &std::io::Error) -> Refused {
    Refused::Local(local_shell::Failure::Host { detail: why.to_string() })
}

fn root(path: &std::path::Path) -> Result<skein_io::kernel::Fd, Refused> {
    skein_shell::open_root(path).map_err(|errno| {
        Refused::Local(local_shell::Failure::Host {
            detail: format!("directory {} failed (errno {errno})", path.display()),
        })
    })
}

/// Run an exec invocation on its one kernel; leave other commands to their owners.
#[must_use]
pub fn start(arguments: &[std::ffi::OsString], at: skein_shell::Now, clock: &Clock) -> Option<u8> {
    let (invocation, interactive) = match crate::command::parse(arguments.iter().skip(1).cloned().collect()) {
        Ok(Command::Exec(invocation)) => (invocation, None),
        Ok(Command::Interactive { shared, plain }) => (
            Exec {
                shared,
                prompt: Prompt::Argument(Box::new([])),
                json: false,
                output: None,
                deliver: false,
                ephemeral: false,
            },
            Some(plain || skein_shell::terminal_modes(skein_io::kernel::Fd::new(0)).is_err()),
        ),
        Ok(Command::Check { .. } | Command::Legacy(_) | Command::Login { .. }) => {
            return None;
        }
        Err(why) => {
            eprintln!("{}", Refused::Usage(why).line());
            return Some(2);
        }
    };
    let result = run(invocation, at, clock, interactive);
    Some(match result {
        Ok(code) => code,
        Err(why) => {
            eprintln!("{}", why.line());
            why.code()
        }
    })
}

fn run(invocation: Exec, at: skein_shell::Now, clock: &Clock, interactive: Option<bool>) -> Result<u8, Refused> {
    let (opened, mut kernel) = if interactive.is_none() {
        open(invocation, at.wall, clock)?
    } else {
        open_mode(invocation, at.wall, clock, interactive)?
    };
    let mut files = Files::prepare(&opened.invocation, &opened.resolved, opened.seed)?;
    let result = compose(opened, AgentKind::Inline, at.now, &mut files).map(|mut started| {
        match skein_shell::drive(&mut kernel, clock, &mut started.local) {
            skein_io::kernel::Exit::Code(code) => code,
            skein_io::kernel::Exit::Signal(_) => 130,
        }
    });
    files.close_startup(&mut kernel, clock);
    result
}
