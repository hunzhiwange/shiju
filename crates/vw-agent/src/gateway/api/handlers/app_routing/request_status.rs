//! Concrete host wiring for request observations.
use super::lifecycle_api::{ControlError, RoutingControl};
use axum::{Extension, Json, extract::Path, http::HeaderMap};
use vw_api_app_identity::AppId;
use vw_api_app_routing::RoutingRequestsStatus;
use vw_api_transport::id::{ProjectId, TenantScope};

pub(crate) async fn request_status(
    control: Option<Extension<RoutingControl>>,
    tenant: Extension<TenantScope>,
    path: Path<(String, ProjectId, AppId)>,
    headers: HeaderMap,
) -> Result<Json<RoutingRequestsStatus>, ControlError> {
    gateway_app_routing::server::request_status(control, tenant, path, headers).await
}
