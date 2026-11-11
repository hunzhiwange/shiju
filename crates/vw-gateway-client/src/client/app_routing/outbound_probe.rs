//! Start, query and cancel a test of the execution host's configured proxy.

use super::AppRoutingError;
use crate::GatewayClient;
use reqwest::Method;
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::{DiagnosticJobIdentity, DiagnosticJobSnapshot},
    app_routing::RoutingOutboundProbeRequest,
    id::{ProjectId, TaskId},
};

impl GatewayClient {
    pub async fn app_routing_start_outbound_probe(
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
            &[host, "projects", project.as_ref(), "apps", app, "egress", "probes"],
            token,
            Some(&RoutingOutboundProbeRequest {}),
        )
        .await
    }

    pub async fn app_routing_outbound_probe_status(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        task: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AppRoutingError> {
        self.app_routing_outbound_probe_request(Method::GET, host, project, app, task, token).await
    }

    pub async fn app_routing_cancel_outbound_probe(
        &self,
        host: &str,
        project: &ProjectId,
        app: AppId,
        task: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AppRoutingError> {
        self.app_routing_outbound_probe_request(Method::DELETE, host, project, app, task, token)
            .await
    }

    async fn app_routing_outbound_probe_request(
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
            &[host, "projects", project.as_ref(), "apps", app, "egress", "probes", task.as_ref()],
            token,
            None::<&()>,
        )
        .await
    }
}
