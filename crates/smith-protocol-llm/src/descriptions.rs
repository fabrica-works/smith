//! Measured schemas and words from service-supplied effective numbers.
//! It stores no conversation state and never chooses an admission limit.
//! Contract: protocol/llm.md, section 3; protocol/limits.md, section 3.

use crate::Stated;
use alloc::boxed::Box;
use skein_lib::{Decimal, Writer};
use smith_domain::tools::{Namespace, Tool};

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

pub(crate) fn schema(tool: Tool, stated: &Stated, namespace: &Namespace) -> Box<[u8]> {
    let mut measure = Text { length: 0, writer: None };
    properties(tool, stated, namespace, &mut measure);
    let mut output = Text { length: 0, writer: Some(Writer::new(measure.length)) };
    properties(tool, stated, namespace, &mut output);
    output.finish()
}

fn words(tool: Tool, stated: &Stated, output: &mut Text) {
    match tool {
        Tool::Read => {
            output.put(b"Read whole lines in byte windows; default ");
            output.number(u64::from(stated.read_window));
            output.put(b" bytes, ceiling ");
            output.number(u64::from(stated.tool_payload));
            output.put(b" bytes. Next line, cuts and clamps are reported. ");
        }
        Tool::List => {
            output.put(b"List matching files and their directory tree; depth defaults to whole, direct children 1. Glob selects files. Max ");
            output.number(u64::from(stated.list_entries));
            output.put(b" entries; omitted files and directories counted once. List / for directory names. ");
        }
        Tool::Search => {
            output.put(b"Search files; max ");
            output.number(u64::from(stated.search_hits));
            output.put(b" matches, ");
            output.number(u64::from(stated.search_bytes));
            output.put(b" bytes; cuts are marked. ");
        }
        Tool::Write => output.put(b"Write a file; read its current version first. "),
        Tool::Edit => output.put(
            b"Apply ordered edits atomically; no earlier read. Each old must match once, or at least once with all. ",
        ),
        Tool::Shell => {
            output.put(b"Run a command; files may change. Max ");
            output.number(u64::from(stated.output_ceiling));
            output.put(b" output bytes; cuts are marked. ");
        }
    }
    match tool {
        Tool::Read | Tool::Write | Tool::Edit => {
            output.put(b"Whole-file limit ");
            output.number(u64::from(stated.file_bytes));
            output.put(b" bytes. ");
        }
        Tool::List | Tool::Search | Tool::Shell => {}
    }
    match tool {
        Tool::Write | Tool::Edit => output.put(b"For a larger file, write its first part within the byte bound, then extend it with edits; put as many edits in one call as fit the bound. "),
        Tool::Read | Tool::List | Tool::Search | Tool::Shell => {}
    }
    match tool {
        Tool::Read | Tool::List | Tool::Search => {
            output.put(b"Batch independent reads, lists and searches side by side.");
        }
        Tool::Write | Tool::Edit | Tool::Shell => {
            output.put(b"Batch writes, edits and commands; run serially in call order.");
        }
    }
}

fn path(stated: &Stated, namespace: &Namespace, output: &mut Text) {
    output.put(br#""path":{"type":"string","description":""#);
    match namespace {
        Namespace::NoWorkspace => output.put(b"No workspace path; max "),
        Namespace::One => output.put(b"Relative to the working directory, e.g. src/lib.rs; max "),
        Namespace::Several { .. } => output.put(b"Relative to the working directory or /name/...; max "),
    }
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

fn properties(tool: Tool, stated: &Stated, namespace: &Namespace, output: &mut Text) {
    output.put(br#"{"type":"object","properties":{"#);
    match tool {
        Tool::Read => {
            path(stated, namespace, output);
            output.put(br#", "offset":{"type":"integer","minimum":1,"maximum":4294967295,"description":"First line, counting from 1; default 1."},"window":{"type":"integer","minimum":1,"description":"Requested byte window; default "#);
            output.number(u64::from(stated.read_window));
            output.put(b" bytes; max ");
            output.number(u64::from(stated.tool_payload));
            output.put(br#" bytes. Larger requests are clamped and reported."}},"required":["path"]}"#);
        }
        Tool::List => {
            path(stated, namespace, output);
            output.put(br#", "depth":{"type":"integer","minimum":1,"maximum":4294967295,"description":"Optional depth; absent lists the whole tree. Direct children are at depth 1."},"glob":{"type":"string","description":"Optional rg file glob; directories holding a matching file remain even when that file is below depth."#);
            input(stated, output);
            output.put(br#""}},"required":["path"]}"#);
        }
        Tool::Search => {
            path(stated, namespace, output);
            output.put(br#", "pattern":{"type":"string","description":"Search pattern."#);
            input(stated, output);
            output.put(br#""},"glob":{"type":"string","description":"Optional file glob."#);
            input(stated, output);
            output.put(br#""}},"required":["pattern"]}"#);
        }
        Tool::Write => {
            path(stated, namespace, output);
            output.put(br#", "content":{"type":"string","description":"Content for this write; max "#);
            payload(stated, output);
            output.put(br#""}},"required":["path","content"]}"#);
        }
        Tool::Edit => {
            path(stated, namespace, output);
            output.put(br#", "edits":{"type":"array","minItems":1,"maxItems":"#);
            output.number(u64::from(stated.tool_payload));
            output.put(br#", "description":"Apply in order and store once; none apply if one fails.","items":{"type":"object","properties":{"old":{"type":"string","description":"Text to match; aggregate old snippets max "#);
            payload(stated, output);
            output.put(br#""},"new":{"type":"string","description":"Replacement text; aggregate new snippets max "#);
            payload(stated, output);
            output.put(br#""},"all":{"type":"boolean","description":"False requires one unique match; true replaces every match."}},"required":["old","new"]}}},"required":["path","edits"]}"#);
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
