use super::super::VtcodeApproval;
use acp_server::server::SessionBackend;
use agent_app_services::mcp::McpCatalogStore;
use agent_client_protocol::{McpServer, McpServerStdio};
use serde_json::json;
use std::{fs, path::Path, sync::Arc, time::Duration};
use vw_api_mcp_management::{McpCatalogScope, McpConnectionDefinition, McpLinuxStdioSelection};
use vw_api_transport::{ProjectScope, TenantScope};

use crate::{
    acp::VibeWindowBackend,
    agent_apps::mcp::acp_mount::{ReviewedStdioSource, reviewed_linux_stdio},
    global_database::GlobalDatabase,
    model_relay::{RelayBinding, RelayIdentity},
};

#[tokio::test]
async fn acp_platform_mcp_review_mount_revokes_already_issued_approval() {
    let Some(root) =
        std::env::var_os("VW_REVIEWED_MCP_MOUNT_TEST_ROOT").map(std::path::PathBuf::from)
    else {
        let fixture = tempfile::tempdir().unwrap();
        fs::create_dir(fixture.path().join("home")).unwrap();
        let name = format!(
            "{}::acp_platform_mcp_review_mount_revokes_already_issued_approval",
            module_path!().split_once("::").unwrap().1,
        );
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &name, "--nocapture"])
            .env("VW_REVIEWED_MCP_MOUNT_TEST_ROOT", fixture.path())
            .env("VW_MODEL_RECOVERY_TEST_ROOT", fixture.path().join("home"))
            .env("VIBEWINDOW_TEST_ROOT", fixture.path().join("home"))
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(fixture.path().join("verified").is_file(), "isolated case did not run");
        return;
    };
    let workspace = root.join("workspace");
    fs::create_dir(&workspace).unwrap();
    let tenant = TenantScope::default();
    let (project, _) = crate::project::from_directory(&tenant, &workspace).await.unwrap();
    let scope = McpCatalogScope::Project {
        scope: ProjectScope::new(tenant.clone(), project.id.as_str().into()).unwrap(),
    };
    let McpCatalogScope::Project { scope: owner } = &scope else { unreachable!() };
    let database = GlobalDatabase::for_home(&crate::global::paths().home).unwrap();
    let executable = Path::new("/usr/bin/true").canonicalize().unwrap();
    let connection = McpConnectionDefinition::Stdio {
        command: executable.to_str().unwrap().to_owned(),
        args: vec![],
        env: Default::default(),
        cwd: None,
    };
    let (store, record) = agent_database::run_sync_database(|| {
        crate::project::store::ProjectStore::new(database.clone())
            .unwrap()
            .create(&tenant, &project)
            .unwrap();
        let store = McpCatalogStore::new(database).unwrap();
        let record = store
            .create(
                &scope,
                vw_agent_apps::mcp::catalog::McpCatalogDraft {
                    id: "reviewed".into(),
                    name: None,
                    description: None,
                    homepage: None,
                    documentation: None,
                    tags: vec![],
                    connection: connection.clone(),
                },
            )
            .unwrap()
            .record;
        store
            .configure_linux_stdio(
                owner,
                &record,
                McpLinuxStdioSelection::Select {
                    sha256: "5a".repeat(32),
                    allowed_tools: vec!["read".into()],
                },
            )
            .unwrap();
        Ok((store, record))
    })
    .unwrap();

    let backend = VibeWindowBackend::new(tenant.clone());
    let session = backend.create_session(workspace.clone()).await.unwrap();
    let id = session.id.as_str();
    let mount = json!({"scope":scope,"record":record});
    let request = |rev, mounted, servers| {
        serde_json::from_value(json!({
            "sessionId":id, "expectedRevision":rev, "skills":[],
            "mcpServers":servers, "mcpCatalog":if mounted { vec![mount.clone()] } else { vec![] },
        }))
        .unwrap()
    };
    let server = McpServer::Stdio(McpServerStdio::new("reviewed", executable));
    let launch = RelayBinding::new(
        RelayIdentity {
            tenant: tenant.clone(),
            project: project.id,
            session: format!("{id}:acp-source-0:iflow:iflow"),
            provider: "fixture".into(),
            model: "fixture".into(),
            generation: 0,
        },
        Duration::from_secs(60),
    )
    .unwrap();
    let context = Arc::new(
        crate::tools::ToolUseContext::new(id, Some(workspace.display().to_string()))
            .with_channel("acp"),
    );
    let project_servers = [server.clone()];
    let project_view = crate::tools::scope_tool_use_context(
        context.clone(),
        reviewed_linux_stdio(launch.clone(), &workspace, &project_servers),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(project_view.source, ReviewedStdioSource::Project);

    let conflict = backend.set_session_mounts(request(0, true, json!([server]))).await.unwrap_err();
    assert!(conflict.to_string().contains("conflicts"));
    let mounted = backend.set_session_mounts(request(0, true, json!([]))).await.unwrap();
    assert_eq!(mounted.mcp_servers, ["reviewed"]);
    assert_eq!(
        backend.session_tool_count(id),
        Some(0),
        "reviewed service must not run in the raw pool"
    );
    assert!(!(project_view.current)());
    assert!(
        crate::tools::scope_tool_use_context(
            context.clone(),
            reviewed_linux_stdio(launch.clone(), &workspace, &project_servers)
        )
        .await
        .is_err()
    );

    let resolve = || {
        crate::tools::scope_tool_use_context(
            context.clone(),
            reviewed_linux_stdio(launch.clone(), &workspace, &[]),
        )
    };
    let first = resolve().await.unwrap().unwrap();
    assert_eq!(first.source, ReviewedStdioSource::Mounted);
    assert_eq!(first.review.sha256(), [0x5a; 32]);
    let approval = VtcodeApproval::new(true);
    let tool = "mcp__reviewed__read";
    let input = json!({});
    assert!(approval.issue(tool, "valid", &input, first.guard(Box::new(|| true))));
    assert!(approval.consume(tool, Some("valid"), &input));
    assert!(approval.issue(tool, "removed", &input, first.guard(Box::new(|| true))));
    backend.set_session_mounts(request(1, false, json!([]))).await.unwrap();
    assert!(!approval.consume(tool, Some("removed"), &input));
    assert!(resolve().await.unwrap().is_none());

    backend.set_session_mounts(request(2, true, json!([]))).await.unwrap();
    assert!(!(first.current)(), "the new revision must not reactivate the old lease");
    let next = resolve().await.unwrap().unwrap();
    assert!(approval.issue(tool, "stale", &input, next.guard(Box::new(|| true))));
    agent_database::run_sync_database(|| {
        store.update_connection(&scope, &record, connection).unwrap();
        Ok(())
    })
    .unwrap();
    assert!(!approval.consume(tool, Some("stale"), &input));
    fs::write(root.join("verified"), []).unwrap();
}
