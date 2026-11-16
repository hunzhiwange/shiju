//! Negative launch assertions executed against the caller’s real host.
use super::AcceptanceHost;
use agent_config::schema::AcpAgentConfig;
use agent_provider::provider;

pub async fn claude_rejects_native_account<H: AcceptanceHost>() {
    let config = AcpAgentConfig { command: "claude-agent-acp".into(), ..Default::default() };
        let active = provider::get_model("claude-negative-test","selected-model").await.unwrap();
        // A persisted active snapshot must not resurrect a now-disabled catalog key.
        let mut disabled = active.clone();
        disabled.id = "disabled".into();
        assert!(H::prepare_launch("claude",&config,&disabled,"negative").await.is_err());
        let launch = H::prepare_launch("claude",&config,&active,"negative").await.unwrap();
        assert_eq!(launch.model_context_limit,12000);
        assert_eq!(launch.model_output_limit,1024);
        let response = model_transport::upstream_http_client().unwrap()
            .post(format!("{}/v1/messages",launch.config().env["ANTHROPIC_BASE_URL"]))
            .bearer_auth(launch.binding.bearer_token())
            .json(&serde_json::json!({"model":"selected-model","max_tokens":16,"messages":[{"role":"user","content":"hello"}]}))
            .send().await.unwrap();
        assert_eq!(response.status(),reqwest::StatusCode::UNAUTHORIZED);

}

pub async fn kimi_rejects_native_account<H: AcceptanceHost>() {
        let config = AcpAgentConfig { command:"kimi".into(), args:vec!["acp".into()], ..Default::default() };

        let active = provider::get_model("kimi-offline-no-account", "active-model").await.unwrap();
        let profile = H::prepare_launch("kimi", &config, &active, "missing-key").await.unwrap();
        let settings: toml::Value = toml::from_str(&std::fs::read_to_string(std::path::Path::new(&profile.config().env["KIMI_CODE_HOME"]).join("config.toml")).unwrap()).unwrap();
        let endpoint = settings["providers"]["session-provider"]["base_url"].as_str().unwrap();
        let response = model_transport::upstream_http_client().unwrap()
            .post(format!("{endpoint}/chat/completions"))
            .bearer_auth(profile.binding.bearer_token())
            .json(&serde_json::json!({"model":"active-model","messages":[{"role":"user","content":"hello"}]}))
            .send().await.unwrap();
        assert_eq!(response.status(), 401);
        // A stale caller snapshot cannot resurrect a catalog-disabled model.
        let mut disabled = active.clone();
        disabled.id = "disabled-model".into();
        let result = H::prepare_launch("kimi", &config, &disabled, "disabled").await;
        let error = match result {
            Ok(_) => panic!("disabled model must not create a Kimi launch"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("disabled-model"));

}

pub async fn hermes_rejects_native_account<H: AcceptanceHost>() {
    use crate::Error;
    use message_types as message;
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
        let result = crate::acp::request::do_stream_request_acp::<H>(
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


pub async fn codex_rejects_native_account<H: AcceptanceHost>(mut profile: crate::acp::model_source::profile::AcpLaunchProfile, active: provider::Model, config: AcpAgentConfig, relay_endpoint: String) {
        crate::acp::model_source::drivers::codex::configure(&mut profile, &relay_endpoint).unwrap();
        let native_config: serde_json::Value =
            serde_json::from_str(&profile.config().env["CODEX_CONFIG"]).unwrap();
        let endpoint = native_config["model_providers"]
            [native_config["model_provider"].as_str().unwrap()]["base_url"]
            .as_str()
            .unwrap();
        let response = model_transport::upstream_http_client()
            .unwrap()
            .post(format!("{endpoint}/responses"))
            .bearer_auth(profile.binding.bearer_token())
            .json(&serde_json::json!({"model":"active-model","input":"hello"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 401);
        assert!(
            provider::get_model("codex-offline-no-account", "disabled-model")
                .await
                .is_err()
        );
        // A previously active snapshot cannot resurrect a now-disabled catalog reference.
        let mut disabled = active.clone();
        disabled.id = "disabled-model".into();
        let result = H::prepare_launch(
            "codex", &config, &disabled, "disabled",
        )
        .await;
        assert!(result.is_err(), "disabled model must not create a Codex launch");
}

pub async fn goose_rejects_native_account<H: AcceptanceHost>(mut profile: crate::acp::model_source::profile::AcpLaunchProfile, active: provider::Model, config: AcpAgentConfig, relay_endpoint: String) {
        crate::acp::model_source::drivers::goose::configure(&mut profile, &relay_endpoint).unwrap();
        let endpoint = &profile.config().env["OPENAI_HOST"];
        let response = model_transport::upstream_http_client().unwrap()
            .post(format!("{endpoint}/v1/chat/completions"))
            .bearer_auth(profile.binding.bearer_token())
            .json(&serde_json::json!({"model":"active-model","messages":[{"role":"user","content":"hello"}]}))
            .send().await.unwrap();
        assert_eq!(response.status(), 401);
        // A stale caller snapshot cannot resurrect a catalog-disabled model.
        let mut disabled = active.clone();
        disabled.id = "disabled-model".into();
        let result = H::prepare_launch("goose", &config, &disabled, "disabled").await;
        let error = match result {
            Ok(_) => panic!("disabled model must not create a Goose launch"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("disabled-model"));
}

pub async fn pi_rejects_native_account<H: AcceptanceHost>(mut profile: crate::acp::model_source::profile::AcpLaunchProfile, active: provider::Model, config: AcpAgentConfig, endpoint: String) {
        crate::acp::model_source::drivers::pi::configure_native(&mut profile, endpoint.trim_end_matches("/v1")).unwrap();
        let response = model_transport::upstream_http_client().unwrap()
            .post(format!("{endpoint}/chat/completions"))
            .bearer_auth(profile.binding.bearer_token())
            .json(&serde_json::json!({"model":"active-model","messages":[{"role":"user","content":"hello"}]}))
            .send().await.unwrap();
        assert_eq!(response.status(), 401);
        // A stale caller snapshot cannot resurrect a catalog-disabled model.
        let mut disabled = active.clone();
        disabled.id = "disabled-model".into();
        let result = H::prepare_launch("pi", &config, &disabled, "disabled").await;
        let error = match result {
            Ok(_) => panic!("disabled model must not create a Pi launch"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("disabled-model"));
}

pub async fn opencode_rejects_native_account<H: AcceptanceHost>(mut profile: crate::acp::model_source::profile::AcpLaunchProfile, active: provider::Model, config: AcpAgentConfig, relay_endpoint: String) {
        crate::acp::model_source::drivers::opencode::configure(&mut profile, &relay_endpoint).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&profile.config().env["OPENCODE_CONFIG_CONTENT"]).unwrap();
        let endpoint = settings["provider"]["session-provider"]["options"]["baseURL"].as_str().unwrap();
        let response = model_transport::upstream_http_client().unwrap()
            .post(format!("{endpoint}/chat/completions"))
            .bearer_auth(profile.binding.bearer_token())
            .json(&serde_json::json!({"model":"active-model","messages":[{"role":"user","content":"hello"}]}))
            .send().await.unwrap();
        assert_eq!(response.status(), 401);
        let mut disabled = active.clone();
        disabled.id = "disabled-model".into();
        let result = H::prepare_launch("opencode", &config, &disabled, "disabled").await;
        let error = match result {
            Ok(_) => panic!("disabled model must not create an OpenCode launch"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("disabled-model"));
}
