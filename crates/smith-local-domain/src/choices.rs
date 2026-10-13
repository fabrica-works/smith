//! One pending local command, applied between activations (shell.md, section 4.1).
//! It owns only the bounded entered value; the store and run keep their existing owners.
use crate::chat::Phase;
use crate::{Domain, Limits, Request};
use alloc::boxed::Box;
use skein_lib::{Env, Queue};

/// A person's local command, applied from the next activation.
#[derive(Debug, PartialEq, Eq)]
pub enum Slash {
    /// Select a main model by its endpoint and model name.
    Model { model: Box<[u8]> },
    /// Select the main model's reasoning effort.
    Effort { level: Box<[u8]> },
    /// Start a new named chat instead of resuming the current one.
    New,
    /// Reopen a chat by its single directory name.
    Resume { name: Box<[u8]> },
    /// End input while allowing received lines to drain.
    Quit,
}

pub(crate) fn receive(domain: &mut Domain, choice: Slash, env: &Env<Limits>, out: &mut Queue<Request>) {
    match choice {
        Slash::Model { .. } => out.push(Request::Show { text: b"/model is not yet available".as_slice().into() }),
        Slash::Effort { .. } => out.push(Request::Show { text: b"/effort is not yet available".as_slice().into() }),
        Slash::Quit => crate::domain::closed(domain, out),
        choice @ (Slash::New | Slash::Resume { .. }) => {
            if domain.choice.is_some() {
                out.push(Request::Show { text: b"One command is waiting for the activation to end".as_slice().into() });
                return;
            }
            domain.choice = Some(choice);
            apply(domain, env, out);
        }
    }
}

pub(crate) fn apply(domain: &mut Domain, env: &Env<Limits>, out: &mut Queue<Request>) {
    if domain.chat.phase != Phase::Idle || domain.run.is_some() || domain.chat.state_pending {
        return;
    }
    let Some(choice) = domain.choice.take() else { return };
    match choice {
        Slash::New => {
            let name = domain.new_chat_name();
            crate::domain::select_chat(domain, env, name, true, out);
        }
        Slash::Resume { name } => crate::domain::select_chat(domain, env, name, false, out),
        Slash::Quit => crate::domain::closed(domain, out),
        Slash::Model { .. } | Slash::Effort { .. } => unreachable!("unavailable choices are refused on receipt"),
    }
}
