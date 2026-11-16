//! Real-host configuration acceptance scenarios; no AppState or lifecycle emulation.
use crate::host::{ConfigHost, HostFuture, InstanceQuery};
use agent_config::runtime as config;
use axum::{Json, extract::{Extension, Query, State}, http::HeaderMap};
use gateway_types::ApiError;
use serde_json::Value;
use std::path::PathBuf;
use vw_api_transport::id::TenantScope;

/// Exercise actual config handlers and storage while the caller supplies native instance effects.
pub async fn assert_manual_agent_save<H, S, L, D>(tenant: TenantScope, home: PathBuf, make_state: S, assert_live: L, dispose: D)
where
    H: ConfigHost,
    S: Fn() -> H::State + Clone + Send + 'static,
    L: Fn(String, String, bool) -> HostFuture<Result<(), ApiError>> + Clone + Send + 'static,
    D: Fn() -> HostFuture<()> + Clone + Send + 'static,
{
    for project_override in [false, true] {
        let project = home.join(if project_override { "project" } else { "global" });
        std::fs::create_dir_all(&project).unwrap();
        let directory = project.to_string_lossy().to_string();
        let scoped_directory = directory.clone();
        let scoped_tenant = tenant.clone();
        let first_live = assert_live.clone();
        let first_dispose = dispose.clone();
        let first_state = make_state.clone();
        H::with_instance(directory.clone(), move || {
                Box::pin(async move {
                    let state = first_state();
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
                            let Json(saved) = crate::handlers::config_patch_stateful::<H>(
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
                        let Json(saved) = crate::handlers::config_patch_stateful::<H>(
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
                        first_live(provider.into(), model.into(), false).await?;
                    }
                    first_dispose().await;
                    Ok(())
                })
            })
        .await
        .unwrap();

        // 从数据库创建新的实例，不能依赖上一实例或桌面草稿中的值。
        H::with_instance(directory.clone(), {
                let assert_live = assert_live.clone();
                let dispose = dispose.clone();
                move || {
                Box::pin(async move {
                    let runtime = config::get_current().await.unwrap();
                    assert_eq!(runtime.default_model.as_deref(), Some("after/saved-model"));
                    assert_live("after".into(), "saved-model".into(), true).await?;
                    dispose().await;
                    Ok(())
                })
                }
            })
        .await
        .unwrap();
        let store = config::SystemConfigStore::new(agent_database::GlobalDatabase::for_home(&home).unwrap()).unwrap();
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

/// Write the malformed plugin consumed by the actual host hook loader.
pub fn write_broken_hook_plugin(current: &config::Config) {
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

}
