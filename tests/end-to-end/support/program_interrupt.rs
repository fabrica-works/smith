//! Observe the third status call before signalling through the shared referee.
use super::Process;
use skein_lib::{Duration, Time};
use std::{io::BufRead as _, path::PathBuf};

pub(super) struct Watch {
    path: PathBuf,
    poll: Time,
    #[expect(clippy::disallowed_types, reason = "The outside observer retains a fixture log, not a ring descriptor")]
    reader: Option<std::io::BufReader<std::fs::File>>,
    pending: Vec<u8>,
    statuses: usize,
    sent: Option<Time>,
    status_process: Option<u32>,
    settled: bool,
}

impl Watch {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            poll: Time::ZERO,
            reader: None,
            pending: Vec::new(),
            statuses: 0,
            sent: None,
            status_process: None,
            settled: false,
        }
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
        // The fake program appends this log outside the world's ring. Keep the
        // ten-millisecond observation period, reading each appended line once.
        if self.reader.is_none() {
            self.reader = std::fs::OpenOptions::new().read(true).open(&self.path).ok().map(std::io::BufReader::new);
        }
        if let Some(reader) = &mut self.reader {
            while reader.read_until(b'\n', &mut self.pending).expect("program log append") > 0 {
                if self.pending.last() != Some(&b'\n') {
                    break;
                }
                if let Ok(record) = serde_json::from_slice::<serde_json::Value>(&self.pending)
                    && record.get("arguments") == Some(&serde_json::json!(["auth", "status"]))
                {
                    self.statuses = self.statuses.checked_add(1).expect("bounded program status observations");
                    if self.statuses == 3 {
                        self.status_process = Some(
                            u32::try_from(
                                record
                                    .get("process")
                                    .and_then(serde_json::Value::as_u64)
                                    .expect("injected process number"),
                            )
                            .expect("process number fits"),
                        );
                    }
                }
                self.pending.clear();
            }
        }
        if self.statuses == 3 {
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
