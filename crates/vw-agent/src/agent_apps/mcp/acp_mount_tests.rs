use std::fs;

use serde_json::json;
use acp_types::McpCatalogMount;
use acp_server::server::SessionBackend;
use {
    vw_api_transport::ProjectScope,
    vw_api_transport::TenantScope,
    vw_api_mcp_management::McpCatalogScope,
    vw_api_mcp_management::McpRuntimeSelection,
};

use super::McpCatalogStore;
use crate::{acp::VibeWindowBackend, global_database::GlobalDatabase};

use agent_app_services::mcp::acceptance::acp::add_server;

#[tokio::test]
async fn cc_mcp_t29_explicit_catalog_selection_mounts_only_the_selected_server() {
    let Some(root) = std::env::var_os("VW_T29_FIXTURE").map(std::path::PathBuf::from) else {
        let fixture = tempfile::tempdir().unwrap();
        fs::create_dir(fixture.path().join("home")).unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "agent_apps::mcp::acp_mount_tests::cc_mcp_t29_explicit_catalog_selection_mounts_only_the_selected_server", "--nocapture"])
            .env("VW_T29_FIXTURE", fixture.path())
            .env("VW_MODEL_RECOVERY_TEST_ROOT", fixture.path().join("home"))
            .env("VIBEWINDOW_TEST_ROOT", fixture.path().join("home"))
            .env("VW_ACP_MOUNT_RECOVERY_DB_ROOT", fixture.path().join("home"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    };
    let workspace = root.join("workspace");
    fs::create_dir(&workspace).unwrap();
    fs::write(workspace.join(".vwacprc.json"), r#"{"mcpServers":[]}"#).unwrap();
    let tenant = TenantScope::default();
    let (project_info, _) = crate::project::from_directory(&tenant, &workspace).await.unwrap();
    let project = ProjectScope::new(tenant.clone(), project_info.id.as_str().into()).unwrap();
    let catalog_scope = McpCatalogScope::Tenant { scope: tenant.clone() };
    let database = GlobalDatabase::for_home(&crate::global::paths().home).unwrap();
    crate::project::store::ProjectStore::new(database.clone())
        .unwrap()
        .create(&tenant, &project_info)
        .unwrap();
    let catalog = McpCatalogStore::new(database).unwrap();
    let project_scope = McpCatalogScope::Project { scope: project.clone() };
    let selected = add_server(&catalog, &project_scope, &root, "selected");
    let unselected = add_server(&catalog, &catalog_scope, &root, "native-runtime");
    catalog
        .select_for_runtime(
            &project,
            &catalog_scope,
            &McpRuntimeSelection { project: project.clone(), record: unselected.record },
        )
        .unwrap();
    let foreign_scope =
        McpCatalogScope::Tenant { scope: TenantScope::new("foreign", "company").unwrap() };
    let foreign = add_server(&catalog, &foreign_scope, &root, "foreign-tenant");
    let other_project_scope = McpCatalogScope::Project {
        scope: ProjectScope::new(tenant.clone(), "other-project".into()).unwrap(),
    };
    let other_project = add_server(&catalog, &other_project_scope, &root, "foreign-project");

    let backend = VibeWindowBackend::new(tenant.clone());
    let session = backend.create_session(workspace.clone()).await.unwrap();
    let id = &session.id;

    assert_eq!(backend.session_tool_count(id), Some(0));
    let context = crate::project::instance::Context {
        directory: workspace.display().to_string(),
        worktree: project_info.worktree.clone(),
        project: project_info,
    };
    let native = crate::project::instance::provide_context(context.clone(), async {
        crate::tools::mcp_common::load_resolved_runtime_mcp_config_from(&workspace).await
    })
    .await
    .unwrap();
    assert!(
        native
            .mcp_servers
            .iter()
            .any(|server| { crate::tools::mcp_common::server_name(server) == "native-runtime" })
    );
    let acp_context = std::sync::Arc::new(
        crate::tools::ToolUseContext::new(id, Some(workspace.display().to_string()))
            .with_channel("acp"),
    );
    let loaded = crate::project::instance::provide_context(context, async {
        crate::tools::scope_tool_use_context(acp_context, async {
            crate::tools::mcp_common::load_resolved_runtime_mcp_config_from(&workspace).await
        })
        .await
    })
    .await
    .unwrap();
    assert!(
        !loaded
            .mcp_servers
            .iter()
            .any(|server| { crate::tools::mcp_common::server_name(server) == "native-runtime" })
    );
    agent_app_services::mcp::acceptance::acp::select_catalog(
        &backend, id, &selected, [foreign, other_project], &root,
    ).await;

    let (_, tools) = backend.mounted_runtime(id);
    assert_eq!(tools.len(), 1);
    let tool = tools.tools()[0].clone();
    let foreign_context = std::sync::Arc::new(crate::tools::ToolUseContext::new(
        "other-session",
        Some(workspace.display().to_string()),
    ));
    assert!(
        crate::tools::scope_tool_use_context(foreign_context, tool.call(json!({}))).await.is_err()
    );
    let result = tool.call(json!({})).await.unwrap();
    assert!(result.is_success());
    assert!(result.model_text().contains("from managed session"));

    let restored_backend = VibeWindowBackend::new(tenant.clone());
    restored_backend.load_session(id, workspace.clone()).await.unwrap();
    assert_eq!(restored_backend.session_tool_count(id), Some(1));

    let pool = tools.mcp_connections().unwrap();
    let current = agent_app_services::mcp::acceptance::acp::revoke_cached(
        &catalog, &project_scope, &selected, &root, &tool, pool.as_ref(),
    ).await;

    let restored_backend = VibeWindowBackend::new(tenant.clone());
    restored_backend.load_session(id, workspace.clone()).await.unwrap();
    assert_eq!(restored_backend.session_tool_count(id), Some(0));
    assert!(!root.join("selected").exists(), "recovery must not start a revoked revision");

    agent_app_services::mcp::acceptance::acp::replace_and_clear(
        &restored_backend, id, selected, McpCatalogMount { scope: project_scope, record: current.record },
        || restored_backend.mounted_runtime(id).1.tools()[0].clone(),
    ).await;
    let after_clear = VibeWindowBackend::new(tenant);
    after_clear.load_session(id, workspace).await.unwrap();
    assert_eq!(after_clear.session_tool_count(id), Some(0));
}
