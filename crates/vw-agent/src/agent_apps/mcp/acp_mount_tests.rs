use std::{fs, path::Path};

use serde_json::json;
use vw_acp::{
    McpCatalogMount, SessionMountRevision, SetSessionMountsRequest, server::SessionBackend,
};
use vw_api_types::{
    ProjectScope, TenantScope,
    mcp_management::{McpCatalogScope, McpConnectionDefinition, McpRuntimeSelection},
};

use super::{McpCatalogStore, catalog_create_tests::http_draft};
use crate::{acp::VibeWindowBackend, global_database::GlobalDatabase, tools::catalog::ToolCatalog};

fn add_server(
    store: &McpCatalogStore,
    scope: &McpCatalogScope,
    root: &Path,
    name: &str,
) -> McpCatalogMount {
    let script = root.join("server.py");
    fs::write(
        &script,
        r#"
import json, sys
from pathlib import Path

Path(sys.argv[1]).touch()
for line in sys.stdin:
    request = json.loads(line)
    if 'id' not in request:
        continue
    method = request['method']
    if method == 'initialize':
        result = {'protocolVersion': request['params']['protocolVersion'], 'capabilities': {'tools': {}}, 'serverInfo': {'name': 'fixture', 'version': '1'}}
    elif method == 'tools/list':
        result = {'tools': [{'name': 'read', 'description': 'Read fixture', 'inputSchema': {'type': 'object'}}]}
    elif method == 'tools/call':
        result = {'content': [{'type': 'text', 'text': 'from managed session'}]}
    else:
        result = {}
    print(json.dumps({'jsonrpc': '2.0', 'id': request['id'], 'result': result}), flush=True)
"#,
    )
    .unwrap();
    let mut draft = http_draft(name, "unused");
    draft.connection = McpConnectionDefinition::Stdio {
        command: "python3".into(),
        args: vec![script.display().to_string(), root.join(name).display().to_string()],
        env: Default::default(),
        cwd: None,
    };
    McpCatalogMount { scope: scope.clone(), record: store.create(scope, draft).unwrap().record }
}

#[tokio::test]
async fn cc_mcp_t29_explicit_catalog_selection_mounts_only_the_selected_server() {
    let Some(root) = std::env::var_os("VW_T29_FIXTURE").map(std::path::PathBuf::from) else {
        let fixture = tempfile::tempdir().unwrap();
        fs::create_dir(fixture.path().join("home")).unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "agent_apps::mcp::acp_mount_tests::cc_mcp_t29_explicit_catalog_selection_mounts_only_the_selected_server", "--nocapture"])
            .env("VW_T29_FIXTURE", fixture.path())
            .env("VW_MODEL_RECOVERY_TEST_ROOT", fixture.path().join("home"))
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
    let update = |revision, mounts: Vec<McpCatalogMount>| {
        serde_json::from_value::<SetSessionMountsRequest>(json!({
            "sessionId": id,
            "expectedRevision": revision,
            "skills": [],
            "mcpServers": [],
            "mcpCatalog": mounts
        }))
        .unwrap()
    };

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
    for rejected in [foreign, other_project] {
        let error = backend
            .set_session_mounts(update(0, vec![selected.clone(), rejected.clone()]))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("PermissionDenied"));
        assert!(!root.join(rejected.record.id.0).exists());
    }
    assert!(!root.join("selected").exists(), "reject the whole invalid selection");

    let response = backend.set_session_mounts(update(0, vec![selected.clone()])).await.unwrap();
    assert_eq!(
        response.mcp_servers,
        ["selected"],
        "the selected catalog revision must mount through the ACP backend"
    );
    assert_eq!(response.revision, SessionMountRevision(1));
    assert!(root.join("selected").exists());
    assert!(!root.join("native-runtime").exists());

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

    let live = ToolCatalog::default();
    live.replace(vec![tool.clone()], [tool.name().to_owned()]).unwrap();
    let pool = tools.mcp_connections().unwrap();
    let definition = catalog.connection_for_revision(&project_scope, &selected.record).unwrap();
    let current = catalog.update_connection(&project_scope, &selected.record, definition).unwrap();
    fs::remove_file(root.join("selected")).unwrap();
    assert!(tool.call(json!({})).await.is_err(), "a cached executor must reject the old revision");
    pool.refresh_tool_catalog(&live).await.unwrap();
    assert!(live.snapshot().tools().is_empty());
    pool.refresh_tool_catalog(&live).await.unwrap();
    assert!(live.snapshot().tools().is_empty());

    let restored_backend = VibeWindowBackend::new(tenant.clone());
    restored_backend.load_session(id, workspace.clone()).await.unwrap();
    assert_eq!(restored_backend.session_tool_count(id), Some(0));
    assert!(!root.join("selected").exists(), "recovery must not start a revoked revision");

    let error = restored_backend.set_session_mounts(update(1, vec![selected])).await.unwrap_err();
    assert!(error.to_string().contains("StaleRevision"));
    let current = McpCatalogMount { scope: project_scope, record: current.record };
    let response = restored_backend.set_session_mounts(update(1, vec![current])).await.unwrap();
    assert_eq!(response.mcp_servers, ["selected"]);
    let (_, replacement) = restored_backend.mounted_runtime(id);
    assert!(replacement.tools()[0].call(json!({})).await.unwrap().is_success());
    let cleared = restored_backend.set_session_mounts(update(2, vec![])).await.unwrap();
    assert!(cleared.mcp_servers.is_empty());
    assert!(replacement.tools()[0].call(json!({})).await.is_err());
    let after_clear = VibeWindowBackend::new(tenant);
    after_clear.load_session(id, workspace).await.unwrap();
    assert_eq!(after_clear.session_tool_count(id), Some(0));
}
