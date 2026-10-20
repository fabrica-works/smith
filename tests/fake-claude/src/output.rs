//! Queued standard output and observation appends (protocol/programs.md,
//! section 8; testing.md, section 4), flushed through skein's streams.

use std::collections::VecDeque;

use skein_io::Request;
use skein_lib::{
    Queue, Token,
    stream::{Down, Read, Up},
};

#[derive(Debug)]
pub(crate) struct Output {
    token: Option<Token>,
    bytes: VecDeque<Box<[u8]>>,
    waiting: bool,
    closing: bool,
}

impl Output {
    pub fn new(token: Option<Token>) -> Self {
        Self { token, bytes: VecDeque::new(), waiting: false, closing: false }
    }

    pub fn record_process(&mut self, process: u32) {
        if let Some(bytes) = self.bytes.front_mut() {
            let mut record: serde_json::Value = serde_json::from_slice(bytes).expect("invocation record");
            record.as_object_mut().expect("invocation object").insert("process".into(), process.into());
            let mut encoded = serde_json::to_vec(&record).expect("process observation");
            encoded.push(b'\n');
            *bytes = encoded.into();
        }
    }

    pub fn token(&self) -> Option<Token> {
        self.token
    }

    pub fn push(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(super::fake::OUTPUT_BYTES) {
            self.bytes.push_back(chunk.into());
        }
    }

    pub fn event(&mut self, up: Up, requests: &mut Queue<Request>) {
        match up {
            Up::Room => {
                let bytes = self.bytes.pop_front().expect("room requested for queued output");
                requests.push(Request::Stream { stream: self.token.expect("open output"), down: Down::Send(bytes) });
                self.waiting = false;
            }
            Up::Failed(_) => {
                self.bytes.clear();
                self.waiting = false;
            }
            Up::Bytes(_) | Up::End => {}
        }
    }

    pub fn closed(&mut self) {
        self.token = None;
    }

    pub fn drained(&self) -> bool {
        self.bytes.is_empty() && !self.waiting
    }

    pub fn is_empty(&self) -> bool {
        self.token.is_none() && self.bytes.is_empty()
    }

    pub fn progress(&mut self, ending: bool, requests: &mut Queue<Request>) {
        if self.waiting || self.closing {
            return;
        }
        if let Some(stream) = self.token {
            if let Some(bytes) = self.bytes.front() {
                requests.push(Request::Stream {
                    stream,
                    down: Down::Demand {
                        read: Read::Nothing,
                        room: u32::try_from(bytes.len()).expect("output chunks fit a u32"),
                    },
                });
                self.waiting = true;
            } else if ending {
                requests.push(Request::Close { entity: stream });
                self.closing = true;
            }
        }
    }
}
