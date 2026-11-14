use super::lifecycle_api::{ControlError, RoutingControl};
use axum::{Extension, Json, extract::Path, http::HeaderMap};
use vw_api_types::{
    agent_apps::AppId,
    app_routing::RoutingRequestsStatus,
    id::{ProjectId, ProjectScope, TenantScope},
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
