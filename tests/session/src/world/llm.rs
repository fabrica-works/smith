//! Render, guard and submit the world's LLM effects (domain/session.md, 10).

use super::{Call, Delivery, Failure, Prompt, Token, World, agent, in_call_order, not_run, translate, window_referee};

impl World {
    pub(super) fn complete(&mut self, owner: Token, prompt: Prompt, deadlines: agent::Deadlines) {
        let opener = self.openers[&owner];
        window_referee::prompt(
            self.sessions.get_mut(&opener).expect("known session"),
            &prompt,
            &self.agent_stage.env.limits,
        );
        self.affordable(owner, &prompt);
        in_call_order(&prompt);
        self.check_results(owner, &prompt);
        self.stats.not_run += not_run(&prompt);
        self.stats.calls += 1;
        let purpose = prompt.purpose;
        let served = prompt.delegated.clone();
        let query = translate::query(prompt, &self.tickets);
        let measured = self.sessions.get_mut(&opener).expect("known session").counting.measure(&query, purpose);
        window_referee::measured(self.sessions.get(&opener).expect("known session"), &measured);
        let input = measured.input;
        let refused = !measured.sent;
        let detail = measured.guard.map(|guard| {
            format!(
                "past the line: last prompt {} + new part {} > line {}",
                guard.last_prompt, measured.new_part, guard.line,
            )
            .into_bytes()
            .into_boxed_slice()
        });
        self.sessions.get_mut(&opener).expect("known session").measures.push(measured);
        if refused {
            self.agent_stage.push(agent::Event::Failed {
                retry_ok: true,
                owner,
                failure: Failure::ContextTooLong,
                evidence: agent::llm::Evidence::Unsent,
                detail: detail.expect("only a guard refuses"),
            });
            return;
        }
        let call = self.wire.name();
        let head = self.now.saturating_add(deadlines.head);
        let phase = if deadlines.whole <= head {
            smith_domain_session::llm::Phase::Whole
        } else {
            smith_domain_session::llm::Phase::Head
        };
        let deadline = self.schedule(deadlines.whole.min(head), Delivery::Deadline { call, phase });
        let opener = *self.openers.get(&owner).expect("a session calls the LLM while it lives");
        self.calls.open(call, Call { owner, deadline, opener, served, input });
        assert!(self.calling.insert(owner, call).is_none(), "a session has one call in flight");
        self.send(Delivery::Query { call, query });
    }
}
