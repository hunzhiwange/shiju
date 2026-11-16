use std::{fs, net::TcpListener};

use agent_client_protocol::McpServer;
use serde_json::json;
use vw_agent_apps::{
    mcp::import::NativeMcpSource,
    providers::live::commit::{ExecutionHostId, ProviderTransactions},
};
use {
    vw_api_transport::ProjectScope,
    vw_api_transport::TenantScope,
    vw_api_app_identity::AppId,
    vw_api_mcp_management::McpCatalogScope,
    vw_api_mcp_management::McpConnectionDefinition,
    vw_api_mcp_management::McpDistributionOutcome,
};

use super::{
    McpBindingTarget, McpCatalogStore,
    binding_tests::{installed, private_root},
    catalog_create_tests::http_draft,
};
use crate::{
    agent_apps::auth_accounts::AuthAccountStore,
    global_database::GlobalDatabase,
    project::instance,
    session::ui_store::with_tenant_scope_sync,
    tools::{
        ToolRuntimeContext, execute_tool_call,
        mcp_common::{load_resolved_acp_config_from, load_resolved_runtime_mcp_config_from},
        tool_specs_for_context,
    },
};

pub(super) use gateway_agent_apps::mcp_acceptance::project::register_project;

#[test]
fn cc_mcp_t27_selected_record_exposes_and_calls_the_real_tool() -> anyhow::Result<()> {
    let fixture = tempfile::tempdir()?;
    let workspace = fixture.path().canonicalize()?;
    fs::write(workspace.join(".vwacprc.json"), r#"{"mcpServers":[]}"#)?;
    let script = workspace.join("server.py");
    fs::write(
        &script,
        gateway_agent_apps::mcp_acceptance::servers::RUNTIME_SERVER,
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
            agent_app_services::mcp::acceptance::projection::selected_record(
                &catalog, &scope, &project, binding.record,
                || tool_specs_for_context(&context),
                |id, input| Ok(execute_tool_call(id, input, &context)?),
            )?;
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

    gateway_agent_apps::mcp_acceptance::runtime_scope::prepare_and_assert(
        &catalog, &scope, &tenant_catalog, &project, &other, &url,
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
