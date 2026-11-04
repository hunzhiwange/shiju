use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

#[test]
fn acp_model_source_kilocode_creates_revocable_private_model_configuration() {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: "kilocode".into(),
            args: vec!["acp".into()],
            ..Default::default()
        },
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 1,
        },
    )
    .unwrap();
    configure(&mut profile, "http://127.0.0.1:1234").unwrap();
    let content = std::fs::read(&profile.config().env["KILO_CONFIG"]).unwrap();
    let config: serde_json::Value = serde_json::from_slice(&content).unwrap();
    assert_eq!(config["model"], config["small_model"]);
    assert_eq!(config["model"], config["subagent_model"]);
    let provider = config["model"].as_str().unwrap().split_once('/').unwrap().0;
    assert_eq!(config["provider"][provider]["models"]["session"]["id"], "selected-model");
    let token = profile.binding.bearer_token().to_owned();
    assert!(!profile.config().args.iter().any(|arg| arg.contains(&token)));
    let directory = profile.directory().to_owned();
    let binding = profile.binding.clone();
    drop(profile);
    assert!(!directory.exists());
    assert!(!binding.accepts(&token, &binding.identity));
}

fn configured_profile(config: AcpAgentConfig) -> AcpLaunchProfile {
    AcpLaunchProfile::new(
        &config,
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 1,
        },
    )
    .unwrap()
}

#[test]
fn acp_model_source_kilocode_rejects_conflicting_config_and_sensitive_arguments() {
    for config in [
        AcpAgentConfig {
            command: "kilocode".into(),
            args: vec!["acp".into(), "--model".into(), "local-model".into()],
            ..Default::default()
        },
        AcpAgentConfig { command: "npx".into(), args: vec!["acp".into()], ..Default::default() },
        AcpAgentConfig {
            command: "kilocode".into(),
            args: vec!["acp".into()],
            env: [("KILO_CONFIG_CONTENT".into(), "{}".into())].into(),
            ..Default::default()
        },
    ] {
        let mut profile = configured_profile(config);
        assert!(configure(&mut profile, "http://127.0.0.1:1234").is_err());
        assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
    }
    let mut profile = configured_profile(AcpAgentConfig {
        command: "kilocode".into(),
        args: vec!["acp".into()],
        ..Default::default()
    });
    assert!(configure(&mut profile, "https://example.com").is_err());
    assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
}

#[test]
fn acp_model_source_kilocode_switch_preserves_history_and_rotates_credentials() {
    let state = tempfile::tempdir().unwrap();
    let config = AcpAgentConfig {
        command: "kilocode".into(),
        args: vec!["acp".into()],
        ..Default::default()
    };
    let mut first = configured_profile(config.clone());
    first.state_root = Some(state.path().to_path_buf());
    configure(&mut first, "http://127.0.0.1:1234").unwrap();
    let mut second = configured_profile(config);
    second.model_alias = "switched-model".into();
    second.state_root = Some(state.path().to_path_buf());
    configure(&mut second, "http://127.0.0.1:5678").unwrap();
    assert_eq!(first.config().env["XDG_DATA_HOME"], second.config().env["XDG_DATA_HOME"]);
    assert_ne!(first.config().env["KILO_CONFIG"], second.config().env["KILO_CONFIG"]);
    assert_ne!(
        first.config().env["VW_KILO_RELAY_TOKEN"],
        second.config().env["VW_KILO_RELAY_TOKEN"]
    );
    let settings: serde_json::Value =
        serde_json::from_str(&second.config().env["KILO_CONFIG_CONTENT"]).unwrap();
    let provider = settings["model"].as_str().unwrap().split_once('/').unwrap().0;
    assert_eq!(settings["provider"][provider]["models"]["session"]["id"], "switched-model");
    let previous: serde_json::Value =
        serde_json::from_str(&first.config().env["KILO_CONFIG_CONTENT"]).unwrap();
    assert_eq!(settings["model"], previous["model"]);
}

#[tokio::test]
async fn acp_model_source_kilocode_subprocess_routes_primary_and_helper_to_private_http() {
    use axum::{Router, http::HeaderMap, routing::post};
    let config = AcpAgentConfig {
        command: "kilocode".into(),
        args: vec!["acp".into()],
        ..Default::default()
    };
    let mut profile = configured_profile(config);
    let binding = profile.binding.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new().route(
        "/v1/chat/completions",
        post(move |headers: HeaderMap, axum::Json(body): axum::Json<serde_json::Value>| {
            let binding = binding.clone();
            async move {
                let token =
                    headers["authorization"].to_str().unwrap().strip_prefix("Bearer ").unwrap();
                assert!(binding.accepts(token, &binding.identity));
                assert_eq!(body["model"], "selected-model");
                axum::Json(serde_json::json!({"accepted":true}))
            }
        }),
    );
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    configure(&mut profile, &endpoint).unwrap();
    // A protocol consumer exercises the generated file/environment boundary; the
    // separately recorded real CLI acceptance proves native interpretation.
    let child = tokio::process::Command::new("python3").args(["-c", r#"
import json, os, urllib.request
config = json.load(open(os.environ['KILO_CONFIG']))
for field in ('model', 'small_model', 'subagent_model', 'memory_model'):
    provider, alias = config[field].split('/')
    settings = config['provider'][provider]
    request = urllib.request.Request(settings['options']['baseURL'] + '/chat/completions',
        json.dumps({'model':settings['models'][alias]['id']}).encode(),
        {'Authorization':'Bearer ' + os.environ['VW_KILO_RELAY_TOKEN'], 'Content-Type':'application/json'})
    assert json.load(urllib.request.urlopen(request))['accepted']
print('four-model-routes-accepted')
"#]).env_clear().envs(&profile.config().env).output().await.unwrap();
    server.abort();
    assert!(child.status.success(), "private configuration consumer failed");
    assert_eq!(String::from_utf8(child.stdout).unwrap().trim(), "four-model-routes-accepted");
}

#[test]
fn acp_model_source_kilocode_preserves_managed_policy_authority() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("managed.json");
    assert!(reject_managed_configuration(&path).is_ok());
    std::fs::write(&path, b"{}").unwrap();
    assert!(reject_managed_configuration(&path).is_err());
    let directory = root.path().join("managed");
    std::fs::create_dir(&directory).unwrap();
    assert!(reject_managed_configuration(&directory).is_err());
    #[cfg(unix)]
    {
        let link = root.path().join("missing-policy");
        std::os::unix::fs::symlink(root.path().join("missing"), &link).unwrap();
        assert!(reject_managed_configuration(&link).is_err());
    }
}
