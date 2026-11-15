//! Host composition preserves session approval, tenant scope and cancellation.
pub(super) struct Host;
impl gateway_mcp::server::host::McpHost for Host {
    async fn inspect(workspace: &std::path::Path) -> anyhow::Result<vw_api_mcp_inspect::McpInspectResponse> {
        crate::tools::mcp_inspect::inspect_mcp_servers_from_workspace(workspace).await
    }
    fn current_tenant() -> vw_api_transport::TenantScope { crate::session::ui_store::current_tenant_scope() }
    fn call(tenant: vw_api_transport::TenantScope, session: &str, server: &str, binding: &str, name: &str, arguments: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        crate::session::ui_store::with_tenant_scope_sync(tenant, || crate::plugins::mcp_app_execution::call(session, server, binding, name, arguments))
    }
}

#[cfg(target_os = "macos")]
fn probe_error(error: crate::agent_apps::mcp::network_probe::McpHandshakeStartError) -> gateway_mcp::server::host::ProbeStartError {
    use crate::agent_apps::mcp::network_probe::McpHandshakeStartError as E;
    use gateway_mcp::server::host::ProbeStartError as P;
    use crate::gateway::agent_apps_diagnostic_jobs::DiagnosticJobError as J;
    use axum::http::StatusCode as S;
    match error {
        E::Access(status) => P::Access(status),
        E::Record(code) => P::Record(code),
        E::Job(error) => P::Job(match error { J::CapacityExceeded => S::TOO_MANY_REQUESTS, J::Unavailable => S::SERVICE_UNAVAILABLE, J::NotFound => S::NOT_FOUND, J::InvalidInput => S::BAD_REQUEST }),
    }
}
#[cfg(target_os = "macos")]
impl gateway_mcp::server::host::ProbeHost for Host {
    type Jobs = crate::gateway::agent_apps_diagnostic_jobs::DiagnosticJobs;
    fn start_details(jobs: &Self::Jobs, database: crate::global_database::GlobalDatabase, policy: Option<std::path::PathBuf>, headers: &axum::http::HeaderMap, scope: vw_api_app_diagnostics::DiagnosticScope, request: vw_api_mcp_management::McpToolDetailsRequest) -> Result<vw_api_app_diagnostics::DiagnosticJobIdentity, gateway_mcp::server::host::ProbeStartError> {
        crate::agent_apps::mcp::network_probe::start_details(jobs, database, policy, headers, scope, request).map_err(probe_error)
    }
    fn start_checks(jobs: &Self::Jobs, database: crate::global_database::GlobalDatabase, policy: Option<std::path::PathBuf>, headers: &axum::http::HeaderMap, scope: vw_api_app_diagnostics::DiagnosticScope, ids: Vec<vw_api_mcp_management::McpRecordId>, generation: u64) -> Result<vw_api_app_diagnostics::DiagnosticJobIdentity, gateway_mcp::server::host::ProbeStartError> {
        crate::agent_apps::mcp::network_probe::start_connection_checks(jobs, database, policy, headers, scope, ids, generation).map_err(probe_error)
    }
    async fn read_mounts(target_host_id: String, project: &vw_api_transport::ProjectScope, session: &str) -> Result<vw_api_mcp_management::McpAcpUsage, axum::http::StatusCode> {
        use axum::http::StatusCode as S;
        let snapshot = crate::acp::read_catalog_mounts(project, session).await.map_err(|error| match error {
            acp_server::server::BackendError::NotFound(_) => S::NOT_FOUND,
            acp_server::server::BackendError::OwnershipMismatch(_) => S::FORBIDDEN,
            _ => S::SERVICE_UNAVAILABLE,
        })?;
        Ok(vw_api_mcp_management::McpAcpUsage {
            target_host_id, project: snapshot.project, session_id: snapshot.session_id.into(), mount_revision: snapshot.revision.0,
            mounts: snapshot.catalog.into_iter().map(|selection| vw_api_mcp_management::McpAcpUsageMount { scope: selection.mount.scope, record: selection.mount.record, failure: selection.failure }).collect(),
        })
    }
}
