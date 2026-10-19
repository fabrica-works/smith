//! The ordinary request's encoded addition, checked before submission.
//! No history or estimate is retained here; the session supplies the last
//! reported prompt and its line. Contract: protocol/llm.md, section 6.

use alloc::boxed::Box;
use core::mem;

use skein_lib::{Decimal, List, Token, Writer, bytes};
use skein_llm::{self as shared, Credential};
use smith_domain::{Event, llm};

use crate::{EndpointOptions, Error};

/// Measure the body added after `from`, restoring every moved message.
pub(crate) fn measure(
    request: &mut shared::Prompt,
    credential: Option<&Credential>,
    options: &EndpointOptions,
    from: usize,
) -> Result<u64, Error> {
    if from > request.messages.len() {
        return Err(Error::Invalid);
    }
    let whole = match shared::client::measure(request, credential, &options.endpoint, &options.client) {
        Ok(measured) => measured,
        Err(error) => return Err(Error::from_shared(error)),
    };
    let capacity = u32::try_from(request.messages.len()).or(Err(Error::Limit))?;
    let mut prefix = List::with_capacity(capacity);
    let mut suffix = List::with_capacity(capacity);
    for (index, message) in mem::replace(&mut request.messages, Box::new([])).into_iter().enumerate() {
        if index < from {
            prefix.push(message).expect("bounded prefix");
        } else {
            suffix.push(message).expect("bounded suffix");
        }
    }
    request.messages = prefix.into_boxed();
    let cut = shared::client::measure(request, credential, &options.endpoint, &options.client);
    let mut restored = List::with_capacity(capacity);
    for message in mem::replace(&mut request.messages, Box::new([])) {
        restored.push(message).expect("bounded prefix");
    }
    for message in suffix.into_boxed() {
        restored.push(message).expect("bounded suffix");
    }
    request.messages = restored.into_boxed();
    let cut = match cut {
        Ok(measured) => measured,
        Err(error) => return Err(Error::from_shared(error)),
    };
    Ok(u64::from(whole.body.checked_sub(cut.body).ok_or(Error::Invalid)?))
}

/// Return an unsent terminal only when the guard or its measurement refuses.
pub(crate) fn check(
    purpose: llm::Purpose,
    request: &mut shared::Prompt,
    credential: Option<&Credential>,
    options: &EndpointOptions,
    owner: Token,
    max_failure_bytes: u32,
) -> Option<Event> {
    let guard = match purpose {
        llm::Purpose::Ordinary { guard: Some(guard) } => guard,
        llm::Purpose::Ordinary { guard: None } | llm::Purpose::Compaction { .. } => return None,
    };
    let measured = match measure(request, credential, options, guard.from) {
        Ok(measured) => measured,
        Err(error) => return Some(crate::refusal(owner, error, max_failure_bytes)),
    };
    match guard.last_prompt.checked_add(measured) {
        Some(total) if total <= guard.line => return None,
        Some(_) | None => {}
    }
    Some(Event::Failed {
        owner,
        failure: llm::Failure::ContextTooLong,
        evidence: llm::Evidence::Unsent,
        detail: detail(guard, measured, max_failure_bytes),
        report: llm::Report::none(),
    })
}

fn detail(guard: llm::Guard, measured: u64, maximum: u32) -> Box<[u8]> {
    let prompt = Decimal::of(guard.last_prompt);
    let measure = Decimal::of(measured);
    let line = Decimal::of(guard.line);
    let parts: [&[u8]; 6] = [
        b"past the line: last prompt ",
        prompt.as_bytes(),
        b" + new part ",
        measure.as_bytes(),
        b" > line ",
        line.as_bytes(),
    ];
    let mut length = 0_usize;
    for part in parts {
        length = length.checked_add(part.len()).expect("three decimals and fixed text");
    }
    let mut writer = Writer::new(length);
    for part in parts {
        writer.put(part).expect("measured diagnostic");
    }
    let output = writer.finish();
    let length = output.len().min(usize::try_from(maximum).expect("u32 fits"));
    bytes::copy_of(output.get(..length).expect("clipped diagnostic"))
}
