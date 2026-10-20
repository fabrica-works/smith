//! Observe the third status call before signalling through the shared referee.
use super::Process;
use skein_lib::{Duration, Time};
use std::path::PathBuf;

pub(super) struct Watch {
    path: PathBuf,
    poll: Time,
    sent: Option<Time>,
    status_process: Option<u32>,
    settled: bool,
}

impl Watch {
    pub fn new(path: PathBuf) -> Self {
        Self { path, poll: Time::ZERO, sent: None, status_process: None, settled: false }
    }

    pub fn act(&mut self, now: Time, procs: &mut [Process]) {
        if self.settled {
            return;
        }
        if let Some(sent) = self.sent {
            if let Some(Process::Binary(binary)) = procs.first()
                && binary.exit_status().is_some()
            {
                assert!(
                    now.saturating_since(sent) < Duration::from_secs(5),
                    "check interruption settles within five seconds"
                );
                assert!(
                    !std::path::Path::new(&format!("/proc/{}", self.status_process.expect("observed status process")))
                        .exists(),
                    "the status child is gone"
                );
                self.settled = true;
            }
            return;
        }
        if now < self.poll {
            return;
        }
        self.poll = now.saturating_add(Duration::from_millis(10));
        let bytes = std::fs::read(&self.path).unwrap_or_default();
        let statuses: Vec<serde_json::Value> = bytes
            .split(|byte| *byte == b'\n')
            .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
            .filter(|record| record.get("arguments") == Some(&serde_json::json!(["auth", "status"])))
            .collect();
        if statuses.len() == 3 {
            self.status_process = Some(
                u32::try_from(
                    statuses
                        .get(2)
                        .expect("third status")
                        .get("process")
                        .and_then(serde_json::Value::as_u64)
                        .expect("injected process number"),
                )
                .expect("process number fits"),
            );
            if let Some(Process::Binary(binary)) = procs.first_mut() {
                binary.signal(skein_io::kernel::Signal::Interrupt);
            }
            self.sent = Some(now);
        }
    }

    pub fn next_deadline(&self) -> Option<Time> {
        if self.settled {
            None
        } else {
            Some(self.sent.map_or(self.poll, |sent| sent.saturating_add(Duration::from_secs(5))))
        }
    }

    pub fn passed(&self) -> bool {
        self.settled
    }
}
