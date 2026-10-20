//! Renewal of a response owned by one program Complete (protocol/events.md 3.5).
//! Retains scalar request metadata only. A retry closes the prior response with
//! unknown transport measures and opens the next attempt under the same owner.

use super::{
    Attempt, Book, Event, Intent, List, Record, ResponseStarted, Started, Time, Token, completion_failure, llm, millis,
    take_started, terminal, unknown_usage,
};

#[derive(Clone, Debug)]
pub(super) struct Opening {
    pub intent: Intent,
    pub output_tokens: u64,
    pub tool_choice: smith_events::ToolChoice,
}

impl Book {
    pub(crate) fn program(&self, request_owner: Token) -> bool {
        let owner = self.owner_for(request_owner).expect("request retains its conversation");
        self.conversations.get(&owner).expect("found above").model.backend == smith_events::Backend::ClaudeCode
    }

    pub(crate) fn timing_observation(&self, request_owner: Token) -> smith_protocol_llm::Observation {
        let owner = self.owner_for(request_owner).expect("request retains its conversation");
        match self.conversations.get(&owner).expect("found above").recorded {
            Some(messages) => smith_protocol_llm::Observation { first: false, messages },
            None => smith_protocol_llm::Observation { first: true, messages: 0 },
        }
    }

    pub(crate) fn prompt_started(&mut self, at: Time, request_owner: Token) {
        let owner = self.owner_for(request_owner).expect("prompt retains its request owner");
        let conversation = self.conversations.get_mut(&owner).expect("found above");
        match &mut conversation.attempt {
            Attempt::Started(started) => started.at = at,
            Attempt::Idle | Attempt::Answered { .. } | Attempt::Failed { .. } => unreachable!("prompt before terminal"),
        }
    }

    pub(crate) fn retried(
        &mut self,
        at: Time,
        origin: Time,
        request_owner: Token,
        status: Option<u16>,
        retry_ms: Option<u64>,
    ) -> List<Record> {
        let owner = self.owner_for(request_owner).expect("retry retains its program owner");
        let conversation = self.conversations.get_mut(&owner).expect("found above");
        let started = take_started(&mut conversation.attempt).expect("retry follows a started response");
        let mut opening = started.opening.clone();
        let observed = started.observed;
        let response_conversation = started.conversation;
        let evidence = match status {
            Some(_) => llm::Evidence::Response { status },
            None => llm::Evidence::Unknown,
        };
        let mut failure = completion_failure(status_failure(status), evidence);
        failure.retry_after_ms = None;
        conversation.attempt = Attempt::Failed { started, at, failure };
        let mut records = List::with_capacity(2);
        records
            .push(terminal(conversation, origin, unknown_usage(), 0, retry_ms).expect("one failed attempt"))
            .expect("one terminal");
        opening.intent.retry = opening.intent.retry.checked_add(1).expect("bounded attempt count");
        let number = self.next;
        self.next = number.checked_add(1).expect("bounded response names");
        conversation.attempts = conversation.attempts.checked_add(1).expect("bounded attempts");
        let prompt = if observed {
            Some(smith_events::Prompt { system: None, tools: None, messages: List::with_capacity(0) })
        } else {
            None
        };
        records
            .push(Record {
                t_ms: millis(at, origin),
                event: Event::ResponseStarted(ResponseStarted {
                    run: conversation.run,
                    conversation: response_conversation,
                    response: number,
                    turn: opening.intent.turn,
                    attempt: u64::from(opening.intent.retry),
                    window: u64::from(opening.intent.window),
                    model: conversation.model.clone(),
                    messages: 0,
                    output_tokens: opening.output_tokens,
                    tool_choice: opening.tool_choice.clone(),
                    prompt,
                }),
            })
            .expect("next attempt start");
        conversation.attempt = Attempt::Started(Started {
            opening,
            number,
            conversation: response_conversation,
            at,
            owner: request_owner,
            observed,
            received: None,
            completion: None,
            measures: None,
            failure: None,
        });
        records
    }
}

fn status_failure(status: Option<u16>) -> llm::Failure {
    match status {
        Some(401) => llm::Failure::Unauthorized,

        Some(429) => llm::Failure::RateLimited { retry_after: skein_lib::Duration::ZERO },
        Some(529) => llm::Failure::Overloaded,
        Some(500..=599) | None => llm::Failure::Unavailable,
        Some(_) => llm::Failure::Invalid,
    }
}
