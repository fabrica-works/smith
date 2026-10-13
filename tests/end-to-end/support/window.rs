//! Outside terminal observations on the existing shipped-binary world.
use super::{Process, Refusal};
use skein_io::kernel::{Fd, Size};
use skein_lib::Time;
use skein_shell::{Modes, resize_terminal, terminal_modes};
use skein_world::Referee;
use smith_local_process_world::process::Proc;

#[derive(Clone, Copy)]
pub(super) enum Story {
    None,
    Modes,
    Resize,
}

enum Resize {
    None,
    Response,
    Done,
}

struct Watch {
    master: Fd,
    before: Modes,
    during: Option<Modes>,
    after: Option<Modes>,
    resize: Resize,
}

pub(super) struct Judge {
    pub common: Refusal,
    watch: Option<Watch>,
}

impl Judge {
    pub fn new(common: Refusal, story: Story, master: Option<Fd>) -> Self {
        let watch = match story {
            Story::None => None,
            Story::Modes | Story::Resize => {
                let master = master.expect("terminal story has a master");
                Some(Watch {
                    master,
                    before: terminal_modes(master).expect("initial terminal modes"),
                    during: None,
                    after: None,
                    resize: match story {
                        Story::Resize => Resize::Response,
                        Story::None | Story::Modes => Resize::None,
                    },
                })
            }
        };
        Self { common, watch }
    }
}

impl Watch {
    fn inspect(&mut self, procs: &[Process]) {
        let binary = procs
            .iter()
            .find_map(|process| match process {
                Process::Binary(binary) => Some(binary),
                Process::Script(_) | Process::Sample(_) => None,
            })
            .expect("shipped binary");
        if binary.exit_status().is_some() {
            if self.after.is_none() {
                self.after = Some(terminal_modes(self.master).expect("terminal modes after exit"));
            }
            return;
        }
        let shown = procs
            .iter()
            .find_map(|process| match process {
                Process::Script(Proc::Terminal(person)) => Some(person.shown()),
                Process::Binary(_) | Process::Script(_) | Process::Sample(_) => None,
            })
            .expect("scripted person");
        let modes = terminal_modes(self.master).expect("running terminal modes");
        assert_eq!(modes, self.before, "smith leaves running terminal modes alone");
        self.during = Some(modes);
        match self.resize {
            Resize::Response if shown.windows(21).any(|bytes| bytes == b"codex/gpt-6-luna: in ") => {
                resize_terminal(self.master, Size { rows: 24, columns: 20 }).expect("second window change");
                self.resize = Resize::Done;
            }
            Resize::None | Resize::Response | Resize::Done => {}
        }
    }

    fn passed(&self) -> bool {
        if self.after.is_none() {
            return false;
        }
        assert_eq!(self.during, Some(self.before), "running smith leaves terminal modes alone");
        assert_eq!(self.after, Some(self.before), "exited smith leaves terminal modes alone");
        matches!(self.resize, Resize::None | Resize::Done)
    }
}

impl Referee<Process> for Judge {
    fn act(&mut self, now: Time, procs: &mut [Process]) {
        if let Some(watch) = &mut self.watch {
            watch.inspect(procs);
        }
        self.common.act(now, procs);
    }

    fn observe(&mut self, now: Time, procs: &[Process]) {
        self.common.observe(now, procs);
    }

    fn next_deadline(&self) -> Option<Time> {
        self.common.next_deadline()
    }

    fn overdue(&self, now: Time) -> Option<String> {
        self.common.overdue(now)
    }

    fn passed(&self) -> bool {
        self.common.passed() && self.watch.as_ref().is_none_or(Watch::passed)
    }
}
