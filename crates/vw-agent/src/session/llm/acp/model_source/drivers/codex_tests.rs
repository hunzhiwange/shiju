use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

fn profile(config: &AcpAgentConfig) -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        config,
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_api = Some(crate::provider::provider::ApiInfo {
        protocols: None,
        id: "selected-model".into(),
        url: "https://example.invalid/v1/responses".into(),
        adapter: "openai-responses".into(),
    });
    profile
}

#[tokio::test]
async fn acp_model_source_codex_child_uses_private_responses_configuration_and_revokes_token() {
    use axum::{Router, extract::State, http::HeaderMap, routing::post};
    let original = AcpAgentConfig { command: "codex-acp".into(), ..Default::default() };
    let mut profile = profile(&original);
    profile.model_context_limit = 8192;
    let binding = profile.binding.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = Router::new()
        .route(
            "/v1/responses",
            post(
                |State(binding): State<std::sync::Arc<crate::model_relay::RelayBinding>>,
                 headers: HeaderMap,
                 axum::Json(body): axum::Json<serde_json::Value>| async move {
                    assert_eq!(body["model"], "selected-model");
                    assert_eq!(body["input"], "private-child-probe");
                    let auth =
                        headers["authorization"].to_str().unwrap().strip_prefix("Bearer ").unwrap();
                    assert!(binding.accepts(auth, &binding.identity));
                    axum::Json(serde_json::json!({"output_text":"PRIVATE_CODEX_LAUNCH_OK"}))
                },
            ),
        )
        .with_state(binding.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    configure(&mut profile, &endpoint).unwrap();
    let disk: serde_json::Value =
        toml::from_str(&std::fs::read_to_string(profile.directory().join("config.toml")).unwrap())
            .unwrap();
    assert_eq!(
        disk,
        serde_json::from_str::<serde_json::Value>(&profile.config().env["CODEX_CONFIG"]).unwrap()
    );
    let directory = profile.directory().to_owned();
    let token = binding.bearer_token().to_owned();
    let history = std::path::PathBuf::from(&profile.config().env["SHIJU_CODEX_HISTORY_HOME"]);
    assert_ne!(history, directory);
    let identity = binding.identity.clone();
    let config = profile.config().clone();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("python3")
            .args(["-c", "import os,sys,json,urllib.request; root=os.environ['CODEX_HOME']; c=json.loads(os.environ['CODEX_CONFIG']); assert c['model_context_window']==8192; assert c['model_provider']==os.environ['MODEL_PROVIDER']; p=c['model_providers'][c['model_provider']]; assert p['wire_api']=='responses'; assert p['requires_openai_auth'] is False; assert c['model']==c['review_model']==os.environ['SHIJU_CODEX_MODEL_LOCK']; key=os.environ[p['env_key']]; assert key and all(key not in a for a in sys.argv); assert sys.argv[1:]==['cli','--shiju-model-source']; assert os.environ['NODE_OPTIONS']==os.environ['NODE_PATH']==os.environ['CODEX_PATH']==''; assert os.stat(root).st_mode & 0o777 == 0o700; assert os.stat(root+'/config.toml').st_mode & 0o777 == 0o600; data=json.dumps({'model':c['model'],'input':'private-child-probe'}).encode(); req=urllib.request.Request(p['base_url']+'/responses', data, {'Authorization':'Bearer '+key,'Content-Type':'application/json'}); print(json.load(urllib.request.urlopen(req,timeout=5))['output_text'])"])
            .args(&config.args).envs(&config.env).output().unwrap()
    }).await.unwrap();
    server.abort();
    assert!(output.status.success(), "private Codex child failed");
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "PRIVATE_CODEX_LAUNCH_OK");
    assert!(!profile.config().args.iter().any(|arg| arg.contains(&token)));
    assert!(original.args.is_empty() && original.env.is_empty());
    drop(profile);
    assert!(!directory.exists());
    assert!(!binding.accepts(&token, &identity));
    let mut restored = self::profile(&original);
    configure(&mut restored, &endpoint).unwrap();
    assert!(token != restored.binding.bearer_token());
    assert_ne!(directory, restored.directory());
    assert_eq!(history.to_str().unwrap(), restored.config().env["SHIJU_CODEX_HISTORY_HOME"]);
    assert!(history.is_dir());
}

#[test]
fn acp_model_source_codex_rejects_conflicting_configuration_before_publication() {
    for key in [
        "CODEX_HOME",
        "CODEX_CONFIG",
        "MODEL_PROVIDER",
        "SHIJU_CODEX_RELAY_TOKEN",
        "SHIJU_CODEX_MODEL_LOCK",
        "SHIJU_CODEX_HISTORY_HOME",
        "APP_SERVER_LOGS",
        "DEFAULT_AUTH_REQUEST",
        "CODEX_PATH",
        "NODE_OPTIONS",
        "NODE_PATH",
    ] {
        let config = AcpAgentConfig {
            command: "codex-acp".into(),
            env: std::collections::HashMap::from([(
                key.to_lowercase(),
                "conflicting-secret".into(),
            )]),
            ..Default::default()
        };
        let mut profile = profile(&config);
        let error = configure(&mut profile, "http://127.0.0.1:12345").unwrap_err().to_string();
        assert!(!error.contains("conflicting-secret"));
        assert!(!error.contains(profile.binding.bearer_token()));
        assert!(!profile.directory().join("config.toml").exists());
        assert_eq!(profile.config().env.len(), config.env.len() + 1);
    }
}

#[test]
fn acp_model_source_codex_rejects_non_acp_arguments_and_non_loopback_relay() {
    for args in [vec!["cli".into()], vec!["--config".into(), "secret".into()]] {
        let mut profile = profile(&AcpAgentConfig { args, ..Default::default() });
        assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
        assert!(!profile.directory().join("config.toml").exists());
    }
    for endpoint in [
        "https://example.invalid",
        "http://127.0.0.1:12345/v1",
        "http://user:secret@127.0.0.1:12345",
    ] {
        let mut profile = profile(&Default::default());
        assert!(configure(&mut profile, endpoint).is_err());
        assert!(!profile.directory().join("config.toml").exists());
    }
}

#[test]
fn acp_model_source_codex_rejects_unsupported_transport_before_launch() {
    for (adapter, url) in [
        ("openai-compatible", "https://example.invalid/v1/messages"),
        ("anthropic", "https://example.invalid/v1/messages"),
        ("google", "https://example.invalid/v1beta"),
    ] {
        let mut profile = profile(&Default::default());
        let api = profile.model_api.as_mut().unwrap();
        api.adapter = adapter.into();
        api.url = url.into();
        let error = configure(&mut profile, "http://127.0.0.1:12345").unwrap_err().to_string();
        assert!(error.contains("Responses or Chat Completions model"));
        assert!(!profile.directory().join("config.toml").exists());
        assert!(!profile.config().env.contains_key("CODEX_HOME"));
    }
    let mut profile = profile(&Default::default());
    profile.model_api = None;
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert!(!profile.directory().join("config.toml").exists());
}

#[test]
fn acp_model_source_codex_chat_accepts_tool_capable_models_and_disables_hosted_search() {
    let mut profile = profile(&Default::default());
    let api = profile.model_api.as_mut().unwrap();
    api.adapter = "openai-compatible".into();
    api.url = "https://example.invalid/v1/chat/completions".into();
    let relay = "http://127.0.0.1:12345";
    assert!(configure(&mut profile, relay).unwrap_err().to_string().contains("tool calls"));

    profile.model_supports_tool_call = true;
    configure(&mut profile, relay).unwrap();
    let config: serde_json::Value =
        serde_json::from_str(&profile.config().env["CODEX_CONFIG"]).unwrap();
    assert_eq!(config["web_search"], "disabled");
    let provider = &config["model_providers"][config["model_provider"].as_str().unwrap()];
    assert_eq!(provider["wire_api"], "responses");
    assert_eq!(provider["base_url"], format!("{relay}/v1"));
}

#[tokio::test]
async fn acp_model_source_codex_missing_credentials_and_disabled_model_never_use_native_account() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    struct Restore(Option<std::ffi::OsString>);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe {
                match self.0.take() {
                    Some(value) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", value),
                    None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
                }
            }
        }
    }
    let root = tempfile::tempdir().unwrap();
    let _restore = Restore(std::env::var_os("VIBEWINDOW_CONFIG_DIR"));
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", root.path().join("config"));
    }
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, root.path()).await.unwrap();
    crate::config::update_project_for_tenant(
        &tenant,
        root.path(),
        serde_json::json!({
            "providers": {"codex-offline-no-account": {
                "api": "http://127.0.0.1:1/v1/responses",
                "adapter":"openai-responses", "env":[],
                "models": {
                    "active-model":{"status":"active","tool_call":true},
                    "disabled-model":{"status":"disabled","tool_call":true}
                }
            }}
        }),
    )
    .await
    .unwrap();
    let context = crate::project::instance::Context {
        directory: root.path().to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    let relay_context = context.clone();
    crate::project::instance::provide_context(context, async {
        let config = AcpAgentConfig { command: "codex".into(), ..Default::default() };
        let active =
            crate::provider::provider::get_model("codex-offline-no-account", "active-model")
                .await
                .unwrap();
        // Exercise the real relay/configuration boundary without requiring an installed CLI.
        // Supplier executable identity is covered separately by real-CLI acceptance.
        let mut profile = AcpLaunchProfile::new(
            &config,
            RelayIdentity {
                tenant: tenant.clone(),
                project: relay_context.project.id.clone(),
                session: "missing-key".into(),
                provider: active.provider_id.clone(),
                model: active.id.clone(),
                generation: 0,
            },
        )
        .unwrap();
        profile.model_api = Some(active.api.clone());
        let server = crate::model_relay::RelayServer::start(
            profile.binding.clone(),
            std::sync::Arc::new(crate::model_relay::ProviderRelayBackend::new().unwrap()),
            relay_context,
        )
        .await
        .unwrap();
        let relay_endpoint = format!("http://{}", server.address());
        profile.attach_server(server);
        configure(&mut profile, &relay_endpoint).unwrap();
        let native_config: serde_json::Value =
            serde_json::from_str(&profile.config().env["CODEX_CONFIG"]).unwrap();
        let endpoint = native_config["model_providers"]
            [native_config["model_provider"].as_str().unwrap()]["base_url"]
            .as_str()
            .unwrap();
        let response = crate::model_relay::upstream_http_client()
            .unwrap()
            .post(format!("{endpoint}/responses"))
            .bearer_auth(profile.binding.bearer_token())
            .json(&serde_json::json!({"model":"active-model","input":"hello"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 401);
        assert!(
            crate::provider::provider::get_model("codex-offline-no-account", "disabled-model")
                .await
                .is_err()
        );
        // A previously active snapshot cannot resurrect a now-disabled catalog reference.
        let mut disabled = active.clone();
        disabled.id = "disabled-model".into();
        let result = crate::session::llm::acp::model_source::prepare(
            "codex", &config, &disabled, "disabled",
        )
        .await;
        assert!(result.is_err(), "disabled model must not create a Codex launch");
    })
    .await;
}

// Vendor-native production acceptance; only the upstream HTTP service is a fixture.
use crate::{project::instance, provider::provider, session::llm::StreamEvent};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

fn codex_accepted(evidence: &Value, mode: &str) -> bool {
    let Some(requests) = evidence["requests"].as_array() else {
        return false;
    };
    let primary = requests.iter().any(|r| r["model"] == "acp-fixture-primary");
    let switched = requests.iter().any(|r| r["model"] == "acp-fixture-switched");
    let route = if mode == "session-model-chat" { "/v1/chat/completions" } else { "/v1/responses" };
    evidence["errors"] == 0
        && requests.len() >= 4
        && primary
        && (mode == "cli-local" || switched)
        && requests.iter().all(|r| {
            r["authenticated"] == true
                && r["route"] == route
                && (r["model"] == "acp-fixture-primary"
                    || (mode != "cli-local" && r["model"] == "acp-fixture-switched"))
        })
        && requests.iter().any(|r| r["tool_results"].as_u64().unwrap_or(0) >= 2)
}

async fn codex_evidence(input: &Value) -> Value {
    crate::model_relay::upstream_http_client()
        .unwrap()
        .get(format!("{}/_evidence", input["fixture"].as_str().unwrap()))
        .bearer_auth(input["control"].as_str().unwrap())
        .send()
        .await
        .expect("owned fixture unavailable")
        .error_for_status()
        .expect("owned fixture rejected control authentication")
        .json()
        .await
        .expect("invalid fixture evidence")
}

async fn codex_request(
    input: &Value,
    model: &provider::Model,
    info: &provider::Info,
    prompt: &str,
    cancel: bool,
) -> bool {
    let (tx, rx) = tokio::sync::watch::channel(false);
    let before = codex_evidence(input).await["requests"].as_array().unwrap().len();
    let fixture = input.clone();
    let cancellation = async {
        if cancel {
            for _ in 0..200 {
                let count = codex_evidence(&fixture).await["requests"].as_array().unwrap().len();
                if count > before {
                    tx.send(true).unwrap();
                    return;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            panic!("cancellation never reached a model request");
        }
    };
    let mut options = json!({
        "acp_agent": input["agent"],
        "acp_model_source": if input["mode"] == "cli-local" { "cli_local" } else { "session_model" },
        "cwd": input["root"],
        "acp_close_client_after_request": true,
    });
    if let Some(effort) = input.get("reasoning_effort") {
        options["reasoning_effort"] = effort.clone();
    }
    let messages = json!([{"role":"user", "content":prompt}]);
    let mut output = String::new();
    let mut done = false;
    let mut on_event = |event| match event {
        StreamEvent::Delta(text) => output.push_str(&text),
        StreamEvent::Done { .. } => done = true,
        _ => {}
    };
    let challenge = input["challenge"].as_str().unwrap();
    let session = if input["mode"] == "session-model-chat" {
        // A started session keeps its selected model. Test the second model in a new session.
        format!("real-cli-offline-{challenge}-{}", model.id)
    } else {
        format!("real-cli-offline-{challenge}")
    };
    let production = crate::session::llm::acp::do_stream_request_acp(
        model,
        info,
        None,
        &options,
        &messages,
        &session,
        Some(&rx),
        &mut on_event,
    );
    let (result, ()) = tokio::time::timeout(Duration::from_secs(60), async {
        tokio::join!(production, cancellation)
    })
    .await
    .expect("production ACP request timed out");
    // Persist only fixed boolean classifications, never error text or supplier payloads.
    let error_text = result.as_ref().err().map(ToString::to_string).unwrap_or_default();
    let diagnostic = json!({
        "result_ok": result.is_ok(), "done": done, "has_output": !output.is_empty(),
        "fixture_reply": output.contains("FIXTURE_MODEL_REPLY"),
        "http_401": error_text.contains("401") || output.contains("401"),
        "http_403": error_text.contains("403") || output.contains("403"),
        "http_422": error_text.contains("422") || output.contains("422"),
        "transport_metadata": error_text.contains("transport metadata"),
        "unsupported_transport": error_text.contains("Responses model"),
        "missing_credentials": error_text.contains("credential"),
        "stale_state": error_text.contains("no rollout") || error_text.contains("session not found"),
        "profile_reissue": error_text.contains("reissued"),
        "initialize_error": error_text.contains("initializ"),
        "aborted": matches!(result, Err(crate::session::llm::Error::Aborted)),
    });
    let path = std::path::PathBuf::from(std::env::var("VW_ACP_ACCEPTANCE_FLOW").unwrap())
        .with_extension("codex-diag.json");
    let mut diagnostics = std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Vec<Value>>(&bytes).ok())
        .unwrap_or_default();
    diagnostics.push(diagnostic);
    std::fs::write(path, serde_json::to_vec(&diagnostics).unwrap()).unwrap();
    if cancel {
        matches!(result, Err(crate::session::llm::Error::Aborted))
    } else {
        result.is_ok()
            && done
            && output.contains("FIXTURE_MODEL_REPLY")
            && output.contains(input["challenge"].as_str().unwrap())
            && codex_evidence(input).await["requests"].as_array().unwrap().len() > before
    }
}

async fn codex_run(mode: &str, effort_check: bool) {
    let manifest = std::env::var("VW_ACP_ACCEPTANCE_MANIFEST")
        .expect("explicit offline manifest required; absence is a failure, never a skip");
    let input: Value = serde_json::from_slice(&std::fs::read(manifest).unwrap()).unwrap();
    assert_eq!(input["mode"], mode, "acceptance source mismatch");
    let fixture = reqwest::Url::parse(input["fixture"].as_str().unwrap()).unwrap();
    assert_eq!(fixture.scheme(), "http");
    assert_eq!(fixture.host_str(), Some("127.0.0.1"));
    assert!(fixture.port().is_some(), "owned fixture port required");
    let root = Path::new(input["root"].as_str().unwrap());
    assert!(root.is_absolute());
    let config: crate::config::schema::AcpAgentConfig =
        serde_json::from_value(input["config"].clone()).unwrap();
    assert!(Path::new(&config.command).is_absolute());
    let isolation: std::collections::HashMap<String, String> =
        serde_json::from_value(input["isolation_env"].clone()).unwrap();
    assert!(!isolation.is_empty(), "native config isolation required");
    assert!(isolation.iter().all(|(key, value)| {
        !matches!(key.to_ascii_uppercase().as_str(), "HOME" | "USERPROFILE" | "PATH")
            && Path::new(value).starts_with(root)
            && std::env::var(key).is_ok_and(|inherited| inherited == *value)
    }));
    if mode == "cli-local" {
        assert_eq!(config.env, isolation);
    } else {
        assert!(config.env.is_empty(), "production profile owns managed overrides");
    }
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    crate::env::set("VW_ACP_OFFLINE_FIXTURE_KEY", input["key"].as_str().unwrap());
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, root).await.unwrap();
    let context = instance::Context {
        directory: root.to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    let agent = input["agent"].as_str().unwrap();
    assert_eq!(agent, "codex");
    let (route, adapter) = if mode == "session-model-chat" {
        ("/v1/chat/completions", "openai-compatible")
    } else {
        ("/v1/responses", "openai-responses")
    };
    // Use a model from the pinned engine's catalog so ACP publishes native efforts.
    let primary = if effort_check { "gpt-5.5" } else { "acp-fixture-primary" };
    crate::config::update_project_for_tenant(&tenant, root, json!({
        "acp": {agent: config},
        "providers": {"acp-offline-acceptance": {
            "api": format!("{}{route}", input["fixture"].as_str().unwrap()),
            "adapter":adapter, "env":["VW_ACP_OFFLINE_FIXTURE_KEY"],
            "models": {
                primary:{"id":primary,"status":"active","tool_call":true,"temperature":true,"reasoning":true},
                "acp-fixture-switched":{"id":"acp-fixture-switched","status":"active","tool_call":true,"temperature":true,"reasoning":true}
            }
        }}
    })).await.unwrap();
    instance::provide_context(context, async {
        let info = provider::get_provider("acp-offline-acceptance").await.unwrap().unwrap();
        let model = provider::get_model("acp-offline-acceptance", primary).await.unwrap();
        if effort_check {
            codex_effort_run(&input, &model, &info).await;
            return;
        }
        let switched =
            provider::get_model("acp-offline-acceptance", "acp-fixture-switched").await.unwrap();
        let mut flow = json!({"isolated":true});
        let prompt = "Read challenge.txt twice using your file read tool, then reply.";
        let first_prompt = if agent == "claude" {
            "AUXILIARY_PROBE: Delegate reading challenge.txt twice to a general-purpose helper, then reply with its contents."
        } else { prompt };
        flow["new"] = json!(codex_request(&input, &model, &info, first_prompt, false).await);
        flow["second_turn"] = json!(codex_request(&input, &model, &info, prompt, false).await);
        flow["cancel"] =
            json!(codex_request(&input, &model, &info, "CANCEL_PROBE: read challenge.txt", true).await);
        flow["restart"] = json!(codex_request(&input, &model, &info, prompt, false).await);
        flow["switch"] = json!(codex_request(&input, &switched, &info, prompt, false).await);
        flow["handshake"] = json!(
            vw_acp::list_sessions()
                .await
                .unwrap()
                .iter()
                .any(|r| { r.cwd == root.to_string_lossy() && !r.acp_session_id.is_empty() })
        );
        let observed = codex_evidence(&input).await;
        if agent == "claude" {
            assert!(observed["requests"].as_array().unwrap().iter().any(|request| request["auxiliary"] == true),
                "Claude auxiliary model did not reach the fixture");
        }
        let path = std::env::var("VW_ACP_ACCEPTANCE_FLOW").expect("flow output path required");
        std::fs::write(path, serde_json::to_vec(&flow).unwrap()).unwrap();
        assert!(
            flow.as_object().unwrap().values().all(|v| v == true),
            "production ACP lifecycle did not complete"
        );
        assert!(codex_accepted(&observed, mode), "actual model observations did not satisfy acceptance");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires explicit pinned real CLI manifest and owned offline model fixture"]
async fn acp_model_source_codex_real_cli_session_model() {
    codex_run("session-model", false).await;
}

#[tokio::test]
#[ignore = "requires the pinned Codex CLI and an owned offline Responses fixture"]
async fn acp_model_source_codex_effort_real_cli() {
    codex_run("session-model", true).await;
}

async fn codex_effort_run(input: &Value, model: &provider::Model, info: &provider::Info) {
    use crate::session::llm::acp::{controls, lifecycle};
    use vw_api_types::{
        acp_model_source::AcpModelSource,
        acp_session::{AcpOptionKind, AcpSessionOptions, AcpSessionSelection},
    };

    let directory = input["root"].as_str().unwrap();
    let session = format!("real-cli-offline-{}", input["challenge"].as_str().unwrap());
    let path = std::env::var("VW_ACP_ACCEPTANCE_FLOW").unwrap();
    let mut flow = json!({});
    let mut checkpoint = |key: &str, passed: bool| {
        flow[key] = json!(passed);
        std::fs::write(&path, serde_json::to_vec(&flow).unwrap()).unwrap();
        assert!(passed, "Codex effort checkpoint failed: {key}");
    };

    checkpoint(
        "default_turn",
        codex_request(input, model, info, "ACP_EFFORT_DEFAULT", false).await,
    );
    let store = lifecycle::Store::open(&session, directory).unwrap();
    let anchor = store.read().unwrap().unwrap();
    let native_id = vw_acp::resolve_session_record(&anchor.record_id).await.unwrap().acp_session_id;
    let options = controls::read(&session, directory).await.unwrap();
    checkpoint(
        "managed_model",
        anchor.agent == "codex"
            && anchor.model.source == AcpModelSource::SessionModel
            && anchor.model.model.as_deref() == Some(model.id.as_str())
            && options.acp_model_source == AcpModelSource::SessionModel
            && options.selected_model.as_ref().and_then(|selection| selection.model.as_deref())
                == Some("acp-offline-acceptance/gpt-5.5"),
    );
    let effort = options.options.iter().find(
        |option| matches!(&option.kind, AcpOptionKind::Config { id } if id == "reasoning_effort"),
    );
    checkpoint(
        "advertised",
        effort.is_some_and(|option| {
            ["low", "high"]
                .iter()
                .all(|value| option.choices.iter().any(|choice| choice.id == *value))
                && !option.choices.iter().any(|choice| choice.id == "max")
        }),
    );
    let kind = effort.unwrap().kind.clone();
    let select = |value: &str| {
        controls::select(
            &session,
            directory,
            AcpSessionSelection { kind: kind.clone(), value: value.into() },
        )
    };
    let current_is = |options: AcpSessionOptions, value: &str| {
        options.options.iter().any(|option| option.kind == kind && option.current == value)
    };

    assert!(current_is(select("low").await.expect("select low"), "low"));
    assert!(codex_request(input, model, info, "ACP_EFFORT_A", false).await);
    assert!(current_is(select("high").await.expect("select high"), "high"));
    let mut conflicting_request = input.clone();
    conflicting_request["reasoning_effort"] = json!("low");
    checkpoint(
        "request_precedence",
        codex_request(&conflicting_request, model, info, "ACP_EFFORT_B", false).await,
    );
    let invalid = select("max").await;
    checkpoint("invalid_rejected", invalid.is_err_and(|e| e.contains("Unsupported value")));
    checkpoint(
        "previous_preserved",
        current_is(controls::read(&session, directory).await.unwrap(), "high"),
    );
    // Each managed request tears down its process and relay lease. The next
    // request must restore both the original native thread and its saved effort.
    checkpoint("restored", codex_request(input, model, info, "ACP_EFFORT_RESTORED", false).await);
    let latest = store.read().unwrap().unwrap();
    let record = vw_acp::resolve_session_record(&latest.record_id).await.unwrap();
    checkpoint(
        "same_session",
        latest.record_id == anchor.record_id
            && !native_id.is_empty()
            && record.acp_session_id == native_id,
    );
}

#[tokio::test]
#[ignore = "requires explicit pinned real CLI manifest and owned offline Chat model fixture"]
async fn acp_model_source_codex_chat_real_cli() {
    codex_run("session-model-chat", false).await;
}

#[tokio::test]
#[ignore = "requires explicit pinned real CLI manifest and owned offline model fixture"]
async fn acp_model_source_codex_real_cli_cli_local() {
    codex_run("cli-local", false).await;
}

#[test]
fn acp_model_source_codex_history_rejects_symlinks_and_broad_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let profile = profile(&Default::default());
    let root = tempfile::tempdir().unwrap();
    let broad = root.path().join("broad");
    std::fs::create_dir(&broad).unwrap();
    std::fs::set_permissions(&broad, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(ensure_private_history(&profile, &broad.join("session")).is_err());
    assert!(!broad.join("session").exists());
    let target = tempfile::tempdir().unwrap();
    let linked = root.path().join("linked");
    std::os::unix::fs::symlink(target.path(), &linked).unwrap();
    assert!(ensure_private_history(&profile, &linked.join("session")).is_err());
    assert!(!target.path().join("session").exists());
}
