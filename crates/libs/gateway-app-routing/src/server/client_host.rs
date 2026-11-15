//! Authorized host observations. Updating awaits an executor-wide native transaction.
use super::{ControlError, RoutingObservation};
use axum::{Extension, Json, extract::{Path, rejection::JsonRejection}, http::HeaderMap};
use {
    vw_api_app_routing::RoutingClientHost,
    vw_api_app_routing::RoutingClientHostUpdate,
    vw_api_app_routing::normalize_routing_client_host,
    vw_api_app_routing::routing_client_host_from_bind,
    vw_api_transport::id::TenantScope,
};

pub async fn get_client_host<C: RoutingObservation>(
    control: Option<Extension<C>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
) -> Result<Json<RoutingClientHost>, ControlError> {
    let (control, _) = C::authorize(control, headers, tenant, host, None, false).await?;
    let bind = control.config_bind().await?;
    Ok(Json(RoutingClientHost { host: None, effective_host: routing_client_host_from_bind(bind) }))
}

pub async fn put_client_host<C: RoutingObservation>(
    control: Option<Extension<C>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
    body: Result<Json<RoutingClientHostUpdate>, JsonRejection>,
) -> Result<Json<RoutingClientHost>, ControlError> {
    let _ = C::authorize(control, headers, tenant, host, None, true).await?;
    let Json(input) = body.map_err(|_| ControlError::InvalidConfig)?;
    normalize_routing_client_host(input.host.as_deref()).map_err(|_| ControlError::InvalidConfig)?;
    // TODO(task 87): require one native transaction that authorizes every affected
    // project/app, reprojects all attachments and commits the host only on success.
    // Neither changing a setting first nor replaying independent mode writes is atomic.
    Err(ControlError::ClientProjectionUnsupported)
}
