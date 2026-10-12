#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Line(Vec<u8>),
    TooLong,
}

#[derive(Debug)]
pub struct Decoder {
    max_payload: usize,
}

impl Decoder {
    pub fn new(max_payload: usize) -> Self {
        Self { max_payload }
    }

    pub fn push(&mut self, bytes: &[u8]) -> Vec<Event> {
        let mut events = Vec::new();
        for part in bytes.split(|byte| *byte == b'\n') {
            if part.is_empty() {
                continue;
            }
            if part.len() > self.max_payload {
                events.push(Event::TooLong);
            } else {
                events.push(Event::Line(part.to_vec()));
            }
        }
        events
    }

    pub fn finish(&mut self) -> Vec<Event> {
        Vec::new()
    }

    pub fn buffered_len(&self) -> usize {
        0
    }
}
