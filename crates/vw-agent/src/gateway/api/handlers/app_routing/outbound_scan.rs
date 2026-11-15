//! Concrete host wiring for outbound scanning.
use super::{config_api::ConfigBody, lifecycle_api::{ControlError, RoutingControl}};
use axum::{Extension, Json, extract::{Path, rejection::PathRejection}, http::{HeaderMap, StatusCode}};
use vw_api_app_identity::AppId;
use vw_api_app_diagnostics::{DiagnosticJobIdentity, DiagnosticJobSnapshot};
use vw_api_transport::id::{ProjectId, TaskId, TenantScope};
#[cfg(test)]
pub(super) use gateway_app_routing::server::outbound_scan::{CANDIDATES, scan_connection};
type ScanPath = Result<Path<(String, ProjectId, AppId)>, PathRejection>;
type JobPath = Result<Path<(String, ProjectId, AppId, TaskId)>, PathRejection>;

pub(crate) async fn start_outbound_scan(
    control: Option<Extension<RoutingControl>>, tenant: Extension<TenantScope>,
    path: ScanPath, headers: HeaderMap, body: ConfigBody,
) -> Result<(StatusCode, Json<DiagnosticJobIdentity>), ControlError> {
    gateway_app_routing::server::outbound_scan::start_outbound_scan(control, tenant, path, headers, body).await
}

pub(crate) async fn outbound_scan_status(
    control: Option<Extension<RoutingControl>>, tenant: Extension<TenantScope>,
    path: JobPath, headers: HeaderMap,
) -> Result<Json<DiagnosticJobSnapshot>, ControlError> {
    gateway_app_routing::server::outbound_scan::outbound_scan_status(control, tenant, path, headers).await
}

pub(crate) async fn cancel_outbound_scan(
    control: Option<Extension<RoutingControl>>, tenant: Extension<TenantScope>,
    path: JobPath, headers: HeaderMap,
) -> Result<Json<DiagnosticJobSnapshot>, ControlError> {
    gateway_app_routing::server::outbound_scan::cancel_outbound_scan(control, tenant, path, headers).await
}
