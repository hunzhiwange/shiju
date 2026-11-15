//! Authorized listener status and readiness observations.
use super::{ControlError, RoutingObservation};
use axum::{Extension, Json, extract::Path, http::HeaderMap};
use vw_api_app_routing::{RoutingHealth, RoutingStatus};
use vw_api_transport::id::TenantScope;

pub async fn status<C: RoutingObservation>(
    control: Option<Extension<C>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
) -> Result<Json<RoutingStatus>, ControlError> {
    let (control, _) =
        C::authorize(control, headers, tenant, host, None, false).await?;
    Ok(Json(control.routing_status()))
}

pub async fn health<C: RoutingObservation>(
    control: Option<Extension<C>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
) -> Result<Json<RoutingHealth>, ControlError> {
    let (control, _) =
        C::authorize(control, headers, tenant, host, None, false).await?;
    if !control.ready() {
        return Err(ControlError::NotReady);
    }
    Ok(Json(RoutingHealth { ready: true }))
}
