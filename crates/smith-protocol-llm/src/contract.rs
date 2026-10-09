//! Contract-derived finish and delivery schemas and typed result decoding.
//! The service supplies the charter's admitted rules; this module keeps no
//! result policy or host state. The run judges every decoded value itself.
//! Contract: protocol/llm.md, sections 3 and 4; protocol/charter.md, section 2.1;
//! domain/run.md, section 7.

use alloc::boxed::Box;

use skein_json::writer::{self, Encoder};
use skein_json::{Document, Kind as JsonKind};
use skein_lib::List;
use skein_llm::{DocumentError, Json};
use smith_domain::{llm, run};

use crate::{Error, Limits};

/// A JSON Schema for each final form the admitted charter allows.
/// Host labels, item kinds and required fields are byte-exact.
pub fn finish_schema(contract: &run::outcome::OutcomeSpec, maximum: u32) -> Result<Box<[u8]>, Error> {
    let limits = writer::Limits { depth: 16, length: maximum };
    let mut measure = Encoder::measure(&limits);
    encode_finish(&mut measure, contract);
    let length = match measure.measured() {
        Ok(length) => length,
        Err(refusal) => return Err(schema_error(refusal)),
    };
    let mut write = Encoder::write(length, &limits);
    encode_finish(&mut write, contract);
    Ok(write.finish())
}

/// The mid-run deliver tool's schema, from its separate Change grant.
pub fn deliver_schema(change: &run::outcome::ChangeSpec, maximum: u32) -> Result<Box<[u8]>, Error> {
    let limits = writer::Limits { depth: 16, length: maximum };
    let mut measure = Encoder::measure(&limits);
    encode_deliver(&mut measure, change);
    let length = match measure.measured() {
        Ok(length) => length,
        Err(refusal) => return Err(schema_error(refusal)),
    };
    let mut write = Encoder::write(length, &limits);
    encode_deliver(&mut write, change);
    Ok(write.finish())
}

fn schema_error(refusal: writer::Refusal) -> Error {
    match refusal {
        writer::Refusal::Text | writer::Refusal::Number => Error::Invalid,
        writer::Refusal::TooLong | writer::Refusal::TooDeep => Error::Limit,
    }
}

fn encode_finish(json: &mut Encoder, contract: &run::outcome::OutcomeSpec) {
    json.object_start();
    json.key(b"oneOf");
    json.array_start();
    if let Some(change) = &contract.change {
        change_branch(json, b"change", change);
    }
    if !contract.verdicts.is_empty() {
        json.object_start();
        json.key(b"oneOf");
        json.array_start();
        for rule in &contract.verdicts {
            verdict_branch(json, rule);
        }
        json.array_end();
        json.object_end();
    }
    if let Some(report) = &contract.report {
        text_branch(json, b"report", b"text", report);
    }
    if let Some(failure) = &contract.failure {
        text_branch(json, b"failure", b"reason", failure);
    }
    json.array_end();
    json.object_end();
}

fn encode_deliver(json: &mut Encoder, change: &run::outcome::ChangeSpec) {
    object_type(json);
    json.key(b"properties");
    json.object_start();
    json.key(b"fields");
    fields_schema(json, &change.fields);
    json.object_end();
    required(json, &[b"fields"]);
    json.object_end();
}

fn change_branch(json: &mut Encoder, form: &[u8], change: &run::outcome::ChangeSpec) {
    object_type(json);
    json.key(b"properties");
    json.object_start();
    const_string(json, b"form", form);
    json.key(b"fields");
    fields_schema(json, &change.fields);
    json.object_end();
    required(json, &[b"form", b"fields"]);
    json.object_end();
}

fn text_branch(json: &mut Encoder, form: &[u8], text_name: &[u8], text: &run::outcome::TextSpec) {
    object_type(json);
    json.key(b"properties");
    json.object_start();
    const_string(json, b"form", form);
    bounded_string(json, text_name, text.max, false);
    json.key(b"fields");
    fields_schema(json, &text.fields);
    json.object_end();
    required(json, &[b"form", text_name, b"fields"]);
    json.object_end();
}

fn verdict_branch(json: &mut Encoder, verdict: &run::outcome::VerdictRule) {
    object_type(json);
    json.key(b"properties");
    json.object_start();
    const_string(json, b"form", b"verdict");
    const_string(json, b"label", &verdict.name);
    bounded_string(json, b"text", verdict.text_max, false);
    json.key(b"fields");
    fields_schema(json, &verdict.fields);
    json.key(b"items");
    json.object_start();
    json.key(b"type");
    json.string(b"array");
    json.key(b"minItems");
    json.unsigned(u64::from(verdict.items.min));
    json.key(b"maxItems");
    json.unsigned(u64::from(verdict.items.max));
    if !verdict.items.kinds.is_empty() {
        json.key(b"items");
        json.object_start();
        json.key(b"oneOf");
        json.array_start();
        for item in &verdict.items.kinds {
            item_branch(json, item);
        }
        json.array_end();
        json.object_end();
    }
    json.object_end();
    json.object_end();
    required(json, &[b"form", b"label", b"text", b"fields", b"items"]);
    json.object_end();
}

fn item_branch(json: &mut Encoder, item: &run::outcome::ItemRule) {
    object_type(json);
    json.key(b"properties");
    json.object_start();
    const_string(json, b"kind", &item.kind);
    json.key(b"fields");
    fields_schema(json, &item.fields);
    json.object_end();
    required(json, &[b"kind", b"fields"]);
    json.object_end();
}

fn fields_schema(json: &mut Encoder, fields: &[run::outcome::FieldRule]) {
    object_type(json);
    json.key(b"properties");
    json.object_start();
    for field in fields {
        bounded_string(json, &field.name, field.max, true);
    }
    json.object_end();
    json.key(b"required");
    json.array_start();
    for field in fields {
        json.string(&field.name);
    }
    json.array_end();
    json.object_end();
}

fn object_type(json: &mut Encoder) {
    json.object_start();
    json.key(b"type");
    json.string(b"object");
}

fn bounded_string(json: &mut Encoder, name: &[u8], maximum: u32, nonempty: bool) {
    json.key(name);
    json.object_start();
    json.key(b"type");
    json.string(b"string");
    if nonempty {
        json.key(b"minLength");
        json.unsigned(1);
    }
    json.key(b"maxLength");
    json.unsigned(u64::from(maximum));
    json.object_end();
}

fn const_string(json: &mut Encoder, name: &[u8], value: &[u8]) {
    json.key(name);
    json.object_start();
    json.key(b"const");
    json.string(value);
    json.object_end();
}

fn required(json: &mut Encoder, names: &[&[u8]]) {
    json.key(b"required");
    json.array_start();
    for name in names {
        json.string(name);
    }
    json.array_end();
}

/// Decode a provider's declared result under a bounded admitted contract.
/// Shape and aggregate policy are validated by the run after this translation.
#[must_use]
pub fn decode_finish(input: &[u8], _contract: &run::outcome::OutcomeSpec, limits: &Limits) -> llm::Decoded {
    let document = match object(input, limits) {
        Ok(document) => document,
        Err(problem) => return llm::Decoded::Invalid { problem },
    };
    let document = document.document();
    let value = Value { start: 0, end: document.len() };
    let outcome = match declared(document, value) {
        Ok(outcome) => outcome,
        Err(problem) => return llm::Decoded::Invalid { problem },
    };
    llm::Decoded::Served { ask: run::Ask::Finish { outcome } }
}

/// Decode the separately granted delivery's host fields.
#[must_use]
pub fn decode_deliver(input: &[u8], _change: &run::outcome::ChangeSpec, limits: &Limits) -> llm::Decoded {
    let document = match object(input, limits) {
        Ok(document) => document,
        Err(problem) => return llm::Decoded::Invalid { problem },
    };
    let fields = match required_fields(document.document(), Value { start: 0, end: document.document().len() }) {
        Ok(fields) => fields,
        Err(problem) => return llm::Decoded::Invalid { problem },
    };
    llm::Decoded::Served { ask: run::Ask::Deliver { change: run::outcome::Change { fields } } }
}

fn object(input: &[u8], limits: &Limits) -> Result<Json, llm::Problem> {
    let document = match Json::from_bytes(
        input,
        &crate::limits::document_limits(
            &limits.client,
            u32::try_from(input.len()).unwrap_or(u32::MAX).min(limits.client.input),
        ),
    ) {
        Ok(document) => document,
        Err(DocumentError::TooLarge { which: _, bound: _ }) => return Err(llm::Problem::TooLarge),
        Err(DocumentError::Malformed | DocumentError::Missing | DocumentError::WrongType) => {
            return Err(llm::Problem::NotAnObject);
        }
    };
    match document.document().token(0) {
        Some(record) if record.kind == JsonKind::ObjectStart => Ok(document),
        Some(_) | None => Err(llm::Problem::NotAnObject),
    }
}

fn declared(document: &Document, value: Value) -> Result<run::outcome::Declared, llm::Problem> {
    let form = required_string(document, value, b"form")?;
    let outcome = match form.as_ref() {
        b"change" => run::outcome::Declared::Change(run::outcome::Change { fields: required_fields(document, value)? }),
        b"report" => run::outcome::Declared::Report(run::outcome::Report {
            text: required_string(document, value, b"text")?,
            fields: required_fields(document, value)?,
        }),
        b"failure" => run::outcome::Declared::Failure(run::outcome::DeclaredFailure {
            reason: required_string(document, value, b"reason")?,
            fields: required_fields(document, value)?,
        }),
        b"verdict" => run::outcome::Declared::Verdict(run::outcome::Verdict {
            name: required_string(document, value, b"label")?,
            text: required_string(document, value, b"text")?,
            fields: required_fields(document, value)?,
            items: items(document, value)?,
        }),
        _ => return Err(llm::Problem::BadValue { field: b"form".as_slice().into() }),
    };
    Ok(outcome)
}

/// An application's indexed object or value range in one admitted document.
/// It borrows no text and is kept only during the current decoding call.
#[derive(Clone, Copy)]
struct Value {
    start: u32,
    end: u32,
}

fn required_fields(document: &Document, object: Value) -> Result<Box<[run::outcome::Field]>, llm::Problem> {
    let value = required_value(document, object, b"fields")?;
    if kind(document, value.start) != Some(JsonKind::ObjectStart) {
        return Err(wrong_type(b"fields"));
    }
    let mut fields = List::with_capacity(value.end.checked_sub(value.start).ok_or(llm::Problem::NotAnObject)?);
    let mut cursor = value.start.checked_add(1).ok_or(llm::Problem::TooLarge)?;
    for _ in value.start..value.end {
        match kind(document, cursor) {
            Some(JsonKind::Key) => {
                let name = text(document, cursor)?;
                let next = cursor.checked_add(1).ok_or(llm::Problem::TooLarge)?;
                if kind(document, next) != Some(JsonKind::String) {
                    return Err(wrong_type(name));
                }
                fields
                    .push(run::outcome::Field { name: name.into(), value: text(document, next)?.into() })
                    .or(Err(llm::Problem::TooLarge))?;
                cursor = next.checked_add(1).ok_or(llm::Problem::TooLarge)?;
            }
            Some(JsonKind::ObjectEnd) => return Ok(fields.into_boxed()),
            Some(_) | None => return Err(llm::Problem::NotAnObject),
        }
    }
    Err(llm::Problem::NotAnObject)
}

fn items(document: &Document, object: Value) -> Result<Box<[run::outcome::Item]>, llm::Problem> {
    let value = required_value(document, object, b"items")?;
    if kind(document, value.start) != Some(JsonKind::ArrayStart) {
        return Err(wrong_type(b"items"));
    }
    let mut items = List::with_capacity(value.end.checked_sub(value.start).ok_or(llm::Problem::NotAnObject)?);
    let mut cursor = value.start.checked_add(1).ok_or(llm::Problem::TooLarge)?;
    for _ in value.start..value.end {
        match kind(document, cursor) {
            Some(JsonKind::ArrayEnd) => return Ok(items.into_boxed()),
            Some(JsonKind::ObjectStart) => {
                let end = value_end(document, cursor, value.end).ok_or(llm::Problem::NotAnObject)?;
                let one = Value { start: cursor, end };
                items
                    .push(run::outcome::Item {
                        kind: required_string(document, one, b"kind")?,
                        fields: required_fields(document, one)?,
                    })
                    .or(Err(llm::Problem::TooLarge))?;
                cursor = end;
            }
            Some(_) => return Err(wrong_type(b"items")),
            None => return Err(llm::Problem::NotAnObject),
        }
    }
    Err(llm::Problem::NotAnObject)
}

fn required_string(document: &Document, object: Value, name: &[u8]) -> Result<Box<[u8]>, llm::Problem> {
    let value = required_value(document, object, name)?;
    if kind(document, value.start) != Some(JsonKind::String) || value.end.checked_sub(value.start) != Some(1) {
        return Err(wrong_type(name));
    }
    Ok(text(document, value.start)?.into())
}

fn required_value(document: &Document, object: Value, name: &[u8]) -> Result<Value, llm::Problem> {
    let mut cursor = object.start.checked_add(1).ok_or(llm::Problem::TooLarge)?;
    for _ in object.start..object.end {
        match kind(document, cursor) {
            Some(JsonKind::Key) => {
                let start = cursor.checked_add(1).ok_or(llm::Problem::TooLarge)?;
                let end = value_end(document, start, object.end).ok_or(llm::Problem::NotAnObject)?;
                if text(document, cursor)? == name {
                    return Ok(Value { start, end });
                }
                cursor = end;
            }
            Some(JsonKind::ObjectEnd) => return Err(llm::Problem::Missing { field: name.into() }),
            Some(_) | None => return Err(llm::Problem::NotAnObject),
        }
    }
    Err(llm::Problem::NotAnObject)
}

fn value_end(document: &Document, start: u32, bound: u32) -> Option<u32> {
    match kind(document, start)? {
        JsonKind::String | JsonKind::Number | JsonKind::True | JsonKind::False | JsonKind::Null => start.checked_add(1),
        JsonKind::ObjectStart | JsonKind::ArrayStart => {
            let mut depth = 0_u32;
            for index in start..bound {
                match kind(document, index)? {
                    JsonKind::ObjectStart | JsonKind::ArrayStart => depth = depth.checked_add(1)?,
                    JsonKind::ObjectEnd | JsonKind::ArrayEnd => {
                        depth = depth.checked_sub(1)?;
                        if depth == 0 {
                            return index.checked_add(1);
                        }
                    }
                    JsonKind::Key
                    | JsonKind::String
                    | JsonKind::Number
                    | JsonKind::True
                    | JsonKind::False
                    | JsonKind::Null
                    | JsonKind::Long => {}
                }
            }
            None
        }
        JsonKind::ObjectEnd | JsonKind::ArrayEnd | JsonKind::Key | JsonKind::Long => None,
    }
}

fn kind(document: &Document, index: u32) -> Option<JsonKind> {
    Some(document.token(index)?.kind)
}

fn text(document: &Document, index: u32) -> Result<&[u8], llm::Problem> {
    document.text(document.token(index).ok_or(llm::Problem::NotAnObject)?).ok_or(llm::Problem::NotAnObject)
}

fn wrong_type(name: &[u8]) -> llm::Problem {
    llm::Problem::WrongType { field: name.into() }
}
