//! Headless stories reuse the chat, workspace and git fixture constructor.
use super::{Scenario, Store, World, agent_world, local, run};

impl World {
    /// One prompt carried in Start; a wait parks without an interactive timeout.
    #[must_use]
    pub fn headless(seed: u64, prompt: &[u8]) -> Self {
        Self::with_mode(
            seed,
            Store::default(),
            true,
            (2, 64),
            Scenario::Chat,
            local::Mode::Headless { prompt: prompt.into() },
            None,
        )
    }

    /// A headless run using the shared workspace report script.
    #[must_use]
    pub fn headless_workspace(seed: u64, prompt: &[u8]) -> Self {
        Self::with_mode(
            seed,
            Store::default(),
            true,
            (2, 64),
            Scenario::Workspace,
            local::Mode::Headless { prompt: prompt.into() },
            None,
        )
    }

    /// A headless change delivered through the existing git fixture.
    #[must_use]
    pub fn headless_delivery(seed: u64, prompt: &[u8]) -> Self {
        Self::with_mode(
            seed,
            Store::default(),
            true,
            (2, 64),
            Scenario::GitChange,
            local::Mode::Headless { prompt: prompt.into() },
            None,
        )
    }

    /// Three turns cannot pay for the shared workspace script's complete work.
    #[must_use]
    pub fn headless_budget(seed: u64, prompt: &[u8]) -> Self {
        Self::with_mode(
            seed,
            Store::default(),
            true,
            (2, 64),
            Scenario::Workspace,
            local::Mode::Headless { prompt: prompt.into() },
            Some(run::Budget { turns: 3, ..agent_world::BUDGET }),
        )
    }

    /// Refuse every provider attempt without changing the local or agent policy.
    pub fn fail_model(&mut self) {
        self.provider_env.limits.unavailable = 1000;
        self.provider = agent_world::scripted_provider(&self.provider_env.limits, self.seed ^ 0x25);
    }

    /// Let the named account's next grant request end without a credential.
    pub fn without_credential(&mut self) {
        self.missing_credential = Some(0);
    }
}
