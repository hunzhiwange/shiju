//! Start, query and cancel the execution host's fixed loopback candidate scan.

use super::AppRoutingError;
use crate::GatewayClient;
use reqwest::Method;
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::{DiagnosticJobIdentity, DiagnosticJobSnapshot},
    app_routing::RoutingOutboundScanRequest,
    id::{ProjectId, TaskId},
};

impl GatewayClient {
    pub async fn app_routing_start_outbound_scan(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        token: &str,
    ) -> Result<DiagnosticJobIdentity, AppRoutingError> {
        let app = serde_json::to_value(app).map_err(|_| AppRoutingError::InvalidRequest)?;
        let app = app.as_str().ok_or(AppRoutingError::InvalidRequest)?;
        self.app_routing_request(
            Method::POST,
            &[host, "projects", project.as_ref(), "apps", app, "egress", "scans"],
            token,
            Some(&RoutingOutboundScanRequest {}),
        )
        .await
    }

    pub async fn app_routing_outbound_scan_status(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        task: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AppRoutingError> {
        self.app_routing_outbound_scan_request(Method::GET, host, project, app, task, token).await
    }

    pub async fn app_routing_cancel_outbound_scan(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        task: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AppRoutingError> {
        self.app_routing_outbound_scan_request(Method::DELETE, host, project, app, task, token)
            .await
    }

    async fn app_routing_outbound_scan_request(
        &self,
        method: Method,
        host: &str,
        project: &ProjectId,
        app: AppId,
        task: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AppRoutingError> {
        let app = serde_json::to_value(app).map_err(|_| AppRoutingError::InvalidRequest)?;
        let app = app.as_str().ok_or(AppRoutingError::InvalidRequest)?;
        self.app_routing_request(
            method,
            &[host, "projects", project.as_ref(), "apps", app, "egress", "scans", task.as_ref()],
            token,
            None::<&()>,
        )
        .await
    }
}
