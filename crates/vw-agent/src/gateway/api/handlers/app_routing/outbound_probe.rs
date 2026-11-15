//! Concrete host wiring for outbound probing.
use super::{config_api::ConfigBody, lifecycle_api::{ControlError, RoutingControl}};
use axum::{Extension, Json, extract::{Path, rejection::PathRejection}, http::{HeaderMap, StatusCode}};
use vw_api_app_identity::AppId;
use vw_api_app_diagnostics::{DiagnosticJobIdentity, DiagnosticJobSnapshot};
use vw_api_transport::id::{ProjectId, TaskId, TenantScope};
#[cfg(test)]
pub(super) use gateway_app_routing::server::outbound_probe::probe;
type ProbePath = Result<Path<(String, ProjectId, AppId)>, PathRejection>;
type JobPath = Result<Path<(String, ProjectId, AppId, TaskId)>, PathRejection>;

pub(crate) async fn start_outbound_probe(
    control: Option<Extension<RoutingControl>>, tenant: Extension<TenantScope>,
    path: ProbePath, headers: HeaderMap, body: ConfigBody,
) -> Result<(StatusCode, Json<DiagnosticJobIdentity>), ControlError> {
    gateway_app_routing::server::outbound_probe::start_outbound_probe(control, tenant, path, headers, body).await
}

pub(crate) async fn outbound_probe_status(
    control: Option<Extension<RoutingControl>>, tenant: Extension<TenantScope>,
    path: JobPath, headers: HeaderMap,
) -> Result<Json<DiagnosticJobSnapshot>, ControlError> {
    gateway_app_routing::server::outbound_probe::outbound_probe_status(control, tenant, path, headers).await
}

pub(crate) async fn cancel_outbound_probe(
    control: Option<Extension<RoutingControl>>, tenant: Extension<TenantScope>,
    path: JobPath, headers: HeaderMap,
) -> Result<Json<DiagnosticJobSnapshot>, ControlError> {
    gateway_app_routing::server::outbound_probe::cancel_outbound_probe(control, tenant, path, headers).await
}
