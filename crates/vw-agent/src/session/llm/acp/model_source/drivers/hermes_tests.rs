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
        protocols: None,
        id: "selected-model".into(),
        url: "https://example.invalid/v1".into(),
        adapter: "openai-compatible".into(),
    });
    profile.model_supports_temperature = true;
    profile.model_output_limit = 1024;
    profile
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
