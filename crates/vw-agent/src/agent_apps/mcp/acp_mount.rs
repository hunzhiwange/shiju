//! ACP host context adapter for managed catalog mounts.
use std::collections::HashSet;
use acp_types::McpCatalogMount;
use agent_client_protocol::McpServer;
use acp_server::server::BackendError;
use crate::{global_database::GlobalDatabase, session::runtime::SessionScope};
#[derive(Default)]
pub(crate) struct ResolvedCatalogMounts {
    pub(crate) leases: Option<AcpMountLeases>,
    pub(crate) servers: Vec<McpServer>,
}
pub(crate) fn resolve_mounts(scope: &SessionScope, session: &str, project_id: &str, selected: &[McpCatalogMount], restoring: bool) -> Result<ResolvedCatalogMounts, BackendError> {
    let resolved = agent_app_services::mcp::acp_mount::resolve_mounts(scope, session, project_id, selected, restoring,
        || GlobalDatabase::for_home(&crate::global::paths().home))?;
    Ok(ResolvedCatalogMounts { leases: resolved.leases.map(AcpMountLeases), servers: resolved.servers })
}
pub(crate) struct AcpMountLeases(agent_app_services::mcp::acp_mount::AcpMountLeases);
impl std::ops::Deref for AcpMountLeases {
    type Target = agent_app_services::mcp::acp_mount::AcpMountLeases;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl AcpMountLeases {
    pub(crate) fn is_current(&self, name: &str) -> anyhow::Result<bool> {
        let context = crate::tools::current_tool_use_context().map(|context| (context.tenant_scope(), context.session().to_owned()));
        self.0.is_current(name, context)
    }
    pub(crate) fn revoked_names(&self) -> anyhow::Result<HashSet<&str>> { self.0.revoked_names() }
}
