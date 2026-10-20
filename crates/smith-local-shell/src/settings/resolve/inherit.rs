//! Custom names take preset defaults, retaining each value supplied by a layer.
//! Only the catalog is read here; no endpoint or credential is opened (shell.md, 5.2).
use super::{
    super::{Origin, Refusal, Settings, Sourced},
    input,
};
use crate::presets::{self, Preset};

pub(super) fn apply(settings: &mut Settings) -> Result<(), Refusal> {
    let endpoints: Vec<_> = settings
        .values
        .keys()
        .filter_map(|key| {
            key.strip_prefix("endpoints.").and_then(|path| path.strip_suffix(".preset")).map(str::to_owned)
        })
        .collect();
    for encoded in endpoints {
        let key = format!("endpoints.{encoded}.preset");
        let provider = input::text(settings, &key)?.to_string();
        let preset = match provider.as_str() {
            "openai" => Preset::OpenAi,
            "codex" => Preset::Codex,
            "anthropic" => Preset::Anthropic,
            _ => return Err(input::refuse(settings, &key, "unknown endpoint preset")),
        };
        let catalog: toml::Table = toml::from_str(presets::built_in(preset)).expect("preset catalog");
        let fields = catalog
            .get("endpoints")
            .expect("preset endpoints")
            .get(&provider)
            .and_then(toml::Value::as_table)
            .expect("preset endpoint");
        for (field, value) in fields {
            input::default(settings, format!("endpoints.{encoded}.{field}"), value.clone());
        }
        let endpoint: String = serde_json::from_str(&encoded).unwrap_or(encoded);
        let mut names = super::catalog(settings);
        if let Some(main) = settings.values.get("model").and_then(|field| field.value.as_str()) {
            names.insert(main.into());
        }
        for full in names {
            let Some((used, model)) = full.split_once('/') else { continue };
            if used != endpoint {
                continue;
            }
            let Some(fields) = catalog
                .get("models")
                .expect("preset models")
                .get(format!("{provider}/{model}"))
                .and_then(toml::Value::as_table)
            else {
                continue;
            };
            for (field, value) in fields {
                settings
                    .values
                    .entry(input::model_key(&full, field))
                    .or_insert(Sourced { value: value.clone(), origin: Origin::BuiltIn });
            }
            presets::fill_prices(settings, &full, &format!("{provider}/{model}"));
        }
    }
    Ok(())
}
