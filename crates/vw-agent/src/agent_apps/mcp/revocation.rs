//! Host tenant and database selection for cached managed runtime leases.
use agent_client_protocol::McpServer;
use vw_api_transport::TenantScope;
use crate::global_database::GlobalDatabase;
pub(crate) fn runtime_selection_is_current(server: &McpServer, owner_tenant: Option<&TenantScope>) -> anyhow::Result<bool> {
    let tenant = crate::tools::current_tool_use_context()
        .map(|context| context.tenant_scope())
        .or_else(|| owner_tenant.cloned())
        .unwrap_or_else(crate::session::ui_store::current_tenant_scope);
    let database = || GlobalDatabase::for_home(&crate::global::paths().home);
    agent_app_services::mcp::revocation::runtime_selection_is_current(server, owner_tenant, tenant, database)
}
