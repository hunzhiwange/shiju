//! Batch job registration adapter for the single execution-host registry.
pub use agent_app_services::mcp::probe_jobs::{McpProbeTarget, McpProbeBatch};
use super::McpCatalogStore;
use std::{future::Future, path::PathBuf, sync::Arc};
use crate::agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore};
use crate::gateway::agent_apps_diagnostic_jobs::{DiagnosticJobs, DiagnosticJobOwner, DiagnosticJobError, DiagnosticProbeError};
use vw_api_app_diagnostics::{DiagnosticJobIdentity, DiagnosticScope, DiagnosticProbeKind, DiagnosticResult};
use vw_api_mcp_management::{McpConnectionDefinition, McpCredentialRef};
use tokio_util::sync::CancellationToken;
use agent_app_services::mcp::probe_jobs_host::BatchHost;
struct NativeBatchHost;
impl BatchHost for NativeBatchHost {
 type Jobs = DiagnosticJobs;
 type Owner = DiagnosticJobOwner;
 type JobError = DiagnosticJobError;
 type ProbeError = DiagnosticProbeError;
 type Runtime = super::connectivity::NativeConnectionCheckRuntime;
 fn runtime() -> Self::Runtime { super::connectivity::NativeConnectionCheckRuntime }
 fn scope(owner: &Self::Owner) -> &DiagnosticScope { &owner.scope }
 fn invalid_input() -> Self::JobError { DiagnosticJobError::InvalidInput }
 fn cancelled() -> Self::ProbeError { DiagnosticProbeError::Cancelled }
 fn start<F, Fut>(jobs: &Self::Jobs, owner: Self::Owner, kind: DiagnosticProbeKind, probes: Vec<F>) -> Result<DiagnosticJobIdentity, Self::JobError>
 where F: FnOnce(CancellationToken) -> Fut + Send + 'static,
 Fut: Future<Output = Result<DiagnosticResult, Self::ProbeError>> + Send + 'static {
    jobs.start(owner, kind, probes)
 }
}
pub fn start_connection_checks(
    jobs: &DiagnosticJobs,
    owner: DiagnosticJobOwner,
    catalog: Arc<McpCatalogStore>,
    accounts: Arc<AuthAccountStore>,
    workspace: PathBuf,
    batch: McpProbeBatch,
    authorize_credential_use: impl for<'a> Fn(
        &'a DiagnosticJobOwner,
        &McpConnectionDefinition,
        &McpCredentialRef,
    ) -> Option<AuthAccountScope<'a>>
    + Send
    + Sync
    + 'static,
) -> Result<DiagnosticJobIdentity, DiagnosticJobError> {
 agent_app_services::mcp::probe_jobs::start_connection_checks::<NativeBatchHost>(jobs, owner, catalog, accounts, workspace, batch, authorize_credential_use)
}
