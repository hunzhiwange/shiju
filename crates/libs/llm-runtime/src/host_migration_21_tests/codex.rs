#![allow(unused_imports)]
use crate::acp::model_source::profile::AcpLaunchProfile;
use std::{collections::HashMap, io::Write, path::Path};
use serde_json::json;
use model_reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};
use crate::acp::model_source::drivers::codex::*;
use agent_config::schema::AcpAgentConfig;
use model_transport::RelayIdentity;
use model_types::ApiInfo;


fn profile(config: &AcpAgentConfig) -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        config,
        RelayIdentity {
            tenant: vw_api_transport::TenantScope::default(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    profile.model_api = Some(model_types::ApiInfo {
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
                |State(binding): State<std::sync::Arc<model_transport::RelayBinding>>,
                 headers: HeaderMap,
                 axum::Json(body): axum::Json<serde_json::Value>| async move {
                    assert_eq!(body["model"], "selected-model");
                    assert_eq!(body["input"], "private-child-probe");
                    let auth =
                        headers["authorization"].to_str().unwrap().strip_prefix("Bearer ").unwrap();
                    assert!(binding.accepts(auth, binding.identity()));
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
    let identity = binding.identity().clone();
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
        assert!(error.contains(if adapter == "openai-compatible" {
            "InvalidEndpoint"
        } else {
            "ConversionUnavailable"
        }));
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
    profile.model_supports_tool_call = false;
    assert!(
        configure(&mut profile, relay).unwrap_err().to_string().contains("CapabilityUnavailable")
    );

    profile.model_supports_tool_call = true;
    configure(&mut profile, relay).unwrap();
    let config: serde_json::Value =
        serde_json::from_str(&profile.config().env["CODEX_CONFIG"]).unwrap();
    assert_eq!(config["web_search"], "disabled");
    let provider = &config["model_providers"][config["model_provider"].as_str().unwrap()];
    assert_eq!(provider["wire_api"], "responses");
    assert_eq!(provider["base_url"], format!("{relay}/v1"));
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

#[test]
fn model_protocol_013_codex_catalog_native_responses_preserves_hosted_tools() {
    let mut launch = profile(&Default::default());
    let api = launch.model_api.as_mut().unwrap();
    api.adapter = "openai-compatible".into();
    api.url = "https://example.invalid/v1".into();
    api.protocols = Some(serde_json::from_value(serde_json::json!([{
        "protocol":"responses", "url":"https://example.invalid/v1/responses",
        "deployment":"selected", "authentication":{"kind":"bearer"},
        "models":{"kind":"models", "ids":["selected-model"]},
        "evidence":[{"kind":"configuration", "source":"fixture", "reference":"explicit", "checked_at":"2026-11-11"}]
    }])).unwrap());
    configure(&mut launch, "http://127.0.0.1:12345").unwrap();
    let config: serde_json::Value =
        serde_json::from_str(&launch.config().env["CODEX_CONFIG"]).unwrap();
    assert!(config.get("web_search").is_none());
    assert_eq!(
        config["model_providers"][config["model_provider"].as_str().unwrap()]["wire_api"],
        "responses"
    );
}
