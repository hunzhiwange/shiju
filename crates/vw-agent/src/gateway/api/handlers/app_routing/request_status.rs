use super::lifecycle_api::{ControlError, RoutingControl};
use axum::{Extension, Json, extract::Path, http::HeaderMap};
use {
    vw_api_app_identity::AppId,
    vw_api_types::app_routing::RoutingRequestsStatus,
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
};
pub(crate) async fn request_status(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path((host, project, app)): Path<(String, ProjectId, AppId)>,
    headers: HeaderMap,
) -> Result<Json<RoutingRequestsStatus>, ControlError> {
    let (control, _) = RoutingControl::authorize(
        control,
        headers,
        tenant.clone(),
        host.clone(),
        Some((project.clone(), app)),
        false,
    )
    .await?;
    let project = ProjectScope::new(tenant, project).map_err(|_| ControlError::InvalidInput)?;
    Ok(Json(control.routing.request_status(&project, &host, app)))
}
