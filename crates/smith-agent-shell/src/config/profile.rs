//! Profile overrides checked before startup (protocol/agent.md, section 4).

use super::{ConfigurationRefusal, DeclaredOverrides, PolicyOverrides, ProfileDocument, duration};
use smith_agent_service::{
    self as service,
    profile::{Layer, LimitName},
};

pub(super) fn build_profile(document: &ProfileDocument) -> Result<service::profile::Profile, ConfigurationRefusal> {
    if document.name != "standard" {
        return Err("unknown profile name".into());
    }
    let mut profile = service::profile::standard();
    apply_declared(&mut profile, &document.declared)?;
    apply_counts(&mut profile, &document.policy)?;
    apply_durations(&mut profile, &document.policy)?;
    Ok(profile)
}

fn apply_declared(
    profile: &mut service::profile::Profile,
    overrides: &DeclaredOverrides,
) -> Result<(), ConfigurationRefusal> {
    if let Some(value) = overrides.conversations {
        if value == 0 {
            return Err("profile.declared.conversations must be positive".into());
        }
        profile.declared.conversations = value;
    }
    if let Some(value) = overrides.tool_payload {
        if value == 0 {
            return Err("profile.declared.tool_payload must be positive".into());
        }
        profile.declared.tool_payload = value;
    }
    if let Some(value) = overrides.file_bytes {
        if value == 0 {
            return Err("profile.declared.file_bytes must be positive".into());
        }
        profile.declared.file_bytes = value;
    }
    if let Some(value) = overrides.calls_per_response {
        if value == 0 {
            return Err("profile.declared.calls_per_response must be positive".into());
        }
        profile.declared.calls_per_response = value;
    }
    if let Some(value) = overrides.jobs {
        profile.declared.jobs = value;
    }
    if let Some(value) = overrides.read_window {
        if value == 0 {
            return Err("profile.declared.read_window must be positive".into());
        }
        profile.declared.read_window = value;
    }
    if let Some(value) = overrides.shell_output {
        if value == 0 {
            return Err("profile.declared.shell_output must be positive".into());
        }
        profile.declared.shell_output = value;
    }
    if let Some(value) = overrides.shell_head {
        if value == 0 {
            return Err("profile.declared.shell_head must be positive".into());
        }
        profile.declared.shell_head = value;
    }
    if let Some(value) = overrides.shell_tail {
        if value == 0 {
            return Err("profile.declared.shell_tail must be positive".into());
        }
        profile.declared.shell_tail = value;
    }
    if let Some(value) = overrides.search_hits {
        if value == 0 {
            return Err("profile.declared.search_hits must be positive".into());
        }
        profile.declared.search_hits = value;
    }
    if let Some(value) = overrides.search_bytes {
        if value == 0 {
            return Err("profile.declared.search_bytes must be positive".into());
        }
        profile.declared.search_bytes = value;
    }
    if let Some(value) = overrides.list_entries {
        if value == 0 {
            return Err("profile.declared.list_entries must be positive".into());
        }
        profile.declared.list_entries = value;
    }
    if let Some(value) = overrides.guide {
        if value == 0 {
            return Err("profile.declared.guide must be positive".into());
        }
        profile.declared.guide = value;
    }
    if let Some(value) = overrides.llm_pool {
        if value == 0 {
            return Err("profile.declared.llm_pool must be positive".into());
        }
        profile.declared.llm_pool = value;
    }
    if let Some(value) = overrides.memory {
        if value == 0 {
            return Err("profile.declared.memory must be positive".into());
        }
        profile.declared.memory = value;
    }
    Ok(())
}

fn apply_counts(
    profile: &mut service::profile::Profile,
    overrides: &PolicyOverrides,
) -> Result<(), ConfigurationRefusal> {
    if let Some(value) = overrides.max_turns {
        if value == 0 {
            return Err("profile.policy.max_turns must be positive".into());
        }
        profile.policy.max_turns = value;
    }
    if let Some(value) = overrides.max_spend {
        if value == 0 {
            return Err("profile.policy.max_spend must be positive".into());
        }
        profile.policy.max_spend = value;
    }
    if let Some(value) = overrides.max_time {
        profile.policy.max_time = duration(value, LimitName::deployment(Layer::Policy, "max_time"))?;
    }
    if let Some(value) = overrides.max_waiting {
        profile.policy.max_waiting = duration(value, LimitName::deployment(Layer::Policy, "max_waiting"))?;
    }
    if let Some(value) = overrides.sections {
        if value == 0 {
            return Err("profile.policy.sections must be positive".into());
        }
        profile.policy.sections = value;
    }
    if let Some(value) = overrides.host_tools {
        if value == 0 {
            return Err("profile.policy.host_tools must be positive".into());
        }
        profile.policy.host_tools = value;
    }
    if let Some(value) = overrides.verdicts {
        if value == 0 {
            return Err("profile.policy.verdicts must be positive".into());
        }
        profile.policy.verdicts = value;
    }
    if let Some(value) = overrides.items {
        if value == 0 {
            return Err("profile.policy.items must be positive".into());
        }
        profile.policy.items = value;
    }
    if let Some(value) = overrides.fields {
        if value == 0 {
            return Err("profile.policy.fields must be positive".into());
        }
        profile.policy.fields = value;
    }
    if let Some(value) = overrides.inbox {
        if value == 0 {
            return Err("profile.policy.inbox must be positive".into());
        }
        profile.policy.inbox = value;
    }
    if let Some(value) = overrides.unacknowledged {
        if value == 0 {
            return Err("profile.policy.unacknowledged must be positive".into());
        }
        profile.policy.unacknowledged = value;
    }
    if let Some(value) = overrides.parallel_tools {
        profile.policy.parallel_tools = value;
    }
    if let Some(value) = &overrides.compaction_threshold {
        if value.numerator == 0 || value.numerator >= value.denominator {
            return Err("profile.policy.compaction_threshold must be between zero and one".into());
        }
        profile.policy.compaction_threshold =
            service::profile::Fraction { numerator: value.numerator, denominator: value.denominator };
    }
    Ok(())
}

fn apply_durations(
    profile: &mut service::profile::Profile,
    overrides: &PolicyOverrides,
) -> Result<(), ConfigurationRefusal> {
    if let Some(value) = overrides.check_timeout {
        profile.policy.check_timeout = duration(value, LimitName::deployment(Layer::Policy, "check_timeout"))?;
    }
    if let Some(value) = overrides.wrap_up {
        profile.policy.wrap_up = duration(value, LimitName::deployment(Layer::Policy, "wrap_up"))?;
    }
    if let Some(value) = overrides.connection_keep {
        profile.policy.connection_keep = duration(value, LimitName::deployment(Layer::Policy, "connection_keep"))?;
    }
    if let Some(value) = overrides.tool_deadline {
        profile.policy.tool_deadline = duration(value, LimitName::deployment(Layer::Policy, "tool_deadline"))?;
    }
    if let Some(value) = overrides.shell_timeout {
        profile.policy.shell_timeout = duration(value, LimitName::deployment(Layer::Policy, "shell_timeout"))?;
    }
    if let Some(value) = overrides.job_poll {
        profile.policy.job_poll = duration(value, LimitName::deployment(Layer::Policy, "job_poll"))?;
    }
    if let Some(value) = overrides.job_wait {
        profile.policy.job_wait = duration(value, LimitName::deployment(Layer::Policy, "job_wait"))?;
    }
    if let Some(value) = overrides.group_stop {
        profile.policy.group_stop = duration(value, LimitName::deployment(Layer::Policy, "group_stop"))?;
    }
    if let Some(value) = overrides.close {
        profile.policy.close = duration(value, LimitName::deployment(Layer::Policy, "close"))?;
    }
    if let Some(value) = overrides.write_deadline {
        profile.policy.write_deadline = duration(value, LimitName::deployment(Layer::Policy, "write_deadline"))?;
    }
    if let Some(value) = overrides.cancel_grace {
        profile.policy.cancel_grace = duration(value, LimitName::deployment(Layer::Policy, "cancel_grace"))?;
    }
    if let Some(value) = overrides.refresh_lead {
        profile.policy.refresh_lead = duration(value, LimitName::deployment(Layer::Policy, "refresh_lead"))?;
    }
    if let Some(value) = overrides.exit_grace {
        profile.policy.exit_grace = duration(value, LimitName::deployment(Layer::Policy, "exit_grace"))?;
    }
    Ok(())
}
