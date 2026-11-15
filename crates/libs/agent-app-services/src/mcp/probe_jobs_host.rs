//! The single host diagnostic registry used by managed MCP batches.
use std::future::Future;
use tokio_util::sync::CancellationToken;
use vw_api_app_diagnostics::{DiagnosticScope, DiagnosticProbeKind, DiagnosticResult, DiagnosticJobIdentity};
use super::connection_runtime::ConnectionCheckRuntime;
pub trait BatchHost: Send + Sync + 'static {
 type Jobs: Send + Sync;
 type Owner: Clone + Send + Sync + 'static;
 type JobError;
 type ProbeError: Send + 'static;
 type Runtime: ConnectionCheckRuntime + Send + 'static;
 fn runtime() -> Self::Runtime;
 fn scope(owner: &Self::Owner) -> &DiagnosticScope;
 fn invalid_input() -> Self::JobError;
 fn cancelled() -> Self::ProbeError;
 fn start<F, Fut>(jobs: &Self::Jobs, owner: Self::Owner, kind: DiagnosticProbeKind, probes: Vec<F>) -> Result<DiagnosticJobIdentity, Self::JobError>
 where F: FnOnce(CancellationToken) -> Fut + Send + 'static,
 Fut: Future<Output = Result<DiagnosticResult, Self::ProbeError>> + Send + 'static;
}
