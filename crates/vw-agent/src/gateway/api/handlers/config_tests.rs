// 覆盖配置 API 的认证、租户隔离、项目覆盖与运行时更新。

use super::*;
use crate::gateway::instance::with_instance;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use model_types::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::body::to_bytes;
use axum::extract::{Extension, Query};
use parking_lot::Mutex;
use std::ffi::{OsStr, OsString};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use vw_api_transport::id::TenantScope;

use crate::global_database::GlobalDatabase;

struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &OsStr) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, previous }
    }

    fn unset(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }
}

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("ok".to_string())
    }
}

fn state(config: config::Config, require_pairing: bool, tokens: &[String]) -> AppState {
    AppState {
        config: Arc::new(Mutex::new(config)),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(require_pairing, tokens)),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: Default::default(),
    }
}

fn instance_query(temp: &TempDir) -> Query<InstanceQuery> {
    Query(InstanceQuery { directory: Some(temp.path().to_string_lossy().to_string()) })
}

fn local_store(home: &std::path::Path) -> config::SystemConfigStore {
    config::SystemConfigStore::new(GlobalDatabase::for_home(home).expect("global database"))
        .expect("system config store")
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    serde_json::from_slice(&bytes).expect("response should be json")
}

// 请求同时覆盖全局、租户和显式项目目录，防止配置更新越过作用域边界。
#[test]
fn routers_build_for_stateless_and_stateful_config_routes() {
    let _: axum::Router<()> = router();
    let _ = stateful_router();
}

// 读取接口分别验证无状态和有状态入口，确保二者返回相同的有效配置形状。
#[tokio::test]
async fn config_providers_returns_defaults_for_available_models() {
    let temp = TempDir::new().expect("temp dir");
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    let Json(response) = config_providers(instance_query(&temp), HeaderMap::new())
        .await
        .expect("providers should load");

    for provider in response.providers.iter().filter(|provider| !provider.models.is_empty()) {
        assert!(response.default.contains_key(&provider.id));
    }
}

// 旧版配置端点继续执行配对认证，并在输出前统一遮蔽密钥。


#[tokio::test]
async fn handle_api_config_put_reports_config_save_failure_without_mutating_runtime_state() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let mut current = config::Config::default();
    current.config_path = std::path::PathBuf::new();
    current.default_provider = Some("anthropic".to_string());
    current.hooks.enabled = true;
    current.hooks.builtin.command_logger = true;
    current.hooks.plugins.enabled = false;

    let mut incoming = current.clone();
    incoming.default_provider = Some("openai".to_string());
    incoming.hooks.builtin.command_logger = false;
    let body = toml::to_string_pretty(&incoming).expect("config should serialize as toml");
    let mut state = state(current.clone(), false, &[]);
    let runtime = crate::app::agent::hooks::build_hook_runtime(&current).expect("hook runtime");
    let initial_runner = runtime.runner_snapshot().expect("initial runner");
    state.hook_runtime = Some(runtime.clone());
    crate::app::agent::gateway::chat::run_gateway_chat_with_tools(
        &state,
        "cache this session",
        "save-failure-session",
    )
    .await
    .expect("gateway chat");
    let cached_engine = state
        .session_query_engines
        .lock()
        .await
        .get(&crate::gateway::chat::session_query_engine_key("save-failure-session"))
        .cloned()
        .expect("cached engine");

    let response = handle_api_config_put(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        response_json(response).await["error"].as_str().unwrap().contains("Failed to save config")
    );
    assert_eq!(state.config.lock().default_provider.as_deref(), Some("anthropic"));
    let retained_runner = runtime.runner_snapshot().expect("runner retained after save failure");
    assert!(Arc::ptr_eq(&initial_runner, &retained_runner));
    let retained_engine = state
        .session_query_engines
        .lock()
        .await
        .get(&crate::gateway::chat::session_query_engine_key("save-failure-session"))
        .cloned()
        .expect("engine retained after save failure");
    assert!(Arc::ptr_eq(&cached_engine, &retained_engine));
}

// 成功更新配置后需同步重载派生运行时；任一步构建失败都必须保持旧状态。
#[tokio::test]
async fn config_put_reloads_hooks_and_invalidates_query_engines() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let _home = EnvGuard::set("HOME", temp.path().as_os_str());
    let mut current = config::Config::default();
    current.config_path = temp.path().join("vibewindow.json");
    current.hooks.enabled = true;
    current.hooks.builtin.command_logger = true;
    current.hooks.plugins.enabled = false;

    let mut state = state(current.clone(), false, &[]);
    let runtime = crate::app::agent::hooks::build_hook_runtime(&current).expect("hook runtime");
    let initial_runner = runtime.runner_snapshot().expect("initial runner");
    state.hook_runtime = Some(runtime.clone());
    crate::app::agent::gateway::chat::run_gateway_chat_with_tools(
        &state,
        "cache this session",
        "reload-session",
    )
    .await
    .expect("gateway chat");
    assert!(
        state
            .session_query_engines
            .lock()
            .await
            .contains_key(&crate::gateway::chat::session_query_engine_key("reload-session"))
    );

    let mut incoming = current;
    incoming.hooks.builtin.command_logger = false;
    let body = toml::to_string_pretty(&incoming).expect("config should serialize as toml");
    let response = handle_api_config_put(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    let reloaded_runner = runtime.runner_snapshot().expect("reloaded runner");
    assert!(!Arc::ptr_eq(&initial_runner, &reloaded_runner));
    assert!(!state.config.lock().hooks.builtin.command_logger);
    assert!(state.session_query_engines.lock().await.is_empty());

    let audit_store = runtime.audit_store();
    reloaded_runner
        .fire_after_tool_call(
            "sleep",
            &crate::app::agent::tools::ToolResult {
                success: true,
                output: String::new(),
                error: None,
            },
            Duration::from_millis(1),
        )
        .await;
    assert!(audit_store.entries().is_empty());
}

#[tokio::test]
async fn config_put_hook_build_failure_is_atomic() {
    let temp = TempDir::new().expect("temp dir");
    let mut current = config::Config::default();
    current.workspace_dir = temp.path().join("workspace");
    current.config_path = temp.path().join("config/vibewindow.json");
    current.hooks.enabled = true;
    current.hooks.builtin.command_logger = true;
    current.hooks.plugins.enabled = false;

    let broken_plugin = current.workspace_dir.join(".vibewindow/plugins/broken");
    let manifest_path = broken_plugin.join(".vibewindow-plugin/plugin.json");
    std::fs::create_dir_all(manifest_path.parent().expect("manifest parent"))
        .expect("create manifest parent");
    std::fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "name": "broken",
            "version": "0.1.0",
            "description": "broken runtime test plugin",
            "hooks": "./hooks/hooks.json"
        }))
        .expect("manifest bytes"),
    )
    .expect("write plugin manifest");
    let hooks_path = broken_plugin.join("hooks/hooks.json");
    std::fs::create_dir_all(hooks_path.parent().expect("hooks parent"))
        .expect("create hooks parent");
    std::fs::write(&hooks_path, "{not json").expect("write broken hooks");

    let mut state = state(current.clone(), false, &[]);
    let runtime = crate::app::agent::hooks::build_hook_runtime(&current).expect("hook runtime");
    let initial_runner = runtime.runner_snapshot().expect("initial runner");
    state.hook_runtime = Some(runtime.clone());
    crate::app::agent::gateway::chat::run_gateway_chat_with_tools(
        &state,
        "cache this session",
        "build-failure-session",
    )
    .await
    .expect("gateway chat");
    let cached_engine = state
        .session_query_engines
        .lock()
        .await
        .get(&crate::gateway::chat::session_query_engine_key("build-failure-session"))
        .cloned()
        .expect("cached engine");

    let mut incoming = current.clone();
    incoming.hooks.plugins.enabled = true;
    let body = toml::to_string_pretty(&incoming).expect("config should serialize as toml");
    let response = handle_api_config_put(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .expect("error message")
            .contains("Failed to build hook runtime")
    );
    assert!(!state.config.lock().hooks.plugins.enabled);
    let retained_runner = runtime.runner_snapshot().expect("runner retained after build failure");
    assert!(Arc::ptr_eq(&initial_runner, &retained_runner));
    let retained_engine = state
        .session_query_engines
        .lock()
        .await
        .get(&crate::gateway::chat::session_query_engine_key("build-failure-session"))
        .cloned()
        .expect("engine retained after build failure");
    assert!(Arc::ptr_eq(&cached_engine, &retained_engine));
    assert!(!current.config_path.exists());
}


#[tokio::test]
async fn manual_agent_save_updates_live_resolution_and_survives_instance_restart() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().unwrap();
    let _root = EnvGuard::set("VW_MODEL_RECOVERY_TEST_ROOT", home.path().as_os_str());
    let _paths_root = EnvGuard::set("VIBEWINDOW_TEST_ROOT", home.path().as_os_str());
    let _config_dir =
        EnvGuard::set("VIBEWINDOW_CONFIG_DIR", home.path().join("config").as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let tenant = TenantScope::new("save-test", "company").unwrap();

    for project_override in [false, true] {
        let project = home.path().join(if project_override { "project" } else { "global" });
        std::fs::create_dir_all(&project).unwrap();
        let directory = project.to_string_lossy().to_string();
        let scoped_directory = directory.clone();
        let scoped_tenant = tenant.clone();
        crate::session::ui_store::with_tenant_scope(
            tenant.clone(),
            with_instance(directory.clone(), move || {
                Box::pin(async move {
                    let state = state(config::Config::default(), false, &[]);
                    // 两次保存之间不销毁实例，先读旧值使历史缓存问题可被复现。
                    for (provider, model) in [("before", "old-model"), ("after", "saved-model")] {
                        let model_ref = format!("{provider}/{model}");
                        let agents =
                            serde_json::json!({"main": {"provider": provider, "model": model}});
                        let mut patch = serde_json::json!({
                            "default_provider": provider,
                            "default_model": model_ref,
                        });
                        if project_override {
                            // Agent definitions are tenant-global even when editing project defaults.
                            let Json(saved) = config_patch_stateful(
                                Extension(scoped_tenant.clone()),
                                State(state.clone()),
                                Query(InstanceQuery { directory: None }),
                                HeaderMap::new(),
                                Json(serde_json::json!({"agents": agents})),
                            )
                            .await?;
                            assert_eq!(saved["agents"]["main"]["model"], model);
                        } else {
                            patch["agents"] = agents;
                        }
                        let Json(saved) = config_patch_stateful(
                            Extension(scoped_tenant.clone()),
                            State(state.clone()),
                            Query(InstanceQuery {
                                directory: project_override.then(|| scoped_directory.clone()),
                            }),
                            HeaderMap::new(),
                            Json(patch),
                        )
                        .await?;
                        assert_eq!(saved["agents"]["main"]["model"], model);
                        let runtime = config::get_current().await.unwrap();
                        assert_eq!(runtime.agents["main"].provider, provider);
                        assert_eq!(runtime.agents["main"].model, model);
                        let agent = crate::agent::get("main").await.expect("live main agent");
                        let resolved = crate::agent::resolve_model_ref(&agent).unwrap();
                        assert_eq!(resolved.provider_id, provider);
                        assert_eq!(resolved.model_id, model);
                        let default_model =
                            crate::provider::provider::default_model().await.unwrap();
                        assert_eq!(default_model.provider_id, provider);
                        assert_eq!(default_model.model_id, model);
                    }
                    crate::project::instance::dispose().await;
                    Ok(())
                })
            }),
        )
        .await
        .unwrap();

        // 从数据库创建新的实例，不能依赖上一实例或桌面草稿中的值。
        crate::session::ui_store::with_tenant_scope(
            tenant.clone(),
            with_instance(directory.clone(), || {
                Box::pin(async {
                    let runtime = config::get_current().await.unwrap();
                    assert_eq!(runtime.default_model.as_deref(), Some("after/saved-model"));
                    let agent = crate::agent::get("main").await.unwrap();
                    assert_eq!(agent.provider, "after");
                    assert_eq!(agent.model, "saved-model");
                    crate::project::instance::dispose().await;
                    Ok(())
                })
            }),
        )
        .await
        .unwrap();
        let store = local_store(home.path());
        if project_override {
            let payload = store.get_project(&tenant, &directory).unwrap().unwrap();
            let payload: Value = serde_json::from_str(&payload).unwrap();
            assert!(payload.get("agents").is_none());
            assert_eq!(payload["default_model"], "after/saved-model");
        }
        let payload = store.get(&tenant).unwrap().unwrap();
        let payload: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(payload["agents"]["main"]["model"], "saved-model");
    }
}

// The same production guard entrypoint serves the actual host and the domain tests.
#[tokio::test]
async fn config_host_enforces_pairing_before_parsing_put() {
    let token = "paired-token".to_string();
    let state = state(config::Config::default(), true, &[token]);
    assert!(<ConfigHost as gateway_config::host::ConfigHost>::require_auth(&state, &HeaderMap::new()).is_err());
    let response = handle_api_config_put(State(state), Extension(TenantScope::default()), HeaderMap::new(), "not = valid = toml".into()).await.into_response();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
