//! Opt-in checks of the real program's interrupt and resume contracts (programs.md 5.7, 8).
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

#[path = "probe.rs"]
mod probe;
use probe::Probe;

struct Live {
    child: Child,
    probe: Probe,
    messages: mpsc::Receiver<serde_json::Value>,
    configuration: std::path::PathBuf,
}

impl Live {
    fn start(text: &str) -> Self {
        Self::start_with(text, false)
    }

    fn start_with(text: &str, permission: bool) -> Self {
        let root = std::env::var_os("SMITH_LIVE").filter(|value| !value.is_empty()).expect("$SMITH_LIVE unset");
        let root = smith_bench::guard::write_path(std::path::Path::new(&root)).expect("guarded $SMITH_LIVE");
        let configuration = guarded_configuration(&root.join("programs/claude"));
        assert!(
            std::fs::symlink_metadata(&configuration).is_ok(),
            "no record in `$SMITH_LIVE/programs/claude`: run `XDG_CONFIG_HOME=$SMITH_LIVE/config smith login claude`"
        );
        let probe = Probe::new();
        let mcp = if permission { prompt_server(&probe) } else { r#"{"mcpServers":{}}"#.into() };
        let mut arguments = vec![
            "--session-id",
            &probe.session,
            "--mcp-config",
            &mcp,
            "--tools",
            if permission { "Read" } else { "" },
            "--max-turns",
            if permission { "4" } else { "1" },
        ];
        if permission {
            arguments.extend([
                "--permission-prompt-tool",
                "mcp__smith__permission_prompt",
                "--settings",
                r#"{"permissions":{"allow":["mcp__smith"],"ask":["Read"]}}"#,
            ]);
        }
        let (child, messages) = Self::spawn(&probe.home, &configuration, &arguments, text);
        Self { child, probe, messages, configuration }
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "live protocol probe bounds blocking stdout; departure from testing-strategy.md 2.9"
    )]
    fn spawn(
        home: &std::path::Path,
        configuration: &std::path::Path,
        arguments: &[&str],
        text: &str,
    ) -> (Child, mpsc::Receiver<serde_json::Value>) {
        let configuration = guarded_configuration(configuration);
        let mut child = Command::new("claude")
            .env_clear()
            .env("PATH", std::env::var_os("PATH").expect("PATH for claude"))
            .env("HOME", home)
            .env("CLAUDE_CONFIG_DIR", &configuration)
            .current_dir(home)
            .args([
                "-p",
                "--input-format",
                "stream-json",
                "--output-format",
                "stream-json",
                "--verbose",
                "--include-partial-messages",
                "--setting-sources",
                "user",
                "--strict-mcp-config",
                "--model",
                "claude-haiku-5-5",
            ])
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("claude executable");
        writeln!(
            child.stdin.as_mut().expect("stdin"),
            "{}",
            serde_json::json!({"type":"user","message":{
                "role":"user","content":text
            }})
        )
        .expect("user message");
        let stdout = child.stdout.take().expect("stdout");
        let (sender, messages) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let line = line.expect("program stdout");
                if let Ok(value) = serde_json::from_str(&line)
                    && sender.send(value).is_err()
                {
                    return;
                }
            }
        });
        (child, messages)
    }

    fn message(&self) -> serde_json::Value {
        let message =
            self.messages.recv_timeout(Duration::from_mins(1)).expect("program message within the live deadline");
        // Check a limit's wire only when one arrives; never spend to force one.
        if message["type"] == "rate_limit_event" {
            assert!(message["rate_limit_info"]["status"].is_string(), "a limit reports its status");
            if let Some(reset) = message["rate_limit_info"].get("resetsAt") {
                assert!(reset.is_null() || reset.is_number(), "a limit reset has the fake's numeric shape");
            }
        }
        message
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn a_live_interrupt_ends_the_turn() {
    let mut live = Live::start("Count from one to ten thousand, writing every number.");
    for _ in 0..1000 {
        let message = live.message();
        assert!(message["type"] != "result", "the turn must still run when interrupted");
        if message["event"]["type"] == "message_start" {
            break;
        }
    }
    assert!(Command::new("/bin/kill").args(["-INT", &live.child.id().to_string()]).status().expect("SIGINT").success());
    for _ in 0..1000 {
        let message = live.message();
        if message["type"] == "result" {
            assert!(message["terminal_reason"] == "interrupted", "interrupt terminal");
            drop(live.child.stdin.take());
            return;
        }
    }
    panic!("interrupt did not end the program turn");
}

#[test]
fn a_live_resume_continues() {
    let mut live = Live::start("Remember the word persimmon. Reply with OK.");
    let id = live.probe.session.clone();
    let mut finished = false;
    for _ in 0..1000 {
        let message = live.message();
        if message["type"] == "result" {
            assert!(message["is_error"] == false, "program turn completed");
            finished = true;
            break;
        }
    }
    assert!(finished, "first turn result");
    drop(live.child.stdin.take());
    assert!(live.child.wait().expect("first exit").success());
    (live.child, live.messages) = Live::spawn(
        &live.probe.home,
        &live.configuration,
        &["--resume", &id, "--mcp-config", r#"{"mcpServers":{}}"#, "--tools", "", "--max-turns", "1"],
        "Which word did I ask you to remember? Reply with that word only.",
    );
    let mut text = String::new();
    for _ in 0..1000 {
        let message = live.message();
        if message["type"] == "assistant" {
            for block in message["message"]["content"].as_array().expect("assistant blocks") {
                if let Some(part) = block["text"].as_str() {
                    text.push_str(part);
                }
            }
        }
        if message["type"] == "result" {
            assert!(message["is_error"] == false, "program turn completed");
            assert!(text.to_lowercase().contains("persimmon"), "resume kept the saved word; output withheld");
            drop(live.child.stdin.take());
            return;
        }
    }
    panic!("resumed turn did not finish");
}

// A bounded fixture records only the prompt's shape, never the program's input.
fn prompt_server(probe: &Probe) -> String {
    assert!(
        std::fs::symlink_metadata("/usr/bin/python3").is_ok(),
        "`/usr/bin/python3` is missing: the prompt-shape probe's MCP fixture needs it"
    );
    let script = probe.home.join("prompt.py");
    std::fs::write(&script, r"import json, sys
from pathlib import Path
for line in sys.stdin:
    request = json.loads(line)
    if 'id' not in request:
        continue
    method = request.get('method')
    if method == 'initialize':
        result = {'protocolVersion':'2025-06-18', 'capabilities':{'tools':{}}, 'serverInfo':{'name':'smith-live-probe','version':'1'}}
    elif method == 'tools/list':
        result = {'tools':[{'name':'permission_prompt','description':'Ask before Read', 'inputSchema':{'type':'object','properties':{'tool_name':{'type':'string'},'input':{'type':'object'},'tool_use_id':{'type':'string'}},'required':['tool_name','input']}}]}
    elif method == 'tools/call':
        params = request['params']
        args = params.get('arguments', {})
        shape = (params.get('name') == 'permission_prompt' and args.get('tool_name') == 'Read' and isinstance(args.get('input'), dict) and isinstance(args.get('tool_use_id'), str))
        Path(sys.argv[1]).write_text('shape-ok' if shape else 'shape-failed')
        answer = {'behavior':'allow','updatedInput':args.get('input', {})}
        result = {'content':[{'type':'text','text':json.dumps(answer)}]}
    else:
        print(json.dumps({'jsonrpc':'2.0','id':request['id'],'error':{'code':-32601,'message':'unknown method'}}), flush=True)
        continue
    print(json.dumps({'jsonrpc':'2.0','id':request['id'],'result':result}), flush=True)
").expect("private prompt fixture");
    std::fs::write(probe.home.join("read-me.txt"), b"probe input\n").expect("private Read input");
    serde_json::json!({"mcpServers":{"smith":{"command":"/usr/bin/python3","args":[script,probe.home.join("prompt-shape")]}}}).to_string()
}

#[test]
fn the_real_program_uses_the_prompt_tool_shape() {
    let mut live = Live::start_with(
        "Use the Read tool to read read-me.txt in the current directory. Do not guess its contents. Then reply with OK.",
        true,
    );
    let mut initialized = false;
    for _ in 0..1000 {
        let message = live.message();
        if message["type"] == "system" && message["subtype"] == "init" {
            let tools = message["tools"].as_array().expect("init tools array");
            assert!(
                tools.len() == 2
                    && tools.iter().any(|tool| tool == "Read")
                    && tools.iter().any(|tool| tool == "mcp__smith__permission_prompt"),
                "init lists exactly the granted built in and prompt tool"
            );
            assert!(
                message
                    .get("capabilities")
                    .is_none_or(|capabilities| capabilities.as_array().is_some_and(Vec::is_empty)),
                "init has no additional capabilities"
            );
            let servers = message["mcp_servers"].as_array().expect("init MCP servers");
            assert!(
                servers.len() == 1
                    && servers.iter().any(|server| server["name"] == "smith" && server["status"] == "connected"),
                "init names the connected probe server"
            );
            initialized = true;
        }
        if message["type"] == "result" {
            assert!(initialized && message["is_error"] == false, "signed in -p completed after init");
            assert!(
                std::fs::read(live.probe.home.join("prompt-shape")).is_ok_and(|bytes| bytes == b"shape-ok"),
                "the real program sent the prompt input the fake assumes"
            );
            drop(live.child.stdin.take());
            assert!(live.child.wait().expect("probe exit").success(), "prompt probe exits successfully");
            return;
        }
    }
    panic!("prompt shape probe did not finish");
}

fn guarded_configuration(configuration: &std::path::Path) -> std::path::PathBuf {
    smith_bench::guard::write_path(configuration)
        .unwrap_or_else(|reason| panic!("$SMITH_LIVE/programs/claude: {reason}"))
}
