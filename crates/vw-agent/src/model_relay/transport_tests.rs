use super::transport::credential;
use crate::{auth, provider::provider};
use axum::http::StatusCode;

#[test]
fn acp_model_source_messages_selects_the_official_alibaba_token_plan_endpoint() {
    let mut api = provider::ApiInfo {
        protocols: None,
        id: "deepseek-v4.1-flash-1017".into(),
        url: "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1".into(),
        adapter: "openai-compatible".into(),
    };
    for host in
        ["token-plan.cn-beijing.maas.aliyuncs.com", "token-plan.ap-southeast-1.maas.aliyuncs.com"]
    {
        for path in ["/compatible-mode/v1", "/compatible-mode/v1/chat/completions/"] {
            api.url = format!("https://{host}{path}");
            assert_eq!(
                super::transport::official_alibaba_messages_url(&api).unwrap().as_str(),
                format!("https://{host}/apps/anthropic/v1/messages")
            );
        }
    }
    for url in [
        "https://token-plan.cn-beijing.maas.aliyuncs.com.example.invalid/compatible-mode/v1",
        "http://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1",
        "https://token-plan.cn-beijing.maas.aliyuncs.com:8443/compatible-mode/v1",
        "https://user@token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1",
        "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1?custom=1",
        "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1#custom",
        "https://token-plan.cn-beijing.maas.aliyuncs.com/custom/v1",
    ] {
        api.url = url.into();
        assert!(super::transport::official_alibaba_messages_url(&api).is_none());
    }
    api.url = "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1".into();
    api.adapter = "google".into();
    assert!(super::transport::official_alibaba_messages_url(&api).is_none());
}

#[test]
fn acp_model_source_messages_selects_the_official_deepseek_native_endpoint() {
    let mut api = provider::ApiInfo {
        protocols: None,
        id: "deepseek-flash".into(),
        url: "https://api.deepseek.com/v1/chat/completions".into(),
        adapter: "openai-compatible".into(),
    };
    let url = super::transport::official_deepseek_messages_url(&api).unwrap();
    assert_eq!(url.as_str(), "https://api.deepseek.com/anthropic/v1/messages");
    for url in [
        "https://api.deepseek.com.example.invalid/v1",
        "http://api.deepseek.com/v1",
        "https://api.deepseek.com/beta",
        "https://api.deepseek.com/v1?custom=1",
        "https://api.deepseek.com:8443/v1",
    ] {
        api.url = url.into();
        assert!(super::transport::official_deepseek_messages_url(&api).is_none());
    }
    api.url = "https://api.deepseek.com".into();
    api.id = "claude-opus-latest".into();
    assert!(super::transport::official_deepseek_messages_url(&api).is_none());
}

fn chat_model(provider_id: &str, id: &str, url: &str) -> provider::Model {
    provider::Model {
        id: id.into(),
        provider_id: provider_id.into(),
        api: provider::ApiInfo {
            protocols: None,
            id: id.into(),
            url: url.into(),
            adapter: "openai-compatible".into(),
        },
        name: id.into(),
        family: None,
        capabilities: provider::default_model_capabilities(),
        cost: provider::ModelCost {
            input: 0.0,
            output: 0.0,
            cache: provider::ModelCostCache { read: 0.0, write: 0.0 },
            experimental_over_200k: None,
        },
        limit: provider::ModelLimit { context: 4096, input: None, output: 1024 },
        status: "active".into(),
        options: Default::default(),
        headers: Default::default(),
        release_date: String::new(),
        variants: Default::default(),
    }
}

#[test]
fn acp_model_source_messages_chat_matches_the_client_thinking_mode() {
    use vw_shared::provider::reasoning::ReasoningOption;
    let request = serde_json::json!({"model":"selected","messages":[]});
    for (provider, id, url, field, expected) in [
        (
            "deepseek",
            "deepseek-flash",
            "https://api.deepseek.com/v1",
            "thinking",
            serde_json::json!({"type":"disabled"}),
        ),
        (
            "alibaba-token-plan-cn",
            "deepseek-v4.1-flash-1017",
            "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1",
            "enable_thinking",
            serde_json::json!(false),
        ),
        (
            "custom",
            "gpt-5.4",
            "https://example.invalid/v1",
            "reasoning_effort",
            serde_json::json!("none"),
        ),
    ] {
        let mut model = chat_model(provider, id, url);
        model.capabilities.reasoning = true;
        model.capabilities.reasoning_options = Some(vec![ReasoningOption::Toggle]);
        let mut body = request.clone();
        super::transport::configure_messages_chat(&mut body, &model);
        assert_eq!(body[field], expected);
    }
    let mut model = chat_model("custom", "custom-model", "https://example.invalid/v1");
    let mut body = request.clone();
    super::transport::configure_messages_chat(&mut body, &model);
    assert_eq!(body, request);
    model.capabilities.reasoning_options = Some(vec![ReasoningOption::Toggle]);
    super::transport::configure_messages_chat(&mut body, &model);
    assert_eq!(body, request);
    model.api.id = "deepseek-v4.1-flash-1017".into();
    model.provider_id = "alibaba-token-plan-cn".into();
    model.capabilities.reasoning_options = Some(Vec::new());
    super::transport::configure_messages_chat(&mut body, &model);
    assert_eq!(body, request);
}

#[test]
fn acp_model_source_relay_chat_oauth_expiry_and_revocation_never_use_cached_keys() {
    let info = provider::Info {
        id: "relay-oauth-fixture".into(),
        name: "fixture".into(),
        source: provider::ProviderSource::Api,
        env: Vec::new(),
        key: Some("stale-api-secret".into()),
        options: Default::default(),
        models: Default::default(),
    };
    let oauth = |expires| {
        Some(auth::Info::Oauth(auth::OauthInfo {
            access: "fresh-access".into(),
            refresh: "refresh-not-a-bearer".into(),
            expires,
            account_id: None,
            enterprise_url: None,
        }))
    };
    assert_eq!(
        credential(&info, oauth(chrono::Utc::now().timestamp_millis() + 60_000)).unwrap(),
        "fresh-access"
    );
    assert_eq!(credential(&info, oauth(0)).unwrap_err(), StatusCode::UNAUTHORIZED);
    assert_eq!(credential(&info, None).unwrap_err(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        credential(
            &info,
            Some(auth::Info::Api(auth::ApiInfo { key: "rotated-api-secret".into() }))
        )
        .unwrap(),
        "rotated-api-secret"
    );
}
