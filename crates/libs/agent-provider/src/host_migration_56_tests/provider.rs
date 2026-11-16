//! 验证供应商装配、模型限制与目录服务的宿主契约。

use crate::provider::*;
use agent_config::runtime as config;
use std::{collections::HashMap, sync::Arc};
use serde_json::Value;
use super::{ensure_host, global_async_env_lock};
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

#[tokio::test]
async fn agent_provider_without_models_is_kept_for_settings_and_dropped_at_runtime() {
    let _env_lock = global_async_env_lock().lock().await;
    ensure_host();
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

#[test]
fn agent_provider_state_includes_remote_models() {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    // env::all 使用启动快照，测试必须显式注入，避免依赖其他用例的初始化顺序。
    agent_env::provide_with(
        HashMap::from([("AGENT_REMOTE_MODELS_KEY".into(), "agent-secret".into())]),
        || {
            runtime.block_on(async {
                let _env_lock = global_async_env_lock().lock().await;
                ensure_host();
                let home = TempDir::new().expect("temp home should be created");
                let config_dir = home.path().join("config");
                let _home = EnvGuard::set("HOME", home.path().as_os_str());
                let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
                let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

                let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
                let address = listener.local_addr().expect("test listener should have an address");
                let server = std::thread::spawn(move || {
                    let (mut stream, _) =
                        listener.accept().expect("test server should accept a request");
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .expect("read timeout should be configured");
                    let mut request = [0_u8; 4096];
                    let read = stream.read(&mut request).expect("request should be readable");
                    let request = String::from_utf8_lossy(&request[..read]);
                    assert!(request.starts_with("GET /v1/models HTTP/1.1\r\n"));
                    assert!(request.lines().any(|line| {
                        line.eq_ignore_ascii_case("authorization: Bearer agent-secret")
                    }));
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
                let provider = state
                    .providers
                    .get("agent-remote-provider")
                    .expect("configured provider should resolve");

                assert!(provider.models.contains_key("configured-model"));
                assert!(provider.models.contains_key("agent-remote-model"));
                server.join().expect("test server should stop");
            })
        },
    );
}

#[tokio::test]
async fn agent_provider_new_model_inherits_unique_builtin_api() {
    let _env_lock = global_async_env_lock().lock().await;
    ensure_host();
    let home = TempDir::new().expect("temp home should be created");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let _provider_env = crate::models::get()
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
    let _env_lock = global_async_env_lock().lock().await;
    ensure_host();
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
    let expected = model_state::resolve_model_limit(None, None, None);

    assert_eq!(model.limit.context, expected.context);
    assert_eq!(model.limit.input, expected.input);
    assert_eq!(model.limit.output, expected.output);
    assert!(model.capabilities.toolcall);
    assert!(model.capabilities.input.text);
    assert!(model.capabilities.output.text);
}

#[tokio::test]
async fn agent_cross_layer_fixture_uses_the_same_default_limit() {
    let _env_lock = global_async_env_lock().lock().await;
    ensure_host();
    let home = TempDir::new().expect("temp home should be created");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../model-state/assets/provider/cross_layer_unknown_model.json"
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
    let expected = model_state::resolve_model_limit(None, None, None);
    assert_eq!(model.limit.context, expected.context);
    assert_eq!(model.limit.input, expected.input);
    assert_eq!(model.limit.output, expected.output);
}

#[tokio::test]
async fn agent_provider_explicit_model_capabilities_match_resolver_contract() {
    let _env_lock = global_async_env_lock().lock().await;
    ensure_host();
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
#[tokio::test]
async fn agent_custom_provider_reasoning_inherits_catalog_for_shiju_and_token_plan() {
    let _env_lock = global_async_env_lock().lock().await;
    ensure_host();
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    config::update(serde_json::json!({"providers": {
        "shiju": {"models": {"deepseek-v4.1-flash": {"status": "active"}}},
        "token-plan": {"api": "https://example.test/v1", "models": {
            "deepseek-v4.1-flash-1017": {"status": "active"},
            "alias": {"id": "qwen3.7-plus", "status": "active"},
            "glm-5.3": {"status": "active", "reasoning": false},
            "wan2.7-image": {"status": "active"}
        }}
    }}))
    .await
    .expect("save config");
    let state = load_state_impl(false).await;
    assert!(state.providers["shiju"].models["deepseek-v4.1-flash"].capabilities.reasoning);
    // 使用当前随包目录中的同提供商模型，严格验证覆盖配置保留各自的推理选项。
    for (id, expected) in [
        (
            "deepseek-v4.1-flash",
            serde_json::json!([
                {"type": "toggle"}, {"type": "effort", "values": ["low", "high", "max"]}
            ]),
        ),
        (
            "deepseek-v4-pro-0813",
            serde_json::json!([
                {"type": "toggle"}, {"type": "effort", "values": ["high", "max"]}
            ]),
        ),
        (
            "glm-5.3",
            serde_json::json!([
                {"type": "effort", "values": ["low", "high", "max"]}
            ]),
        ),
    ] {
        let model = &state.providers["shiju"].models[id];
        assert_eq!(
            serde_json::to_value(&model.capabilities.reasoning_options).unwrap(),
            expected,
            "{id}"
        );
    }

    let models = &state.providers["token-plan"].models;
    for id in ["deepseek-v4.1-flash-1017", "alias"] {
        assert!(models[id].capabilities.reasoning, "{id}");
        assert_eq!(models[id].api.url, "https://example.test/v1");
    }
    for id in ["glm-5.3", "wan2.7-image"] {
        assert!(!models[id].capabilities.reasoning, "{id}");
    }
}

#[tokio::test]
async fn provider_catalog_read_failure_preserves_config_error_and_recovers() {
    let _env_lock = global_async_env_lock().lock().await;
    ensure_host();
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    config::update(serde_json::json!({"providers": {"catalog-recovery": {
        "api": "https://example.test/v1", "models": {"original": {"status": "active"}}
    }}}))
    .await
    .unwrap();
    invalidate_cache().await;
    assert!(get_model("catalog-recovery", "original").await.is_ok());
    let unavailable = home.path().join("not-a-directory");
    std::fs::write(&unavailable, "storage unavailable").unwrap();
    {
        let _fault = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", unavailable.as_os_str());
        assert!(
            config::get_global_for_tenant(&vw_api_transport::TenantScope::default()).await.is_err()
        );
        let error = get_model("catalog-recovery", "original").await.unwrap_err();
        assert!(matches!(error, CatalogError::Configuration(_)), "{error}");
        assert!(std::error::Error::source(&error).is_some());
        assert!(matches!(list().await, Err(CatalogError::Configuration(_))));
        assert!(matches!(list_for_settings().await, Err(CatalogError::Configuration(_))));
        assert!(matches!(
            get_provider("catalog-recovery").await,
            Err(CatalogError::Configuration(_))
        ));
        assert!(matches!(
            get_small_model("catalog-recovery").await,
            Err(CatalogError::Configuration(_))
        ));
        invalidate_cache().await;
        assert!(matches!(
            get_model("catalog-recovery", "original").await,
            Err(CatalogError::Configuration(_))
        ));
    }
    assert!(get_model("catalog-recovery", "original").await.is_ok());
    invalidate_cache().await;
}

#[tokio::test]
async fn provider_catalog_interleaved_config_uses_one_snapshot() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let _env_lock = global_async_env_lock().lock().await;
    ensure_host();
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    config::update(serde_json::json!({"providers": {"catalog-interleaved": {
        "api": "https://example.test/v1", "models": {"original": {"status": "active"}}
    }}}))
    .await
    .unwrap();
    invalidate_cache().await;
    let entered = Arc::new(tokio::sync::Notify::new());
    let resume = Arc::new(tokio::sync::Notify::new());
    let first = Arc::new(AtomicBool::new(true));
    let hook = {
        let entered = entered.clone();
        let resume = resume.clone();
        Arc::new(move || -> agent_project::BoxFuture<()> {
            let entered = entered.clone();
            let resume = resume.clone();
            let first = first.swap(false, Ordering::SeqCst);
            Box::pin(async move {
                if first {
                    entered.notify_one();
                    resume.notified().await;
                }
            })
        }) as Arc<dyn Fn() -> agent_project::BoxFuture<()> + Send + Sync>
    };
    let reading = CONFIG_READ_BOUNDARY.scope(hook, get_model("catalog-interleaved", "original"));
    let updating = async {
        entered.notified().await;
        config::update(serde_json::json!({"providers": {"catalog-interleaved": {
            "models": {"original": null, "replacement": {"status": "active"}}
        }}}))
        .await
        .unwrap();
        resume.notify_one();
    };
    let (result, ()) = tokio::join!(reading, updating);
    assert!(result.is_ok(), "fingerprint snapshot was not used to build the catalog: {result:?}");
    assert!(get_model("catalog-interleaved", "original").await.is_err());
    assert!(get_model("catalog-interleaved", "replacement").await.is_ok());
    invalidate_cache().await;
}

#[tokio::test]
async fn provider_catalog_disable_and_delete_revoke_cached_models() {
    let _env_lock = global_async_env_lock().lock().await;
    ensure_host();
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    config::update(serde_json::json!({"providers": {"catalog-revocation": {
        "api": "https://example.test/v1", "models": {"original": {"status": "active"}}
    }}}))
    .await
    .unwrap();
    invalidate_cache().await;
    assert!(get_model("catalog-revocation", "original").await.is_ok());
    config::update(serde_json::json!({"providers": {"catalog-revocation": {
        "models": {"original": {"status": "disabled"}}
    }}}))
    .await
    .unwrap();
    assert!(get_model("catalog-revocation", "original").await.is_err());
    config::update(serde_json::json!({"providers": {"catalog-revocation": {
        "models": {"original": {"status": "active"}}
    }}}))
    .await
    .unwrap();
    assert!(get_model("catalog-revocation", "original").await.is_ok());
    config::update(serde_json::json!({"providers": {"catalog-revocation": null}})).await.unwrap();
    assert!(get_model("catalog-revocation", "original").await.is_err());
    invalidate_cache().await;
}
