//! Exact host authorization, job registration and transport operations for MCP diagnostics.
use std::{future::Future, path::{Path, PathBuf}, time::Duration};
use axum::http::{HeaderMap, StatusCode};
use agent_client_protocol::McpServer;
use tokio_util::sync::CancellationToken;
use vw_api_app_diagnostics::{DiagnosticScope, DiagnosticProbeKind, DiagnosticResult, DiagnosticJobIdentity};
use vw_api_mcp_management::{McpRecordId, McpCatalogScope, McpConnectionDefinition, McpCredentialRef};
use super::connection_runtime::ConnectionCheckRuntime;

pub trait ProbeGrant: Eq + Send + Sync + 'static {
    fn principal(&self) -> &str;
    fn credential_root(&self) -> Option<&Path>;
    fn catalog_scope(&self, scope: &DiagnosticScope) -> McpCatalogScope;
    fn connection_revision(&self) -> u64;
    fn stdio_workspace(&self) -> Option<&Path>;
    fn permits_stdio(&self, definition: &McpConnectionDefinition) -> bool;
    fn permits_endpoint(&self, url: &str) -> bool;
    fn permits_credential(&self, definition: &McpConnectionDefinition, reference: &McpCredentialRef) -> bool;
}
pub trait ProbeAccess: Send + Sync + 'static {
    type Grant: ProbeGrant;
    fn new(policy: Option<PathBuf>, headers: &HeaderMap) -> Result<Self, StatusCode> where Self: Sized;
    fn authorize(&self, scope: &DiagnosticScope, id: &McpRecordId) -> Result<Self::Grant, StatusCode>;
    fn authorize_details(&self, scope: &DiagnosticScope, id: &McpRecordId) -> Result<Self::Grant, StatusCode>;
}
pub trait ProbeHost: Send + Sync + 'static {
    type Access: ProbeAccess;
    type Jobs: Send + Sync;
    type JobError: Send + 'static;
    type ProbeError: Send + 'static;
    type Runtime: ConnectionCheckRuntime + Send + 'static;
    fn runtime() -> Self::Runtime;
    fn invalid_input() -> Self::JobError;
    fn cancelled() -> Self::ProbeError;
    fn max_probes() -> usize;
    fn start<F, Fut>(jobs: &Self::Jobs, scope: DiagnosticScope, principal: String, kind: DiagnosticProbeKind, probes: Vec<F>) -> Result<DiagnosticJobIdentity, Self::JobError>
    where F: FnOnce(CancellationToken) -> Fut + Send + 'static,
          Fut: Future<Output = Result<DiagnosticResult, Self::ProbeError>> + Send + 'static;
    fn open(server: McpServer, timeout: Duration, http_io: CancellationToken, child: Option<&mut Option<mcp_client::ProbeChild>>, cleanup: Option<mcp_client::ConnectionCheckCleanup>) -> impl Future<Output = anyhow::Result<mcp_client::RunningMcpPeer>> + Send;
    fn tool_specs(peer: &mcp_client::RunningMcpPeer, id: &str) -> impl Future<Output = anyhow::Result<Vec<vw_api_tools::ToolSpecDto>>> + Send;
}
