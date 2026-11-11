use super::transport::credential;
use crate::{auth, provider::provider};
use axum::http::StatusCode;

#[test]
fn acp_model_source_messages_selects_the_official_alibaba_token_plan_endpoint() {
    let mut api = provider::ApiInfo {
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

#[test]
fn acp_model_source_messages_chat_disables_default_thinking_for_official_deepseek() {
    let request = serde_json::json!({"model":"deepseek-flash","messages":[]});
    for url in [
        "https://api.deepseek.com/v1/chat/completions",
        "https://api.deepseek.com/chat/completions",
    ] {
        let mut body = request.clone();
        super::transport::configure_messages_chat(&mut body, &url.parse().unwrap());
        assert_eq!(body["thinking"], serde_json::json!({"type":"disabled"}));
    }
    for url in [
        "https://api.deepseek.com.example.invalid/v1/chat/completions",
        "https://example.invalid/v1/chat/completions",
    ] {
        let mut body = request.clone();
        super::transport::configure_messages_chat(&mut body, &url.parse().unwrap());
        assert_eq!(body, request);
    }
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
