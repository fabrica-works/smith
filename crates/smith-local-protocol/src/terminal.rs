//! Bounded line and signal translation for one person at a terminal
//! (protocol/hosts.md, section 5.1). Only completed lines reach the local
//! domain; a second interrupt asks the selected child to drop through local policy.

use alloc::boxed::Box;
use skein_lib::{List, Queue};
use smith_local_domain as local;

/// Receiving and display bounds of one local terminal.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum bytes in one continued message.
    pub line_bytes: u32,
    /// Maximum bytes in one shown notice.
    pub show_bytes: u32,
}

/// One output to the local service.
#[derive(Debug)]
pub enum Event {
    /// A completed person line for the local domain.
    Domain(local::Event),
    /// A second interrupt before an answer drops the selected child; its terminals still settle.
    DropRun,
    /// A bounded notice to write on the terminal.
    Write(Box<[u8]>),
}

/// Maximum output cells emitted by one entry point.
#[must_use]
pub const fn max_out() -> u32 {
    1
}

/// Checked retained upper bound for the terminal.
#[must_use]
pub fn worst_case(limits: &Limits) -> Option<u64> {
    if limits.line_bytes == 0 || limits.show_bytes == 0 {
        return None;
    }
    List::<u8>::worst_case(limits.line_bytes)?.checked_add(u64::from(limits.show_bytes))
}

/// One line accumulator and the live run's interrupt count.
#[derive(Debug)]
pub struct Terminal {
    limits: Limits,
    line: List<u8>,
    discarding: bool,
    cancelled: bool,
}

impl Terminal {
    /// Build the bounded terminal translator.
    #[must_use]
    pub fn new(limits: Limits) -> Option<Self> {
        worst_case(&limits)?;
        Some(Self { line: List::with_capacity(limits.line_bytes), limits, discarding: false, cancelled: false })
    }

    /// Feed one byte from standard input; only a complete line emits an event.
    pub fn feed(&mut self, byte: u8, out: &mut Queue<Event>) {
        if byte == b'\n' {
            self.finish_line(out);
            return;
        }
        if self.discarding {
            return;
        }
        if self.line.push(byte).is_err() {
            self.line = List::with_capacity(self.limits.line_bytes);
            self.discarding = true;
        }
    }

    /// Translate a terminal interrupt; the second one asks local policy to drop the child.
    pub fn interrupt(&mut self, out: &mut Queue<Event>) {
        if self.cancelled {
            out.push(Event::DropRun);
        } else {
            self.cancelled = true;
            out.push(Event::Domain(local::Event::Interrupt));
        }
    }

    /// A new run can be interrupted independently of the previous answer.
    pub fn answer_finished(&mut self) {
        self.cancelled = false;
    }

    /// Translate terminal closure after any complete line.
    pub fn closed(&mut self, out: &mut Queue<Event>) {
        out.push(Event::Domain(local::Event::Closed));
    }

    /// Forward bounded local-domain text to the terminal.
    pub fn show(&self, text: Box<[u8]>, out: &mut Queue<Event>) {
        let cap = usize::try_from(self.limits.show_bytes).expect("u32 fits usize");
        let kept = text.get(..cap).unwrap_or(text.as_ref());
        out.push(Event::Write(Box::from(kept)));
    }

    /// Render a typed host notice as bounded UTF-8 lines ending in newlines.
    pub fn notice(&self, notice: local::Notice, chat: &[u8], out: &mut Queue<Event>) {
        let mut text = Shown { bytes: List::with_capacity(self.limits.show_bytes), cut: false };
        match notice {
            local::Notice::Waiting => append(&mut text, b"Waiting for a message"),
            local::Notice::Checking(checks) => append(
                &mut text,
                match checks {
                    local::Checks::Started => b"Running checks",
                    local::Checks::Ended => b"Checks finished",
                },
            ),
            local::Notice::NotDelivered { line, why } => {
                if let Some(name) = line {
                    append(&mut text, b"Line ");
                    decimal(&mut text, name.raw());
                    append(&mut text, b" not delivered: ");
                }
                append(
                    &mut text,
                    match why {
                        local::Undelivered::TooLong => b"Line is too long",
                        local::Undelivered::Full => b"Too many lines waiting",
                        local::Undelivered::Refused(reason) => match reason {
                            smith_host_domain::MessageRefusal::TooLarge => b"it is too large",
                            smith_host_domain::MessageRefusal::Full => b"the inbox is full",
                            smith_host_domain::MessageRefusal::Ending => b"the run is ending",
                            smith_host_domain::MessageRefusal::NameInUse => b"its name is in use",
                        },
                    },
                );
            }
            local::Notice::Failure(failure) => match failure {
                local::Failure::NamesExhausted => append(&mut text, b"Chat names exhausted"),
                local::Failure::Account(account) => append(
                    &mut text,
                    match account {
                        local::AccountFailure::Unavailable => b"A model account is unavailable",
                        local::AccountFailure::NotRefreshed => b"A model account could not refresh",
                        local::AccountFailure::Exhausted => b"A model account is exhausted",
                    },
                ),
                local::Failure::HeadUnavailable => append(&mut text, b"Repository head unavailable"),
                local::Failure::NotStarted => append(&mut text, b"Agent failed"),
                local::Failure::Agent(end) => append(
                    &mut text,
                    match end {
                        local::AgentEnd::Failed => b"Agent process failed",
                        local::AgentEnd::Dropped => b"Run dropped",
                        local::AgentEnd::NoAnswer => b"Agent process ended without an answer",
                    },
                ),
                local::Failure::DeliveryUnsaved => {
                    append(&mut text, b"A delivery committed, but its answer could not be saved");
                }
                local::Failure::Store { turn, reason } => {
                    append(&mut text, b"The chat could not be saved");
                    if let Some(number) = turn {
                        newline(&mut text);
                        append(&mut text, b"Chat ");
                        append(&mut text, chat);
                        append(&mut text, b" turn ");
                        decimal(&mut text, u64::from(number));
                        append(
                            &mut text,
                            match reason {
                                local::StoreFailure::Read => b" could not be kept: read failed",
                                local::StoreFailure::Write => b" could not be kept: write failed",
                            },
                        );
                    }
                }
            },
        }
        newline(&mut text);
        out.push(Event::Write(text.bytes.into_boxed()));
    }

    fn finish_line(&mut self, out: &mut Queue<Event>) {
        if self.discarding {
            self.discarding = false;
            out.push(Event::Write(Box::from(&b"Line is too long\n"[..])));
            return;
        }
        if self.line.as_slice().last() == Some(&b'\r') {
            self.pop_last();
        }
        if self.line.as_slice().last() == Some(&b'\\') {
            self.pop_last();
            return;
        }
        let completed = core::mem::replace(&mut self.line, List::with_capacity(self.limits.line_bytes));
        out.push(Event::Domain(local::Event::Line { text: completed.into_boxed() }));
    }

    fn pop_last(&mut self) {
        let count = self.line.as_slice().len().saturating_sub(1);
        let mut kept = List::with_capacity(self.limits.line_bytes);
        for byte in self.line.as_slice().get(..count).unwrap_or_default() {
            kept.push(*byte).expect("prefix fits line capacity");
        }
        self.line = kept;
    }
}

struct Shown {
    bytes: List<u8>,
    cut: bool,
}

// Keep one byte for a newline and cut only at a complete character.
fn append(text: &mut Shown, bytes: &[u8]) {
    if text.cut {
        return;
    }
    let mut skip = 0_u8;
    for (index, byte) in bytes.iter().enumerate() {
        if skip > 0 {
            skip = skip.checked_sub(1).expect("continuation count is positive");
            continue;
        }
        let width = if *byte < 0x80 {
            1
        } else if *byte < 0xe0 {
            2
        } else if *byte < 0xf0 {
            3
        } else {
            4
        };
        if width >= text.bytes.room() {
            text.cut = true;
            return;
        }
        let end = index.checked_add(usize::try_from(width).expect("character width fits usize")).expect("bounded text");
        let Some(character) = bytes.get(index..end) else {
            text.cut = true;
            return;
        };
        for byte in character {
            text.bytes.push(*byte).expect("character and newline room checked");
        }
        skip = u8::try_from(width.checked_sub(1).expect("positive width")).expect("character width fits u8");
    }
}

fn newline(text: &mut Shown) {
    if text.bytes.room() > 0 {
        text.bytes.push(b'\n').expect("newline room checked");
    }
}

fn decimal(text: &mut Shown, mut number: u64) {
    let mut digits = [0; 20];
    let mut first = digits.len();
    for _ in 0..digits.len() {
        first = first.checked_sub(1).expect("u64 fits twenty decimal digits");
        *digits.get_mut(first).expect("bounded decimal position") =
            u8::try_from(number % 10).expect("decimal digit").checked_add(b'0').expect("ASCII digit");
        number /= 10;
        if number == 0 {
            break;
        }
    }
    append(text, digits.get(first..).expect("bounded decimal suffix"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terminal() -> Terminal {
        Terminal::new(Limits { line_bytes: 32, show_bytes: 32 }).expect("valid terminal")
    }

    fn rendered(notice: local::Notice) -> Box<[u8]> {
        let terminal = Terminal::new(Limits { line_bytes: 32, show_bytes: 128 }).expect("valid terminal");
        let mut out = Queue::with_capacity(max_out());
        terminal.notice(notice, b"main", &mut out);
        let Some(Event::Write(text)) = out.pop() else { panic!("one notice written") };
        assert!(out.is_empty());
        text
    }

    #[test]
    fn waiting_is_one_line_ending_in_a_newline() {
        assert_eq!(rendered(local::Notice::Waiting).as_ref(), b"Waiting for a message\n");
    }

    #[test]
    fn checking_spans_are_shown_as_whole_lines() {
        for (checks, expected) in [
            (local::Checks::Started, b"Running checks\n".as_slice()),
            (local::Checks::Ended, b"Checks finished\n".as_slice()),
        ] {
            assert_eq!(rendered(local::Notice::Checking(checks)).as_ref(), expected);
        }
    }

    #[test]
    fn a_refused_line_is_shown_by_name_and_reason_as_one_line() {
        for (reason, expected) in [
            (smith_host_domain::MessageRefusal::Full, b"Line 3 not delivered: the inbox is full\n".as_slice()),
            (smith_host_domain::MessageRefusal::TooLarge, b"Line 3 not delivered: it is too large\n".as_slice()),
            (smith_host_domain::MessageRefusal::Ending, b"Line 3 not delivered: the run is ending\n".as_slice()),
            (smith_host_domain::MessageRefusal::NameInUse, b"Line 3 not delivered: its name is in use\n".as_slice()),
        ] {
            let notice = local::Notice::NotDelivered {
                line: Some(skein_lib::Token::new(3)),
                why: local::Undelivered::Refused(reason),
            };
            assert_eq!(rendered(notice).as_ref(), expected);
        }
    }

    #[test]
    fn unnamed_lines_show_the_local_admission_reason() {
        for (why, expected) in [
            (local::Undelivered::TooLong, b"Line is too long\n".as_slice()),
            (local::Undelivered::Full, b"Too many lines waiting\n".as_slice()),
        ] {
            assert_eq!(rendered(local::Notice::NotDelivered { line: None, why }).as_ref(), expected);
        }
    }

    #[test]
    fn a_store_failure_names_the_chat_and_its_turn_on_two_lines() {
        let notice =
            local::Notice::Failure(local::Failure::Store { turn: Some(3), reason: local::StoreFailure::Write });
        assert_eq!(
            rendered(notice).as_ref(),
            b"The chat could not be saved\nChat main turn 3 could not be kept: write failed\n"
        );
    }

    #[test]
    fn a_store_failure_without_a_turn_is_one_line() {
        let notice = local::Notice::Failure(local::Failure::Store { turn: None, reason: local::StoreFailure::Read });
        assert_eq!(rendered(notice).as_ref(), b"The chat could not be saved\n");
    }

    #[test]
    fn local_failures_keep_their_own_words() {
        for (failure, expected) in [
            (local::Failure::NamesExhausted, b"Chat names exhausted\n".as_slice()),
            (local::Failure::HeadUnavailable, b"Repository head unavailable\n".as_slice()),
            (local::Failure::NotStarted, b"Agent failed\n".as_slice()),
            (local::Failure::DeliveryUnsaved, b"A delivery committed, but its answer could not be saved\n".as_slice()),
        ] {
            assert_eq!(rendered(local::Notice::Failure(failure)).as_ref(), expected);
        }
    }

    #[test]
    fn account_failures_keep_their_own_words() {
        for (account, expected) in [
            (local::AccountFailure::Unavailable, b"A model account is unavailable\n".as_slice()),
            (local::AccountFailure::NotRefreshed, b"A model account could not refresh\n".as_slice()),
            (local::AccountFailure::Exhausted, b"A model account is exhausted\n".as_slice()),
        ] {
            assert_eq!(rendered(local::Notice::Failure(local::Failure::Account(account))).as_ref(), expected);
        }
    }

    #[test]
    fn unsuccessful_agent_ends_keep_their_own_words() {
        for (end, expected) in [
            (local::AgentEnd::Failed, b"Agent process failed\n".as_slice()),
            (local::AgentEnd::Dropped, b"Run dropped\n".as_slice()),
            (local::AgentEnd::NoAnswer, b"Agent process ended without an answer\n".as_slice()),
        ] {
            assert_eq!(rendered(local::Notice::Failure(local::Failure::Agent(end))).as_ref(), expected);
        }
    }

    #[test]
    fn a_capped_store_notice_ends_on_a_character_boundary_with_a_newline() {
        let terminal = Terminal::new(Limits { line_bytes: 32, show_bytes: 37 }).expect("valid terminal");
        let mut out = Queue::with_capacity(max_out());
        terminal.notice(
            local::Notice::Failure(local::Failure::Store { turn: Some(1), reason: local::StoreFailure::Write }),
            "éé".as_bytes(),
            &mut out,
        );
        let Some(Event::Write(text)) = out.pop() else { panic!("one notice written") };
        assert_eq!(text.as_ref(), "The chat could not be saved\nChat é\n".as_bytes());
    }

    #[test]
    fn a_line_and_its_continuation_are_one_message() {
        let mut terminal = terminal();
        let mut out = Queue::with_capacity(max_out());
        for byte in b"hello\\\nworld\n" {
            terminal.feed(*byte, &mut out);
        }
        match out.pop().expect("one line") {
            Event::Domain(local::Event::Line { text }) => assert_eq!(text.as_ref(), b"helloworld"),
            Event::Domain(_) | Event::DropRun | Event::Write(_) => panic!("unexpected terminal event"),
        }
        assert!(out.is_empty());
    }

    #[test]
    fn a_second_interrupt_drops_the_selected_child() {
        let mut terminal = terminal();
        let mut out = Queue::with_capacity(max_out());
        terminal.interrupt(&mut out);
        match out.pop().expect("first interrupt") {
            Event::Domain(local::Event::Interrupt) => {}
            Event::Domain(_) | Event::DropRun | Event::Write(_) => panic!("expected cancel"),
        }
        terminal.interrupt(&mut out);
        match out.pop().expect("second interrupt") {
            Event::DropRun => {}
            Event::Domain(_) | Event::Write(_) => panic!("expected process stop"),
        }
        terminal.answer_finished();
        terminal.interrupt(&mut out);
        match out.pop().expect("new run interrupt") {
            Event::Domain(local::Event::Interrupt) => {}
            Event::Domain(_) | Event::DropRun | Event::Write(_) => panic!("expected cancel"),
        }
    }

    #[test]
    fn an_overlong_line_is_discarded_until_newline() {
        let mut terminal = Terminal::new(Limits { line_bytes: 3, show_bytes: 32 }).expect("valid terminal");
        let mut out = Queue::with_capacity(max_out());
        for byte in b"abcd\n" {
            terminal.feed(*byte, &mut out);
        }
        match out.pop().expect("too long notice") {
            Event::Write(_) => {}
            Event::Domain(_) | Event::DropRun => panic!("expected notice"),
        }
        for byte in b"ok\n" {
            terminal.feed(*byte, &mut out);
        }
        match out.pop().expect("following line") {
            Event::Domain(local::Event::Line { text }) => assert_eq!(text.as_ref(), b"ok"),
            Event::Domain(_) | Event::DropRun | Event::Write(_) => panic!("expected line"),
        }
    }
}
