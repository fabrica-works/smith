//! Bounded random provider and tool terminals.

use super::{
    Ask, Call, Change, Completion, Declared, DeclaredFailure, Decoded, Done, Driver, Entry, Exit, Families, Fault,
    Field, Hit, Item, Kind, Limits, OpKind, Problem, Receiving, Report, Said, Stop, Tools, Usage, Verdict, Version,
    bytes, index, path, run,
};

impl Driver {
    /// A bounded terminal, independently pricing provider and decoded
    /// ownership; history capacity cannot enlarge the advertised receiving cap.
    pub(super) fn completion(
        &mut self,
        limits: &Limits,
        finish: bool,
        agents: bool,
        receiving: Receiving,
    ) -> Completion {
        let completion_cell_bytes = completion_cell();
        let usage = Usage {
            input_tokens: Some(100),
            output_tokens: Some(20),
            cache_read_tokens: Some(50),
            cache_write_tokens: Some(50),
            reasoning_tokens: None,
        };
        if self.rng.chance(150) {
            let available =
                receiving.bytes.checked_sub(completion_cell_bytes).expect("admitted receiving cap holds one block");
            let size = if self.rng.chance(500) { available } else { self.rng.below(available + 1) };
            let text = Said::Text { text: bytes(size), replay: None };
            return Completion {
                reasoning_dropped: Box::default(),
                content: Box::new([text]),
                stop: Stop::EndTurn,
                usage,
            };
        }
        let classification = u64::try_from(core::mem::size_of::<Decoded>()).expect("fixed cell");
        let minimum = completion_cell_bytes + classification + 6;
        let count = (1 + self.rng.below(2))
            .min(u64::from(receiving.blocks))
            .min(receiving.bytes / minimum)
            .min(receiving.decoded / classification);
        assert!(count > 0, "admitted receiving contract holds one call and its classification");
        let decoded_cap = receiving.decoded / count;
        let block_cap = receiving.bytes / count;
        let most = block_cap.min(decoded_cap) / 4;
        let content = (0..count)
            .map(|at| {
                let size = self.rng.below(most + 1);
                let mut call = self.call(limits, size, finish, agents);
                let mut owned = call.owned_bytes().expect("bounded decoded fixture");
                if owned > decoded_cap || completion_cell_bytes + 6 + owned > block_cap {
                    call = Decoded::Invalid { problem: Problem::TooLarge };
                    owned = classification;
                }
                let input_cap = block_cap - completion_cell_bytes - 6 - owned;
                Said::ToolCall {
                    id: format!("c{at}").into_bytes().into(),
                    name: bytes(4),
                    input: bytes(self.rng.below(input_cap + 1)),
                    call,
                    replay: None,
                }
            })
            .collect();
        Completion { reasoning_dropped: Box::default(), content, stop: Stop::ToolUse, usage }
    }

    fn call(&mut self, limits: &Limits, size: u64, finish: bool, agents: bool) -> Decoded {
        let lib = || path(&[b"src", b"lib.rs"]);
        let call = match self.rng.below(9) {
            0 => Call::Read { path: lib(), offset: 1, window: smith_domain_tools::Budget::Default },
            1 => Call::List { path: path(&[b"src"]), depth: smith_domain_tools::Depth::Whole, glob: None },
            2 => Call::Search { path: path(&[b"src"]), pattern: bytes(3), glob: None },
            3 => Call::Write { path: lib(), content: bytes(size) },
            4 => Call::Edit {
                path: lib(),
                edits: Box::new([smith_domain::tools::Replacement {
                    old: bytes(1),
                    new: bytes(size),
                    occurrences: smith_domain::tools::Occurrences::Once,
                }]),
            },
            5 => Call::Shell {
                after: smith_domain_tools::Yield::Never,
                command: bytes(8),
                timeout: smith_domain::tools::Timeout::Default,
                output: smith_domain::tools::Budget::Default,
            },
            6 if finish => return Decoded::Served { ask: self.finish(limits) },
            7 if agents => {
                let families = Families {
                    tools: Tools { inspect: true, modify: self.rng.chance(500), shell: false },

                    agents: self.rng.chance(500),
                };
                return Decoded::Served {
                    ask: Ask::SubAgent { brief: bytes(size), families, llm: None, share: run::Share::UNASKED },
                };
            }
            _ => return Decoded::Invalid { problem: Problem::Missing { field: bytes(size.min(16)) } },
        };
        Decoded::Owned { call }
    }

    /// A finish of every form, with ownership near or beyond the aggregate cap.
    fn finish(&mut self, limits: &Limits) -> Ask {
        let most = limits.run.outcome_bytes;
        let outcome = match self.rng.below(4) {
            0 => Declared::Report(Report {
                text: bytes(self.rng.below(most.saturating_add(1))),
                fields: Box::new([Field { name: b"extra".as_slice().into(), value: bytes(self.rng.below(most)) }]),
            }),
            1 => Declared::Failure(DeclaredFailure {
                reason: bytes(self.rng.below(most.saturating_add(1))),
                fields: Box::new([]),
            }),
            2 => Declared::Change(Change {
                fields: Box::new([
                    smith_domain::run::outcome::Field { name: b"title".as_slice().into(), value: bytes(1) },
                    smith_domain::run::outcome::Field {
                        name: b"body".as_slice().into(),
                        value: bytes(self.rng.below(most)),
                    },
                ]),
            }),
            _ => {
                let child = || Item {
                    kind: (*b"nit").into(),
                    fields: Box::new([Field { name: (*b"path").into(), value: bytes(8) }]),
                };
                let children = (0..self.rng.below(3)).map(|_| child()).collect();
                let name = (*b"request-changes").into();
                Declared::Verdict(Verdict {
                    name,
                    text: bytes(self.rng.below(most / 2)),
                    items: children,
                    fields: Box::new([]),
                })
            }
        };
        Ask::Finish { outcome }
    }

    /// A terminal io may end an operation of `op` with, as large as the
    /// tools take.
    pub(super) fn done(&mut self, limits: &Limits, op: OpKind) -> Done {
        let tools = &limits.session.tools;
        let version = Version::new([self.rng.below(3), 0, 0, 0]);
        if self.rng.chance(100) {
            return [Done::Failed { fault: Fault::Other }, Done::TimedOut][index(&mut self.rng, 2)].clone();
        }
        match op {
            OpKind::Load => Done::Loaded { content: bytes(self.rng.below(u64::from(tools.file_bytes) + 1)), version },
            OpKind::Scan => {
                let entries = (0..tools.list_entries)
                    .map(|entry| Entry { name: format!("e{entry}").into_bytes().into(), kind: Kind::File });
                Done::Listed { entries: entries.collect(), more: 3 }
            }
            OpKind::Store => Done::Stored { version },
            OpKind::Spawn => {
                let (head, tail) = (u64::from(tools.shell_head), u64::from(tools.shell_tail));
                Done::Exited { exit: Exit::Code { code: 1 }, head: bytes(head), tail: bytes(tail), dropped: 100 }
            }
            OpKind::Search { hits, byte_cap } => {
                assert!(hits > 0, "the fixture requests a nonempty search result");
                let paths = 6 * u64::from(hits);
                let remaining = u64::from(byte_cap).checked_sub(paths).expect("requested cap holds every hit path");
                let per_hit = remaining / u64::from(hits);
                let extra = remaining % u64::from(hits);
                let found: Box<[Hit]> = (0..hits)
                    .map(|line| Hit { path: bytes(6), line, text: bytes(per_hit + u64::from(u64::from(line) < extra)) })
                    .collect();
                assert_eq!(found.len(), usize::try_from(hits).expect("bounded requested hit count"));
                assert_eq!(
                    found.iter().map(|hit| hit.path.len() + hit.text.len()).sum::<usize>(),
                    usize::try_from(byte_cap).expect("bounded requested path plus text cap"),
                    "actual lower search result attains the full aggregate path and text bound"
                );
                self.seen[6] += 1;
                Done::Found { hits: found, more: 1, timed_out: false }
            }
        }
    }
}

/// Full translated terminal cells reserve the larger root/session layout.
fn completion_cell() -> u64 {
    u64::try_from(core::mem::size_of::<Said>().max(core::mem::size_of::<smith_domain::session::llm::Block>()))
        .expect("fixed translated block cell")
}

/// Price every original provider field, replay and decoded owning value, plus
/// all translated cells. This boundary check precedes the actual step call.
pub(super) fn completion_owned(completion: &Completion) -> (u64, u64) {
    let mut owned = completion_cell() * u64::try_from(completion.content.len()).expect("bounded actual cells");
    let mut decoded = 0;
    for block in &completion.content {
        let payload = match block {
            Said::Opaque { bytes } => u64::try_from(bytes.len()).expect("bounded opaque bytes"),
            Said::Text { text, replay } | Said::Refusal { text, replay } => {
                u64::try_from(text.len()).expect("bounded text")
                    + replay.as_ref().map_or(0, |replay| u64::try_from(replay.bytes.len()).expect("bounded replay"))
            }
            Said::ToolCall { id, name, input, call, replay } => {
                let call_bytes = call.owned_bytes().expect("bounded decoded ownership");
                decoded += call_bytes;
                u64::try_from(id.len() + name.len() + input.len()).expect("bounded provider fields")
                    + replay.as_ref().map_or(0, |replay| u64::try_from(replay.bytes.len()).expect("bounded replay"))
                    + call_bytes
            }
        };
        owned += payload;
    }
    (owned, decoded)
}

/// Return unread capture at an ended read, or a stopped group.
pub(super) fn job_done(act: smith_domain_tools::JobAct) -> Done {
    let (head, tail, end) = match act {
        smith_domain_tools::JobAct::Hasten => unreachable!("this driver answers every read at once"),
        smith_domain_tools::JobAct::Read { head, tail, .. } => {
            (head, tail, smith_domain_tools::JobEnd::Ended(Exit::Code { code: 0 }))
        }
        smith_domain_tools::JobAct::Stop { head, tail } => (head, tail, smith_domain_tools::JobEnd::Stopped),
    };
    Done::Job { head: bytes(u64::from(head)), tail: bytes(u64::from(tail)), dropped: 0, end }
}
