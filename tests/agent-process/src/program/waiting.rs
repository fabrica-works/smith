//! A host message released after an independently observed held wait (programs.md 5.3).
use super::{Proc, Time};
use skein_io::kernel::Spawn;
use skein_lib::{Duration, Writer};

pub(super) fn timeout(spawn: &Spawn) {
    if spawn.args.first().is_some_and(|argument| argument.as_ref() == b"-p")
        && spawn.env.iter().any(|entry| entry.as_ref() == b"WAIT_PROOF=1")
    {
        assert!(spawn.env.iter().any(|entry| entry.as_ref() == b"MCP_TOOL_TIMEOUT=60000"), "run time in milliseconds");
        assert_eq!(spawn.env.iter().filter(|entry| entry.starts_with(b"MCP_TOOL_TIMEOUT=")).count(), 1);
    }
}

pub(super) struct Release {
    enabled: bool,
    at: Option<Time>,
    sent: bool,
}

impl Release {
    pub(super) fn new(enabled: bool) -> Self {
        Self { enabled, at: None, sent: false }
    }

    pub(super) fn deadline(&self) -> Option<Time> {
        if self.sent { None } else { self.at }
    }

    pub(super) fn passed(&self) -> bool {
        !self.enabled || self.sent
    }

    pub(super) fn observe(&mut self, now: Time, procs: &[Proc]) {
        if !self.enabled || self.sent {
            return;
        }
        let Proc::Parent(parent) = &procs[0] else { panic!("parent first") };
        if self.at.is_none()
            && parent.peer.observed().iter().any(|frame| frame.kind == 0x010a)
            && procs.iter().any(|process| matches!(process, Proc::Fake(fake) if fake.pending_tool() == Some("wait")))
        {
            self.at = Some(now.saturating_add(Duration::from_millis(50)));
        }
    }

    pub(super) fn act(&mut self, now: Time, procs: &mut [Proc]) {
        if !self.enabled || self.sent {
            return;
        }
        if self.at.is_none_or(|at| at > now) {
            return;
        }
        assert!(
            procs.iter().any(|process| matches!(process, Proc::Fake(fake) if fake.pending_tool() == Some("wait"))),
            "the MCP wait stays held before the host message"
        );
        let Proc::Parent(parent) = &mut procs[0] else { panic!("parent first") };
        parent.program_message = true;
        self.sent = true;
    }
}

pub(super) fn initial(enabled: bool) -> Option<smith_channel::Message> {
    enabled.then(|| {
        smith_channel::Message::new(
            &smith_channel::CEILINGS,
            smith_channel::MessageParts {
                finish_requested: false,
                name: 0,
                label: b"host".as_slice().into(),
                text: b"Begin".as_slice().into(),
            },
        )
        .expect("initial message")
    })
}

pub(crate) fn message() -> skein_channel::Frame {
    let message = smith_channel::Message::new(
        &smith_channel::CEILINGS,
        smith_channel::MessageParts {
            finish_requested: false,
            name: 1,
            label: b"host".as_slice().into(),
            text: b"The next instruction".as_slice().into(),
        },
    )
    .expect("host message");
    let mut body = Writer::new(message.measure() as usize);
    message.encode(&mut body).expect("message body");
    let mut frame = skein_channel::frame_writer(0x0101, message.measure()).expect("message frame");
    frame.put(&body.finish()).expect("message bytes");
    frame.finish().expect("host message frame")
}
