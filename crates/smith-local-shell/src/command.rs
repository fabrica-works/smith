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
    if arguments.first().is_some_and(|argument| argument == "login") {
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
    let mut arguments = arguments.into_iter().skip(1).collect::<Vec<_>>().into_iter();
    let mut shared = Shared::default();
    let mut account = None;
    while let Some(argument) = arguments.next() {
        let argument = argument.to_str().ok_or_else(|| Usage("an argument is not UTF-8".into()))?;
        match argument {
            "-C" => shared.directory = Some(value(&mut arguments, "-C")?.into()),
            "-c" => shared.overrides.push(text(&mut arguments, "-c")?),
            name if !name.starts_with('-') && account.is_none() => account = Some(name.into()),
            _ => return Err(Usage(format!("unexpected argument `{argument}`"))),
        }
    }
    Ok(Command::Login { shared, account: account.ok_or_else(|| Usage("smith login requires an account".into()))? })
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
