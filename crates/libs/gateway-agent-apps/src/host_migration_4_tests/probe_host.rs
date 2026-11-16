//! Trusted host bindings for the domain MCP diagnostic orchestrator.
use std::{path::{Path, PathBuf}, future::Future};
use axum::http::{HeaderMap, StatusCode};
use agent_app_services::mcp::probe_host::{ProbeHost, ProbeAccess, ProbeGrant};
use app_diagnostics_runtime::agent_apps_diagnostic_jobs::{DiagnosticJobs, DiagnosticJobOwner, DiagnosticJobError, DiagnosticProbeError, MAX_PROBES_PER_JOB};
use app_access_service::access::{McpProbeAccess, AuthorizedMcpProbe};
use vw_api_app_diagnostics::{DiagnosticScope, DiagnosticResult, DiagnosticProbeKind, DiagnosticJobIdentity};
use vw_api_mcp_management::{McpRecordId, McpCatalogScope, McpConnectionDefinition, McpCredentialRef};
use tokio_util::sync::CancellationToken;
pub(crate) struct NativeProbeHost;
pub(crate) struct NativeProbeAccess(McpProbeAccess);
#[derive(PartialEq, Eq)]
pub(crate) struct NativeProbeGrant(AuthorizedMcpProbe);
impl ProbeGrant for NativeProbeGrant {
 fn principal(&self) -> &str { &self.0.principal }
 fn credential_root(&self) -> Option<&Path> { self.0.credential_root.as_deref() }
 fn catalog_scope(&self, scope: &DiagnosticScope) -> McpCatalogScope { self.0.permit.catalog_scope(scope) }
 fn connection_revision(&self) -> u64 { self.0.permit.connection_revision }
 fn stdio_workspace(&self) -> Option<&Path> { self.0.permit.stdio_workspace.as_deref() }
 fn permits_stdio(&self, definition: &McpConnectionDefinition) -> bool { self.0.permit.permits_stdio(definition) }
 fn permits_endpoint(&self, url: &str) -> bool { self.0.permit.permits_endpoint(url) }
 fn permits_credential(&self, definition: &McpConnectionDefinition, reference: &McpCredentialRef) -> bool { self.0.permit.permits_credential(definition, reference) }
}
impl ProbeAccess for NativeProbeAccess {
 type Grant = NativeProbeGrant;
 fn new(policy: Option<PathBuf>, headers: &HeaderMap) -> Result<Self, StatusCode> { McpProbeAccess::new(policy, headers).map(Self) }
 fn authorize(&self, scope: &DiagnosticScope, id: &McpRecordId) -> Result<Self::Grant, StatusCode> { self.0.authorize(scope, id).map(NativeProbeGrant) }
 fn authorize_details(&self, scope: &DiagnosticScope, id: &McpRecordId) -> Result<Self::Grant, StatusCode> { self.0.authorize_details(scope, id).map(NativeProbeGrant) }
}
impl ProbeHost for NativeProbeHost {
 type Access = NativeProbeAccess;
 type Jobs = DiagnosticJobs;
 type JobError = DiagnosticJobError;
 type ProbeError = DiagnosticProbeError;
 type Runtime = NativeConnectionCheckRuntime;
 fn runtime() -> Self::Runtime { NativeConnectionCheckRuntime }
 fn invalid_input() -> Self::JobError { DiagnosticJobError::InvalidInput }
 fn cancelled() -> Self::ProbeError { DiagnosticProbeError::Cancelled }
 fn max_probes() -> usize { MAX_PROBES_PER_JOB }
 fn start<F, Fut>(jobs: &Self::Jobs, scope: DiagnosticScope, principal: String, kind: DiagnosticProbeKind, probes: Vec<F>) -> Result<DiagnosticJobIdentity, Self::JobError>
 where F: FnOnce(CancellationToken) -> Fut + Send + 'static,
       Fut: Future<Output = Result<DiagnosticResult, Self::ProbeError>> + Send + 'static {
     jobs.start(DiagnosticJobOwner { scope, principal }, kind, probes)
 }
 async fn open(server: agent_client_protocol::McpServer, timeout: std::time::Duration, http_io: CancellationToken, child: Option<&mut Option<mcp_client::ProbeChild>>, cleanup: Option<mcp_client::ConnectionCheckCleanup>) -> anyhow::Result<mcp_client::RunningMcpPeer> {
     mcp_client::open_connection_check_peer(server, timeout, http_io, child, cleanup).await
 }
 async fn tool_specs(peer: &mcp_client::RunningMcpPeer, id: &str) -> anyhow::Result<Vec<vw_api_tools::ToolSpecDto>> {
     let tools = plugin_mcp::mcp_catalog::list_tools(peer.peer()).await?;
     Ok(tool_mcp::mcp_tool::mcp_tools_to_specs(id, None, &tools, &[])?.into_iter().map(|spec| spec.to_dto()).collect())
 }
}

use agent_app_services::mcp::connection_runtime::{CheckPeer, ConnectionCheckRuntime};
use mcp_client::{RunningMcpPeer, open_connection_check_peer};
use mcp_client::ProbeChild;
pub(crate) struct NativeConnectionCheckRuntime;
pub(crate) struct NativeCheckPeer(RunningMcpPeer);
impl CheckPeer for NativeCheckPeer {
    fn has_tools(&self) -> bool {
        self.0.peer_info().is_some_and(|info| info.capabilities.tools.is_some())
    }
    async fn tool_count(&self) -> anyhow::Result<u32> {
        Ok(plugin_mcp::mcp_catalog::list_tools(self.0.peer()).await?.len() as u32)
    }
    async fn cancel(self) { let _ = RunningMcpPeer::cancel(self.0).await; }
}
impl ConnectionCheckRuntime for NativeConnectionCheckRuntime {
    type Peer = NativeCheckPeer;
    async fn open(
        &self,
        server: agent_client_protocol::McpServer,
        timeout: std::time::Duration,
        http_io: tokio_util::sync::CancellationToken,
        child: Option<&mut Option<ProbeChild>>,
    ) -> anyhow::Result<NativeCheckPeer> {
        open_connection_check_peer(server, timeout, http_io, child, None).await.map(NativeCheckPeer)
    }
}
