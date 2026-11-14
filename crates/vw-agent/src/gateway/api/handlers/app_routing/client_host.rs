//! Authorized host observations. Updating awaits an executor-wide native transaction.
use super::lifecycle_api::{ControlError, RoutingControl};
use axum::{Extension, Json, extract::{Path, rejection::JsonRejection}, http::HeaderMap};
use vw_api_types::{
    app_routing::{RoutingClientHost, RoutingClientHostUpdate, normalize_routing_client_host, routing_client_host_from_bind},
    id::TenantScope,
};

pub(crate) async fn get_client_host(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
) -> Result<Json<RoutingClientHost>, ControlError> {
    let (control, _) = RoutingControl::authorize(control, headers, tenant, host, None, false).await?;
    let bind = control.config().await?.bind.ip();
    Ok(Json(RoutingClientHost { host: None, effective_host: routing_client_host_from_bind(bind) }))
}

pub(crate) async fn put_client_host(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
    body: Result<Json<RoutingClientHostUpdate>, JsonRejection>,
) -> Result<Json<RoutingClientHost>, ControlError> {
    let _ = RoutingControl::authorize(control, headers, tenant, host, None, true).await?;
    let Json(input) = body.map_err(|_| ControlError::InvalidConfig)?;
    normalize_routing_client_host(input.host.as_deref()).map_err(|_| ControlError::InvalidConfig)?;
    // TODO(task 87): require one native transaction that authorizes every affected
    // project/app, reprojects all attachments and commits the host only on success.
    // Neither changing a setting first nor replaying independent mode writes is atomic.
    Err(ControlError::ClientProjectionUnsupported)
}
