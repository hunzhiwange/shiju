//! 验证供应商装配、模型限制与目录服务的宿主契约。

use super::*;
use std::ffi::{OsStr, OsString};
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
        "../../../libs/model-state/assets/provider/cross_layer_explicit_model.json"
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
async fn provider_catalog_same_path_tenants_stay_isolated_on_worker() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let project_root = home.path().join("project");
    std::fs::create_dir(&project_root).unwrap();
    for model in ["tenant-a", "tenant-b", "tenant-a"] {
        let tenant = vw_api_transport::TenantScope::new("catalog", model).unwrap();
        let (project, _) = crate::project::from_directory(&tenant, &project_root).await.unwrap();
        config::update_project_for_tenant(
            &tenant,
            &project_root,
            serde_json::json!({
                "providers": {"scoped": {"api": "https://example.test/v1",
                    "models": {model: {"status": "active"}}}}
            }),
        )
        .await
        .unwrap();
        let context = instance::Context {
            directory: project_root.to_string_lossy().into_owned(),
            worktree: project.worktree.clone(),
            project,
        };
        // 与 LLM 工作线程相同的显式 task-local 上下文恢复路径。
        let worker = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(
                crate::session::ui_store::with_tenant_scope(
                    tenant,
                    instance::provide_context(context, async move {
                        let resolved = get_model("scoped", model).await.unwrap();
                        assert_eq!(resolved.id, model);
                        let other = if model == "tenant-a" { "tenant-b" } else { "tenant-a" };
                        assert!(get_model("scoped", other).await.is_err());
                    }),
                ),
            );
        });
        worker.join().unwrap();
    }
}
