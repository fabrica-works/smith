//! Test-only peers replay recorded events and close their terminal descriptors.
//! They read no login or live provider; the terminal peer refuses an ignored
//! hangup signal so the caller cannot pass without observing a hangup.

use std::error::Error;
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

fn replay(recording: &Path, trace: &Path) -> Result<(), Box<dyn Error>> {
    let mut input = std::io::stdin().lock();
    let mut line = String::new();
    input.read_line(&mut line)?;
    fs::copy(recording, trace)?;
    line.clear();
    input.read_line(&mut line)?;
    Ok(())
}

fn terminal(recording: &Path, trace: &Path) -> Result<(), Box<dyn Error>> {
    let status = fs::read_to_string("/proc/self/status")?;
    let ignored = status.lines().find_map(|line| line.strip_prefix("SigIgn:")).ok_or("status needs SigIgn")?;
    if u64::from_str_radix(ignored.trim(), 16)? & 0x1 != 0 {
        eprintln!("test-peer: SIGHUP is ignored; a hangup would pass unseen");
        std::process::exit(3);
    }
    let mut input = std::io::stdin().lock();
    let mut line = String::new();
    input.read_line(&mut line)?;
    fs::copy(recording, trace)?;
    line.clear();
    input.read_line(&mut line)?;
    drop(input);
    skein_shell::close_keeper_fd(skein_io::kernel::Fd::new(0));
    skein_shell::close_keeper_fd(skein_io::kernel::Fd::new(1));
    std::io::stderr().write_all(b"tty stdio closed before leader exit\n")?;
    std::thread::sleep(std::time::Duration::from_millis(5));
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let raw: Vec<_> = std::env::args_os().collect();
    if raw.get(1).is_some_and(|argument| argument == "check") {
        std::io::stdout()
            .write_all(br#"{"version":2,"limits":[{"name":"service.teardown","value":1000,"unit":"ms"}]}"#)?;
        return Ok(());
    }
    if let Some(index) = raw.iter().position(|argument| argument == "--trace") {
        let trace = raw.get(index + 1).ok_or("product replay needs trace")?;
        return replay(&std::env::current_exe()?.with_file_name("recording.jsonl"), &PathBuf::from(trace));
    }
    let mut arguments = raw.into_iter().skip(1);
    let mode = arguments.next().ok_or("test-peer needs a mode")?;
    match mode.to_str() {
        Some("replay") => replay(
            &PathBuf::from(arguments.next().ok_or("replay needs a recording")?),
            &PathBuf::from(arguments.next().ok_or("replay needs trace")?),
        ),
        Some("terminal") => terminal(
            &PathBuf::from(arguments.next().ok_or("terminal needs a recording")?),
            &PathBuf::from(arguments.next().ok_or("terminal needs a trace")?),
        ),
        Some(_) | None => Err("unknown test-peer mode".into()),
    }
}
