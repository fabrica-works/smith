//! Caller-selected live endpoints and private, durable test credentials
//! (testing.md, section 2.3; protocol/hosts.md, section 5.4).

use smith_real_world::Scratch;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

#[must_use]
pub fn setting(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn required(name: &str) -> String {
    setting(name).unwrap_or_else(|| panic!("live suite requires {name}"))
}

#[must_use]
pub fn enabled() -> bool {
    setting("NEXTEST_PROFILE").as_deref() == Some("live")
}

/// One real provider and its caller-supplied public OAuth registration.
pub struct Backend {
    pub name: String,
    pub model: String,
    account: serde_json::Value,
    pub tokens: PathBuf,
    pub borrowed: Option<PathBuf>,
}

fn small_model(provider_name: &str) -> String {
    let models_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benchmarks/agents/models.toml");
    let models = smith_bench::read_model_tiers(&models_file).expect("pinned live models");
    let provider =
        if provider_name == "codex" { smith_bench::Provider::Codex } else { smith_bench::Provider::Anthropic };
    models.lookup(&models_file, "small", provider).expect("small live model").model.clone().expect("resolved model")
}

/// Public endpoint choices and a token directory belonging only to these tests.
pub struct Environment {
    pub backends: Vec<Backend>,
    pub remote: Option<String>,
}

impl Environment {
    pub fn load(bootstrap: bool) -> Self {
        if bootstrap {
            return Self::legacy(true, false);
        }
        let providers = required("SMITH_TEST_LIVE_PROVIDERS");
        let mut backends = Vec::new();
        for name in providers.split(',').map(str::trim) {
            assert!(matches!(name, "codex" | "anthropic"), "selected live providers");
            if name == "codex" {
                let file = PathBuf::from(required("SMITH_TEST_LIVE_CODEX_LOGIN_FILE"));
                assert!(file.is_absolute(), "caller-named Codex login file must be absolute");
                backends.push(Backend {
                    name: name.into(),
                    model: setting("SMITH_TEST_LIVE_CODEX_MODEL").unwrap_or_else(|| small_model(name)),
                    account: serde_json::Value::Null,
                    tokens: PathBuf::new(),
                    borrowed: Some(file),
                });
            } else {
                backends.extend(Self::legacy(false, true).backends);
            }
        }
        Self { backends, remote: setting("SMITH_TEST_LIVE_GIT_REMOTE") }
    }

    #[must_use]
    pub fn issuer() -> Option<Self> {
        setting("SMITH_TEST_LIVE_TOKEN_DIR").map(|_| Self::legacy(false, false))
    }

    fn legacy(bootstrap: bool, only_anthropic: bool) -> Self {
        let root = smith_bench::guard::write_path(Path::new(&required("SMITH_TEST_LIVE_TOKEN_DIR")))
            .expect("guarded live token directory");
        if bootstrap {
            std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&root).expect("create private test tokens");
        }
        let metadata =
            std::fs::metadata(&root).expect("SMITH_TEST_LIVE_TOKEN_DIR must exist; bootstrap and sign in first");
        assert!(
            metadata.is_dir() && metadata.permissions().mode().trailing_zeros() >= 6,
            "live token directory must be private (0700)"
        );
        let marker = smith_bench::guard::write_path(&root.join(".smith-live-test-tokens")).expect("guarded marker");
        if bootstrap {
            std::fs::write(&marker, b"Dedicated Smith live-test token directory\n").expect("mark test-only directory");
        }
        assert!(marker.is_file(), "SMITH_TEST_LIVE_TOKEN_DIR needs the bootstrap's test-only marker");
        let providers = required("SMITH_TEST_LIVE_PROVIDERS");
        let mut backends = Vec::new();
        for name in providers.split(',').map(str::trim) {
            assert!(matches!(name, "codex" | "anthropic"), "SMITH_TEST_LIVE_PROVIDERS accepts codex,anthropic");
            if only_anthropic && name != "anthropic" {
                continue;
            }
            assert!(!backends.iter().any(|backend: &Backend| backend.name == name), "each live provider appears once");
            let prefix = format!("SMITH_TEST_LIVE_{}", name.to_ascii_uppercase());
            let account_file = required(&format!("{prefix}_ACCOUNT_FILE"));
            assert!(
                std::fs::metadata(&account_file).expect("public account registration file").len() <= 32_768,
                "bounded public registration"
            );
            let mut account: serde_json::Value =
                serde_json::from_slice(&std::fs::read(account_file).expect("public account registration"))
                    .expect("account registration must be JSON");
            assert!(
                account.get("account_id").is_some_and(serde_json::Value::is_string)
                    && account.get("oauth").is_some_and(serde_json::Value::is_object),
                "registration requires account_id and oauth"
            );
            assert!(
                account
                    .as_object()
                    .expect("account object")
                    .keys()
                    .all(|key| matches!(key.as_str(), "number" | "account_id" | "oauth")),
                "public account registration has unknown fields"
            );
            assert!(
                account.get("oauth").expect("OAuth registration").as_object().expect("OAuth object").keys().all(
                    |key| matches!(
                        key.as_str(),
                        "authorization_url"
                            | "token_endpoint"
                            | "client_id"
                            | "redirect_uri"
                            | "scope"
                            | "address"
                            | "server_name"
                            | "json"
                    )
                ),
                "public OAuth registration cannot contain credentials or unknown fields"
            );
            smith_bench::guard::check_credential_record(&account)
                .expect("public registration cannot copy refresh tokens");
            account.as_object_mut().expect("account object").insert("number".into(), serde_json::json!(0));
            let tokens = smith_bench::guard::write_path(&root.join(name)).expect("guarded provider directory");
            if bootstrap {
                std::fs::DirBuilder::new()
                    .mode(0o700)
                    .create(&tokens)
                    .or_else(
                        |error| if error.kind() == std::io::ErrorKind::AlreadyExists { Ok(()) } else { Err(error) },
                    )
                    .expect("private provider token directory");
            }
            assert!(
                tokens.canonicalize().expect("provider token directory; bootstrap first").starts_with(&root),
                "provider tokens must stay in the dedicated directory"
            );
            if !bootstrap {
                let saved = smith_local_shell::local_tokens::Tokens::new(
                    &tokens,
                    smith_local_shell::local_host::token_limits(),
                )
                .expect("private test token store")
                .load(0)
                .expect("valid private token record");
                assert!(saved.is_some(), "provider must be signed in by hand; missing dedicated 0.json record");
            }
            backends.push(Backend {
                name: name.into(),
                model: setting(&format!("{prefix}_MODEL")).unwrap_or_else(|| small_model(name)),
                account,
                tokens,
                borrowed: None,
            });
        }
        Self { backends, remote: setting("SMITH_TEST_LIVE_GIT_REMOTE") }
    }
}

impl Backend {
    pub fn write_product(&self, scratch: &Scratch, settings: &serde_json::Value) {
        let directory =
            smith_bench::guard::write_path(&scratch.path().join("config/smith")).expect("guarded user layer");
        std::fs::create_dir_all(&directory).expect("scratch user layer");
        let settings: toml::Value = serde_json::from_value(settings.clone()).expect("product settings");
        std::fs::write(directory.join("config.toml"), toml::to_string(&settings).expect("user layer TOML"))
            .expect("private user layer");
    }

    #[must_use]
    pub fn settings(&self, directory: &Path, change: bool) -> serde_json::Value {
        if let Some(file) = &self.borrowed {
            return serde_json::json!({"model":format!("codex/{}",self.model),"effort":"low",
                "instructions":"Carry out the requested outcome and call finish when done.",
                "accounts":{"codex":{"source":"borrow","file":file,"format":"codex","program":"codex"}},
                "budget":{"turns":512,"spend":25,"seconds":3600},
                "workspace":{"directories":[{"name":"repo","path":directory.join("repo"),"writable":true,"git":change}],
                    "conventions":{"guide":".smith-test/guide","checks":".smith-test/check"},
                    "contract":if change {serde_json::json!({"form":"change","checks_must_pass":true,
                        "fields":[{"name":"title","max":256},{"name":"body","max":4096}]})}
                        else {serde_json::json!({"form":"report","max":4096})}},
                "environment":{"inherit":[],"set":{"PATH":required("PATH")}}});
        }
        let (host, identity, headers) = if self.name == "codex" {
            ("chatgpt.com", "plain", skein_llm::openai::identity::headers().iter().map(|header| serde_json::json!({"name":String::from_utf8_lossy(&header.name),"value":String::from_utf8_lossy(&header.value)})).collect::<Vec<_>>())
        } else {
            let mut headers = skein_llm::anthropic::identity::claude_code_headers();
            for header in &mut headers {
                if header.name.eq_ignore_ascii_case(b"anthropic-beta") {
                    header.value = b"claude-code-20250219,oauth-2025-04-20".as_slice().into();
                }
            }
            ("api.anthropic.com", "claude-code", headers.iter().map(|header| serde_json::json!({"name":String::from_utf8_lossy(&header.name),"value":String::from_utf8_lossy(&header.value)})).collect::<Vec<_>>())
        };
        let mut settings = serde_json::json!({
            "agent":{"profile":{"name":"standard","declared":{"memory":2_147_483_648_u64},"policy":{"group_stop":1000}},
                "endpoints":[{"name":self.name,"number":0,"dialect":0,"account":0,"provider":self.name,"connect":10000,"handshake":10000,"address":format!("{host}:443"),"server_name":host,"transport":"tls","identity":identity,"headers":headers,"reasoning_effort":if self.name == "codex" { Some("low") } else { None }}],
                "environment":[],"trace":{"path":directory.join("agent-trace.jsonl"),"capture":"calls"}},
            "chat":"chat","instructions":"@local-shell Follow the user's requested outcome exactly. Use the tools and call finish when done.",
            "models":[{"endpoint":self.name,"name":self.model,"window":8192,"output":4096,"reasoning_item":2048,"head":60000,"idle":30000,"input_price":1_250_000,"cached_price":125_000,"output_price":10_000_000,"price_unit":1_000_000}],
            "budget":{"turns":512,"spend":25_000_000,"seconds":3600},"waiting_seconds":30,
            "contract":{"form":"report","max":4096},"token_directory":self.tokens,"accounts":[self.account]
        });
        if change {
            settings.as_object_mut().expect("settings object").insert(
                "directories".into(),
                serde_json::json!([{"name":"repo","path":directory.join("repo"),"writable":true,"git":true}]),
            );
            settings.as_object_mut().expect("settings object").insert(
                "conventions".into(),
                serde_json::json!({"guide":".smith-test/guide","checks":".smith-test/check"}),
            );
            settings.as_object_mut().expect("settings object").insert("contract".into(), serde_json::json!({"form":"change","checks_must_pass":true,"fields":[{"name":"title","max":256},{"name":"body","max":4096}]}));
        }
        settings
    }

    pub fn configure(&self, scratch: &Scratch, change: bool, push: Option<(&str, &str)>) {
        let mut settings = self.settings(scratch.path(), change);
        if self.borrowed.is_some() {
            if push.is_some() {
                settings
                    .get_mut("workspace")
                    .expect("product workspace")
                    .as_object_mut()
                    .expect("workspace table")
                    .insert("delivery".into(), serde_json::json!({"pushes":true}));
                if let Some(agent) = setting("SSH_AUTH_SOCK") {
                    settings
                        .get_mut("environment")
                        .expect("product environment")
                        .get_mut("set")
                        .expect("set table")
                        .as_object_mut()
                        .expect("environment values")
                        .insert("SSH_AUTH_SOCK".into(), serde_json::json!(agent));
                }
            }
            self.write_product(scratch, &settings);
            return;
        }
        if let Some((remote, branch)) = push {
            settings
                .as_object_mut()
                .expect("settings object")
                .insert("push".into(), serde_json::json!([{"remote":remote,"branch":branch}]));
            settings.as_object_mut().expect("settings object").insert(
                "delivery_environment".into(),
                serde_json::json!(
                    ["HOME", "PATH", "SSH_AUTH_SOCK"]
                        .into_iter()
                        .filter_map(|name| setting(name).map(|value| format!("{name}={value}")))
                        .collect::<Vec<_>>()
                ),
            );
        }
        std::fs::write(
            smith_bench::guard::write_path(&scratch.path().join("settings.json")).expect("guarded live settings"),
            serde_json::to_vec(&settings).expect("live settings JSON"),
        )
        .expect("live settings");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_provider_settings_use_machine_verified_tls_and_their_identity() {
        let scratch = Scratch::new();
        for name in ["codex", "anthropic"] {
            let backend = Backend {
                name: name.into(),
                model: "configured-model".into(),
                account: serde_json::json!({"number":0,"account_id":"public-account","oauth":{}}),
                tokens: scratch.path().join("dedicated-tokens"),
                borrowed: None,
            };
            let settings = backend.settings(scratch.path(), true);
            let endpoint = &settings["agent"]["endpoints"][0];
            assert_eq!(endpoint["transport"], "tls");
            assert!(endpoint.get("trust_der").is_none());
            assert_eq!(endpoint["identity"], if name == "anthropic" { "claude-code" } else { "plain" });
            assert_eq!(settings["token_directory"], serde_json::json!(backend.tokens));
            assert!(settings.get("push").is_none());
            assert_eq!(settings["contract"]["checks_must_pass"], true);
        }
    }

    fn pinned_product_check(conversations: i64) -> smith_bench::process::ProcessResult {
        use skein_lib::Duration;
        let scratch = Scratch::new();
        let root = scratch.path();
        let guard = smith_bench::guard::Guard::new(&root.join("user"), &root.join("config"), &root.join("state"), &[])
            .expect("guard");
        let private = smith_bench::isolate::empty(&guard, &root.join("attempt")).expect("private paths");
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks");
        let pin_file = project.join("agents/smith/first-request.pin.toml");
        let pin = smith_bench::read_configuration(&pin_file).expect("pin");
        let mut task = smith_bench::read_task(&project.join("tasks/probes/first-request/task.toml")).expect("task");
        task.smith.insert("conversations".into(), toml::Value::Integer(conversations));
        let model = smith_bench::ModelChoice { model: Some("gpt-6-luna".into()), effort: Some("low".into()) };
        let arm = smith_bench::arm::ArmRef::Installed { program: "/unused".into() };
        let environment = smith_bench::isolate::environment(
            &smith_bench::isolate::Machine { path: "/usr/bin:/bin".into(), rustup_home: None },
            &private,
            &std::collections::BTreeMap::new(),
        )
        .expect("environment");
        let clock = skein_shell::Clock::new();
        let token = smith_local_process_world::world::login_at(clock.now().wall, false);
        let login = root.join("unissued-auth.json");
        std::fs::write(&login,serde_json::to_vec(&serde_json::json!({"tokens":{"access_token":std::str::from_utf8(&token).expect("unissued token"),"account_id":"acc"}})).expect("fixture encoding")).expect("fixture file");
        let context = smith_bench::agent::Context {
            guard: &guard,
            private: &private,
            pin: &pin,
            pin_file: &pin_file,
            task: &task,
            variant: None,
            model: &model,
            arm: &arm,
            installed_version: "unused",
            home: root,
            wall: clock.now().wall,
            lifetime: Duration::from_secs(20),
            environment: &environment,
        };
        let base: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../benchmarks/agents/smith/first-request.json"))
                .expect("template");
        let rendered = smith_bench::agent::smith::product_settings(&context, &base, &login).expect("product rendering");
        let directory = private.config.join("smith");
        std::fs::create_dir(&directory).expect("user layer directory");
        let document: toml::Value = serde_json::from_value(rendered).expect("TOML document");
        std::fs::write(directory.join("config.toml"), toml::to_string(&document).expect("user layer"))
            .expect("scratch settings");
        let result = smith_bench::process::execute(
            &clock,
            skein_world::end_to_end::Command {
                program: env!("CARGO_BIN_EXE_smith").into(),
                arguments: vec![
                    "check".into(),
                    "--json".into(),
                    "exec".into(),
                    "--json".into(),
                    "-C".into(),
                    private.workspace.clone().into(),
                    "-o".into(),
                    root.join("unopened/result").into(),
                    "--ephemeral".into(),
                ],
                environment,
                directory: private.workspace.clone(),
            },
            Box::default(),
            smith_bench::process::Capture::Raw,
            smith_bench::process::Bounds {
                hard: Duration::from_secs(1),
                teardown: Duration::ZERO,
                grace: Duration::from_millis(20),
                settlement: Duration::from_secs(1),
                stdout: 1_048_576,
                stderr: 65_536,
                force_walk: true,
            },
        )
        .expect("check settles");
        assert!(!root.join("unopened").exists());
        assert_eq!(std::fs::read_dir(&private.state).expect("scratch state").count(), 0);
        result
    }

    #[test]
    fn the_product_adapter_user_layer_passes_the_shipped_check() {
        let result = pinned_product_check(5);
        assert_eq!(result.exit, smith_bench::Exit::Code(0), "pinned product settings pass check");
        assert!(
            smith_bench::agent::smith::teardown_from_check(&result.stdout)
                .is_ok_and(|bound| bound == skein_lib::Duration::from_secs(40)),
            "check reports its teardown bound"
        );
    }

    #[test]
    fn the_shipped_exec_check_refuses_the_whole_startup_footprint() {
        let result = pinned_product_check(6);
        assert_eq!(result.exit, smith_bench::Exit::Code(2));
        assert!(String::from_utf8_lossy(&result.stderr).contains("memory"));
        assert!(String::from_utf8_lossy(&result.stderr).contains("service.worst_case"));
    }
}
