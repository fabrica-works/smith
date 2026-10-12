//! Map native comparisons to settings paths and origins (shell.md, section 5.3).
//! Refusals keep typed sides; no diagnostic string is used to recover a rule.
use super::super::{BudgetComparison, Origin, Problem, Refusal, Relationship, Settings};
use super::{input, models::Model};
use skein_lib::Duration;
use smith_agent_service::profile::{self, Layer, LimitName, Per, Value};
use smith_agent_shell::config::ConfigurationRefusal;
use smith_domain::run;
use std::collections::BTreeSet;

pub(super) fn budget(settings: &Settings, profile: &profile::Profile) -> Result<(), Refusal> {
    let requested = run::Budget {
        turns: input::count(settings, "budget.turns")?,
        spend: input::money(settings, "budget.spend")?,
        time: Duration::from_secs(input::number(settings, "budget.seconds")?),
    };
    let ceiling =
        run::Budget { turns: profile.policy.max_turns, spend: profile.policy.max_spend, time: profile.policy.max_time };
    requested.check(&ceiling).map_err(|reason| budget_refusal(settings, reason, false, None))
}

fn budget_refusal(settings: &Settings, reason: run::BudgetRefusal, waiting: bool, most: Option<u64>) -> Refusal {
    let (field, other_key, other) = match reason {
        run::BudgetRefusal::Ceiling(run::Ceiling::Turns(bound)) => {
            ("budget.turns", "deployment.max_turns", Value::Count(u64::from(bound)))
        }
        run::BudgetRefusal::Ceiling(run::Ceiling::Spend(bound)) => {
            ("budget.spend", "deployment.max_spend", Value::Spend(bound))
        }
        run::BudgetRefusal::Ceiling(run::Ceiling::Time(bound)) => {
            if waiting {
                ("waiting_seconds", "deployment.max_waiting", Value::Duration(bound))
            } else {
                ("budget.seconds", "deployment.max_time", Value::Duration(bound))
            }
        }
        run::BudgetRefusal::Empty(part) => (budget_key(part), "zero", Value::Count(0)),
        run::BudgetRefusal::Reserve(part) => match part {
            run::Part::Turns => (
                "budget.turns",
                "budget.reserve.turns",
                Value::Count(u64::from(input::count(settings, "budget.reserve.turns").expect("decoded reserve"))),
            ),
            run::Part::Time => (
                "budget.seconds",
                "budget.reserve.seconds",
                Value::Duration(Duration::from_secs(
                    input::number(settings, "budget.reserve.seconds").expect("decoded reserve"),
                )),
            ),
            run::Part::Spend => (
                "budget.spend",
                "main completion times budget.reserve.turns",
                Value::Spend(
                    most.and_then(|most| {
                        most.checked_mul(u64::from(
                            input::count(settings, "budget.reserve.turns").expect("decoded reserve"),
                        ))
                    })
                    .unwrap_or(u64::MAX),
                ),
            ),
        },
        run::BudgetRefusal::Completion { most } => ("budget.spend", "main completion", Value::Spend(most)),
        run::BudgetRefusal::Checks { reserve, checks } => {
            return comparison_budget(
                settings,
                "budget.reserve.seconds",
                Value::Duration(reserve),
                "deployment.check_timeout",
                Value::Duration(checks),
                reason,
            );
        }
    };
    let requested = if field.ends_with("turns") {
        Value::Count(input::number(settings, field).expect("decoded turns"))
    } else if field.ends_with("spend") {
        Value::Spend(input::money(settings, field).expect("decoded spend"))
    } else {
        Value::Duration(Duration::from_secs(input::number(settings, field).expect("decoded time")))
    };
    comparison_budget(settings, field, requested, other_key, other, reason)
}

fn budget_key(part: run::Part) -> &'static str {
    match part {
        run::Part::Turns => "budget.turns",
        run::Part::Spend => "budget.spend",
        run::Part::Time => "budget.seconds",
    }
}

fn comparison_budget(
    settings: &Settings,
    key: &str,
    requested: Value,
    other_key: &str,
    other: Value,
    reason: run::BudgetRefusal,
) -> Refusal {
    Refusal {
        origin: origin(settings, key),
        key: key.into(),
        problem: Problem::Budget(Box::new(BudgetComparison {
            reason,
            requested,
            other,
            other_key: other_key.into(),
            other_origin: origin(settings, other_key),
            profile: input::text(settings, "profile").expect("decoded profile").into(),
        })),
    }
}

pub(super) fn local(
    settings: &Settings,
    local: &smith_local_domain::Config,
    limits: &smith_local_domain::Limits,
    why: smith_local_domain::Invalid,
) -> Refusal {
    use smith_local_domain::Invalid;
    let (key, message) = match why {
        Invalid::Budget(reason) => {
            return budget_refusal(
                settings,
                reason,
                local.mode.waiting() > limits.smith.run.waiting,
                local.models.first().and_then(run::most_costly),
            );
        }
        Invalid::Price(_) => ("models", "model rate must be positive"),
        Invalid::Chat => ("chat", "chat name exceeds its receiving bound"),
        Invalid::Text => ("instructions", "local text exceeds its receiving bound"),
        Invalid::Models => ("model", "model choices exceed their receiving bound"),
        Invalid::Accounts => ("accounts", "accounts exceed their receiving bound"),
        Invalid::Endpoint => ("endpoints", "local endpoint is absent from the agent"),
        Invalid::Limits => ("workspace.directories", "local policy exceeds its receiving bounds"),
        Invalid::Contract => ("workspace.contract.fields", "change requires its bounded title field"),
        Invalid::Push => ("workspace.delivery.pushes", "push targets require writable git directories"),
    };
    input::refuse(settings, key, message)
}

pub(crate) fn budget_line(refusal: &Refusal, comparison: &BudgetComparison) -> String {
    let BudgetComparison { reason, requested, other, other_key, other_origin, profile } = comparison;
    let left = format!("{} = {} (from {})", refusal.key, value(*requested), refusal.origin.description());
    let right = format!("{other_key} = {} (from {})", value(*other), other_origin.description());
    match *reason {
        run::BudgetRefusal::Ceiling(_) if *other_origin == Origin::BuiltIn => {
            let (amount, unit) = match *other {
                Value::Count(value) => (value.to_string(), "turns"),
                Value::Spend(value) => (self::value(Value::Spend(value)), "notional"),
                Value::Duration(value) => ((value.as_nanos() / 1_000_000_000).to_string(), "seconds"),
                Value::Bytes(_) | Value::Tokens(_) | Value::Fraction(_) => unreachable!("budget units"),
            };
            let requested = match *requested {
                Value::Count(value) => value.to_string(),
                other @ (Value::Bytes(_)
                | Value::Tokens(_)
                | Value::Spend(_)
                | Value::Duration(_)
                | Value::Fraction(_)) => value(other),
            };
            format!(
                "smith: {} = {requested} (from {}) exceeds the {profile} profile's ceiling of {amount} {unit}",
                refusal.key,
                refusal.origin.description()
            )
        }
        run::BudgetRefusal::Ceiling(_) => format!("smith: budget-ceiling: {left} exceeds {right}"),
        run::BudgetRefusal::Empty(_) => format!("smith: positive-budget: {left} must be positive"),
        run::BudgetRefusal::Reserve(_) => format!("smith: finishing-reserve: {left} cannot hold {right}"),
        run::BudgetRefusal::Completion { .. } => format!("smith: completion-affordability: {left} cannot pay {right}"),
        run::BudgetRefusal::Checks { .. } => format!("smith: reserve-over-checks: {left} is below {right}"),
    }
}

pub(super) fn native(settings: &Settings, models: &[Model], why: ConfigurationRefusal) -> Refusal {
    match why {
        ConfigurationRefusal::Input(message) => input::refuse(settings, "configuration", &message),
        ConfigurationRefusal::Profile(why) => profile(settings, models, why),
    }
}

pub(super) fn profile(settings: &Settings, models: &[Model], why: profile::Refusal) -> Refusal {
    match why {
        profile::Refusal::Broken { check, left, right } => {
            let key = setting_key(models, left.name);
            let right_key = setting_key(models, right.name);
            Refusal {
                origin: origin(settings, &key),
                key,
                problem: Problem::Relationship(Box::new(Relationship {
                    check,
                    left: left.value,
                    right: right.value,
                    right_origin: origin(settings, &right_key),
                    right_key,
                })),
            }
        }
        profile::Refusal::Field { .. }
        | profile::Refusal::Derived { .. }
        | profile::Refusal::Environment { .. }
        | profile::Refusal::Connection(_)
        | profile::Refusal::Llm { .. } => {
            let words = String::from_utf8(profile::words(&why).into_vec()).expect("native refusal is UTF-8");
            input::refuse(settings, "deployment", &words)
        }
    }
}

fn origin(settings: &Settings, key: &str) -> Origin {
    settings.values.get(key).map_or(Origin::BuiltIn, |value| value.origin.clone())
}

fn setting_key(models: &[Model], name: LimitName) -> String {
    let endpoints: BTreeSet<_> = models.iter().map(|model| model.endpoint.as_str()).collect();
    match name.per {
        Per::Deployment => match name.layer {
            Layer::Declared | Layer::Policy => format!("deployment.{}", name.field),
            Layer::Llm
            | Layer::Render
            | Layer::Session
            | Layer::Run
            | Layer::Tools
            | Layer::Machine
            | Layer::Transcript
            | Layer::Channel
            | Layer::Events
            | Layer::Io
            | Layer::Domain
            | Layer::Service
            | Layer::Local
            | Layer::Inline
            | Layer::Host => format!("{}.{}", name.layer.name(), name.field),
        },
        Per::Endpoint(number) => {
            let endpoint = endpoints
                .iter()
                .nth(usize::try_from(number).expect("endpoint fits usize"))
                .expect("native endpoint is selected");
            format!("endpoints.{}.{}", super::super::name(endpoint), name.field)
        }
        Per::Model { endpoint, model } => {
            let endpoint = endpoints
                .iter()
                .nth(usize::try_from(endpoint).expect("endpoint fits usize"))
                .expect("native endpoint is selected");
            let model = models
                .iter()
                .filter(|entry| entry.endpoint.as_str() == *endpoint)
                .nth(usize::try_from(model).expect("model fits usize"))
                .expect("native model is selected");
            input::model_key(&model.full, name.field)
        }
    }
}

pub(crate) fn value(value: Value) -> String {
    match value {
        Value::Bytes(value) => format!("{value} bytes"),
        Value::Count(value) => format!("{value} items"),
        Value::Tokens(value) => format!("{value} tokens"),
        Value::Spend(value) => format!("${}.{:06}", value / 1_000_000, value % 1_000_000),
        Value::Duration(value) => format!("{} ns", value.as_nanos()),
        Value::Fraction(value) => format!("{}/{}", value.numerator, value.denominator),
    }
}
