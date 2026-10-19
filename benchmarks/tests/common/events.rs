//! Codec-derived synthetic outside records; no native or live run is claimed.

use smith_events::{Capture, Event, Limits, Record};

pub const LIMITS: Limits = Limits { string: 128, content: 1024, items: 16 };

#[must_use]
pub fn record(source: &str) -> Record {
    let line = source.trim_end().as_bytes();
    // The committed codec golden is the outside line. Retain every semantic
    // cap while supplying its exact encoded ceiling, including the newline.
    let ceiling = u32::try_from(line.len() + 1).expect("bounded codec golden");
    smith_events::read_with_ceiling(line, &LIMITS, ceiling).expect("codec golden").expect("known record")
}

fn encoded(record: &Record) -> Vec<u8> {
    // Keep the synthetic semantic envelope and actual emitted inventory; the
    // independent golden sweep still retains the complete maximum LIMITS.
    {
        let items = match &record.event {
            Event::SessionEnded(ended) => ended.emitted.len().max(4),
            Event::Budget(_)
            | Event::SessionStarted(_)
            | Event::RunStarted(_)
            | Event::RunCompleted(_)
            | Event::ConversationOpened(_)
            | Event::ConversationClosed(_)
            | Event::ResponseStarted(_)
            | Event::ResponseCompleted(_)
            | Event::TextDelta(_)
            | Event::ToolStarted(_)
            | Event::JobEnded(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Notice(_) => 4,
        };
        let limits = Limits { items, ..LIMITS };
        smith_events::write(record, &Capture::Everything, &limits)
            .expect("typed record")
            .expect("captured record")
            .into_vec()
    }
}

// Cache only complete immutable wire lines. The key's complete Record equality
// includes timestamp and every field; every distinct record enters the codec,
// and each use still enters the real Reader rather than reusing parsed state.
type EncodedLines = std::collections::BTreeMap<u64, Vec<(Record, Vec<u8>)>>;

std::thread_local! {
    static ENCODED: std::cell::RefCell<EncodedLines> = const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}

#[must_use]
pub fn bytes(records: &[Record]) -> Vec<u8> {
    ENCODED.with(|cache| {
        let mut cache = cache.borrow_mut();
        let mut bytes = Vec::new();
        for record in records {
            let bucket = cache.entry(record.t_ms).or_default();
            if let Some((_, line)) = bucket.iter().find(|(previous, _)| previous == record) {
                bytes.extend_from_slice(line);
            } else {
                let line = encoded(record);
                bytes.extend_from_slice(&line);
                bucket.push((record.clone(), line));
            }
        }
        bytes
    })
}

pub fn balanced() -> Vec<Record> {
    static TEMPLATE: std::sync::LazyLock<Vec<Record>> = std::sync::LazyLock::new(build_balanced);
    TEMPLATE.clone()
}

fn text_blocks(text: &[u8]) -> skein_lib::List<smith_events::Block> {
    let mut blocks = skein_lib::List::with_capacity(1);
    blocks.push(smith_events::Block::Text { text: Box::from(text) }).expect("one text block");
    blocks
}

fn model() -> smith_events::Model {
    smith_events::Model {
        account: None,
        backend: smith_events::Backend::Direct,
        endpoint: Box::from(&b"endpoint"[..]),
        model: Box::from(&b"model"[..]),
        window_tokens: 10_000,
        output_tokens: 4096,
        effort: None,
    }
}

fn completed(value: &mut smith_events::ResponseCompleted) {
    value.run = 0;
    value.conversation = 0;
    value.response = 0;
    value.spent = 3;
    value.usage = smith_events::Usage {
        input_tokens: Some(11),
        cache_read_tokens: Some(7),
        cache_write_tokens: Some(2),
        output_tokens: Some(5),
        reasoning_tokens: Some(1),
    };
    value.failure = None;
    value.blocks = 1;
    value.calls = 0;
    value.invalid = 0;
    value.oversized = 0;
    value.cut = 0;
    value.completion = Some(smith_events::Completion { blocks: text_blocks(b"done") });
}

fn build_balanced() -> Vec<Record> {
    // Semantic controls use a small coherent exchange. The separate golden
    // sweep still reads every unmodified maximum-value codec record.
    let model = model();
    let mut values = vec![
        record(include_str!("../../../crates/smith-events/golden/v4/session_started_everything.jsonl")),
        record(include_str!("../../../crates/smith-events/golden/v4/run_started_everything.jsonl")),
        record(include_str!("../../../crates/smith-events/golden/v4/conversation_opened_everything.jsonl")),
        record(include_str!("../../../crates/smith-events/golden/v4/response_started_everything.jsonl")),
        record(include_str!("../../../crates/smith-events/golden/v4/response_completed_everything.jsonl")),
        record(include_str!("../../../crates/smith-events/golden/v4/conversation_closed_everything.jsonl")),
        record(include_str!("../../../crates/smith-events/golden/v4/run_completed_everything.jsonl")),
        record(include_str!("../../../crates/smith-events/golden/v4/session_ended_everything.jsonl")),
    ];
    for (index, record) in values.iter_mut().enumerate() {
        record.t_ms = u64::try_from(index).expect("small index");
        match &mut record.event {
            Event::SessionStarted(value) => {
                value.mode = smith_events::Mode::Exec;
                value.versions.events = 3;
            }
            Event::RunStarted(value) => {
                value.run = 0;
                value.messages = 0;
                value.main = model.clone();
            }
            Event::ConversationOpened(value) => {
                value.run = 0;
                value.conversation = 0;
                value.parent = None;
                value.call = None;
                value.kind = smith_events::ConversationKind::Main;
                value.thread = 0;
                value.model = model.clone();
                value.requested = None;
                value.share = None;
                value.clamped = skein_lib::List::with_capacity(0);
            }
            Event::ResponseStarted(value) => {
                value.run = 0;
                value.conversation = 0;
                value.response = 0;
                value.attempt = 0;
                value.turn = 1;
                value.window = 1;
                value.model = model.clone();
                value.messages = 1;
                value.output_tokens = 4096;
                let mut messages = skein_lib::List::with_capacity(1);
                messages
                    .push(smith_events::Message { role: smith_events::Role::User, blocks: text_blocks(b"task") })
                    .expect("one message");
                value.prompt = Some(smith_events::Prompt { system: None, tools: None, messages });
            }
            Event::ResponseCompleted(value) => {
                completed(value);
            }
            Event::ConversationClosed(value) => {
                value.run = 0;
                value.conversation = 0;
                value.spent = 3;
            }
            Event::RunCompleted(value) => {
                value.run = 0;
                value.spent = 3;
                value.failure = None;
                value.read = None;
                value.unread = 0;
            }
            Event::SessionEnded(value) => {
                value.exit = 0;
                value.teardown_ms = Some(5);
                value.loss.events = 0;
                value.loss.channel = Some(0);
            }
            Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Budget(_)
            | Event::TextDelta(_)
            | Event::ToolStarted(_)
            | Event::JobEnded(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::Notice(_) => panic!("unexpected fixture record"),
        }
    }
    inventory(&mut values);
    values
}

/// Synthetic peers report the inventory of the exchange they actually emit.
pub fn inventory(records: &mut [Record]) {
    type Names = Vec<(std::mem::Discriminant<Event>, Box<[u8]>)>;
    static NAMES: std::sync::LazyLock<std::sync::Mutex<Names>> = std::sync::LazyLock::new(Default::default);
    let mut names = NAMES.lock().expect("single test peer inventory");
    let mut counts = std::collections::BTreeMap::<Box<[u8]>, u64>::new();
    for record in records.iter() {
        // Only the immutable wire spelling is cached. Mutated payloads still
        // pass through Writer and Reader, and every golden is read unchanged.
        let kind = std::mem::discriminant(&record.event);
        let name = if let Some((_, name)) = names.iter().find(|(seen, _)| *seen == kind) {
            name.clone()
        } else {
            let json: serde_json::Value =
                serde_json::from_slice(&bytes(std::slice::from_ref(record))).expect("wire record");
            let name: Box<[u8]> = json["type"].as_str().expect("wire type").as_bytes().into();
            names.push((kind, name.clone()));
            name
        };
        *counts.entry(name.clone()).or_default() += 1;
    }
    for record in records {
        if let Event::SessionEnded(ended) = &mut record.event {
            let mut emitted = skein_lib::List::with_capacity(u32::try_from(counts.len()).expect("small inventory"));
            for (name, count) in &counts {
                emitted.push(smith_events::Count { record: name.clone(), count: *count }).expect("one per type");
            }
            ended.emitted = emitted;
        }
    }
}
