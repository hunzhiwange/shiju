use super::*;
use acp_server::server::SessionBackend;
use agent_client_protocol::McpServerStdio;
use serde_json::json;
use std::{fs, path::PathBuf, time::Duration};
use vw_api_mcp_management::{McpCatalogScope, McpConnectionDefinition, McpLinuxStdioSelection};
use vw_api_transport::TenantScope;

use crate::{acp::VibeWindowBackend, model_relay::RelayIdentity};

#[tokio::test]
async fn acp_platform_mcp_review_list_revokes_all_sources() {
    const ROOT_ENV: &str = "VW_REVIEWED_MCP_LIST_TEST_ROOT";
    let Some(root) = std::env::var_os(ROOT_ENV).map(PathBuf::from) else {
        let fixture = tempfile::tempdir().unwrap();
        let home = fixture.path().join("home");
        fs::create_dir(&home).unwrap();
        let name = format!(
            "{}::acp_platform_mcp_review_list_revokes_all_sources",
            module_path!().split_once("::").unwrap().1,
        );
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &name, "--nocapture"])
            .env(ROOT_ENV, fixture.path())
            .env("VW_MODEL_RECOVERY_TEST_ROOT", &home)
            .env("VIBEWINDOW_TEST_ROOT", &home)
            .output()
            .unwrap();
        assert!(
            output.status.success() && fixture.path().join("verified").is_file(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        return;
    };

    let workspace = root.join("workspace");
    fs::create_dir(&workspace).unwrap();
    let tenant = TenantScope::default();
    let (project, _) = crate::project::from_directory(&tenant, &workspace).await.unwrap();
    let owner = ProjectScope::new(tenant.clone(), project.id.as_str().into()).unwrap();
    let catalog = McpCatalogScope::Project { scope: owner.clone() };
    let database = GlobalDatabase::for_home(&crate::global::paths().home).unwrap();
    let executable = Path::new("/usr/bin/true").canonicalize().unwrap();
    let connection = McpConnectionDefinition::Stdio {
        command: executable.to_str().unwrap().to_owned(),
        args: vec![],
        env: Default::default(),
        cwd: None,
    };
    let approve = || McpLinuxStdioSelection::Select {
        sha256: "5a".repeat(32),
        allowed_tools: vec!["read".to_owned()],
    };
    let (store, [project_record, mounted_record]) = run_sync_database(|| {
        crate::project::store::ProjectStore::new(database.clone())
            .unwrap()
            .create(&tenant, &project)
            .unwrap();
        let store = McpCatalogStore::new(database).unwrap();
        let records = ["reviewed-project", "reviewed-mounted"].map(|name| {
            let record = store
                .create(
                    &catalog,
                    vw_agent_apps::mcp::catalog::McpCatalogDraft {
                        id: name.into(),
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
            store.configure_linux_stdio(&owner, &record, approve()).unwrap();
            record
        });
        Ok((store, records))
    })
    .unwrap();
    let backend = VibeWindowBackend::new(tenant.clone());
    let session = backend.create_session(workspace.clone()).await.unwrap();
    let id = session.id.as_str();
    let mount = |revision| {
        serde_json::from_value(json!({
            "sessionId":id, "expectedRevision":revision, "skills":[], "mcpServers":[],
            "mcpCatalog":[{"scope":catalog,"record":mounted_record}],
        }))
        .unwrap()
    };
    backend.set_session_mounts(mount(0)).await.unwrap();
    assert_eq!(backend.session_tool_count(id), Some(0));
    let launch = RelayBinding::new(
        RelayIdentity {
            tenant,
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
    let resolve = |servers: Vec<McpServer>| {
        let context = context.clone();
        let launch = launch.clone();
        let workspace = &workspace;
        async move {
            crate::tools::scope_tool_use_context(
                context,
                reviewed_linux_stdio_list(launch, workspace, &servers),
            )
            .await
        }
    };
    let project_server =
        McpServer::Stdio(McpServerStdio::new("reviewed-project", executable.clone()));
    let mounted_server = McpServer::Stdio(McpServerStdio::new("reviewed-mounted", executable));
    let candidates = [project_server.clone()];
    let conflict = resolve(vec![mounted_server]).await.err().unwrap();
    assert!(conflict.to_string().contains("conflicts"));
    assert!(resolve(vec![project_server.clone(), project_server.clone()]).await.is_err());
    let over_limit =
        resolve(vec![project_server; MAX_REVIEWED_STDIO_SERVICES + 1]).await.err().unwrap();
    assert!(over_limit.to_string().contains("Too many"));
    let legacy = crate::tools::scope_tool_use_context(
        context.clone(),
        reviewed_linux_stdio(launch.clone(), &workspace, &candidates),
    );
    assert!(legacy.await.is_err());

    let first = resolve(candidates.to_vec()).await.unwrap();
    assert_eq!(first.len(), 2);
    for service in &first {
        let (name, record) = match service.source {
            ReviewedStdioSource::Project => ("reviewed-project", &project_record),
            ReviewedStdioSource::Mounted => ("reviewed-mounted", &mounted_record),
        };
        assert_eq!(mcp_client::server_name(&service.server), name);
        assert_eq!(service.review.record(), record);
    }
    let guards = |services: &[ReviewedStdioMount]| {
        services.iter().map(|service| service.guard(Box::new(|| true))).collect::<Vec<_>>()
    };
    let first_guards = guards(&first);
    assert!(first_guards.iter().all(|current| current()));

    run_sync_database(|| {
        let updated =
            store.update_connection(&catalog, &project_record, connection.clone()).unwrap();
        store.configure_linux_stdio(&owner, &updated.record, approve()).unwrap();
        Ok(())
    })
    .unwrap();
    assert!(first_guards.iter().all(|current| !current()));

    let second = resolve(candidates.to_vec()).await.unwrap();
    let second_guards = guards(&second);
    assert_eq!(second_guards.len(), 2);
    assert!(second_guards.iter().all(|current| current()));
    backend.set_session_mounts(mount(1)).await.unwrap();
    assert!(second_guards.iter().all(|current| !current()));

    let third = resolve(candidates.to_vec()).await.unwrap();
    let third_guards = guards(&third);
    assert_eq!(third_guards.len(), 2);
    assert!(third_guards.iter().all(|current| current()));
    run_sync_database(|| {
        store.update_connection(&catalog, &mounted_record, connection).unwrap();
        Ok(())
    })
    .unwrap();
    assert!(third_guards.iter().all(|current| !current()));
    fs::write(root.join("verified"), []).unwrap();
}
