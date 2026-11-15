//! Diagnostic submission and polling; probes and credentials stay on the execution host.
use super::{AgentAppsSettingsError, GatewayClient};
use gateway_app_diagnostics::GatewayAppDiagnosticsClient;
use {
    vw_api_transport::TaskId,
    vw_api_app_diagnostics::DiagnosticEndpointCandidatesQuery,
    vw_api_app_diagnostics::DiagnosticEndpointCandidatesResponse,
    vw_api_app_diagnostics::DiagnosticJobIdentity,
    vw_api_app_diagnostics::DiagnosticJobSnapshot,
    vw_api_app_diagnostics::DiagnosticScopeQuery,
    vw_api_app_diagnostics::DiagnosticStartRequest,
};

impl GatewayClient {
    /// Read host-normalized candidates and their sources without probing or saving drafts.
    pub async fn diagnostic_endpoint_candidates(
        &self,
        scope: &DiagnosticScopeQuery,
        query: &DiagnosticEndpointCandidatesQuery,
        token: &str,
    ) -> Result<DiagnosticEndpointCandidatesResponse, AgentAppsSettingsError> {
        GatewayAppDiagnosticsClient::new(self.transport.clone())
            .diagnostic_endpoint_candidates(scope, query, token).await
    }

    /// Submit one probe batch. Acceptance identifies a job, not a successful probe.
    /// An ambiguous failure must not be retried automatically.
    pub async fn diagnostic_start(
        &self,
        scope: &DiagnosticScopeQuery,
        request: &DiagnosticStartRequest,
        token: &str,
    ) -> Result<DiagnosticJobIdentity, AgentAppsSettingsError> {
        GatewayAppDiagnosticsClient::new(self.transport.clone())
            .diagnostic_start(scope, request, token).await
    }

    /// Read the original task's progress and partial/final evidence in its authorized scope.
    pub async fn diagnostic_status(
        &self,
        scope: &DiagnosticScopeQuery,
        task_id: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AgentAppsSettingsError> {
        GatewayAppDiagnosticsClient::new(self.transport.clone())
            .diagnostic_status(scope, task_id, token).await
    }

    /// Request cleanup. Only `Cancelled` confirms cleanup has finished; keep polling
    /// a `Cancelling` snapshot. Terminal jobs retain their server-issued state.
    pub async fn diagnostic_cancel(
        &self,
        scope: &DiagnosticScopeQuery,
        task_id: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AgentAppsSettingsError> {
        GatewayAppDiagnosticsClient::new(self.transport.clone())
            .diagnostic_cancel(scope, task_id, token).await
    }
}
