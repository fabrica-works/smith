//! The product's setting names and representable leaf bounds; relationships
//! belong to resolution (shell.md, sections 5.2–5.3 and 7).
use super::Problem;
use toml::Value;

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Table,
    Text(u64),
    Choice(&'static [&'static str]),
    Integer(u64),
    Number,
    Boolean,
    Strings,
    Efforts,
    Directories,
    Fields,
}

pub(super) fn kind(path: &[String]) -> Option<Kind> {
    let parts: Vec<_> = path.iter().map(String::as_str).collect();
    match parts.as_slice() {
        ["model"] => Some(Kind::Text(193)),
        ["endpoints", _, "program"] => Some(Kind::Choice(&["claude_code"])),
        ["effort"] => Some(Kind::Text(u64::from(smith_charter::CEILINGS.llm_effort_some))),
        ["workspace", "contract", "form"] => Some(Kind::Text(32)),
        ["models", _, "efforts"] => Some(Kind::Efforts),
        ["chat"]
        | ["models", _, "oversized_reasoning"]
        | ["workspace", "delivery", "title"]
        | ["build", "preset"]
        | ["trace", "capture" | "delivery"] => Some(Kind::Text(128)),
        ["profile"] => Some(Kind::Choice(&["standard"])),
        ["instructions"] => Some(Kind::Text(262_144)),
        ["sub_agents"]
        | ["endpoints", _, "trust", "system"]
        | ["workspace", "contract", "checks_must_pass"]
        | ["workspace", "delivery", "pushes"] => Some(Kind::Boolean),
        ["waiting_seconds"]
        | ["budget", "seconds"]
        | ["budget", "reserve", "seconds"]
        | ["models", _, "head" | "idle"]
        | ["endpoints", _, "connect" | "handshake"]
        | [
            "deployment",
            "max_time" | "max_waiting" | "connection_keep" | "tool_deadline" | "shell_timeout" | "job_poll"
            | "job_wait" | "check_timeout" | "wrap_up" | "group_stop" | "close" | "write_deadline" | "cancel_grace"
            | "exit_grace" | "refresh_lead",
        ] => Some(seconds()),
        [
            "budget" | "models" | "endpoints" | "accounts" | "deployment" | "workspace" | "environment" | "build"
            | "trace" | "programs",
        ]
        | ["budget", "reserve"]
        | ["models" | "endpoints" | "accounts", _]
        | ["endpoints", _, "headers" | "trust"]
        | ["accounts", _, "trust"]
        | ["deployment", "compaction_threshold"]
        | ["workspace", "conventions" | "contract" | "delivery"]
        | ["environment", "set"] => Some(Kind::Table),
        ["budget", "turns"]
        | ["budget", "reserve", "turns"]
        | ["models", _, "window" | "output" | "reasoning_item" | "price_unit"]
        | [
            "deployment",
            "max_turns" | "sections" | "host_tools" | "verdicts" | "items" | "fields" | "inbox" | "unacknowledged"
            | "parallel_tools" | "jobs" | "conversations" | "tool_payload" | "file_bytes" | "calls_per_response"
            | "read_window" | "guide" | "shell_output" | "shell_head" | "shell_tail" | "search_hits" | "search_bytes"
            | "list_entries",
        ]
        | ["deployment", "compaction_threshold", "numerator" | "denominator"]
        | ["workspace", "contract", "max"] => Some(count()),
        ["budget", "spend"]
        | ["models", _, "input_price" | "cached_price" | "cache_write_price" | "output_price"]
        | ["deployment", "max_spend"] => Some(Kind::Number),
        ["credentials"]
        | ["programs", "claude_code"]
        | ["endpoints", _, "preset" | "dialect" | "address" | "server_name" | "identity" | "account"]
        | [
            "accounts",
            _,
            "variable" | "program" | "file" | "format" | "client_id" | "scope" | "redirect_uri" | "authorization_url"
            | "token_url" | "issuer" | "keys_url" | "revocation_url",
        ]
        | ["workspace", "conventions", "guide" | "checks"]
        | ["trace", "path"] => Some(Kind::Text(4096)),
        ["endpoints", _, "headers", _] => Some(Kind::Text(8192)),
        ["endpoints" | "accounts", _, "trust", "der" | "pem"] | ["environment", "inherit"] | ["build", "caches"] => {
            Some(Kind::Strings)
        }
        ["accounts", _, "provider"] => Some(Kind::Choice(&["openai", "anthropic"])),
        ["accounts", _, "redirect_port"] => Some(Kind::Integer(65535)),
        ["accounts", _, "source"] => Some(Kind::Choice(&["sign_in", "borrow", "env", "key", "program", "none"])),
        ["accounts", _, "token_format"] => Some(Kind::Choice(&["json", "form"])),
        ["deployment", "memory" | "llm_pool"] => Some(Kind::Integer(i64::MAX.unsigned_abs())),
        ["workspace", "directories"] => Some(Kind::Directories),
        ["workspace", "contract" | "delivery", "fields"] => Some(Kind::Fields),
        ["environment", "set", _] => Some(Kind::Text(1 << 20)),
        _ => None,
    }
}

fn count() -> Kind {
    Kind::Integer(u64::from(u32::MAX))
}

fn seconds() -> Kind {
    Kind::Integer(u64::MAX / 1_000_000_000)
}

pub(super) fn check(value: &Value, kind: Kind) -> Result<(), Problem> {
    let holds = match kind {
        Kind::Table => value.is_table(),
        Kind::Choice(choices) => value.as_str().is_some_and(|value| choices.contains(&value)),
        Kind::Text(bound) => {
            value.as_str().is_some_and(|text| u64::try_from(text.len()).expect("length fits") <= bound)
        }
        Kind::Integer(bound) => value.as_integer().is_some_and(|number| number >= 0 && number.unsigned_abs() <= bound),
        Kind::Number => value
            .as_float()
            .or_else(|| value.as_integer().and_then(|n| n.to_string().parse().ok()))
            .is_some_and(|number| number.is_finite() && number >= 0.0),
        Kind::Boolean => value.is_bool(),
        Kind::Strings => value.as_array().is_some_and(|values| values.iter().all(Value::is_str)),
        Kind::Efforts => value.as_array().is_some_and(|values| {
            values.iter().all(|value| {
                value.as_str().is_some_and(|name| {
                    !name.is_empty()
                        && name.len()
                            <= usize::try_from(smith_charter::CEILINGS.llm_effort_some)
                                .expect("effort bound fits usize")
                })
            })
        }),
        Kind::Directories | Kind::Fields => value.as_array().is_some_and(|array| array.iter().all(Value::is_table)),
    };
    if holds { Ok(()) } else { Err(Problem::WrongValue) }
}

pub(super) fn field(kind: Kind, key: &str) -> Option<Kind> {
    match kind {
        Kind::Directories => match key {
            "name" | "path" => Some(Kind::Text(4096)),
            "writable" | "git" => Some(Kind::Boolean),
            _ => None,
        },
        Kind::Fields => match key {
            "name" => Some(Kind::Text(128)),
            "max" => Some(count()),
            _ => None,
        },
        Kind::Table
        | Kind::Choice(_)
        | Kind::Text(_)
        | Kind::Integer(_)
        | Kind::Number
        | Kind::Boolean
        | Kind::Strings
        | Kind::Efforts => None,
    }
}

/// Keys that reach beyond repository run content (shell.md, section 5.1).
pub(super) fn authority(path: &[String]) -> bool {
    let parts: Vec<_> = path.iter().map(String::as_str).collect();
    matches!(
        parts.as_slice(),
        ["endpoints" | "accounts" | "trust" | "trace" | "deployment" | "credentials" | "programs", ..]
            | ["environment", "inherit", ..]
            | ["build", "caches", ..]
    )
}
