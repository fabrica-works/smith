//! Completed line syntax (shell.md, section 4.1). The terminal owns no model
//! catalog or chat store; the local host applies slash choices at a boundary.
use alloc::boxed::Box;

/// One completed line, sent by the terminal to the local host.
#[derive(Debug, PartialEq, Eq)]
pub enum Entered {
    /// Person text, with one slash removed from an escaped `//` prefix.
    Message(Box<[u8]>),
    /// A local choice that must never enter a run's inbox.
    Slash(Slash),
    /// The line names no supported command or omits its one value.
    Refused,
}

pub use smith_local_domain::Slash;

/// Classify an already bounded completed line without looking up its choices.
#[must_use]
pub fn entered(line: Box<[u8]>) -> Entered {
    if line.starts_with(b"//") {
        return Entered::Message(line.get(1..).expect("escaped slash has a first byte").into());
    }
    if !line.starts_with(b"/") {
        return Entered::Message(line);
    }
    let command = line.trim_ascii();
    match command {
        b"/new" => Entered::Slash(Slash::New),
        b"/quit" => Entered::Slash(Slash::Quit),
        _ => {
            if let Some(model) = value(command, b"/model ") {
                return Entered::Slash(Slash::Model { model, selected: None });
            }
            if let Some(level) = value(command, b"/effort ") {
                return Entered::Slash(Slash::Effort { level });
            }
            if let Some(name) = value(command, b"/resume ") {
                return Entered::Slash(Slash::Resume { name });
            }
            Entered::Refused
        }
    }
}

fn value(line: &[u8], prefix: &[u8]) -> Option<Box<[u8]>> {
    let bytes = match line.strip_prefix(prefix) {
        Some(bytes) => bytes.trim_ascii(),
        None => return None,
    };
    if bytes.is_empty() {
        return None;
    }
    for byte in bytes {
        if byte.is_ascii_whitespace() {
            return None;
        }
    }
    Some(bytes.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_slash_line_is_a_local_command() {
        for (line, expected) in [
            (
                b"/model codex/small".as_slice(),
                Slash::Model { model: b"codex/small".as_slice().into(), selected: None },
            ),
            (b"/effort high".as_slice(), Slash::Effort { level: b"high".as_slice().into() }),
            (b"/new".as_slice(), Slash::New),
            (b"/resume work".as_slice(), Slash::Resume { name: b"work".as_slice().into() }),
            (b"/quit".as_slice(), Slash::Quit),
        ] {
            assert_eq!(entered(line.into()), Entered::Slash(expected));
        }
    }

    #[test]
    fn an_escaped_slash_is_person_text() {
        assert_eq!(entered(b"//new".as_slice().into()), Entered::Message(b"/new".as_slice().into()));
        assert_eq!(entered(b"hello".as_slice().into()), Entered::Message(b"hello".as_slice().into()));
    }

    #[test]
    fn missing_or_extra_command_values_never_become_messages() {
        for line in [b"/model".as_slice(), b"/effort ", b"/resume", b"/quit extra", b"/new extra", b"/unknown"] {
            assert_eq!(entered(line.into()), Entered::Refused);
        }
    }
}
