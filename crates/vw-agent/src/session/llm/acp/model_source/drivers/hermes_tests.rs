use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

fn profile(config: &AcpAgentConfig) -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        config,
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "hermes-test".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_api = Some(crate::provider::provider::ApiInfo {
        id: "selected-model".into(),
        url: "https://example.invalid/v1".into(),
        adapter: "openai-compatible".into(),
    });
    profile.model_supports_temperature = true;
    profile.model_output_limit = 1024;
    profile
}

#[test]
fn acp_model_source_hermes_private_routes_revoke_and_reissue() {
    let original =
        AcpAgentConfig { command: "hermes".into(), args: vec!["acp".into()], ..Default::default() };
    let mut launch = profile(&original);
    configure(&mut launch, "http://127.0.0.1:12345").unwrap();
    let directory = launch.directory().to_owned();
    let config: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("config.yaml")).unwrap()).unwrap();
    assert_eq!(launch.config().env["HERMES_HOME"], directory.to_string_lossy());
    assert_eq!(launch.config().env["CUSTOM_BASE_URL"], "http://127.0.0.1:12345/v1");
    let mut expected_args = original.args.clone();
    expected_args.push("--session-model-binding".into());
    assert_eq!(launch.config().args, expected_args);
    assert_eq!(original.env.len(), 0);
    assert!(config["fallback_providers"].as_array().unwrap().is_empty());
    assert!(config["fallback_model"].is_null());
    for route in std::iter::once(&config["model"])
        .chain(std::iter::once(&config["delegation"]))
        .chain(config["auxiliary"].as_object().unwrap().values().filter(|v| v.is_object()))
    {
        assert_eq!(route["provider"], "custom");
        assert_eq!(route["base_url"], "http://127.0.0.1:12345/v1");
        assert!(route["api_key"].as_str() == Some(launch.binding.bearer_token()));
    }
    assert_eq!(config["model"]["default"], "selected-model");
    assert_eq!(config["model"]["max_tokens"], 1024);
    assert_eq!(config["delegation"]["model"], "selected-model");
    for route in config["auxiliary"].as_object().unwrap().values().filter(|v| v.is_object()) {
        assert_eq!(route["model"], "selected-model");
        assert!(route["fallback_chain"].as_array().unwrap().is_empty());
    }
    assert!(std::fs::read(directory.join(".env")).unwrap().is_empty());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(directory.join("config.yaml")).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(std::fs::metadata(&directory).unwrap().permissions().mode() & 0o777, 0o700);
    }
    let binding = launch.binding.clone();
    let token = binding.bearer_token().to_owned();
    assert!(!launch.config().args.iter().any(|arg| arg.contains(&token)));
    drop(launch);
    assert!(!directory.exists());
    assert!(!binding.accepts(&token, &binding.identity));
    let mut reissued = profile(&original);
    configure(&mut reissued, "http://127.0.0.1:12345").unwrap();
    assert_ne!(reissued.directory(), directory);
    assert!(reissued.binding.bearer_token() != token);
}

#[test]
fn acp_model_source_hermes_conflicting_environment_and_flags_fail_before_publication() {
    for key in [
        "HERMES_HOME",
        "hermes_home",
        "CUSTOM_BASE_URL",
        "HERMES_PROFILE",
        "HERMES_CONFIG",
        "HERMES_ENV",
        "HERMES_MANAGED_DIR",
        "HERMES_HOME_MODE",
    ] {
        let mut launch = profile(&AcpAgentConfig {
            args: vec!["acp".into()],
            env: std::collections::HashMap::from([(key.into(), "must-not-leak".into())]),
            ..Default::default()
        });
        let error = configure(&mut launch, "http://127.0.0.1:12345").unwrap_err();
        assert!(!error.to_string().contains("must-not-leak"));
        assert!(!launch.directory().join("config.yaml").exists());
    }
    for args in [vec!["--profile", "local", "acp"], vec!["acp", "--setup"], vec!["chat"]] {
        let mut launch = profile(&AcpAgentConfig {
            args: args.into_iter().map(str::to_owned).collect(),
            ..Default::default()
        });
        assert!(configure(&mut launch, "http://127.0.0.1:12345").is_err());
        assert!(!launch.directory().join("config.yaml").exists());
    }
}

#[test]
fn acp_model_source_hermes_rejects_nonlocal_endpoint_and_unsupported_sampling() {
    for endpoint in
        ["https://example.invalid", "http://127.0.0.1:12345/path", "http://token@127.0.0.1:12345"]
    {
        let mut launch =
            profile(&AcpAgentConfig { args: vec!["acp".into()], ..Default::default() });
        assert!(configure(&mut launch, endpoint).is_err());
        assert!(!launch.directory().join("config.yaml").exists());
    }
    let mut launch = profile(&AcpAgentConfig { args: vec!["acp".into()], ..Default::default() });
    launch.model_supports_temperature = false;
    assert!(configure(&mut launch, "http://127.0.0.1:12345").is_err());
    assert!(!launch.directory().join("config.yaml").exists());
}

#[test]
fn acp_model_source_hermes_rejects_unknown_protocol_before_publication() {
    let mut launch = profile(&AcpAgentConfig { args: vec!["acp".into()], ..Default::default() });
    launch.model_api.as_mut().unwrap().adapter = "unsupported".into();
    assert!(configure(&mut launch, "http://127.0.0.1:12345").is_err());
    assert!(!launch.directory().join("config.yaml").exists());
    launch.model_api = None;
    assert!(configure(&mut launch, "http://127.0.0.1:12345").is_err());
    assert!(!launch.directory().join("config.yaml").exists());
}

#[tokio::test]
async fn acp_model_source_hermes_rejects_missing_credentials_and_disabled_model_before_launch() {
    use crate::{
        provider::provider,
        session::{llm::Error, message},
    };
    use serde_json::json;
    let io = json!({"text":true,"audio":false,"image":false,"video":false,"pdf":false});
    let mut model: provider::Model = serde_json::from_value(json!({
        "id":"selected-model","providerID":"hermes-bound",
        "api":{"id":"selected-model","url":"http://127.0.0.1:1/v1","adapter":"openai-compatible"},
        "name":"selected-model", "release_date":"2026-01-01", "status":"active",
        "capabilities":{"temperature":true,"reasoning":false,"attachment":false,"toolcall":true,"input":io.clone(),"output":io,"interleaved":false},
        "cost":{"input":0,"output":0,"cache":{"read":0,"write":0}},
        "limit":{"context":8192,"output":1024}
    })).unwrap();
    let mut info = provider::Info {
        id: "hermes-bound".into(),
        name: "Hermes fixture".into(),
        source: provider::ProviderSource::Custom,
        env: Vec::new(),
        key: None,
        options: Default::default(),
        models: Default::default(),
    };
    for disabled in [false, true] {
        model.status = if disabled { "disabled" } else { "active" }.into();
        info.key = disabled.then(|| "unused-fixture-key".into());
        let mut events = 0;
        let result = crate::session::llm::acp::do_stream_request_acp(
            &model,
            &info,
            None,
            &json!({"acp_agent":"hermes", "acp_model_source":"session_model"}),
            &json!([{"role":"user","content":"must not launch"}]),
            "hermes-rejected",
            None,
            &mut |_| events += 1,
        )
        .await;
        match (disabled, result) {
            (false, Err(Error::Api(message::AssistantError::ProviderAuthError { .. }))) => {}
            (true, Err(Error::Api(message::AssistantError::Unknown { message }))) => {
                assert!(message.contains("active non-ACP"));
            }
            _ => panic!("Hermes invalid model reached launch or returned the wrong error"),
        }
        assert_eq!(events, 0);
    }
}

/// Uses the installed pinned package, never a synthetic ACP agent. The local
/// HTTP fixture replaces only the external model service.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires VW_HERMES_PYTHON pointing to hermes-agent[acp] 0.19.0"]
async fn acp_model_source_hermes_real_auxiliary_and_delegation_use_profile() {
    use axum::{
        Json,
        extract::State,
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        routing::{get, post},
    };
    use serde_json::{Value, json};
    use std::sync::{Arc, Mutex};
    let python = std::env::var("VW_HERMES_PYTHON").expect("explicit fixed Hermes Python required");
    let mut launch = profile(&AcpAgentConfig { args: vec!["acp".into()], ..Default::default() });
    let token = launch.binding.bearer_token().to_owned();
    let records = Arc::new(Mutex::new(Vec::<Value>::new()));
    let app = axum::Router::new()
        .route("/v1/models", get(|| async { Json(json!({"data":[{"id":"selected-model","context_length":131072}]})) }))
        .route("/v1/chat/completions", post(move |State(records): State<Arc<Mutex<Vec<Value>>>>, headers: HeaderMap, Json(body): Json<Value>| {
            let expected = format!("Bearer {token}");
            async move {
                let valid = headers.get("authorization").and_then(|v| v.to_str().ok()) == Some(expected.as_str());
                let stream = body["stream"] == true;
                records.lock().unwrap().push(json!({"model":body["model"],"authenticated":valid}));
                if !valid || body["messages"].as_array().is_some_and(|messages| messages.iter().any(|m| m["content"] == "AUTH_FAILURE_PROBE")) {
                    return (StatusCode::UNAUTHORIZED, Json(json!({"error":{"message":"offline authentication failure","type":"authentication_error"}}))).into_response();
                }
                if body["messages"].as_array().is_some_and(|messages| messages.iter().any(|m| m["content"] == "PAYMENT_FAILURE_PROBE")) {
                    return (StatusCode::PAYMENT_REQUIRED, Json(json!({"error":{"message":"offline payment failure","type":"payment_error"}}))).into_response();
                }
                if stream {
                    let chunks = [
                        json!({"id":"probe","object":"chat.completion.chunk","model":"selected-model","choices":[{"index":0,"delta":{"role":"assistant","content":"HERMES_BOUND_REPLY"},"finish_reason":null}]}),
                        json!({"id":"probe","object":"chat.completion.chunk","model":"selected-model","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}),
                    ];
                    let data = format!("data: {}\n\ndata: {}\n\ndata: [DONE]\n\n", chunks[0], chunks[1]);
                    return ([("content-type", "text/event-stream")], data).into_response();
                }
                Json(json!({"id":"probe","object":"chat.completion","model":"selected-model","choices":[{"index":0,"message":{"role":"assistant","content":"HERMES_BOUND_REPLY"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}})).into_response()
            }
        }))
        .with_state(records.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    configure(&mut launch, &endpoint).unwrap();
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/vibewindow/acp-model-source/evidence/hermes/probe_auxiliary.py");
    let mut command = tokio::process::Command::new(python);
    command
        .arg(script)
        .env_clear()
        .envs(
            ["PATH", "HOME", "TMPDIR"]
                .into_iter()
                .filter_map(|key| std::env::var_os(key).map(|value| (key, value))),
        )
        .env("OPENAI_API_KEY", "forbidden-native-account")
        .envs(&launch.config().env)
        .current_dir(launch.directory())
        .kill_on_drop(true);
    let output = tokio::time::timeout(std::time::Duration::from_secs(90), command.output())
        .await
        .expect("real Hermes probe timed out")
        .unwrap();
    server.abort();
    assert!(output.status.success(), "real Hermes auxiliary/delegation probe failed");
    assert!(String::from_utf8_lossy(&output.stdout).contains("HERMES_AUXILIARY_AND_DELEGATION_OK"));
    let records = records.lock().unwrap();
    assert_eq!(records.len(), 21, "18 auxiliary routes, delegated turn and two surfaced failures");
    assert!(records.iter().all(|r| r["model"] == "selected-model" && r["authenticated"] == true));
}
