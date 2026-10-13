//! Product arguments parsed before startup reads (shell.md, sections 4 and 5.1).
//! The parser keeps only named shared flags and ordered overrides; it knows no
//! settings or credentials. `parse` returns one command or a usage refusal.
use std::{ffi::OsString, path::PathBuf};

/// Flags shared by product commands, supplied by their caller before resolution.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Shared {
    pub directory: Option<PathBuf>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub chat: Option<String>,
    pub trace: Option<PathBuf>,
    pub overrides: Vec<String>,
}

/// One product invocation, or the legacy arguments retained until their replacements ship.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Resolve and validate settings without starting a run.
    Check { shared: Shared, json: bool },
    /// One headless run, with its prompt supplied before the activation starts.
    Exec(Exec),
    /// The existing agent and local entry points, removed by product migration.
    Legacy(Vec<OsString>),
}

/// A headless invocation's prompt and output choices, before any startup reads.
#[derive(Debug, PartialEq, Eq)]
pub struct Exec {
    pub shared: Shared,
    pub prompt: Prompt,
    pub deliver: bool,
    pub json: bool,
}

/// Where the headless front end obtains its one bounded person message.
#[derive(Debug, PartialEq, Eq)]
pub enum Prompt {
    /// The argument's UTF-8 bytes are the whole message.
    Argument(Box<[u8]>),
    /// Read standard input to its end before starting a run.
    Stdin,
}

/// An invalid argument form, rendered by the binary with exit 2.
#[derive(Debug, PartialEq, Eq)]
pub struct Usage(pub String);

/// Parse product flags and the single exec prompt, preserving legacy invocations.
pub fn parse(arguments: Vec<OsString>) -> Result<Command, Usage> {
    if arguments.first().is_some_and(|argument| argument == "agent" || argument == "local") {
        return Ok(Command::Legacy(arguments));
    }
    let mut arguments = arguments.into_iter();
    let name = arguments.next();
    let exec = match name.as_deref().and_then(std::ffi::OsStr::to_str) {
        Some("exec") => true,
        Some("check") => false,
        _ => return Err(Usage("expected `smith exec [FLAGS] [PROMPT | -]` or `smith check [--json]`".into())),
    };
    let mut shared = Shared::default();
    let mut json = false;
    let mut prompt = None;
    let mut deliver = false;
    while let Some(argument) = arguments.next() {
        let argument = argument.to_str().ok_or_else(|| Usage("a flag is not UTF-8".into()))?;
        match argument {
            "--json" => json = true,
            "-C" => shared.directory = Some(value(&mut arguments, "-C")?.into()),
            "--trace" => shared.trace = Some(value(&mut arguments, "--trace")?.into()),
            "-m" => shared.model = Some(text(&mut arguments, "-m")?),
            "--effort" => shared.effort = Some(text(&mut arguments, "--effort")?),
            "--chat" => shared.chat = Some(text(&mut arguments, "--chat")?),
            "-c" => shared.overrides.push(text(&mut arguments, "-c")?),
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
    if exec {
        Ok(Command::Exec(Exec { shared, prompt: prompt.unwrap_or(Prompt::Stdin), deliver, json }))
    } else {
        Ok(Command::Check { shared, json })
    }
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
    fn check_collects_shared_flags_and_ordered_overrides() {
        let Command::Check { shared, json } = parse(arguments(&[
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
        assert_eq!(shared.directory, Some(PathBuf::from("work")));
        assert_eq!(shared.model.as_deref(), Some("codex/model"));
        assert_eq!(shared.effort.as_deref(), Some("high"));
        assert_eq!(shared.chat.as_deref(), Some("plan"));
        assert_eq!(shared.trace, Some(PathBuf::from("trace")));
        assert_eq!(shared.overrides, ["budget.turns=4", "budget.turns=5"]);
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
    fn legacy_arguments_are_preserved_until_the_product_replaces_them() {
        let original = arguments(&["local", "settings.json", "state"]);
        assert_eq!(parse(original.clone()), Ok(Command::Legacy(original)));
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
}
