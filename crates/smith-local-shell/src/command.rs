//! Product arguments parsed before startup reads (shell.md, sections 4 and 5.1).
//! The parser keeps only named shared flags and ordered overrides; it knows no
//! settings or credentials. `parse` returns one command or a usage refusal.
use std::{ffi::OsString, path::PathBuf};

/// Flags shared by product commands, supplied by their caller before resolution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shared {
    pub directory: Option<PathBuf>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub chat: Option<String>,
    pub trace: Option<PathBuf>,
    pub overrides: Vec<String>,
    pub dirs: Vec<Dir>,
}

/// One `--dir NAME=PATH[:ro|rw]`, sent by the caller beside the working directory; four in all (shell.md, section 4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dir {
    pub name: String,
    pub path: PathBuf,
    pub writable: bool,
}

/// One product invocation, parsed before settings are read.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// The workspace chat in line mode, with optional plain output.
    Interactive { shared: Shared, plain: bool },
    /// Resolve and validate settings without starting a run.
    Check { shared: Shared, json: bool, execution: Option<Exec> },
    /// One headless run, with its prompt supplied before the activation starts.
    Exec(Exec),
    /// Sign in to a named account, or inspect its handed-in source.
    Login { shared: Shared, account: String },
    /// Revoke or remove the credential held for a named account.
    Logout { shared: Shared, account: String },
}

/// A headless invocation's prompt and output choices, before any startup reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exec {
    pub shared: Shared,
    pub prompt: Prompt,
    pub output: Option<PathBuf>,
    pub ephemeral: bool,
    pub deliver: bool,
    pub json: bool,
}

/// Where the headless front end obtains its one bounded person message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Prompt {
    /// The argument's UTF-8 bytes are the whole message.
    Argument(Box<[u8]>),
    /// Read standard input to its end before starting a run.
    Stdin,
}

/// An invalid argument form, rendered by the binary with exit 2.
#[derive(Debug, PartialEq, Eq)]
pub struct Usage(pub String);

/// Parse product flags and the single exec prompt.
pub fn parse(arguments: Vec<OsString>) -> Result<Command, Usage> {
    if arguments.first().is_some_and(|argument| argument == "login" || argument == "logout") {
        return login(arguments);
    }
    let interactive = !arguments.first().is_some_and(|argument| argument == "exec" || argument == "check");
    let mut arguments = arguments.into_iter();
    let name = if interactive { None } else { arguments.next() };
    let mut exec = match name.as_deref().and_then(std::ffi::OsStr::to_str) {
        Some("exec") => true,
        Some("check") => false,
        None if interactive => false,
        _ => return Err(Usage("expected `smith exec [FLAGS] [PROMPT | -]` or `smith check [--json]`".into())),
    };
    let checking = !interactive && !exec;
    let mut plain = false;
    let mut report_json = false;
    let mut shared = Shared::default();
    let mut json = false;
    let mut prompt = None;
    let mut output = None;
    let mut ephemeral = false;
    let mut deliver = false;
    while let Some(argument) = arguments.next() {
        let argument = argument.to_str().ok_or_else(|| Usage("a flag is not UTF-8".into()))?;
        match argument {
            "exec" if checking && !exec => {
                exec = true;
                report_json = json;
                json = false;
            }
            "--json" if !interactive => json = true,
            "--plain" if interactive => plain = true,
            "-C" => shared.directory = Some(value(&mut arguments, "-C")?.into()),
            "--dir" => shared.dirs.push(dir(&text(&mut arguments, "--dir")?)?),
            "--trace" => shared.trace = Some(value(&mut arguments, "--trace")?.into()),
            "-m" => shared.model = Some(text(&mut arguments, "-m")?),
            "--effort" => shared.effort = Some(text(&mut arguments, "--effort")?),
            "--chat" => shared.chat = Some(text(&mut arguments, "--chat")?),
            "-c" => shared.overrides.push(text(&mut arguments, "-c")?),
            "--ephemeral" if exec => ephemeral = true,
            "-o" if exec => output = Some(value(&mut arguments, "-o")?.into()),
            "--deliver" if exec => deliver = true,
            "-" if exec => set_prompt(&mut prompt, Prompt::Stdin)?,
            value if exec && !value.starts_with('-') => {
                if value.trim().is_empty() {
                    return Err(Usage("the prompt is empty".into()));
                }
                set_prompt(&mut prompt, Prompt::Argument(value.as_bytes().into()))?;
            }
            _ => return Err(Usage(format!("unexpected argument `{argument}`"))),
        }
    }
    if interactive {
        Ok(Command::Interactive { shared, plain })
    } else if checking {
        let execution = exec.then(|| Exec {
            shared: shared.clone(),
            prompt: prompt.unwrap_or(Prompt::Stdin),
            output,
            ephemeral,
            deliver,
            json,
        });
        Ok(Command::Check { shared, json: if exec { report_json } else { json }, execution })
    } else {
        Ok(Command::Exec(Exec { shared, prompt: prompt.unwrap_or(Prompt::Stdin), output, ephemeral, deliver, json }))
    }
}

fn login(arguments: Vec<OsString>) -> Result<Command, Usage> {
    let logout = arguments.first().is_some_and(|name| name == "logout");
    let mut arguments = arguments.into_iter().skip(1).collect::<Vec<_>>().into_iter();
    let mut shared = Shared::default();
    let mut account = None;
    while let Some(argument) = arguments.next() {
        let argument = argument.to_str().ok_or_else(|| Usage("an argument is not UTF-8".into()))?;
        match argument {
            "-C" => shared.directory = Some(value(&mut arguments, "-C")?.into()),
            "--dir" => shared.dirs.push(dir(&text(&mut arguments, "--dir")?)?),
            "-c" => shared.overrides.push(text(&mut arguments, "-c")?),
            name if !name.starts_with('-') && account.is_none() => account = Some(name.into()),
            _ => return Err(Usage(format!("unexpected argument `{argument}`"))),
        }
    }
    let verb = if logout { "logout" } else { "login" };
    let account = account.ok_or_else(|| Usage(format!("smith {verb} requires an account")))?;
    Ok(if logout { Command::Logout { shared, account } } else { Command::Login { shared, account } })
}

/// `NAME=PATH`, `NAME=PATH:ro` or `NAME=PATH:rw`; the name is one safe path component.
/// After the last `:` comes the mode, so a path holding `:` ends in `:rw` or `:ro`.
fn dir(value: &str) -> Result<Dir, Usage> {
    let malformed = || Usage(format!("--dir `{value}`: expected NAME=PATH[:ro|rw]"));
    let (name, rest) = value.split_once('=').ok_or_else(malformed)?;
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\0']) {
        return Err(Usage(format!("--dir `{value}`: the name must be one path component")));
    }
    let (path, writable) = match rest.rsplit_once(':') {
        Some((path, "ro")) => (path, false),
        Some((path, "rw")) => (path, true),
        Some(_) => return Err(malformed()),
        None => (rest, true),
    };
    if path.is_empty() {
        return Err(malformed());
    }
    Ok(Dir { name: name.into(), path: path.into(), writable })
}

fn set_prompt(place: &mut Option<Prompt>, prompt: Prompt) -> Result<(), Usage> {
    if place.replace(prompt).is_some() {
        return Err(Usage("supply one prompt argument or `-`, not both".into()));
    }
    Ok(())
}

fn value(arguments: &mut std::vec::IntoIter<OsString>, flag: &str) -> Result<OsString, Usage> {
    arguments.next().filter(|value| !value.is_empty()).ok_or_else(|| Usage(format!("{flag} requires a value")))
}

fn text(arguments: &mut std::vec::IntoIter<OsString>, flag: &str) -> Result<String, Usage> {
    value(arguments, flag)?.into_string().map_err(|_| Usage(format!("{flag} requires UTF-8")))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn arguments(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn the_interactive_command_accepts_shared_flags_and_plain_mode() {
        let Command::Interactive { shared, plain } = parse(arguments(&[
            "--plain",
            "-C",
            "work",
            "-m",
            "codex/small",
            "--effort",
            "low",
            "--chat",
            "work",
            "--trace",
            "trace",
            "-c",
            "budget.turns=4",
        ]))
        .expect("interactive flags") else {
            panic!("interactive invocation")
        };
        assert!(plain);
        assert_eq!(shared.directory, Some("work".into()));
        assert_eq!(shared.model.as_deref(), Some("codex/small"));
        assert_eq!(shared.effort.as_deref(), Some("low"));
        assert_eq!(shared.chat.as_deref(), Some("work"));
        assert_eq!(shared.trace, Some("trace".into()));
        assert_eq!(shared.overrides, ["budget.turns=4"]);
        assert!(matches!(parse(Vec::new()), Ok(Command::Interactive { plain: false, .. })));
        assert!(parse(arguments(&["--json"])).is_err());
        assert!(parse(arguments(&["a prompt"])).is_err());
    }

    #[test]
    fn check_collects_shared_flags_and_ordered_overrides() {
        let Command::Check { shared, json, execution } = parse(arguments(&[
            "check",
            "--json",
            "-C",
            "work",
            "-m",
            "codex/model",
            "--effort",
            "high",
            "--chat",
            "plan",
            "--trace",
            "trace",
            "-c",
            "budget.turns=4",
            "-c",
            "budget.turns=5",
        ]))
        .expect("check arguments") else {
            panic!("check command")
        };
        assert!(json);
        assert!(execution.is_none());
        assert_eq!(shared.directory, Some(PathBuf::from("work")));
        assert_eq!(shared.model.as_deref(), Some("codex/model"));
        assert_eq!(shared.effort.as_deref(), Some("high"));
        assert_eq!(shared.chat.as_deref(), Some("plan"));
        assert_eq!(shared.trace, Some(PathBuf::from("trace")));
        assert_eq!(shared.overrides, ["budget.turns=4", "budget.turns=5"]);
    }

    #[test]
    fn check_separates_its_report_form_from_execs_output_choices() {
        let Command::Check { shared, json, execution: Some(exec) } = parse(arguments(&[
            "check",
            "--json",
            "-c",
            "budget.turns=4",
            "exec",
            "--json",
            "-C",
            "work",
            "--trace",
            "events",
            "-o",
            "result",
            "--ephemeral",
            "--deliver",
            "-",
        ]))
        .expect("checked invocation") else {
            panic!("checked exec")
        };
        assert!(json && exec.json && exec.ephemeral && exec.deliver);
        assert_eq!(shared, exec.shared);
        assert_eq!(shared.overrides, ["budget.turns=4"]);
        assert_eq!(exec.output, Some("result".into()));
        assert_eq!(exec.prompt, Prompt::Stdin);
        let Command::Check { json, execution: Some(exec), .. } =
            parse(arguments(&["check", "exec", "--json"])).expect("text check")
        else {
            panic!("checked exec")
        };
        assert!(!json && exec.json);
        let Command::Check { json, execution: Some(exec), .. } =
            parse(arguments(&["check", "--json", "exec"])).expect("JSON check")
        else {
            panic!("checked exec")
        };
        assert!(json && !exec.json);
    }

    #[test]
    fn every_command_with_dash_c_collects_dirs_in_order_with_their_modes() {
        let dir = |name: &str, path: &str, writable| Dir { name: name.into(), path: path.into(), writable };
        let flags = ["--dir", "a=x", "--dir", "b=../y:ro", "--dir", "c=/z:rw", "--dir", "d=w:x:rw"];
        let expected = [dir("a", "x", true), dir("b", "../y", false), dir("c", "/z", true), dir("d", "w:x", true)];
        let Command::Interactive { shared, .. } = parse(arguments(&flags)).expect("interactive dirs") else {
            panic!("interactive invocation")
        };
        assert_eq!(shared.dirs, expected);
        let checked = [["check"].as_slice(), &flags].concat();
        let Command::Check { shared, .. } = parse(arguments(&checked)).expect("check dirs") else { panic!("check") };
        assert_eq!(shared.dirs, expected);
        let exec = [["exec"].as_slice(), &flags, &["message"]].concat();
        let Command::Exec(exec) = parse(arguments(&exec)).expect("exec dirs") else { panic!("exec") };
        assert_eq!(exec.shared.dirs, expected);
        let login = [["login", "work"].as_slice(), &flags].concat();
        let Command::Login { shared, .. } = parse(arguments(&login)).expect("login dirs") else { panic!("login") };
        assert_eq!(shared.dirs, expected);
        assert!(
            parse(Vec::new())
                .is_ok_and(|command| command == Command::Interactive { shared: Shared::default(), plain: false })
        );
    }

    #[test]
    fn a_malformed_dir_is_a_usage_refusal() {
        for value in ["a", "=x", "a/b=x", "..=x", ".=x", "a=", "a=:ro", "a=x:rx"] {
            assert!(
                matches!(parse(arguments(&["check", "--dir", value])), Err(Usage(line)) if line.starts_with("--dir `")),
                "{value}"
            );
        }
        assert_eq!(parse(arguments(&["check", "--dir"])), Err(Usage("--dir requires a value".into())));
    }

    #[test]
    fn a_flag_without_its_value_is_a_usage_refusal() {
        assert_eq!(parse(arguments(&["check", "-C"])), Err(Usage("-C requires a value".into())));
    }

    #[test]
    fn an_unknown_argument_is_a_usage_refusal() {
        assert_eq!(parse(arguments(&["check", "--unknown"])), Err(Usage("unexpected argument `--unknown`".into())));
    }

    #[test]
    fn the_removed_local_and_agent_commands_are_usage_refusals() {
        for name in ["local", "agent"] {
            assert!(parse(arguments(&[name, "settings.json", "state"])).is_err());
        }
    }

    #[test]
    fn exec_collects_its_prompt_and_output_choices() {
        let Command::Exec(exec) =
            parse(arguments(&["exec", "--chat", "plan", "--deliver", "--json", "Ask first"])).expect("exec arguments")
        else {
            panic!("exec command")
        };
        assert_eq!(exec.shared.chat.as_deref(), Some("plan"));
        assert_eq!(exec.prompt, Prompt::Argument(b"Ask first".as_slice().into()));
        assert!(exec.deliver && exec.json);
    }

    #[test]
    fn a_dash_supplies_the_prompt_from_standard_input() {
        let Command::Exec(exec) = parse(arguments(&["exec", "-"])).unwrap() else { panic!("exec command") };
        assert_eq!(exec.prompt, Prompt::Stdin);
    }

    #[test]
    fn two_prompt_inputs_are_a_usage_refusal() {
        assert_eq!(
            parse(arguments(&["exec", "message", "-"])),
            Err(Usage("supply one prompt argument or `-`, not both".into()))
        );
    }

    #[test]
    fn an_empty_prompt_argument_is_a_usage_refusal() {
        assert_eq!(parse(arguments(&["exec", " "])), Err(Usage("the prompt is empty".into())));
    }
    #[test]
    fn exec_names_a_result_file_without_changing_its_prompt() {
        let Command::Exec(exec) = parse(arguments(&["exec", "-o", "result.txt", "message"])).unwrap() else {
            panic!("exec command")
        };
        assert_eq!(exec.output, Some(PathBuf::from("result.txt")));
        assert_eq!(exec.prompt, Prompt::Argument(b"message".as_slice().into()));
    }

    #[test]
    fn a_result_file_without_its_name_is_a_usage_refusal() {
        assert_eq!(parse(arguments(&["exec", "-o"])), Err(Usage("-o requires a value".into())));
    }
    #[test]
    fn exec_requests_a_private_chat_without_changing_its_prompt() {
        let Command::Exec(exec) = parse(arguments(&["exec", "--ephemeral", "message"])).unwrap() else {
            panic!("exec command")
        };
        assert!(exec.ephemeral);
        assert_eq!(exec.prompt, Prompt::Argument(b"message".as_slice().into()));
    }
}

#[cfg(test)]
mod logout_tests;
