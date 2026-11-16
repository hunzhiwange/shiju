//! 请求头、鉴权优先级与可选参数的库内测试。

use std::collections::HashMap;

use serde_json::{Value, json};

use model_types as provider;
use crate::Error;
use message_types::AssistantError;
use agent_auth as auth;
use agent_env as env;
use tool_types as tools;

use super::do_stream_request_aisdk;

// These cases fail before stream execution; reaching host permissions or usage
// accounting would invalidate the library-only boundary under test.
struct UnreachableHost;

impl super::super::host::Host for UnreachableHost {
    fn begin_usage(&self, _: &str, _: &str) -> Option<agent_usage::native::NativeRequest> {
        panic!("request validation must finish before host usage");
    }

    fn authorize_model_request<'a>(
        &'a self,
        _: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<bool>> + Send + 'a>> {
        panic!("request validation must finish before host authorization");
    }
}

const API_KEY_EMPTY: &str = "API key 为空";
const OAUTH_ACCESS_TOKEN_EMPTY: &str = "OAuth access token 为空";

struct EnvGuard {
    key: &'static str,
    saved: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let saved = env::get(key);
        env::set(key, value);
        Self { key, saved }
    }

    fn remove(key: &'static str) -> Self {
        let saved = env::get(key);
        env::remove(key);
        Self { key, saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.saved {
            Some(value) => env::set(self.key, value.clone()),
            None => env::remove(self.key),
        }
    }
}

fn test_model(api_url: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": "gpt-test",
        "providerID": "openai",
        "api": {
            "id": "gpt-test",
            "url": api_url,
            "adapter": "openai-compatible"
        },
        "name": "GPT Test",
        "family": null,
        "capabilities": {
            "temperature": true,
            "reasoning": true,
            "attachment": false,
            "toolcall": true,
            "input": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "output": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "interleaved": false
        },
        "cost": {
            "input": 0.0,
            "output": 0.0,
            "cache": {
                "read": 0.0,
                "write": 0.0
            },
            "experimental_over_200k": null
        },
        "limit": {
            "context": 8192,
            "input": null,
            "output": 4096
        },
        "status": "active",
        "options": {},
        "headers": {},
        "release_date": "2026-01-01",
        "variants": {}
    }))
    .expect("test model should deserialize")
}

fn test_provider(key: Option<&str>, env_keys: Vec<&str>) -> provider::Info {
    provider::Info {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        source: provider::ProviderSource::Custom,
        env: env_keys.into_iter().map(ToString::to_string).collect(),
        key: key.map(ToString::to_string),
        options: HashMap::new(),
        models: HashMap::new(),
    }
}

fn user_messages() -> Value {
    json!([{ "role": "user", "content": "hello" }])
}

fn empty_tools() -> HashMap<String, tools::ToolSpec> {
    HashMap::new()
}

async fn call_request(
    provider_info: &provider::Info,
    auth_info: Option<&auth::Info>,
    headers: &HashMap<String, String>,
    merged_options: &Value,
) -> Result<(), Error> {
    let model = test_model("not a url");
    let messages = user_messages();
    let tools = empty_tools();
    let mut events = Vec::new();

    do_stream_request_aisdk(
        &UnreachableHost,
        provider_info,
        auth_info,
        headers,
        merged_options,
        &model,
        &messages,
        &tools,
        Some(0.7),
        Some(0.9),
        Some(1234),
        2,
        None,
        &mut |event| events.push(event),
    )
    .await
}

fn assert_provider_auth_error(result: Result<(), Error>, expected_message: &str) {
    match result {
        Err(Error::Api(AssistantError::ProviderAuthError { provider_id, message })) => {
            assert_eq!(provider_id, "openai");
            assert_eq!(message, expected_message);
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

fn assert_invalid_base_url(result: Result<(), Error>) {
    match result {
        Err(Error::Api(AssistantError::Unknown { message })) => {
            assert!(message.contains("Invalid base URL"), "message was {message:?}");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[tokio::test]
// 自定义请求头在认证和网络分发前拒绝，防止绕过受控头部策略。
async fn custom_headers_are_rejected_before_auth_or_dispatch() {
    let mut headers = HashMap::new();
    headers.insert("x-extra".to_string(), "1".to_string());

    let result = call_request(&test_provider(None, vec![]), None, &headers, &json!({})).await;

    match result {
        Err(Error::Api(AssistantError::Unknown { message })) => {
            assert!(message.contains("headers"));
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[tokio::test]
async fn user_agent_header_is_allowed_case_insensitively() {
    let headers = HashMap::from([("uSeR-aGeNt".to_string(), "vw-test".to_string())]);
    let provider = test_provider(Some("provider-token"), vec![]);

    let result = call_request(&provider, None, &headers, &json!({})).await;

    assert_invalid_base_url(result);
}

#[tokio::test]
async fn absent_optional_parameters_reach_dispatch_without_emitting_events_on_build_failure() {
    let provider = test_provider(Some("provider-token"), vec![]);
    let headers = HashMap::new();
    let model = test_model("not a url");
    let messages = user_messages();
    let tools = empty_tools();
    let mut events = Vec::new();

    let result = do_stream_request_aisdk(
        &UnreachableHost,
        &provider,
        None,
        &headers,
        &json!({}),
        &model,
        &messages,
        &tools,
        None,
        None,
        None,
        0,
        None,
        &mut |event| events.push(event),
    )
    .await;

    assert_invalid_base_url(result);
    assert!(events.is_empty());
}

#[tokio::test]
async fn missing_auth_defaults_to_api_key_empty_error() {
    let headers = HashMap::new();

    let result = call_request(&test_provider(None, vec![]), None, &headers, &json!({})).await;

    assert_provider_auth_error(result, API_KEY_EMPTY);
}

#[tokio::test]
async fn blank_provider_key_is_ignored_and_preserves_provider_id_in_error() {
    let headers = HashMap::new();
    let mut provider = test_provider(Some("  \t"), vec![]);
    provider.id = "blank-key-provider".to_string();

    let result = call_request(&provider, None, &headers, &json!({})).await;

    match result {
        Err(Error::Api(AssistantError::ProviderAuthError { provider_id, message })) => {
            assert_eq!(provider_id, "blank-key-provider");
            assert_eq!(message, API_KEY_EMPTY);
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[tokio::test]
async fn wellknown_auth_is_rejected_without_emitting_events() {
    let provider = test_provider(None, vec![]);
    let auth = auth::Info::Wellknown(auth::WellKnownInfo {
        key: "well-known-key".to_string(),
        token: "well-known-token".to_string(),
    });
    let headers = HashMap::new();
    let model = test_model("not a url");
    let messages = user_messages();
    let tools = empty_tools();
    let mut events = Vec::new();

    let result = do_stream_request_aisdk(
        &UnreachableHost,
        &provider,
        Some(&auth),
        &headers,
        &json!({}),
        &model,
        &messages,
        &tools,
        None,
        None,
        None,
        0,
        None,
        &mut |event| events.push(event),
    )
    .await;

    assert_provider_auth_error(result, API_KEY_EMPTY);
    assert!(events.is_empty());
}

#[tokio::test]
async fn empty_api_and_oauth_auth_report_specific_messages() {
    let headers = HashMap::new();
    let provider = test_provider(None, vec![]);

    let api = auth::Info::Api(auth::ApiInfo { key: "  ".to_string() });
    let oauth = auth::Info::Oauth(auth::OauthInfo {
        refresh: "refresh".to_string(),
        access: "  ".to_string(),
        expires: 0,
        account_id: None,
        enterprise_url: None,
    });

    assert_provider_auth_error(
        call_request(&provider, Some(&api), &headers, &json!({})).await,
        API_KEY_EMPTY,
    );
    assert_provider_auth_error(
        call_request(&provider, Some(&oauth), &headers, &json!({})).await,
        OAUTH_ACCESS_TOKEN_EMPTY,
    );
}

#[tokio::test]
async fn provider_key_takes_priority_over_empty_auth_info() {
    let headers = HashMap::from([("User-Agent".to_string(), "vw-test".to_string())]);
    let provider = test_provider(Some(" provider-token "), vec![]);
    let auth = auth::Info::Api(auth::ApiInfo { key: "  ".to_string() });

    let result =
        call_request(&provider, Some(&auth), &headers, &json!({ "z": true, "a": 1 })).await;

    assert_invalid_base_url(result);
}

#[tokio::test]
async fn provider_env_token_is_used_before_auth_info() {
    let key = "VW_AGENT_AISDK_REQUEST_TEST_TOKEN";
    let _guard = EnvGuard::set(key, " env-token ");

    let headers = HashMap::new();
    let provider = test_provider(None, vec![key]);
    let auth = auth::Info::Api(auth::ApiInfo { key: "auth-token".to_string() });
    let result = call_request(&provider, Some(&auth), &headers, &json!(null)).await;

    assert_invalid_base_url(result);
}

#[tokio::test]
// 空环境变量不应遮蔽后续有效认证来源。
async fn provider_env_skips_missing_and_blank_values_before_later_token() {
    let blank_key = "VW_AGENT_AISDK_REQUEST_TEST_BLANK_TOKEN";
    let token_key = "VW_AGENT_AISDK_REQUEST_TEST_LATER_TOKEN";
    let _blank_guard = EnvGuard::set(blank_key, "  \t");
    let _token_guard = EnvGuard::set(token_key, " later-token ");
    let missing_key = "VW_AGENT_AISDK_REQUEST_TEST_MISSING_TOKEN";
    let _missing_guard = EnvGuard::remove(missing_key);

    let headers = HashMap::new();
    let provider = test_provider(None, vec![missing_key, blank_key, token_key]);
    let result = call_request(&provider, None, &headers, &json!({})).await;

    assert_invalid_base_url(result);
}

#[tokio::test]
async fn api_and_oauth_auth_fallbacks_reach_dispatch_when_non_empty() {
    let headers = HashMap::new();
    let provider = test_provider(None, vec![]);
    let api = auth::Info::Api(auth::ApiInfo { key: " api-token ".to_string() });
    let oauth = auth::Info::Oauth(auth::OauthInfo {
        refresh: "refresh".to_string(),
        access: " oauth-token ".to_string(),
        expires: 0,
        account_id: Some("account".to_string()),
        enterprise_url: None,
    });

    assert_invalid_base_url(call_request(&provider, Some(&api), &headers, &json!({})).await);
    assert_invalid_base_url(call_request(&provider, Some(&oauth), &headers, &json!({})).await);
}
