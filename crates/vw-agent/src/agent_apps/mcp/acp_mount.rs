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

/// Read the reviewed project selection using the host-minted relay owner.
/// The returned check must guard discovery, approval, and dispatch.
#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
pub(crate) fn reviewed_project_stdio(
    launch: std::sync::Arc<crate::model_relay::RelayBinding>,
    requested: &[McpServer],
) -> anyhow::Result<(
    std::sync::Arc<agent_app_services::mcp::reviewed_stdio::ReviewedProjectStdio>,
    impl Fn() -> bool + Send + Sync + use<>,
)> {
    use agent_app_services::mcp::McpCatalogStore;
    use agent_database::run_sync_database;
    use std::{sync::Arc, time::Instant};

    let identity = launch.identity();
    let owner = vw_api_transport::ProjectScope::new(
        identity.tenant.clone(),
        identity.project.as_str().into(),
    )?;
    let load = |error| anyhow::anyhow!("reviewed project MCP selection: {error:?}");
    let database = GlobalDatabase::for_home(&crate::global::paths().home)?;
    let (store, review) = run_sync_database(|| {
        let store = McpCatalogStore::new(database).map_err(load)?;
        let review = store.reviewed_project_stdio(&owner, requested).map_err(load)?;
        Ok((store, Arc::new(review)))
    })?;
    let current = review.clone();
    let valid = move || {
        !launch.cancelled().is_cancelled()
            && launch.expires().is_none_or(|expires| Instant::now() < expires)
            && run_sync_database(|| current.is_current(&store, &owner).map_err(load))
                .unwrap_or(false)
    };
    anyhow::ensure!(valid(), "reviewed project MCP selection was revoked");
    Ok((review, valid))
}
