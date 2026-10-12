//! Local policy values and workspace roots (shell.md, 5.2–5.4 and 6.4).
//! Native policy and limits retain these choices; no credential or directory is opened.
use super::super::{Refusal, Settings};
use super::{Account, input, models::Model};
use skein_lib::{Duration, Token};
use smith_domain::run::{self, charter, outcome};
use smith_local_domain as local;
use toml::Value;

fn fields(settings: &Settings, key: &str) -> Result<Box<[outcome::FieldRule]>, Refusal> {
    let Some(value) = settings.values.get(key) else { return Ok(Box::new([])) };
    let rows = value.value.as_array().expect("schema fields");
    let mut fields = Vec::new();
    for row in rows {
        let name = row
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| input::refuse(settings, key, "field name is required"))?;
        let max = row
            .get("max")
            .and_then(Value::as_integer)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| input::refuse(settings, key, "field maximum is required"))?;
        fields.push(outcome::FieldRule { name: name.as_bytes().into(), max });
    }
    Ok(fields.into_boxed_slice())
}

fn contract(settings: &Settings) -> Result<local::Contract, Refusal> {
    match settings.values.get("workspace.contract.form").and_then(|value| value.value.as_str()).unwrap_or("report") {
        "report" => Ok(local::Contract::Report(outcome::TextSpec {
            max: if settings.values.contains_key("workspace.contract.max") {
                input::count(settings, "workspace.contract.max")?
            } else {
                4096
            },
            fields: Box::new([]),
        })),
        "change" => Ok(local::Contract::Change(outcome::ChangeSpec {
            checks_must_pass: settings
                .values
                .get("workspace.contract.checks_must_pass")
                .and_then(|value| value.value.as_bool())
                .unwrap_or(true),
            fields: fields(settings, "workspace.contract.fields")?,
        })),
        _ => Err(input::refuse(settings, "workspace.contract.form", "expected report or change")),
    }
}

pub(super) fn limits(
    settings: &Settings,
    models: &[Model],
    agent: &smith_agent_shell::config::Configuration,
) -> Result<local::Limits, Refusal> {
    let native = agent.service.limits.domain;
    let endpoints = agent.declarations.endpoints.iter().map(|endpoint| charter::Endpoint(endpoint.number)).collect();
    Ok(local::Limits {
        smith: native,
        agent: local::AgentLimits::inline(
            native,
            2,
            Duration::from_secs(input::number(settings, "budget.seconds")?),
            agent.profile.policy.cancel_grace,
        )
        .ok_or_else(|| input::refuse(settings, "deployment", "inline bounds overflow"))?,
        endpoints,
        chat_bytes: 256,
        text_bytes: 1 << 16,
        models: u32::try_from(models.len()).map_err(|_| input::refuse(settings, "models", "too many models"))?,
        line_bytes: local::max_line_bytes(native.run.message_bytes)
            .ok_or_else(|| input::refuse(settings, "deployment", "message bound cannot hold its label"))?,
        show_bytes: 8192,
        unsaved: 2,
        facts: 1024,
    })
}

pub(super) fn build(
    settings: &Settings,
    models: &[Model],
    agent: &smith_agent_shell::config::Configuration,
    accounts: &[Account],
) -> Result<local::Config, Refusal> {
    let mut configured = Vec::new();
    let mut allowances = Vec::new();
    for model in models {
        let endpoint = agent.service.channel_endpoints.resolve(model.endpoint.as_bytes()).expect("configured endpoint");
        configured.push(model.local(endpoint.number, endpoint.dialect, endpoint.account));
        let declaration =
            agent.declarations.endpoints.iter().find(|entry| entry.number == endpoint.number).expect("declaration");
        allowances.push(
            smith_agent_service::profile::endpoint_allowance(
                &agent.profile,
                declaration,
                &agent.service.limits.domain.session.tools,
            )
            .map_err(|why| super::relationships::profile(settings, models, why))?,
        );
    }
    let directories = directories(settings, agent)?;
    let conventions = if settings.values.keys().any(|key| key.starts_with("workspace.conventions.")) {
        Some(run::Conventions {
            guide: input::text(settings, "workspace.conventions.guide")?.as_bytes().into(),
            checks: input::text(settings, "workspace.conventions.checks")?.as_bytes().into(),
        })
    } else {
        None
    };
    let delivery = fields(settings, "workspace.delivery.fields")?;
    let deliver =
        if delivery.is_empty() { None } else { Some(outcome::ChangeSpec { checks_must_pass: true, fields: delivery }) };
    let pushes =
        settings.values.get("workspace.delivery.pushes").and_then(|value| value.value.as_bool()).unwrap_or(false);
    let push = pushes.then(|| {
        directories
            .iter()
            .map(|directory| {
                (directory.git && directory.writable)
                    .then(|| local::PushTarget { remote: Box::from(&b"origin"[..]), branch: Box::from(&b"HEAD"[..]) })
            })
            .collect()
    });
    Ok(local::Config {
        chat: input::text(settings, "chat")?.as_bytes().into(),
        instructions: settings
            .values
            .get("instructions")
            .and_then(|value| value.value.as_str())
            .unwrap_or(
                "Carry out the person's request in this workspace. Finish with what changed and what you checked.",
            )
            .as_bytes()
            .into(),
        brief: charter::Brief { sections: Box::new([]) },
        models: configured.into_boxed_slice(),
        sub_agents: settings
            .values
            .get("sub_agents")
            .and_then(|value| value.value.as_bool())
            .expect("default delegation"),
        progress: models
            .iter()
            .map(|model| smith_domain::session::Progress { head: model.declared.head, idle: model.declared.idle })
            .collect(),
        protocol_allowances: allowances.into_boxed_slice(),
        budget: run::Budget {
            turns: input::count(settings, "budget.turns")?,
            spend: input::money(settings, "budget.spend")?,
            time: Duration::from_secs(input::number(settings, "budget.seconds")?),
        },
        reserve: charter::Reserve {
            turns: input::count(settings, "budget.reserve.turns")?,
            time: Duration::from_secs(input::number(settings, "budget.reserve.seconds")?),
        },
        conventions,
        contract: contract(settings)?,
        deliver,
        title_field: settings
            .values
            .get("workspace.delivery.title")
            .and_then(|value| value.value.as_str())
            .unwrap_or("title")
            .as_bytes()
            .into(),
        mode: local::Mode::Chat { waiting: Duration::from_secs(input::number(settings, "waiting_seconds")?) },
        resume: true,
        accounts: accounts.iter().map(|account| account.number).collect(),
        workspace: Some(run::Workspace { directories: directories.into_boxed_slice() }),
        push,
    })
}

fn directories(
    settings: &Settings,
    agent: &smith_agent_shell::config::Configuration,
) -> Result<Vec<run::Directory>, Refusal> {
    let mut directories = Vec::new();
    let rows = settings
        .values
        .get("workspace.directories")
        .and_then(|value| value.value.as_array())
        .cloned()
        .unwrap_or_else(|| {
            vec![Value::Table(toml::Table::from_iter([
                ("name".into(), Value::String("workspace".into())),
                ("path".into(), Value::String(".".into())),
                ("writable".into(), Value::Boolean(true)),
                ("git".into(), Value::Boolean(settings.workspace.ancestors().any(|path| path.join(".git").exists()))),
            ]))]
        });
    for (index, row) in rows.iter().enumerate() {
        let key = format!("workspace.directories[{index}]");
        let path = row
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| input::refuse(settings, &key, "directory path is required"))?;
        let name = row
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| input::refuse(settings, &key, "directory name is required"))?;
        let path = settings.workspace.join(path);
        let host = smith_protocol_channel::host_path(
            path.as_os_str().as_encoded_bytes(),
            agent.service.limits.domain.run.host_path_bytes,
        )
        .ok_or_else(|| input::refuse(settings, &key, "directory path exceeds its receiving bound"))?;
        directories.push(run::Directory {
            host,
            name: name.as_bytes().into(),
            root: Token::new(u64::try_from(index).expect("bounded roots").checked_add(1).expect("bounded root")),
            writable: row.get("writable").and_then(Value::as_bool).unwrap_or(true),
            git: row.get("git").and_then(Value::as_bool).unwrap_or(false),
            conflicts: Box::new([]),
        });
    }
    Ok(directories)
}
