//! Measured schemas and words from service-supplied effective numbers.
//! It stores no conversation state and never chooses an admission limit.
//! Contract: protocol/llm.md, section 3; protocol/limits.md, section 3.

use crate::Stated;
use alloc::boxed::Box;
use skein_lib::{Decimal, Writer};
use smith_domain::tools::Tool;

struct Text {
    length: usize,
    writer: Option<Writer>,
}

impl Text {
    fn put(&mut self, bytes: &[u8]) {
        self.length = self.length.checked_add(bytes.len()).expect("fixed descriptions and u32 decimals fit");
        if let Some(writer) = &mut self.writer {
            writer.put(bytes).expect("measured description room");
        }
    }

    fn number(&mut self, number: u64) {
        self.put(Decimal::of(number).as_bytes());
    }

    fn finish(self) -> Box<[u8]> {
        self.writer.expect("writing pass owns its measured buffer").finish()
    }
}

pub(crate) fn description(tool: Tool, stated: &Stated) -> Box<[u8]> {
    let mut measure = Text { length: 0, writer: None };
    words(tool, stated, &mut measure);
    let mut output = Text { length: 0, writer: Some(Writer::new(measure.length)) };
    words(tool, stated, &mut output);
    output.finish()
}

pub(crate) fn schema(tool: Tool, stated: &Stated) -> Box<[u8]> {
    let mut measure = Text { length: 0, writer: None };
    properties(tool, stated, &mut measure);
    let mut output = Text { length: 0, writer: Some(Writer::new(measure.length)) };
    properties(tool, stated, &mut output);
    output.finish()
}

fn words(tool: Tool, stated: &Stated, output: &mut Text) {
    match tool {
        Tool::Read => output.put(b"Read lines; output is cut and marked. "),
        Tool::List => {
            output.put(b"List a directory; at most ");
            output.number(u64::from(stated.list_entries));
            output.put(b" entries; omitted entries are counted. ");
        }
        Tool::Search => {
            output.put(b"Search files; at most ");
            output.number(u64::from(stated.search_hits));
            output.put(b" matches and ");
            output.number(u64::from(stated.search_bytes));
            output.put(b" bytes; cuts are marked. ");
        }
        Tool::Write => output.put(b"Write a file; read its current version first. "),
        Tool::Edit => output.put(b"Edit a file already read; old must match once, or at least once with all. "),
        Tool::Shell => {
            output.put(b"Run a command; files may change. Keep at most ");
            output.number(u64::from(stated.output_ceiling));
            output.put(b" output bytes; cuts are marked. ");
        }
    }
    match tool {
        Tool::Read | Tool::List | Tool::Search => {
            output.put(b"Batch independent reads, listings and searches; they run side by side.");
        }
        Tool::Write | Tool::Edit | Tool::Shell => {
            output.put(b"Batch writes, edits and commands; one at a time, in call order.");
        }
    }
}

fn path(stated: &Stated, output: &mut Text) {
    output.put(br#""path":{"type":"string","description":"Workspace path, relative to the working directory or /name/...; max "#);
    output.number(u64::from(stated.path_bytes));
    output.put(br#" bytes."}"#);
}

fn input(stated: &Stated, output: &mut Text) {
    output.put(b" Entire JSON call input max ");
    output.number(u64::from(stated.call_input_bytes));
    output.put(b" bytes.");
}

fn payload(stated: &Stated, output: &mut Text) {
    output.number(u64::from(stated.tool_payload));
    output.put(b" bytes.");
    input(stated, output);
}

fn properties(tool: Tool, stated: &Stated, output: &mut Text) {
    output.put(br#"{"type":"object","properties":{"#);
    match tool {
        Tool::Read => {
            path(stated, output);
            output.put(br#", "first_line":{"type":"integer","minimum":1,"description":"First line, counting from 1; max 4294967295."},"lines":{"type":"integer","minimum":1,"description":"Requested line count; max 4294967295. Kept window max "#);
            output.number(u64::from(stated.read_window));
            output.put(br#" bytes."}},"required":["path"]}"#);
        }
        Tool::List => {
            path(stated, output);
            output.put(br#"},"required":["path"]}"#);
        }
        Tool::Search => {
            path(stated, output);
            output.put(br#", "pattern":{"type":"string","description":"Search pattern."#);
            input(stated, output);
            output.put(br#""},"glob":{"type":"string","description":"Optional file glob."#);
            input(stated, output);
            output.put(br#""}},"required":["pattern"]}"#);
        }
        Tool::Write => {
            path(stated, output);
            output.put(br#", "content":{"type":"string","description":"Whole file content; max "#);
            payload(stated, output);
            output.put(br#""}},"required":["path","content"]}"#);
        }
        Tool::Edit => {
            path(stated, output);
            output
                .put(br#", "old":{"type":"string","description":"Text to match once, or at least once with all; max "#);
            payload(stated, output);
            output.put(br#""},"new":{"type":"string","description":"Replacement text; max "#);
            payload(stated, output);
            output.put(br#""},"all":{"type":"boolean","description":"False replaces one unique match; true replaces every match."}},"required":["path","old","new"]}"#);
        }
        Tool::Shell => {
            output.put(br#""command":{"type":"string","description":"Shell command text."#);
            input(stated, output);
            output.put(br#""},"timeout":{"type":"integer","minimum":1,"description":"Seconds; default deadline "#);
            output.number(stated.shell_timeout.as_nanos() / 1_000_000);
            output.put(b" ms; effective maximum ");
            output.number(stated.tool_deadline.as_nanos() / 1_000_000);
            output.put(br#" ms, within the remaining session time."}},"required":["command"]}"#);
        }
    }
}
