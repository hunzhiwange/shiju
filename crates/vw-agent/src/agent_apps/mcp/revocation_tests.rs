use std::{fs, path::Path, sync::Arc, time::Duration};

use {
    vw_api_transport::ProjectScope,
    vw_api_transport::TenantScope,
    vw_api_mcp_management::McpCatalogScope,
    vw_api_mcp_management::McpConnectionDefinition,
    vw_api_mcp_management::McpRuntimeSelection,
};

use super::{McpCatalogStore, catalog_create_tests::http_draft, runtime_tests::register_project};
use crate::{
    global_database::GlobalDatabase,
    project::instance,
    session::ui_store::with_tenant_scope_sync,
    tools::{Tool, ToolRuntimeContext, execution_tools_for_context},
};

fn connection(script: &Path, calls: &Path, label: &str) -> McpConnectionDefinition {
    McpConnectionDefinition::Stdio {
        command: "python3".into(),
        args: vec![script.display().to_string(), calls.display().to_string(), label.into()],
        env: Default::default(),
        cwd: None,
    }
}

#[test]
fn cc_mcp_t28_update_and_delete_revoke_cached_tools_without_interrupting_admitted_call()
-> anyhow::Result<()> {
    let fixture = tempfile::tempdir()?;
    let workspace = fixture.path().canonicalize()?;
    fs::write(workspace.join(".vwacprc.json"), r#"{"mcpServers":[]}"#)?;
    let script = workspace.join("server.py");
    let calls = workspace.join("calls");
    fs::write(
        &script,
        gateway_agent_apps::mcp_acceptance::servers::REVOCATION_SERVER,
    )?;

    let database = GlobalDatabase::for_home(&crate::global::paths().home)?;
    let tenant = TenantScope::new("mcp-runtime-revocation", "company")?;
    let project = ProjectScope::new(tenant.clone(), "managed-project".into())?;
    let project_context = register_project(&database, &project, &workspace)?;
    let scope = McpCatalogScope::Project { scope: project.clone() };
    let catalog = McpCatalogStore::new(database).unwrap();
    let mut draft = http_draft("managed", "unused");
    draft.connection = connection(&script, &calls, "first");
    let first = catalog.create(&scope, draft).unwrap().record;
    let select = |record| {
        catalog
            .select_for_runtime(
                &project,
                &scope,
                &McpRuntimeSelection { project: project.clone(), record },
            )
            .unwrap();
    };
    select(first.clone());
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;

    with_tenant_scope_sync(tenant, || {
        instance::provide_context_sync(project_context, || -> anyhow::Result<()> {
            let context =
                ToolRuntimeContext::new("revocation", Some(workspace.display().to_string()));
            let pool = crate::tools::mcp_connections_for_context(&context);
            let id = "mcp__managed__read";
            let discover = || -> Option<Arc<dyn Tool>> {
                execution_tools_for_context(&context)
                    .into_iter()
                    .find(|tool| tool.name() == id)
                    .map(Arc::from)
            };
            agent_app_services::mcp::acceptance::revocation::update_delete(
                &catalog, &scope, &project, &first, &script, &calls, &runtime, &pool, discover,
            )?;
            crate::plugins::close_session_mcp_connections("revocation", &workspace);
            Ok(())
        })
    })
}

#[test]
fn cc_mcp_t28_disable_revokes_loopback_tools_until_a_fresh_selection() -> anyhow::Result<()> {
    let fixture = tempfile::tempdir()?;
    let workspace = fixture.path().canonicalize()?;
    fs::write(workspace.join(".vwacprc.json"), r#"{"mcpServers":[]}"#)?;
    let script = workspace.join("server.py");
    fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t18/http_server.py"))?;
    let database = GlobalDatabase::for_home(&crate::global::paths().home)?;
    let tenant = TenantScope::new("mcp-runtime-disable", "company")?;
    let project = ProjectScope::new(tenant.clone(), "managed-project".into())?;
    let other = ProjectScope::new(tenant.clone(), "other-project".into())?;
    let project_context = register_project(&database, &project, &workspace)?;
    let scope = McpCatalogScope::Tenant { scope: tenant.clone() };
    let catalog = McpCatalogStore::new(database).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;

    with_tenant_scope_sync(tenant, || {
        instance::provide_context_sync(project_context, || -> anyhow::Result<()> {
            let context = ToolRuntimeContext::new("disable", Some(workspace.display().to_string()));
            let pool = crate::tools::mcp_connections_for_context(&context);
            for mode in ["http", "sse"] {
                let root = workspace.join(mode);
                fs::create_dir(&root)?;
                let (mut child, port) = runtime.block_on(async {
                    let child = tokio::process::Command::new("python3")
                        .arg(&script)
                        .arg(&root)
                        .arg(mode)
                        .kill_on_drop(true)
                        .spawn()?;
                    let port = tokio::time::timeout(Duration::from_secs(5), async {
                        loop {
                            if let Ok(value) = fs::read_to_string(root.join("port"))
                                && let Ok(port) = value.parse::<std::num::NonZeroU16>()
                            {
                                break port;
                            }
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                    })
                    .await?;
                    Ok::<_, anyhow::Error>((child, port))
                })?;
                let url = format!("http://127.0.0.1:{port}/mcp");
                let mut draft = http_draft(mode, &url);
                if mode == "sse" {
                    draft.connection =
                        McpConnectionDefinition::Sse { url, headers: Default::default() };
                }
                let record = catalog.create(&scope, draft).unwrap().record;
                let selection = McpRuntimeSelection { project: project.clone(), record };
                catalog.select_for_runtime(&project, &scope, &selection).unwrap();
                let id = format!("mcp__{mode}__pid");
                let discover = || -> Option<Arc<dyn Tool>> {
                    execution_tools_for_context(&context)
                        .into_iter()
                        .find(|tool| tool.name() == id)
                        .map(Arc::from)
                };
                agent_app_services::mcp::acceptance::revocation::disable_reselect(
                    &catalog, &scope, &project, &other, &selection, &root, &id, &runtime, &pool, discover,
                )?;
                runtime.block_on(child.kill())?;
            }
            crate::plugins::close_session_mcp_connections("disable", &workspace);
            Ok(())
        })
    })
}
