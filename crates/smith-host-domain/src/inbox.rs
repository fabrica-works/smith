//! Accepted names, the sent prefix, the read fence and one unread report
//! (domain/host.md, sections 4 and 9.1). Both agent kinds call admission,
//! sending, fence and terminal reporting. This state knows no message text,
//! process, provider or host policy.

use alloc::boxed::Box;
use skein_lib::{List, Queue, Token};

use crate::MessageRefusal;

#[derive(Debug)]
enum Report {
    Owed,
    Told,
}

#[cfg(test)]
mod tests {
    use super::Inbox;
    use crate::MessageRefusal;
    use skein_lib::Token;

    #[test]
    fn admission_uses_the_runs_rendered_bound_and_keeps_each_accepted_name() {
        let mut inbox = Inbox::new(1);
        assert_eq!(inbox.admit(Token::new(1), b"p", b"hi", 4), Err(MessageRefusal::TooLarge));
        assert_eq!(inbox.admit(Token::new(1), b"p", b"hi", 5), Ok(()));
        assert_eq!(inbox.admit(Token::new(1), b"p", b"hi", 5), Err(MessageRefusal::NameInUse));
        assert_eq!(inbox.admit(Token::new(2), b"p", b"hi", 5), Err(MessageRefusal::Full));
        assert_eq!(inbox.report().expect("one report").as_ref(), &[Token::new(1)]);
    }

    #[test]
    fn a_fence_covers_only_the_actual_sent_prefix_and_reports_queued_names() {
        let mut inbox = Inbox::new(3);
        for name in 1..=3 {
            inbox.admit(Token::new(name), b"p", b"hi", 5).expect("accepted message");
        }
        inbox.sent(Token::new(1));
        inbox.sent(Token::new(2));
        assert!(!inbox.known(Some(Token::new(3))), "queued names cannot be read");
        inbox.fence(Some(Token::new(1)));
        assert!(inbox.refused(Token::new(2)));
        assert_eq!(inbox.report().expect("one report").as_ref(), &[Token::new(3)]);
        assert!(inbox.report().is_none());
        assert!(!inbox.refused(Token::new(3)), "no refusal follows an unread terminal");
    }

    #[test]
    fn an_empty_report_is_still_once_and_closes_message_admission() {
        let mut inbox = Inbox::new(1);
        inbox.admit(Token::new(1), b"p", b"hi", 5).expect("accepted message");
        inbox.sent(Token::new(1));
        inbox.fence(Some(Token::new(1)));
        assert!(inbox.report().expect("empty report").is_empty());
        assert!(inbox.report().is_none());
        assert_eq!(inbox.admit(Token::new(2), b"p", b"hi", 5), Err(MessageRefusal::Ending));
    }
}

/// An agent capability's accepted names, ending read, refused or unread once.
#[derive(Debug)]
pub struct Inbox {
    names: Queue<Token>,
    sent: u32,
    read: Option<Token>,
    report: Report,
}

impl Inbox {
    /// Allocate the capability's fixed count of accepted unread names.
    #[must_use]
    pub fn new(capacity: u32) -> Inbox {
        Inbox { names: Queue::with_capacity(capacity), sent: 0, read: None, report: Report::Owed }
    }

    /// Accepted messages still awaiting a terminal, sent and queued together.
    #[must_use]
    pub fn len(&self) -> u32 {
        self.names.len()
    }

    /// Whether every accepted message already has a terminal.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// The last message ended by an accepted read fence.
    pub(crate) const fn read(&self) -> Option<Token> {
        self.read
    }

    /// Admit by the run's rendered size, name and count arithmetic.
    pub fn admit(&mut self, name: Token, label: &[u8], text: &[u8], bound: u64) -> Result<(), MessageRefusal> {
        match self.report {
            Report::Told => return Err(MessageRefusal::Ending),
            Report::Owed => {}
        }
        match smith_domain::run::rendered_bytes(label, text) {
            Some(bytes) if bytes <= bound => {}
            Some(_) | None => return Err(MessageRefusal::TooLarge),
        }
        if self.contains(name) {
            return Err(MessageRefusal::NameInUse);
        }
        if self.names.room() == 0 {
            return Err(MessageRefusal::Full);
        }
        self.names.push(name);
        Ok(())
    }

    /// Whether a name is accepted or is the current final read fence.
    #[must_use]
    pub fn contains(&self, name: Token) -> bool {
        if self.read == Some(name) {
            return true;
        }
        for old in &self.names {
            if *old == name {
                return true;
            }
        }
        false
    }

    /// Mark the next accepted name sent, preserving the arrival prefix.
    pub fn sent(&mut self, name: Token) {
        let mut position = 0_u32;
        for accepted in &self.names {
            if position == self.sent {
                assert_eq!(*accepted, name, "accepted messages are sent in arrival order");
                self.sent = self.sent.checked_add(1).expect("sent prefix is within accepted names");
                return;
            }
            position = position.checked_add(1).expect("bounded accepted names");
        }
        unreachable!("a sent message was first accepted");
    }

    /// Whether a fence is current or names a sent unread message.
    #[must_use]
    pub fn known(&self, read: Option<Token>) -> bool {
        match read {
            None => self.read.is_none(),
            Some(name) => {
                if self.read == Some(name) {
                    return true;
                }
                let mut position = 0_u32;
                for sent in &self.names {
                    if position == self.sent {
                        break;
                    }
                    if *sent == name {
                        return true;
                    }
                    position = position.checked_add(1).expect("bounded sent prefix");
                }
                false
            }
        }
    }

    /// End every sent name through a validated turn, wait or answer fence.
    pub fn fence(&mut self, read: Option<Token>) {
        assert!(self.known(read), "the capability validates each fence before accepting it");
        match read {
            Some(name) if self.read != Some(name) => {
                let count = self.sent;
                for _ in 0..count {
                    let old = self.names.pop().expect("known sent prefix");
                    self.sent = self.sent.checked_sub(1).expect("one sent name ended read");
                    self.read = Some(old);
                    if old == name {
                        return;
                    }
                }
                unreachable!("validated fence belongs to sent prefix");
            }
            Some(_) | None => {}
        }
    }

    /// End an accepted sent name refused by the run, preserving the other names.
    pub fn refused(&mut self, name: Token) -> bool {
        if !self.refusal_known(name) {
            return false;
        }
        let count = self.names.len();
        let sent = self.sent;
        for position in 0..count {
            let old = self.names.pop().expect("bounded accepted names snapshot");
            if old == name {
                assert!(position < sent, "the run refuses a name it received");
                self.sent = self.sent.checked_sub(1).expect("one sent name ended refused");
            } else {
                self.names.push(old);
            }
        }
        true
    }

    /// Whether a run refusal owns a sent name still awaiting its terminal.
    #[must_use]
    pub fn refusal_known(&self, name: Token) -> bool {
        self.read != Some(name) && self.known(Some(name))
    }

    /// End the remaining accepted names unread, once, including unsent names.
    #[must_use]
    pub fn report(&mut self) -> Option<Box<[Token]>> {
        match self.report {
            Report::Told => return None,
            Report::Owed => self.report = Report::Told,
        }
        let count = self.names.len();
        let mut names = List::with_capacity(count);
        for _ in 0..count {
            names.push(self.names.pop().expect("bounded unread snapshot")).expect("one entry per accepted name");
        }
        self.sent = 0;
        Some(names.into_boxed())
    }
}
