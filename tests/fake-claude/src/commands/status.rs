//! Cross-process status answers, using the fake's configuration files only.
use super::{Config, Effect, FILE_OWNER, error, reply};
use crate::{LOGIN_FILE, Status};
use skein_io::{file, kernel};
const COUNTER: &str = "fake-claude.status-counter";

#[derive(Debug)]
pub(crate) enum StatusCommand {
    Reading,
    Counter { choices: Vec<Status>, config: Config },
    Saving { status: Status, config: Config },
    Silent,
    Done,
}

impl StatusCommand {
    pub(super) fn start(status: Status, config: Config) -> (Self, Vec<Effect>) {
        match status {
            Status::NotSignedIn => (Self::Done, status_reply(None)),
            Status::Sequence(choices) => match config.root {
                Some(root) if !choices.is_empty() => (
                    Self::Counter { choices, config },
                    vec![Effect::File(file::Request::Load {
                        owner: FILE_OWNER,
                        root,
                        path: COUNTER.as_bytes().into(),
                        max: 32,
                        no_follow: true,
                    })],
                ),
                Some(_) | None => (Self::Done, error("status sequence needs a configuration directory and answers", 1)),
            },
            Status::Silent => (Self::Silent, Vec::new()),
            Status::Unreadable => (Self::Done, reply(b"not json\n".to_vec(), 0)),
            Status::Truthful => match config.root {
                Some(root) => (
                    Self::Reading,
                    vec![Effect::File(file::Request::Load {
                        owner: FILE_OWNER,
                        root,
                        path: LOGIN_FILE.as_bytes().into(),
                        max: crate::fake::FILE_BYTES,
                        no_follow: true,
                    })],
                ),
                None => (Self::Done, status_reply(None)),
            },
        }
    }

    pub(super) fn file(&mut self, event: file::Event) -> Vec<Effect> {
        match std::mem::replace(self, Self::Done) {
            Self::Counter { choices, config } => {
                let position = match event {
                    file::Event::Loaded { bytes, .. } => {
                        std::str::from_utf8(&bytes).ok().and_then(|text| text.parse::<usize>().ok())
                    }
                    file::Event::Failed { error: kernel::Error::NotFound, .. } => Some(0),
                    _ => None,
                };
                let Some(position) = position else { return error("cannot read status counter", 1) };
                let status = choices.get(position).or_else(|| choices.last()).expect("nonempty sequence").clone();
                let next = position.checked_add(1).expect("bounded test invocations");
                let root = config.root.expect("sequence root");
                *self = Self::Saving { status, config };
                return vec![Effect::File(file::Request::Store {
                    owner: FILE_OWNER,
                    root,
                    path: COUNTER.as_bytes().into(),
                    bytes: next.to_string().into_bytes().into(),
                    expected: file::Expect::Any,
                    no_follow: true,
                })];
            }
            Self::Saving { status, config } => match event {
                file::Event::Stored { .. } => {
                    let (state, effects) = Self::start(status, config);
                    *self = state;
                    return effects;
                }
                _ => return error("cannot save status counter", 1),
            },
            Self::Reading => {}
            Self::Silent | Self::Done => return Vec::new(),
        }
        match event {
            file::Event::Loaded { bytes, .. } => match String::from_utf8(bytes.into_vec()) {
                Ok(method) => status_reply(Some(&method)),
                Err(_) => error("login method is not UTF-8", 1),
            },
            file::Event::Failed { error: kernel::Error::NotFound, .. } => status_reply(None),
            _ => error("cannot read login", 1),
        }
    }
}

fn status_reply(method: Option<&str>) -> Vec<Effect> {
    let value = serde_json::json!({
        "loggedIn": method.is_some(),
        "authMethod": method.unwrap_or("none"),
        "apiProvider": "firstParty",
    });
    let mut bytes = serde_json::to_vec(&value).expect("status is JSON data");
    bytes.push(b'\n');
    reply(bytes, if method.is_some() { 0 } else { 1 })
}
