//! One print conversation (protocol/programs.md, sections 5 and 8). Keeps
//! parsed flags, a turn cursor and file phase; never reads prompt contents.
//! The shared driver supplies file/input completions and service signals.

use std::collections::BTreeMap;

use serde_json::{Value, json};
use skein_io::file;

use crate::{
    End, LOGIN_FILE, Resume, SETTINGS_FILE, Seen, Step, Story, Turn,
    commands::{Config, Effect, FILE_OWNER, error},
    stream,
};

#[derive(Debug)]
struct Flags {
    values: BTreeMap<String, String>,
    tools: Vec<String>,
    session: String,
}

impl Flags {
    fn parse(arguments: &[String]) -> Option<Self> {
        let mut values = BTreeMap::new();
        let mut arguments = arguments.iter();
        while let Some(flag) = arguments.next() {
            let value = match flag.as_str() {
                "-p" | "--verbose" | "--strict-mcp-config" | "--include-partial-messages" => String::new(),
                "--input-format"
                | "--output-format"
                | "--setting-sources"
                | "--model"
                | "--append-system-prompt-file"
                | "--mcp-config"
                | "--permission-prompt-tool"
                | "--tools"
                | "--session-id"
                | "--resume"
                | "--effort"
                | "--json-schema"
                | "--max-turns" => arguments.next()?.clone(),
                _ => return None,
            };
            if value.starts_with('-')
                || (value.is_empty()
                    && !matches!(
                        flag.as_str(),
                        "-p" | "--verbose" | "--strict-mcp-config" | "--include-partial-messages" | "--tools"
                    ))
                || values.insert(flag.clone(), value).is_some()
            {
                return None;
            }
        }
        for flag in [
            "-p",
            "--verbose",
            "--strict-mcp-config",
            "--model",
            "--append-system-prompt-file",
            "--mcp-config",
            "--permission-prompt-tool",
            "--tools",
        ] {
            values.get(flag)?;
        }
        for (flag, value) in
            [("--input-format", "stream-json"), ("--output-format", "stream-json"), ("--setting-sources", "user")]
        {
            if values.get(flag)?.as_str() != value {
                return None;
            }
        }
        if values.contains_key("--session-id") == values.contains_key("--resume") {
            return None;
        }
        let session = values.get("--session-id").or_else(|| values.get("--resume"))?.clone();
        if session.len() != 36
            || !session.bytes().enumerate().all(|(index, byte)| {
                if [8, 13, 18, 23].contains(&index) { byte == b'-' } else { byte.is_ascii_hexdigit() }
            })
        {
            return None;
        }
        if let Some(turns) = values.get("--max-turns")
            && (!turns.bytes().all(|byte| byte.is_ascii_digit()) || turns.parse::<u64>().ok()? == 0)
        {
            return None;
        }
        let (server, tool) = values.get("--permission-prompt-tool")?.strip_prefix("mcp__")?.split_once("__")?;
        if server.is_empty() || tool.is_empty() {
            return None;
        }
        if let Some(schema) = values.get("--json-schema") {
            serde_json::from_str::<Value>(schema).ok()?;
        }
        let list = values.get("--tools")?;
        let tools: Vec<String> =
            if list.is_empty() { Vec::new() } else { list.split(',').map(str::to_owned).collect() };
        for (index, tool) in tools.iter().enumerate() {
            if !["Read", "Glob", "Grep", "Write", "Edit", "Bash"].contains(&tool.as_str())
                || tools[..index].contains(tool)
            {
                return None;
            }
        }
        Some(Self { values, tools, session })
    }
}

#[derive(Debug)]
enum Phase {
    Configuration,
    Settings,
    Idle,
    Login,
    Removing,
    Hanging,
    Done,
}

#[derive(Debug)]
pub(crate) struct Print {
    flags: Flags,
    story: Story,
    config: Config,
    phase: Phase,
    server: String,
    input: Vec<u8>,
    next_turn: usize,
    turn: Turn,
    step: usize,
    texts: u64,
    last_text: String,
    interrupted: bool,
}

impl Print {
    pub fn start(arguments: &[String], story: Story, config: Config) -> Result<(Self, Vec<Effect>), Vec<Effect>> {
        let flags = Flags::parse(arguments).ok_or_else(|| error("unknown command or flag", 2))?;
        if flags.values.contains_key("--resume") && story.resume == Resume::Missing {
            return Err(vec![
                Effect::Error(format!("No conversation found with session ID: {}\n", flags.session).into_bytes()),
                Effect::End(1),
            ]);
        }
        let path = std::path::Path::new(&flags.values["--mcp-config"]);
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| error("invalid MCP configuration path", 1))?;
        let root = config.mcp.ok_or_else(|| error("cannot read MCP configuration", 1))?;
        let effects = vec![load(root, name)];
        Ok((
            Self {
                flags,
                story,
                config,
                phase: Phase::Configuration,
                server: String::new(),
                input: Vec::new(),
                next_turn: 0,
                turn: Turn::default(),
                step: 0,
                texts: 0,
                last_text: String::new(),
                interrupted: false,
            },
            effects,
        ))
    }

    fn emit(&self, value: Value) -> Effect {
        Effect::Output(stream::line(value, &self.flags.session))
    }

    pub fn file(&mut self, event: file::Event) -> Vec<Effect> {
        match self.phase {
            Phase::Configuration => {
                let bytes = match event {
                    file::Event::Loaded { bytes, .. } => bytes,
                    _ => return error("cannot read MCP configuration", 1),
                };
                let Some(server) = configuration(&bytes, &self.flags.values["--permission-prompt-tool"]) else {
                    return error("invalid MCP configuration", 1);
                };
                self.server = server;
                self.phase = Phase::Settings;
                match self.config.root {
                    Some(root) => vec![load(root, SETTINGS_FILE)],
                    None => self.initialized(Seen::UnreadableSettings),
                }
            }
            Phase::Settings => {
                let seen = match event {
                    file::Event::Loaded { bytes, .. } => settings(&bytes),
                    _ => Seen::UnreadableSettings,
                };
                self.initialized(seen)
            }
            Phase::Login => {
                if self.interrupted {
                    return self.interrupt_result();
                }
                match event {
                    file::Event::Loaded { .. } => self.steps(),
                    _ => self.finish(stream::failure(Some(401), None, vec!["not signed in".into()])),
                }
            }
            Phase::Removing => {
                if self.interrupted {
                    self.interrupt_result()
                } else {
                    self.steps()
                }
            }
            Phase::Idle | Phase::Hanging | Phase::Done => Vec::new(),
        }
    }

    fn initialized(&mut self, seen: Seen) -> Vec<Effect> {
        self.phase = Phase::Idle;
        vec![
            Effect::Seen(seen),
            self.emit(stream::init(&self.story.init, &self.flags.tools, &self.server, &self.flags.values["--model"])),
            Effect::ReadCode,
        ]
    }

    pub fn input(&mut self, bytes: Option<&[u8]>) -> Vec<Effect> {
        match bytes {
            None if matches!(self.phase, Phase::Login | Phase::Removing | Phase::Hanging) => Vec::new(),
            None if self.input.is_empty() => {
                self.phase = Phase::Done;
                vec![Effect::End(0)]
            }
            None => error("incomplete input line", 1),
            Some(bytes) => {
                self.input.extend_from_slice(bytes);
                if self.input.len() > super::fake::FILE_BYTES as usize {
                    return error("input line too long", 1);
                }
                if self.input.last() != Some(&b'\n') {
                    return vec![Effect::ReadCode];
                }
                if !stream::user(&self.input) {
                    return error("invalid user message", 1);
                }
                self.input.clear();
                self.turn = self.story.turns.get(self.next_turn).cloned().unwrap_or_default();
                self.next_turn = self.next_turn.checked_add(1).expect("turn count fits");
                self.step = 0;
                self.texts = 0;
                self.last_text.clear();
                self.interrupted = false;
                self.phase = Phase::Login;
                match self.config.root {
                    Some(root) => vec![load(root, LOGIN_FILE)],
                    None => self.finish(stream::failure(Some(401), None, vec!["not signed in".into()])),
                }
            }
        }
    }

    fn steps(&mut self) -> Vec<Effect> {
        let mut effects = Vec::new();
        while let Some(step) = self.turn.steps.get(self.step).cloned() {
            self.step = self.step.checked_add(1).expect("step count fits");
            match step {
                Step::Text(text) => {
                    self.texts = self.texts.checked_add(1).expect("text count fits");
                    let id = self
                        .next_turn
                        .checked_mul(super::fake::FILE_BYTES as usize)
                        .and_then(|base| base.checked_add(self.step))
                        .expect("message id fits");
                    if self.flags.values.contains_key("--include-partial-messages") {
                        effects.extend(
                            stream::partials(&text, &self.flags.values["--model"], id)
                                .into_iter()
                                .map(|value| self.emit(value)),
                        );
                    }
                    effects.push(self.emit(stream::assistant(
                        &text,
                        &self.flags.values["--model"],
                        id,
                        &self.story.usage,
                    )));
                    self.last_text = text;
                }
                Step::RateLimit { resets_at } => {
                    let mut value = json!({"type":"rate_limit_event", "rate_limit_info":{"status":"rejected"}});
                    if let Some(reset) = resets_at {
                        value["rate_limit_info"]["resetsAt"] = json!(reset);
                    }
                    effects.push(self.emit(value));
                }
                Step::Retry { attempt, status } => {
                    effects.push(self.emit(
                        json!({"type":"system", "subtype":"api_retry", "attempt":attempt, "error_status":status}),
                    ))
                }
                Step::LoseLogin => {
                    if let Some(root) = self.config.directory {
                        self.phase = Phase::Removing;
                        effects.push(Effect::File(file::Request::Remove {
                            owner: FILE_OWNER,
                            root,
                            name: LOGIN_FILE.as_bytes().into(),
                            directory: false,
                        }));
                        return effects;
                    }
                }
                Step::Hang => {
                    self.phase = Phase::Hanging;
                    return effects;
                }
            }
        }
        match self.turn.end.clone() {
            End::Success { structured_output } => {
                let mut value = json!({"type":"result", "subtype":"success", "is_error":false, "result":self.last_text, "usage":stream::usage(&self.story.usage, self.texts)});
                if self.flags.values.contains_key("--json-schema")
                    && let Some(output) = structured_output
                {
                    value["structured_output"] = output;
                }
                effects.extend(self.finish(value));
            }
            End::Error { api_error_status, terminal_reason, errors } => {
                effects.extend(self.finish(stream::failure(api_error_status, terminal_reason, errors)))
            }
            End::Exit(code) => effects.push(Effect::End(code)),
            End::Undecodable => effects.extend([Effect::Output(b"not json\n".to_vec()), Effect::End(1)]),
        }
        effects
    }

    fn finish(&mut self, value: Value) -> Vec<Effect> {
        self.phase = Phase::Idle;
        vec![self.emit(value), Effect::ReadCode]
    }

    fn interrupt_result(&mut self) -> Vec<Effect> {
        self.interrupted = false;
        self.finish(stream::failure(None, Some("interrupted".into()), Vec::new()))
    }

    pub fn interrupt(&mut self) -> Vec<Effect> {
        match self.phase {
            Phase::Hanging => self.interrupt_result(),
            Phase::Login | Phase::Removing => {
                self.interrupted = true;
                Vec::new()
            }
            Phase::Configuration | Phase::Settings | Phase::Idle | Phase::Done => Vec::new(),
        }
    }
}

fn load(root: skein_lib::Token, name: &str) -> Effect {
    Effect::File(file::Request::Load {
        owner: FILE_OWNER,
        root,
        path: name.as_bytes().into(),
        max: super::fake::FILE_BYTES,
        no_follow: true,
    })
}

fn configuration(bytes: &[u8], prompt: &str) -> Option<String> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let servers = value["mcpServers"].as_object()?;
    if servers.len() != 1 {
        return None;
    }
    let (name, server) = servers.iter().next()?;
    if name.is_empty()
        || server.get("type").is_some_and(|kind| kind != "stdio")
        || server["command"].as_str()?.is_empty()
    {
        return None;
    }
    if let Some(args) = server.get("args")
        && !args.as_array()?.iter().all(Value::is_string)
    {
        return None;
    }
    if let Some(env) = server.get("env")
        && !env.as_object()?.values().all(Value::is_string)
    {
        return None;
    }
    if prompt.strip_prefix(&format!("mcp__{name}__"))?.is_empty() {
        return None;
    }
    Some(name.clone())
}

fn settings(bytes: &[u8]) -> Seen {
    let Ok(Value::Object(value)) = serde_json::from_slice::<Value>(bytes) else { return Seen::UnreadableSettings };
    if value.get("permissions").is_some_and(|permissions| !permissions.is_object()) {
        return Seen::UnreadableSettings;
    }
    let rules = |name| -> Option<Vec<String>> {
        match value.get("permissions").and_then(|permissions| permissions.get(name)) {
            None => Some(Vec::new()),
            Some(rules) => rules.as_array()?.iter().map(|rule| rule.as_str().map(str::to_owned)).collect(),
        }
    };
    match (rules("allow"), rules("ask"), rules("deny")) {
        (Some(allow), Some(ask), Some(deny)) => {
            Seen::Settings { allow, ask, deny, keys: value.keys().cloned().collect() }
        }
        _ => Seen::UnreadableSettings,
    }
}
