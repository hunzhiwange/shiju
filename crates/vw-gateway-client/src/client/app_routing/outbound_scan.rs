//! Start, query and cancel the execution host's fixed loopback candidate scan.

use super::AppRoutingError;
use crate::GatewayClient;
use {
    vw_api_app_identity::AppId,
    vw_api_app_diagnostics::DiagnosticJobIdentity,
    vw_api_app_diagnostics::DiagnosticJobSnapshot,
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::TaskId,
};

impl GatewayClient {
    pub async fn app_routing_start_outbound_scan(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        token: &str,
    ) -> Result<DiagnosticJobIdentity, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_start_outbound_scan(host, project, app, token).await
    }

    pub async fn app_routing_outbound_scan_status(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        task: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_outbound_scan_status(host, project, app, task, token).await
    }

    pub async fn app_routing_cancel_outbound_scan(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        task: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AppRoutingError> {
        gateway_app_routing::GatewayAppRoutingClient::new(self.transport.clone())
            .app_routing_cancel_outbound_scan(host, project, app, task, token).await
    }
}
