#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Line(Vec<u8>),
    TooLong,
}

#[derive(Debug)]
pub struct Decoder {
    max_payload: usize,
    pending: Vec<u8>,
    discarding: bool,
}

impl Decoder {
    pub fn new(max_payload: usize) -> Self {
        Self {
            max_payload,
            pending: Vec::new(),
            discarding: false,
        }
    }

    pub fn push(&mut self, bytes: &[u8]) -> Vec<Event> {
        let mut events = Vec::new();
        for &byte in bytes {
            if byte == b'\n' {
                if !self.discarding {
                    if self.pending.last() == Some(&b'\r') {
                        self.pending.pop();
                    }
                    events.push(Event::Line(std::mem::take(&mut self.pending)));
                }
                self.pending.clear();
                self.discarding = false;
            } else if !self.discarding {
                if self.pending.len() < self.max_payload
                    || (self.pending.len() == self.max_payload && byte == b'\r')
                {
                    self.pending.push(byte);
                } else {
                    self.pending.clear();
                    self.discarding = true;
                    events.push(Event::TooLong);
                }
            }
        }
        events
    }

    pub fn finish(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        if !self.discarding && !self.pending.is_empty() {
            if self.pending.len() > self.max_payload {
                events.push(Event::TooLong);
            } else {
                events.push(Event::Line(std::mem::take(&mut self.pending)));
            }
        }
        self.pending.clear();
        self.discarding = false;
        events
    }

    pub fn buffered_len(&self) -> usize {
        self.pending.len()
    }
}
