//! Access to the host's single diagnostic registry after membership authorization.
use super::{ControlError, RoutingObservation};
use agent_gateway::agent_apps_diagnostic_jobs::DiagnosticJobs;
use axum::{Extension, http::HeaderMap};
use std::{future::Future, path::PathBuf};
use vw_api_app_identity::AppId;
use vw_api_transport::id::{ProjectId, TenantScope};

pub trait RoutingDiagnostics: RoutingObservation {
    fn authorize_member(
        control: Option<Extension<Self>>,
        headers: HeaderMap,
        tenant: TenantScope,
        host: String,
        app: Option<(ProjectId, AppId)>,
        manage: bool,
    ) -> impl Future<Output = Result<(Self, Option<PathBuf>, String), ControlError>> + Send;

    fn diagnostic_jobs(&self) -> Option<DiagnosticJobs>;
}

pub trait ProbeEgress: Send + Sync + 'static {
    fn probe_client(&self, target: &reqwest::Url, listener: Option<std::net::SocketAddr>)
        -> Result<Option<reqwest::Client>, agent_gateway::agent_apps_diagnostic_jobs::DiagnosticProbeError>;
}

pub trait RoutingProbe: RoutingDiagnostics {
    type Egress: ProbeEgress;
    fn configured_egress(&self, tenant: &TenantScope, host: &str) -> Option<Self::Egress>;
    fn bound_address(&self) -> Option<std::net::SocketAddr>;
}
