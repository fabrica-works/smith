//! Resume admission and recoverable teardown (protocol/programs.md 5.8).
//! Reuses the stopped invocation's storage; never reads the session database.
use super::{Backend, Phase, tail};
use smith_domain::llm;
use smith_protocol_llm::FromDomain;

pub(super) fn restartable(failure: llm::Failure) -> bool {
    match failure {
        llm::Failure::SessionGone
        | llm::Failure::Overloaded
        | llm::Failure::Unavailable
        | llm::Failure::RateLimited { .. }
        | llm::Failure::Unauthorized => true,
        llm::Failure::TimedOut { phase } => match phase {
            llm::Phase::Connect | llm::Phase::Handshake | llm::Phase::Head | llm::Phase::Idle => true,
            llm::Phase::Whole => false,
        },
        llm::Failure::Limit { .. }
        | llm::Failure::ApplicationLimit
        | llm::Failure::Protocol
        | llm::Failure::Cancelled
        | llm::Failure::ContextTooLong
        | llm::Failure::Invalid
        | llm::Failure::SignedOut
        | llm::Failure::Program { .. }
        | llm::Failure::Refused
        | llm::Failure::Unsupported
        | llm::Failure::Exhausted { .. } => false,
    }
}

impl Backend {
    pub(super) fn resume_input(&mut self) -> Option<()> {
        let (prompt, capture) = match self.request.as_mut()? {
            FromDomain::Complete { prompt, observation, .. } => (prompt, observation.is_some()),
            FromDomain::Cancel { .. } => return None,
        };
        let rendered =
            tail::render_observed(prompt, &self.ledger, &self.settings.adapter, self.settings.input_bytes, capture)?;
        self.outgoing = Some(rendered.line);
        if let Some(text) = rendered.text {
            self.prompt_text_measure(skein_lib::Time::ZERO, text);
        }
        Some(())
    }

    pub(super) fn exit_failure(&self) -> llm::Failure {
        if self.phase == Phase::Starting
            && self.resuming
            && !self.stderr_wrapped
            && self.stderr.as_slice().starts_with(b"No conversation found with session ID")
        {
            return llm::Failure::SessionGone;
        }
        llm::Failure::Program { exit: self.exit }
    }
}
