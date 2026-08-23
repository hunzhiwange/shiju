//! 验证供应商配置指纹、候选建议和模型解析的跨层契约。

use super::*;
use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;
use tempfile::TempDir;

struct EnvGuard {
    key: String,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &OsStr) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key: key.to_string(), previous }
    }

    fn unset(key: &'static str) -> Self {
        Self::unset_owned(key.to_string())
    }

    fn unset_owned(key: String) -> Self {
        let previous = std::env::var_os(&key);
        unsafe { std::env::remove_var(&key) };
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(&self.key, value) },
            None => unsafe { std::env::remove_var(&self.key) },
        }
    }
}

#[test]
fn config_fingerprint_tracks_provider_snapshot() {
    let base = config::Config::default();
    let mut changed = base.clone();
    changed.providers.insert("custom".to_string(), serde_json::json!({"models": {}}));

    assert_eq!(config_fingerprint(&base), config_fingerprint(&base));
    assert_ne!(config_fingerprint(&base), config_fingerprint(&changed));
}

#[test]
fn suggestions_prefer_close_candidates() {
    let suggestions = suggest("opnai", vec!["openai", "anthropic"]);
    assert!(suggestions.contains(&"openai".to_string()));
}

#[test]
// 指纹只跟踪供应商快照，避免无关配置变化触发实例重建。
fn config_fingerprint_ignores_unrelated_configuration() {
    let base = config::Config::default();
    let mut changed = base.clone();
    changed.default_model = Some("openai/gpt-test".to_string());

    assert_eq!(config_fingerprint(&base), config_fingerprint(&changed));
}

#[test]
fn suggestions_drop_distant_candidates() {
    let suggestions = suggest("openai", vec!["anthropic", "gemini", "local"]);

    assert!(suggestions.is_empty());
}

#[test]
fn suggestions_rank_exact_match_first_and_limit_results() {
    let suggestions =
        suggest("openai", vec!["openai-compatible", "openair", "openapi", "openai", "opena"]);

    assert_eq!(suggestions.len(), 3);
    assert_eq!(suggestions.first().map(String::as_str), Some("openai"));
}

#[test]
fn suggestions_handle_empty_inputs() {
    assert!(suggest("", vec![]).is_empty());
    assert!(suggest("provider", vec![]).is_empty());
}

#[tokio::test]
async fn agent_provider_without_models_is_kept_for_settings_and_dropped_at_runtime() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home should be created");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    config::update(serde_json::json!({
        "providers": {
            "empty-provider": {
                "name": "Empty Provider",
                "models": {}
            }
        }
    }))
    .await
    .expect("isolated provider config should be saved");

    let settings = load_state_impl(false).await;
    let provider =
        settings.providers.get("empty-provider").expect("settings should keep editable provider");
    assert!(provider.models.is_empty());
    assert!(!load_state_impl(true).await.providers.contains_key("empty-provider"));
}

#[tokio::test]
async fn agent_provider_state_includes_remote_models() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home should be created");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let _provider_env = crate::provider::models::get()
        .await
        .values()
        .flat_map(|provider| provider.env.iter().cloned())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .map(EnvGuard::unset_owned)
        .collect::<Vec<_>>();
    let _api_key = EnvGuard::set("AGENT_REMOTE_MODELS_KEY", OsStr::new("agent-secret"));

    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
    let address = listener.local_addr().expect("test listener should have an address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("test server should accept a request");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read timeout should be configured");
        let mut request = [0_u8; 4096];
        let read = stream.read(&mut request).expect("request should be readable");
        let request = String::from_utf8_lossy(&request[..read]);
        assert!(request.starts_with("GET /v1/models HTTP/1.1\r\n"));
        assert!(
            request
                .lines()
                .any(|line| line.eq_ignore_ascii_case("authorization: Bearer agent-secret"))
        );
        let body = r#"{"data":[{"id":"agent-remote-model"}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).expect("response should be written");
    });

    config::update(serde_json::json!({
        "providers": {
            "agent-remote-provider": {
                "api": format!("http://{address}/v1"),
                "env": ["AGENT_REMOTE_MODELS_KEY"],
                "models": {
                    "configured-model": {"status": "active"}
                }
            }
        }
    }))
    .await
    .expect("isolated provider config should be saved");

    let state = load_state_impl(false).await;
    let provider =
        state.providers.get("agent-remote-provider").expect("configured provider should resolve");

    assert!(provider.models.contains_key("configured-model"));
    assert!(provider.models.contains_key("agent-remote-model"));
    server.join().expect("test server should stop");
}

#[tokio::test]
async fn agent_provider_new_model_inherits_unique_builtin_api() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home should be created");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let _provider_env = crate::provider::models::get()
        .await
        .values()
        .flat_map(|provider| provider.env.iter().cloned())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .map(EnvGuard::unset_owned)
        .collect::<Vec<_>>();

    config::update(serde_json::json!({
        "providers": {
            "deepseek": {
                "models": {
                    "deepseek-future": {"status": "active"}
                }
            }
        }
    }))
    .await
    .expect("isolated provider config should be saved");

    let state = load_state_impl(false).await;
    let model = state.providers["deepseek"].models["deepseek-future"].clone();

    assert_eq!(model.api.url, "https://api.deepseek.com");
    assert_eq!(model.api.adapter, default_adapter());
}

#[tokio::test]
async fn agent_provider_unknown_model_uses_shared_limit_and_capability_defaults() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home should be created");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    config::update(serde_json::json!({
        "providers": {
            "agent-default-provider": {
                "models": {
                    "agent-unknown-model": {"status": "active"}
                }
            }
        }
    }))
    .await
    .expect("isolated provider config should be saved");

    let state = load_state_impl(false).await;
    let model = state.providers["agent-default-provider"].models["agent-unknown-model"].clone();
    let expected = vw_shared::provider::state::resolve_model_limit(None, None, None);

    assert_eq!(model.limit.context, expected.context);
    assert_eq!(model.limit.input, expected.input);
    assert_eq!(model.limit.output, expected.output);
    assert!(model.capabilities.toolcall);
    assert!(model.capabilities.input.text);
    assert!(model.capabilities.output.text);
}

#[tokio::test]
async fn agent_cross_layer_fixture_uses_the_same_default_limit() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home should be created");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../vw-shared/assets/provider/cross_layer_unknown_model.json"
    ))
    .expect("cross-layer fixture should parse");
    let provider_id = fixture["provider_id"].as_str().unwrap();
    let model_id = fixture["model_id"].as_str().unwrap();

    let mut provider = serde_json::json!({
        "api": fixture["base_url"],
        "models": {model_id: {"name": fixture["model_name"], "status": "active"}}
    });
    let mut providers = serde_json::Map::new();
    providers.insert(provider_id.to_string(), provider.take());
    config::update(serde_json::json!({"providers": providers}))
        .await
        .expect("isolated provider config should be saved");

    let state = load_state_impl(false).await;
    let model = &state.providers[provider_id].models[model_id];
    let expected = vw_shared::provider::state::resolve_model_limit(None, None, None);
    assert_eq!(model.limit.context, expected.context);
    assert_eq!(model.limit.input, expected.input);
    assert_eq!(model.limit.output, expected.output);
}

#[tokio::test(flavor = "multi_thread")]
// 跨层夹具锁定显式模型及元数据最终进入下一次供应商请求。
async fn agent_cross_layer_explicit_model_reaches_the_next_request_with_metadata() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home should be created");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../vw-shared/assets/provider/cross_layer_explicit_model.json"
    ))
    .expect("cross-layer fixture should parse");
    let provider_id = fixture["provider_id"].as_str().unwrap();
    let model_id = fixture["model_id"].as_str().unwrap();
    let model_config = serde_json::json!({
        "name": fixture["model_name"],
        "family": fixture["family"],
        "reasoning": fixture["reasoning"],
        "tool_call": fixture["tool_call"],
        "temperature": fixture["temperature"],
        "attachment": fixture["attachment"],
        "interleaved": fixture["interleaved"],
        "modalities": {
            "input": fixture["input_modalities"],
            "output": fixture["output_modalities"]
        },
        "limit": {
            "context": fixture["context_limit"],
            "input": fixture["input_limit"],
            "output": fixture["output_limit"]
        },
        "status": "active"
    });
    let mut models = serde_json::Map::new();
    models.insert(model_id.to_string(), model_config);
    let mut provider = serde_json::json!({
        "api": fixture["base_url"],
        "models": models
    });
    let mut providers = serde_json::Map::new();
    providers.insert(provider_id.to_string(), provider.take());
    config::update(serde_json::json!({"providers": providers}))
        .await
        .expect("isolated provider config should be saved");

    let state = load_state_impl(false).await;
    let model = &state.providers[provider_id].models[model_id];
    let request_limit = crate::session::llm::model_limit_for_request(
        Some(format!("{provider_id}/{model_id}")),
        &serde_json::json!({}),
    );
    assert_eq!(model.limit.context, request_limit.context);
    assert_eq!(model.limit.input, request_limit.input);
    assert_eq!(model.limit.output, request_limit.output);
    assert!(model.capabilities.reasoning);
    assert!(!model.capabilities.toolcall);
    assert!(model.capabilities.input.image);
}

#[tokio::test]
async fn agent_provider_explicit_model_capabilities_match_resolver_contract() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home should be created");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    config::update(serde_json::json!({
        "providers": {
            "agent-capability-provider": {
                "models": {
                    "custom-model": {
                        "status": "active",
                        "family": "custom-family",
                        "reasoning": true,
                        "tool_call": false,
                        "temperature": true,
                        "attachment": true,
                        "modalities": {
                            "input": ["text", "image"],
                            "output": ["text"]
                        },
                        "limit": {"context": 65536, "input": 60000, "output": 4096}
                    }
                }
            }
        }
    }))
    .await
    .expect("isolated provider config should be saved");

    let state = load_state_impl(false).await;
    let model = &state.providers["agent-capability-provider"].models["custom-model"];

    assert_eq!(model.family.as_deref(), Some("custom-family"));
    assert!(model.capabilities.reasoning);
    assert!(!model.capabilities.toolcall);
    assert!(model.capabilities.temperature);
    assert!(model.capabilities.attachment);
    assert!(model.capabilities.input.text);
    assert!(model.capabilities.input.image);
    assert!(model.capabilities.output.text);
    assert!(!model.capabilities.output.image);
    assert_eq!(model.limit.context, 65536);
    assert_eq!(model.limit.input, Some(60000));
    assert_eq!(model.limit.output, 4096);
}
