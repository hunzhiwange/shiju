#![allow(unused_imports)]
use crate::acp::model_source::profile::AcpLaunchProfile;
use std::{collections::HashMap, io::Write, path::Path};
use serde_json::json;
use model_reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};
use crate::acp::model_source::drivers::gemini::*;
use agent_config::schema::AcpAgentConfig;
use model_transport::RelayIdentity;
use model_types::ApiInfo;


fn profile() -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: "gemini".into(),
            args: vec!["--experimental-acp".into()],
            ..Default::default()
        },
        RelayIdentity {
            tenant: vw_api_transport::TenantScope::default(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 1,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    profile.model_api = Some(model_types::ApiInfo {
        id: "selected-model".into(),
        url: "https://example.invalid/v1beta".into(),
        adapter: "google".into(),
        protocols: None,
    });
    profile
}

#[test]
fn acp_model_source_gemini_private_configuration_routes_all_models() {
    let mut profile = profile();
    configure(&mut profile, "http://127.0.0.1:1234").unwrap();
    let config: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&profile.config().env["GEMINI_CLI_SYSTEM_SETTINGS_PATH"]).unwrap(),
    )
    .unwrap();
    assert_eq!(config["security"]["auth"]["selectedType"], "gateway");
    assert_eq!(config["model"]["name"], "selected-model");
    assert_eq!(config["modelConfigs"]["overrides"][0]["modelConfig"]["model"], "selected-model");
    let token = profile.binding.bearer_token().to_owned();
    assert_eq!(profile.config().env["GEMINI_API_KEY"], token);
    assert!(!profile.config().args.iter().any(|arg| arg.contains(&token)));
    let directory = profile.directory().to_owned();
    let binding = profile.binding.clone();
    drop(profile);
    assert!(!directory.exists());
    assert!(!binding.accepts(&token, binding.identity()));
    let mut replacement = self::profile();
    configure(&mut replacement, "http://127.0.0.1:1234").unwrap();
    assert_ne!(replacement.directory(), directory);
    assert_ne!(replacement.binding.bearer_token(), token);
}

#[test]
fn acp_model_source_gemini_rejects_remote_endpoint() {
    let mut profile = profile();
    assert!(configure(&mut profile, "https://example.com").is_err());
    assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
}

#[tokio::test]
async fn acp_model_source_gemini_subprocess_sends_private_key_to_bound_model() {
    use axum::{Router, http::HeaderMap, routing::post};
    let mut profile = profile();
    let binding = profile.binding.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = Router::new().route(
        "/v1beta/models/selected-model:generateContent",
        post(
            move |headers: HeaderMap, axum::Json(body): axum::Json<serde_json::Value>| async move {
                assert!(
                    binding.accepts(headers["x-goog-api-key"].to_str().unwrap(), binding.identity())
                );
                assert_eq!(body["contents"][0]["parts"][0]["text"], "private-child-probe");
                axum::Json(serde_json::json!({"response":"PRIVATE_GEMINI_OK"}))
            },
        ),
    );
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    configure(&mut profile, &endpoint).unwrap();
    let config = profile.config().clone();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("python3").args(["-c", r#"
import os,sys,json,urllib.request
key=os.environ['GEMINI_API_KEY']
assert all(key not in arg for arg in sys.argv)
assert os.environ['GOOGLE_GENAI_USE_VERTEXAI']=='false'
assert os.environ['GOOGLE_GENAI_USE_GCA']=='false'
assert os.stat(os.environ['GEMINI_CLI_HOME']).st_mode & 0o777 == 0o700
assert os.stat(os.environ['GEMINI_CLI_SYSTEM_SETTINGS_PATH']).st_mode & 0o777 == 0o600
url=os.environ['GOOGLE_GEMINI_BASE_URL']+'/v1beta/models/'+os.environ['GEMINI_MODEL']+':generateContent'
body=json.dumps({'contents':[{'parts':[{'text':'private-child-probe'}]}]}).encode()
request=urllib.request.Request(url,body,{'x-goog-api-key':key,'Content-Type':'application/json'})
print(json.load(urllib.request.urlopen(request,timeout=5))['response'])
"#]).args(config.args).envs(config.env).output().unwrap()
    }).await.unwrap();
    server.abort();
    assert!(output.status.success(), "private child failed");
    assert_eq!(output.stdout, b"PRIVATE_GEMINI_OK\n");
}

#[test]
fn acp_model_source_gemini_rejects_configuration_conflicts_before_publication() {
    for key in [
        "GEMINI_API_KEY",
        "GEMINI_MODEL",
        "GEMINI_CLI_HOME",
        "GOOGLE_GEMINI_BASE_URL",
        "GOOGLE_GENAI_USE_VERTEXAI",
        "NODE_OPTIONS",
    ] {
        let config = AcpAgentConfig {
            command: "gemini".into(),
            args: vec!["--experimental-acp".into()],
            env: std::collections::HashMap::from([(key.into(), "conflict".into())]),
            ..Default::default()
        };
        let fixture = profile();
        let mut profile = AcpLaunchProfile::new(&config, fixture.binding.identity().clone()).unwrap();
        profile.model_api = fixture.model_api.clone();
        profile.model_supports_tool_call = fixture.model_supports_tool_call;
        let error = configure(&mut profile, "http://127.0.0.1:1234").unwrap_err().to_string();
        assert!(error.contains("conflicting Gemini model or authentication environment"));
        assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
    }
}

#[test]
fn model_protocol_013_gemini_startup_accepts_native_and_each_converted_protocol() {
    for (adapter, accepted) in [
        ("google", true),
        ("openai-compatible", true),
        ("anthropic", true),
        ("openai-responses", true),
        ("unknown-sdk", false),
    ] {
        let mut launch = profile();
        launch.model_api.as_mut().unwrap().adapter = adapter.into();
        let result = configure(&mut launch, "http://127.0.0.1:12345");
        assert_eq!(result.is_ok(), accepted, "{adapter}");
        assert_eq!(launch.directory().join("managed-settings.json").exists(), accepted);
        if accepted {
            assert_eq!(launch.config().env["GEMINI_MODEL"], "selected-model");
            assert_eq!(launch.config().env["GOOGLE_GEMINI_BASE_URL"], "http://127.0.0.1:12345");
        }
    }
}
