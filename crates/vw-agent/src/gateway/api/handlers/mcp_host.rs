//! Host composition preserves session approval, tenant scope and cancellation.
pub(super) struct Host;
impl gateway_mcp::server::host::McpHost for Host {
    async fn inspect(
        workspace: &std::path::Path,
        directory: Option<&str>,
        server: Option<&str>,
    ) -> anyhow::Result<vw_api_mcp_inspect::McpInspectResponse> {
        use crate::tools::mcp_inspect;
        use std::path::Path;

        let mut workspace = workspace;
        if let Some(directory) = directory.filter(|directory| !directory.trim().is_empty()) {
            let requested = Path::new(directory);
            if requested != workspace {
                let projects = crate::project::list(&Self::current_tenant()).await?;
                anyhow::ensure!(
                    projects.iter().any(|project| {
                        Path::new(&project.worktree) == requested
                            || project.sandboxes.iter().any(|path| Path::new(path) == requested)
                    }),
                    "MCP inspection directory is not registered in this tenant"
                );
                workspace = requested;
            }
        }
        let mut response = if let Some(name) = server {
            let mut config = crate::tools::mcp_common::load_resolved_acp_config_from(workspace).await?;
            config.mcp_servers.retain(|server| mcp_client::server_name(server) == name);
            mcp_inspect::inspect_mcp_servers(&config).await
        } else {
            mcp_inspect::inspect_mcp_servers_from_workspace(workspace).await?
        };
        let workspace_directory = workspace.to_string_lossy();
        for server in &mut response.servers {
            server.workspace_directory = Some(workspace_directory.to_string());
        }
        Ok(response)
    }
    fn current_tenant() -> vw_api_transport::TenantScope { crate::session::ui_store::current_tenant_scope() }
    fn call(tenant: vw_api_transport::TenantScope, session: &str, server: &str, binding: &str, name: &str, arguments: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        crate::session::ui_store::with_tenant_scope_sync(tenant, || crate::plugins::mcp_app_execution::call(session, server, binding, name, arguments))
    }
}
