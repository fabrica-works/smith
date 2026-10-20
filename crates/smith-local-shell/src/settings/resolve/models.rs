//! Declared models and rates after precedence and tier selection (shell.md, 5.2 and 6.1).
use super::super::{Origin, Problem, Refusal, Relationship, Settings};
use super::input::{self, count, model_key, money, number, text};
use skein_lib::Duration;
use smith_agent_service::profile;
use smith_domain::run::{self, charter};
use toml::Value;

#[derive(Clone)]
/// One active model's resolved declaration and rates, translated for the native owners.
pub(in crate::settings) struct Model {
    pub full: String,
    pub endpoint: String,
    pub declared: profile::Model,
    pub prices: run::Prices,
}

pub(in crate::settings) fn resolve(settings: &mut Settings, active: &[String]) -> Result<Vec<Model>, Refusal> {
    let main = text(settings, "model")?.to_string();
    let mut names = super::catalog(settings);
    names.remove(&main);
    let mut models = Vec::new();
    for full in std::iter::once(main).chain(names) {
        let (endpoint, name) =
            full.split_once('/').ok_or_else(|| input::refuse(settings, "model", "expected endpoint/model"))?;
        input::default(settings, model_key(&full, "price_unit"), Value::Integer(1_000_000));
        let window = count(settings, &model_key(&full, "window"))?;
        let unit = count(settings, &model_key(&full, "price_unit"))?;
        if unit == 0 {
            return Err(input::refuse(settings, &model_key(&full, "price_unit"), "price unit must be positive"));
        }
        let write = model_key(&full, "cache_write_price");
        let prices = run::Prices {
            input: money(settings, &model_key(&full, "input_price"))?,
            cached: money(settings, &model_key(&full, "cached_price"))?,
            cache_write: if settings.values.contains_key(&write) { Some(money(settings, &write)?) } else { None },
            output: money(settings, &model_key(&full, "output_price"))?,
            unit,
        };
        if let Err(rate) = prices.check() {
            let field = match rate {
                run::Rate::Input => "input_price",
                run::Rate::Cached => "cached_price",
                run::Rate::CacheWrite => "cache_write_price",
                run::Rate::Output => "output_price",
            };
            return Err(input::refuse(settings, &model_key(&full, field), "model rate must be positive"));
        }
        let output = count(settings, &model_key(&full, "output"))?;
        if output == 0 {
            return Err(input::refuse(settings, &model_key(&full, "output"), "output must be positive"));
        }
        if output >= window {
            let key = model_key(&full, "output");
            let right_key = model_key(&full, "window");
            return Err(Refusal {
                origin: input::sourced(settings, &key)?.origin.clone(),
                key,
                problem: Problem::Relationship(Box::new(Relationship {
                    check: profile::Check::OutputInWindow,
                    left: profile::Value::Tokens(u64::from(output)),
                    right: profile::Value::Tokens(u64::from(window)),
                    right_origin: settings.values.get(&right_key).map_or(Origin::BuiltIn, |value| value.origin.clone()),
                    right_key,
                })),
            });
        }
        if !active.contains(&full) {
            continue;
        }
        let prefix = format!("endpoints.{}", super::super::name(endpoint));
        let dialect = text(settings, &format!("{prefix}.dialect"))?.to_string();
        let default_idle = match dialect.as_str() {
            "openai" | "codex" => 300,
            "anthropic" => 120,
            _ => return Err(input::refuse(settings, &format!("{prefix}.dialect"), "unsupported dialect")),
        };
        for (field, value) in [("reasoning_item", 65_536), ("head", 60), ("idle", default_idle)] {
            input::default(settings, model_key(&full, field), Value::Integer(value));
        }
        let head = number(settings, &model_key(&full, "head"))?;
        let idle = number(settings, &model_key(&full, "idle"))?;
        let policy = model_key(&full, "oversized_reasoning");
        let oversized = match settings.values.get(&policy).and_then(|value| value.value.as_str()).unwrap_or("fail") {
            "fail" => profile::OversizedReasoning::Fail,
            "drop" => profile::OversizedReasoning::Drop,
            _ => return Err(input::refuse(settings, &policy, "expected fail or drop")),
        };
        models.push(Model {
            full: full.clone(),
            endpoint: endpoint.into(),
            prices,
            declared: profile::Model {
                name: name.as_bytes().into(),
                efforts: settings
                    .values
                    .get(&model_key(&full, "efforts"))
                    .and_then(|value| value.value.as_array())
                    .into_iter()
                    .flatten()
                    .map(|value| value.as_str().expect("schema effort").as_bytes().into())
                    .collect(),
                window,
                output,
                reasoning_item: count(settings, &model_key(&full, "reasoning_item"))?,
                head: Duration::from_secs(head),
                idle: Duration::from_secs(idle),
                oversized_reasoning: oversized,
            },
        });
    }
    Ok(models)
}

impl Model {
    pub fn document(&self) -> serde_json::Value {
        let model = &self.declared;
        let policy = match model.oversized_reasoning {
            profile::OversizedReasoning::Fail => "fail",
            profile::OversizedReasoning::Drop => "drop",
        };
        serde_json::json!({"name":String::from_utf8_lossy(&model.name),"window":model.window,"output":model.output,
            "prices":{"input":self.prices.input,"cached":self.prices.cached,"cache_write":self.prices.cache_write,"output":self.prices.output,"unit":self.prices.unit},
            "reasoning_item":model.reasoning_item,"head":model.head.as_nanos()/1_000_000,
            "idle":model.idle.as_nanos()/1_000_000,"oversized_reasoning":policy,
            "efforts":self.declared.efforts.iter().map(|name|std::str::from_utf8(name).expect("settings UTF-8")).collect::<Vec<_>>()})
    }

    pub fn local(&self, endpoint: u32, dialect: u32, account: Option<u32>) -> charter::Llm {
        charter::Llm {
            effort: None,
            prices: self.prices,
            dialect,
            account,
            endpoint: charter::Endpoint(endpoint),
            model: self.declared.name.clone(),
            window: self.declared.window,
            output: self.declared.output,
        }
    }
}
