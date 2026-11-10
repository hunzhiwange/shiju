use std::{fs, net::TcpListener, path::Path};

use agent_client_protocol::McpServer;
use serde_json::json;
use vw_agent_apps::{
    mcp::import::NativeMcpSource,
    providers::live::commit::{ExecutionHostId, ProviderTransactions},
};
use vw_api_types::{
    ProjectScope, TenantScope,
    agent_apps::AppId,
    agent_apps_auth::AuthService,
    mcp_management::{
        McpCatalogScope, McpConfigValue, McpConnectionDefinition, McpCredentialRef,
        McpDistributionOutcome, McpFailureCode, McpRuntimeSelection,
    },
};

use super::{
    McpBindingTarget, McpCatalogStore,
    binding_tests::{installed, private_root},
    catalog_create_tests::http_draft,
};
use crate::{
    agent_apps::auth_accounts::AuthAccountStore,
    global_database::GlobalDatabase,
    project::{Info, TimeInfo, instance, store::ProjectStore},
    session::ui_store::with_tenant_scope_sync,
    tools::{
        ToolRuntimeContext, execute_tool_call,
        mcp_common::{load_resolved_acp_config_from, load_resolved_runtime_mcp_config_from},
        tool_specs_for_context,
    },
};

pub(super) fn register_project(
    database: &GlobalDatabase,
    project: &ProjectScope,
    workspace: &Path,
) -> anyhow::Result<instance::Context> {
    let directory = workspace.display().to_string();
    let info = Info {
        id: project.project_id().as_ref().to_owned(),
        worktree: directory.clone(),
        vcs: None,
        name: None,
        icon: None,
        commands: None,
        time: TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    };
    ProjectStore::new(database.clone())?.create(project.tenant_scope(), &info)?;
    Ok(instance::Context { directory: directory.clone(), worktree: directory, project: info })
}

#[test]
fn cc_mcp_t27_selected_record_exposes_and_calls_the_real_tool() -> anyhow::Result<()> {
    let fixture = tempfile::tempdir()?;
    let workspace = fixture.path().canonicalize()?;
    fs::write(workspace.join(".vwacprc.json"), r#"{"mcpServers":[]}"#)?;
    let script = workspace.join("server.py");
    fs::write(
        &script,
        r#"
import json
import os
import sys

for line in sys.stdin:
    request = json.loads(line)
    if "id" not in request:
        continue
    method = request["method"]
    if method == "initialize":
        result = {
            "protocolVersion": request["params"]["protocolVersion"],
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "fixture", "version": "1"},
        }
    elif method == "tools/list":
        result = {"tools": [{"name": "echo", "inputSchema": {"type": "object"}, "annotations": {"readOnlyHint": True}}]}
    elif method == "tools/call":
        text = request["params"]["arguments"]["message"] + "|" + os.path.basename(os.getcwd())
        result = {"content": [{"type": "text", "text": text}]}
    else:
        result = {}
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
"#,
    )?;
    let database = GlobalDatabase::for_home(&crate::global::paths().home)?;
    let tenant = TenantScope::new("mcp-runtime-fixture", "company")?;
    let project = ProjectScope::new(tenant.clone(), "managed-project".into())?;
    let project_context = register_project(&database, &project, &workspace)?;
    let scope = McpCatalogScope::Tenant { scope: tenant.clone() };
    let catalog = McpCatalogStore::new(database.clone()).unwrap();
    let cwd = workspace.join("server-cwd");
    fs::create_dir(&cwd)?;
    let mut draft = http_draft("selected", "unused");
    draft.connection = McpConnectionDefinition::Stdio {
        command: "python3".into(),
        args: vec![script.display().to_string()],
        env: Default::default(),
        cwd: Some(cwd.display().to_string()),
    };
    let record = catalog.create(&scope, draft).unwrap().record;
    let home = tempfile::tempdir()?;
    let private = private_root()?;
    let transactions =
        ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let accounts = AuthAccountStore::new(database, private.path()).unwrap();
    let (installation, target) = installed(home.path(), AppId::Codex, "codex")?;
    let codex = home.path().join(".codex");
    fs::create_dir(&codex)?;
    fs::write(codex.join("config.toml"), "")?;
    let binding = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(catalog.set_app_binding(
            &scope,
            &record,
            &McpBindingTarget {
                target: &target,
                installation: &installation,
                source: NativeMcpSource::HostHome(home.path()),
                transactions: &transactions,
            },
            true,
            &accounts,
            |_, _| None,
        ))
        .unwrap();
    assert_eq!(binding.native_outcome, McpDistributionOutcome::Applied);

    with_tenant_scope_sync(tenant, || {
        instance::provide_context_sync(project_context, || -> anyhow::Result<()> {
            let context =
                ToolRuntimeContext::new("managed-runtime", Some(workspace.display().to_string()));
            assert!(
                tool_specs_for_context(&context)
                    .iter()
                    .all(|spec| !spec.id.starts_with("mcp__selected__")),
                "an active CLI binding must not enable the native runtime",
            );

            catalog
                .select_for_runtime(
                    &project,
                    &scope,
                    &McpRuntimeSelection { record: binding.record, project: project.clone() },
                )
                .unwrap();

            let specs = tool_specs_for_context(&context);
            let tool = specs
                .iter()
                .find(|spec| spec.id.starts_with("mcp__selected__"))
                .expect("the selected catalog record must expose the real MCP tool");
            let result = execute_tool_call(
                &tool.id,
                r#"{"message":"catalog-discovery-fixture"}"#,
                &context,
            )?;
            assert!(result.model_text().contains("catalog-discovery-fixture|server-cwd"));
            Ok(())
        })
    })
}

#[tokio::test]
async fn cc_mcp_t27_runtime_selection_keeps_project_and_acp_boundaries() -> anyhow::Result<()> {
    let fixture = tempfile::tempdir()?;
    let workspace = fixture.path().canonicalize()?;
    let rc = workspace.join(".vwacprc.json");
    fs::write(&rc, r#"{"mcpServers":[]}"#)?;
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    listener.set_nonblocking(true)?;
    let url = format!("http://{}/mcp", listener.local_addr()?);

    let database = GlobalDatabase::for_home(&crate::global::paths().home)?;
    let tenant = TenantScope::new("mcp-runtime-scope-fixture", "company")?;
    let project = ProjectScope::new(tenant.clone(), "runtime".into())?;
    let other = ProjectScope::new(tenant.clone(), "other".into())?;
    let context = register_project(&database, &project, &workspace)?;
    let scope = McpCatalogScope::Project { scope: project.clone() };
    let tenant_catalog = McpCatalogScope::Tenant { scope: tenant.clone() };
    let catalog = McpCatalogStore::new(database).unwrap();

    let http = catalog.create(&scope, http_draft("remote", &url)).unwrap().record;
    let mut sse = http_draft("events", &url);
    sse.connection = McpConnectionDefinition::Sse { url: url.clone(), headers: Default::default() };
    let sse = catalog.create(&scope, sse).unwrap().record;
    assert_eq!(
        catalog.select_for_runtime(
            &other,
            &scope,
            &McpRuntimeSelection { project: other.clone(), record: http.clone() },
        ),
        Err(McpFailureCode::PermissionDenied),
    );
    for record in [http, sse] {
        let selected = McpRuntimeSelection { project: project.clone(), record };
        catalog.select_for_runtime(&project, &scope, &selected).unwrap();
        catalog.select_for_runtime(&project, &scope, &selected).unwrap();
    }

    let mut private = http_draft("private", &url);
    if let McpConnectionDefinition::StreamableHttp { headers, .. } = &mut private.connection {
        headers.insert(
            "Authorization".into(),
            McpConfigValue::Credential {
                reference: McpCredentialRef {
                    service: AuthService::CodexOauth,
                    account_id: "fixture".into(),
                },
            },
        );
    }
    let private = catalog.create(&tenant_catalog, private).unwrap().record;
    assert_eq!(
        catalog.select_for_runtime(
            &project,
            &tenant_catalog,
            &McpRuntimeSelection { project: project.clone(), record: private },
        ),
        Err(McpFailureCode::PermissionDenied),
    );
    let same_name = catalog.create(&tenant_catalog, http_draft("remote", &url)).unwrap().record;
    assert_eq!(
        catalog.select_for_runtime(
            &project,
            &tenant_catalog,
            &McpRuntimeSelection { project: project.clone(), record: same_name },
        ),
        Err(McpFailureCode::Conflict),
    );

    crate::session::ui_store::with_tenant_scope(tenant, instance::provide_context(context, async {
        let resolved = load_resolved_runtime_mcp_config_from(&workspace).await?;
        assert_eq!(resolved.mcp_servers.len(), 2);
        assert!(resolved.mcp_servers.iter().any(|server| {
            matches!(server, McpServer::Http(server) if server.name == "remote" && server.url == url)
        }));
        assert!(resolved.mcp_servers.iter().any(|server| {
            matches!(server, McpServer::Sse(server) if server.name == "events" && server.url == url)
        }));
        assert!(load_resolved_acp_config_from(&workspace).await?.mcp_servers.is_empty());
        assert_eq!(listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);

        fs::write(&rc, json!({
            "mcpServers": [{"name": "remote", "command": "/usr/bin/false"}]
        }).to_string())?;
        let error = load_resolved_runtime_mcp_config_from(&workspace)
            .await.expect_err("selected name must not replace a project declaration");
        assert!(error.to_string().contains("conflicts with an existing declaration"));
        Ok(())
    })).await
}
