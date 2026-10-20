//! Product entry points over resolved settings and Skein's one-thread loop.
//! Contract: shell.md, sections 4, 5 and 9.
use std::{env, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().collect();
    let clock = skein_shell::Clock::new();
    let environment = smith_local_shell::credentials::Environment { variables: env::vars_os().collect() };
    if let Some(code) = smith_local_shell::credentials::command(&args, &clock, &environment) {
        return ExitCode::from(code);
    }
    if let Some(code) = smith_local_shell::compose::start(&args, clock.now(), &clock) {
        return ExitCode::from(code);
    }
    if let Some(code) = smith_local_shell::check::command(&args, clock.now().wall, &clock) {
        return ExitCode::from(code);
    }
    ExitCode::from(2)
}
