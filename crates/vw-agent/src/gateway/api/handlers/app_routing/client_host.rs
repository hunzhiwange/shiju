//! Concrete host wiring for routing client-host endpoints.
use super::lifecycle_api::{ControlError, RoutingControl};
use axum::{Extension, Json, extract::{Path, rejection::JsonRejection}, http::HeaderMap};
use vw_api_app_routing::{RoutingClientHost, RoutingClientHostUpdate};
use vw_api_transport::id::TenantScope;

pub(crate) async fn get_client_host(
    control: Option<Extension<RoutingControl>>,
    tenant: Extension<TenantScope>,
    host: Path<String>,
    headers: HeaderMap,
) -> Result<Json<RoutingClientHost>, ControlError> {
    gateway_app_routing::server::get_client_host(control, tenant, host, headers).await
}

pub(crate) async fn put_client_host(
    control: Option<Extension<RoutingControl>>,
    tenant: Extension<TenantScope>,
    host: Path<String>,
    headers: HeaderMap,
    body: Result<Json<RoutingClientHostUpdate>, JsonRejection>,
) -> Result<Json<RoutingClientHost>, ControlError> {
    gateway_app_routing::server::put_client_host(control, tenant, host, headers, body).await
}
