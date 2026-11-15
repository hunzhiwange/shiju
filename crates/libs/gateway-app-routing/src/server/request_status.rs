use super::{ControlError, RoutingObservation};
use axum::{Extension, Json, extract::Path, http::HeaderMap};
use {
    vw_api_app_identity::AppId,
    vw_api_app_routing::RoutingRequestsStatus,
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
};
pub async fn request_status<C: RoutingObservation>(
    control: Option<Extension<C>>,
    Extension(tenant): Extension<TenantScope>,
    Path((host, project, app)): Path<(String, ProjectId, AppId)>,
    headers: HeaderMap,
) -> Result<Json<RoutingRequestsStatus>, ControlError> {
    let (control, _) = C::authorize(
        control,
        headers,
        tenant.clone(),
        host.clone(),
        Some((project.clone(), app)),
        false,
    )
    .await?;
    let project = ProjectScope::new(tenant, project).map_err(|_| ControlError::InvalidInput)?;
    Ok(Json(control.request_status(&project, &host, app)))
}
