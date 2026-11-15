use crate::*;
use crate::test_support::*;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpListener;

#[tokio::test]
async fn settings_state_keeps_disabled_config_models() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "zz-test-provider": {
                "models": {
                    "alpha": {"status": "active"},
                    "beta": {"status": "disabled"}
                }
            }
        }),
    ))
    .await;

    let providers = list_for_settings().await;
    let provider = providers.get("zz-test-provider").expect("provider should exist");

    assert!(provider.models.contains_key("alpha"));
    assert!(provider.models.contains_key("beta"));
    assert_eq!(provider.models["beta"].status, "disabled");
}

#[tokio::test]
async fn provider_without_models_is_kept_for_settings_and_dropped_at_runtime() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "empty-provider": {
                "name": "Empty Provider"
            }
        }),
    ))
    .await;

    let settings = list_for_settings().await;
    let provider = settings.get("empty-provider").expect("settings should keep editable provider");
    assert!(provider.models.is_empty());
    assert!(!list().await.contains_key("empty-provider"));
}

#[tokio::test]
async fn filtered_state_keeps_only_active_models_and_merges_config_fields() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    let providers = list().await;
    let provider = providers.get("zz-test-provider").expect("configured provider should exist");
    let model = provider.models.get("alpha").expect("active model should exist");

    assert_eq!(provider.name, "Configured Provider");
    assert!(matches!(provider.source, ProviderSource::Config));
    assert_eq!(provider.options["timeout_seconds"], json!(30));
    assert!(!provider.models.contains_key("beta"));
    assert_eq!(model.api.id, "alpha-api");
    assert_eq!(model.api.url, "https://config.example/test");
    assert_eq!(model.api.adapter, "custom-adapter");
    assert_eq!(model.name, "Alpha Config");
    assert_eq!(model.family.as_deref(), Some("config-family"));
    assert_eq!(model.release_date, "2026-01-01");
    assert_eq!(model.headers["x-config"], "config");
    assert_eq!(model.options["configured"], json!(true));
    assert_eq!(model.limit.context, 32768);
    assert_eq!(model.limit.input, Some(16384));
    assert_eq!(model.limit.output, 8192);
    assert!(model.capabilities.toolcall);
    assert!(model.capabilities.input.text);
    assert_eq!(model.cost.input, 0.0);
}

#[tokio::test]
async fn env_source_does_not_expose_key_when_provider_declares_multiple_env_vars() {
    let env = ProviderTestEnv::new().await;
    env.set_env("SECOND_PROVIDER_KEY", "second-secret");
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "multi-env-provider": {
                "env": ["MISSING_PROVIDER_KEY", "SECOND_PROVIDER_KEY"],
                "models": {
                    "alpha": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    let provider = get_provider("multi-env-provider").await.expect("provider should resolve");

    assert!(matches!(provider.source, ProviderSource::Env));
    assert_eq!(provider.key, None);
}

#[tokio::test]
async fn env_source_sets_key_only_for_single_env_provider() {
    let env = ProviderTestEnv::new().await;
    env.set_env("TEST_PROVIDER_KEY", "env-secret");
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    let provider = get_provider("zz-test-provider").await.expect("provider should resolve");

    assert!(matches!(provider.source, ProviderSource::Env));
    assert_eq!(provider.key.as_deref(), Some("env-secret"));
}

#[tokio::test]
async fn remote_models_merge_preserves_explicit_status_and_static_metadata() {
    let env = ProviderTestEnv::new().await;
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
    let address = listener.local_addr().expect("test listener should have an address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("test server should accept a request");
        let mut request = [0_u8; 4096];
        let read = stream.read(&mut request).expect("request should be readable");
        let request = String::from_utf8_lossy(&request[..read]);
        assert!(request.starts_with("GET /v1/models HTTP/1.1\r\n"));
        assert!(
            request
                .lines()
                .any(|line| line.eq_ignore_ascii_case("authorization: Bearer resolver-secret"))
        );
        let body = r#"{"data":[{"id":"gpt-5"},{"id":"remote-resolver-model"}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).expect("response should be written");
    });
    env.set_env("TEST_PROVIDER_KEY", "resolver-secret");
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "openai": {
                "api": format!("http://{address}/v1/"),
                "env": ["TEST_PROVIDER_KEY"],
                "models": {
                    "gpt-5": {"status": "disabled"}
                }
            }
        }),
    ))
    .await;

    let providers = list_for_settings().await;
    let provider = providers.get("openai").expect("OpenAI provider should resolve");
    let explicit = provider.models.get("gpt-5").expect("explicit model should remain");
    let remote =
        provider.models.get("remote-resolver-model").expect("remote model should be merged");

    assert_eq!(explicit.status, "disabled");
    assert_eq!(explicit.name, "GPT-5");
    assert!(explicit.capabilities.reasoning);
    assert_eq!(remote.status, "active");
    assert_eq!(remote.api.url, format!("http://{address}/v1"));
    server.join().expect("test server should stop");
}

#[tokio::test]
async fn remote_models_http_failure_preserves_existing_models() {
    let env = ProviderTestEnv::new().await;
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
    let address = listener.local_addr().expect("test listener should have an address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("test server should accept a request");
        let mut request = [0_u8; 4096];
        let read = stream.read(&mut request).expect("request should be readable");
        assert!(String::from_utf8_lossy(&request[..read]).starts_with("GET /models HTTP/1.1\r\n"));
        let response =
            "HTTP/1.1 404 Not Found\r\nContent-Length: 7\r\nConnection: close\r\n\r\nmissing";
        stream.write_all(response.as_bytes()).expect("response should be written");
    });
    env.set_env("TEST_PROVIDER_KEY", "resolver-secret");
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "failure-provider": {
                "api": format!("http://{address}"),
                "env": ["TEST_PROVIDER_KEY"],
                "models": {
                    "configured-model": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    let providers = list().await;
    let provider =
        providers.get("failure-provider").expect("provider loading should degrade gracefully");

    assert!(provider.models.contains_key("configured-model"));
    assert_eq!(provider.models.len(), 1);
    server.join().expect("test server should stop");
}
