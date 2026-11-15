//! Trusted host bindings for the domain MCP diagnostic orchestrator.
use std::{path::{Path, PathBuf}, future::Future};
use axum::http::{HeaderMap, StatusCode};
use agent_app_services::mcp::probe_host::{ProbeHost, ProbeAccess, ProbeGrant};
use crate::gateway::agent_apps_diagnostic_jobs::{DiagnosticJobs, DiagnosticJobOwner, DiagnosticJobError, DiagnosticProbeError, MAX_PROBES_PER_JOB};
use crate::gateway::api::handlers::agent_apps::{McpProbeAccess, AuthorizedMcpProbe};
use crate::global_database::GlobalDatabase;
use vw_api_app_diagnostics::{DiagnosticScope, DiagnosticResult, DiagnosticProbeKind, DiagnosticJobIdentity, DiagnosticEvidenceSource};
use vw_api_mcp_management::{McpRecordId, McpCatalogScope, McpConnectionDefinition, McpCredentialRef, McpToolDetailsRequest};
use tokio_util::sync::CancellationToken;
pub type McpHandshakeStartError = agent_app_services::mcp::network_probe::McpHandshakeStartError<DiagnosticJobError>;
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
 type Runtime = super::connectivity::NativeConnectionCheckRuntime;
 fn runtime() -> Self::Runtime { super::connectivity::NativeConnectionCheckRuntime }
 fn invalid_input() -> Self::JobError { DiagnosticJobError::InvalidInput }
 fn cancelled() -> Self::ProbeError { DiagnosticProbeError::Cancelled }
 fn max_probes() -> usize { MAX_PROBES_PER_JOB }
 fn start<F, Fut>(jobs: &Self::Jobs, scope: DiagnosticScope, principal: String, kind: DiagnosticProbeKind, probes: Vec<F>) -> Result<DiagnosticJobIdentity, Self::JobError>
 where F: FnOnce(CancellationToken) -> Fut + Send + 'static,
       Fut: Future<Output = Result<DiagnosticResult, Self::ProbeError>> + Send + 'static {
     jobs.start(DiagnosticJobOwner { scope, principal }, kind, probes)
 }
 async fn open(server: agent_client_protocol::McpServer, timeout: std::time::Duration, http_io: CancellationToken, child: Option<&mut Option<mcp_client::ProbeChild>>, cleanup: Option<mcp_client::ConnectionCheckCleanup>) -> anyhow::Result<mcp_client::RunningMcpPeer> {
     crate::tools::mcp_common::open_connection_check_peer(server, timeout, http_io, child, cleanup).await
 }
 async fn tool_specs(peer: &mcp_client::RunningMcpPeer, id: &str) -> anyhow::Result<Vec<vw_api_tools::ToolSpecDto>> {
     let tools = crate::plugins::list_mcp_catalog_tools(peer.peer()).await?;
     Ok(crate::tools::mcp_tool::mcp_tools_to_specs(id, None, &tools, &[])?.into_iter().map(|spec| spec.to_dto()).collect())
 }
}

pub fn start_handshakes(
    jobs: &DiagnosticJobs,
    database: GlobalDatabase,
    policy: Option<PathBuf>,
    headers: &HeaderMap,
    scope: DiagnosticScope,
    ids: Vec<McpRecordId>,
) -> Result<DiagnosticJobIdentity, McpHandshakeStartError> {
    agent_app_services::mcp::network_probe::start_handshakes::<NativeProbeHost>(jobs, database, policy, headers, scope, ids)
}

pub(super) fn start_with_source(
    jobs: &DiagnosticJobs,
    database: GlobalDatabase,
    policy: Option<PathBuf>,
    headers: &HeaderMap,
    scope: DiagnosticScope,
    ids: Vec<McpRecordId>,
    source: DiagnosticEvidenceSource,
) -> Result<DiagnosticJobIdentity, McpHandshakeStartError> {
    agent_app_services::mcp::network_probe::start_with_source::<NativeProbeHost>(jobs, database, policy, headers, scope, ids, source)
}

pub fn start_connection_checks(
    jobs: &DiagnosticJobs,
    database: GlobalDatabase,
    policy: Option<PathBuf>,
    headers: &HeaderMap,
    scope: DiagnosticScope,
    ids: Vec<McpRecordId>,
    generation: u64,
) -> Result<DiagnosticJobIdentity, McpHandshakeStartError> {
    agent_app_services::mcp::network_probe::start_connection_checks::<NativeProbeHost>(jobs, database, policy, headers, scope, ids, generation)
}

pub fn start_details(
    jobs: &DiagnosticJobs,
    database: GlobalDatabase,
    policy: Option<PathBuf>,
    headers: &HeaderMap,
    scope: DiagnosticScope,
    request: McpToolDetailsRequest,
) -> Result<DiagnosticJobIdentity, McpHandshakeStartError> {
    agent_app_services::mcp::network_probe::start_details::<NativeProbeHost>(jobs, database, policy, headers, scope, request)
}
