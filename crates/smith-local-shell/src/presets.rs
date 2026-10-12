//! Presets supply literal settings and list prices; layers retain their origins.
//! They know no file, credential or running service. `built_in` supplies the
//! catalog; `prices` selects the tier after a window resolves (shell.md, 5.2, 6.1–6.2).
use crate::settings::{Origin, Settings, Sourced};
use smith_domain::run::Prices;

const MODELS: [&str; 5] = [
    "codex/gpt-6-astra",
    "codex/gpt-6-luna",
    "anthropic/claude-opus-5-5",
    "anthropic/claude-sonnet-5-5",
    "anthropic/claude-haiku-5-5",
];

/// Extra inherited names for a build toolchain, never patterns or values.
pub(crate) fn build_homes(preset: &str) -> Option<&'static [&'static str]> {
    match preset {
        "rust" => Some(&["CARGO_HOME", "RUSTUP_HOME"]),
        _ => None,
    }
}

/// One provider's built-in endpoint, account metadata and known models.
#[derive(Clone, Copy, Debug)]
pub enum Preset {
    /// Codex's endpoint, account metadata and known model declarations.
    Codex,
    /// Anthropic's endpoint, account metadata and known model declarations.
    Anthropic,
}

/// Settings decoded by the same strict layer reader as the person's files.
#[must_use]
pub const fn built_in(preset: Preset) -> &'static str {
    match preset {
        Preset::Codex => {
            r"
[endpoints.codex]
preset = 'codex'
dialect = 'codex'
address = 'chatgpt.com:443'
server_name = 'chatgpt.com'
identity = 'plain'
account = 'codex'
connect = 10
handshake = 10
[accounts.codex]
format = 'codex'
program = 'codex'
[models.'codex/gpt-6-astra']
window = 272000
output = 32000
reasoning_item = 65536
head = 60
idle = 300
price_unit = 1000000
[models.'codex/gpt-6-luna']
window = 272000
output = 32000
reasoning_item = 65536
head = 60
idle = 300
price_unit = 1000000
"
        }
        Preset::Anthropic => {
            r"
[endpoints.anthropic]
preset = 'anthropic'
dialect = 'anthropic'
address = 'api.anthropic.com:443'
server_name = 'api.anthropic.com'
identity = 'claude-code'
account = 'anthropic'
connect = 10
handshake = 10
[accounts.anthropic]
variable = 'ANTHROPIC_API_KEY'
[models.'anthropic/claude-opus-5-5']
window = 200000
output = 32000
reasoning_item = 65536
head = 60
idle = 120
price_unit = 1000000
[models.'anthropic/claude-sonnet-5-5']
window = 200000
output = 32000
reasoning_item = 65536
head = 60
idle = 120
price_unit = 1000000
[models.'anthropic/claude-haiku-5-5']
window = 200000
output = 32000
reasoning_item = 65536
head = 60
idle = 120
price_unit = 1000000
"
        }
    }
}

/// List rates in micro-dollars per million tokens for the resolved window.
/// Standard rates: developers.openai.com/api/docs/models/{gpt-6-astra,gpt-6-luna}
/// and platform.claude.com/docs/en/about-claude/pricing (2026-10-12).
#[must_use]
pub(crate) fn prices(model: &str, window: u32) -> Option<Prices> {
    let (input, cached, cache_write, output) = match model {
        "codex/gpt-6-astra" if window > 272_000 => (20_000_000, 2_000_000, 25_000_000, 75_000_000),
        "codex/gpt-6-astra" => (10_000_000, 1_000_000, 12_500_000, 50_000_000),
        "codex/gpt-6-luna" if window > 272_000 => (200_000, 20_000, 250_000, 750_000),
        "anthropic/claude-opus-5-5" => (4_000_000, 200_000, 5_000_000, 20_000_000),
        "anthropic/claude-sonnet-5-5" => (2_000_000, 100_000, 2_500_000, 10_000_000),
        "anthropic/claude-haiku-5-5" if window > 100_000 => (500_000, 50_000, 625_000, 2_500_000),
        "codex/gpt-6-luna" | "anthropic/claude-haiku-5-5" => (100_000, 10_000, 125_000, 500_000),
        _ => return None,
    };
    Some(Prices { input, cached, cache_write: Some(cache_write), output, unit: 1_000_000 })
}

/// Fill preset rates after all window and unit overrides, preserving supplied rates.
pub(crate) fn apply_prices(settings: &mut Settings) {
    for model in MODELS {
        fill_prices(settings, model, model);
    }
}

pub(crate) fn fill_prices(settings: &mut Settings, model: &str, preset_model: &str) {
    let prefix = format!("models.{}", crate::settings::name(model));
    let window = settings.values.get(&format!("{prefix}.window")).expect("preset window");
    let window = u32::try_from(window.value.as_integer().expect("validated window")).expect("window fits");
    let unit = settings.values.get(&format!("{prefix}.price_unit")).expect("preset price unit");
    let unit = u32::try_from(unit.value.as_integer().expect("validated unit")).expect("unit fits");
    let Some(rates) = prices(preset_model, window) else { return };
    for (field, rate) in [
        ("input_price", rates.input),
        ("cached_price", rates.cached),
        ("cache_write_price", rates.cache_write.expect("preset write rate")),
        ("output_price", rates.output),
    ] {
        let key = format!("{prefix}.{field}");
        if settings.values.get(&key).is_some_and(|value| value.origin != Origin::BuiltIn) {
            continue;
        }
        let rate = f64::from(u32::try_from(rate).expect("list rate fits u32"));
        let value = toml::Value::Float(rate / 1_000_000.0 * f64::from(unit) / 1_000_000.0);
        settings.values.insert(key, Sourced { value, origin: Origin::BuiltIn });
    }
}
