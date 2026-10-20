//! Program accounts keep host answers separate from the spawned agent's facts.
//! Both captures are bounded by the attempt log ceiling (benchmarks.md 6.2/8).

use std::path::Path;

use crate::{AttemptResult, guard::Guard, process::ProcessResult};

pub(super) fn collect(
    guard: &Guard,
    directory: &Path,
    result: &mut AttemptResult,
    process: &mut ProcessResult,
    maximum: usize,
) -> Result<(), String> {
    if let Some(crate::agent::Observing::Smith(observing)) = process.observing.as_deref_mut()
        && observing.prepared.program_source()
    {
        use std::io::Read;
        let mut path = observing
            .prepared
            .trace
            .as_ref()
            .ok_or("program account requires a trace path")?
            .as_os_str()
            .to_os_string();
        path.push(".agent");
        let captured = (|| {
            let file = std::fs::File::open(path).map_err(|error| format!("agent trace open: {error}"))?;
            let mut bytes = Vec::new();
            file.take(u64::try_from(maximum).map_err(|error| error.to_string())?.saturating_add(1))
                .read_to_end(&mut bytes)
                .map_err(|error| format!("agent trace read: {error}"))?;
            Ok::<_, String>(bytes)
        })();
        if let Ok(bytes) = &captured {
            super::archive(guard, directory, result, "trace.jsonl.agent", &bytes[..bytes.len().min(maximum)])?;
        }
        observing.agent_trace(captured.as_deref().map_err(Clone::clone).and_then(|bytes| {
            if bytes.len() > maximum { Err("agent trace exceeds the attempt log bound".into()) } else { Ok(bytes) }
        }));
    }
    Ok(())
}
