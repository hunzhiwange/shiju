//! Reuse the native mode transaction and the listener's single credential owner.
use super::lifecycle_api::{ControlError, RoutingControl};
use axum::{
    Extension, Json,
    extract::{Path, rejection::JsonRejection},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use vw_api_types::{
    agent_apps::AppId,
    app_routing::RoutingModeRequest,
    id::{ProjectId, TenantScope},
};

pub(crate) async fn attach_mode(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path((host, project, app)): Path<(String, ProjectId, AppId)>,
    headers: HeaderMap,
    body: Result<Json<RoutingModeRequest>, JsonRejection>,
) -> Result<Response, ControlError> {
    let (control, _) = RoutingControl::authorize(
        control,
        headers.clone(),
        tenant.clone(),
        host.clone(),
        Some((project.clone(), app)),
        true,
    )
    .await?;
    let Json(request) = body.map_err(|_| ControlError::InvalidConfig)?;
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (control, tenant, host, project, app, headers, request);
        Err(ControlError::UnsupportedHost)
    }
    #[cfg(target_os = "macos")]
    {
        // Awaiting the task does not cancel a native transaction when the caller disconnects.
        tokio::task::spawn_blocking(move || {
            use axum::http::StatusCode;
            use vw_api_types::{
                error::{ApiErrorBody, ApiErrorDetail},
                id::ProjectScope,
            };
            let apply = || {
                let project = ProjectScope::new(tenant.clone(), project)
                    .map_err(|_| StatusCode::BAD_REQUEST)?;
                let (root, _) = control.authorize_mode(&headers, &project, &host, app)?;
                let (home, provider_root) = super::super::agent_apps::provider_editor_context(
                    Some(&control.policy),
                    &headers,
                    &tenant,
                    &host,
                    true,
                )?;
                if root != provider_root {
                    return Err(StatusCode::CONFLICT);
                }
                let scope = vw_api_types::agent_apps_high_availability::FailoverQueueScope {
                    project,
                    target_host_id: host,
                    app_id: app,
                };
                crate::app_routing::provider_publication::apply(
                    control.database.clone(),
                    &control.routing,
                    &home,
                    &root,
                    &scope,
                    &request.action,
                )
            };
            match apply() {
                Ok(result) => Json(result).into_response(),
                Err(status) => {
                    let code = match status {
                        StatusCode::FORBIDDEN => "forbidden",
                        StatusCode::UNAUTHORIZED => "unauthenticated",
                        StatusCode::NOT_FOUND => "not_found",
                        StatusCode::BAD_REQUEST => "invalid_input",
                        StatusCode::UNPROCESSABLE_ENTITY => "unsupported_target",
                        StatusCode::CONFLICT => "revision_conflict",
                        _ => "unavailable",
                    };
                    (
                        status,
                        Json(ApiErrorBody {
                            error: ApiErrorDetail {
                                code: code.into(),
                                message: "Application mode transaction failed".into(),
                                details: None,
                            },
                        }),
                    )
                        .into_response()
                }
            }
        })
        .await
        .map_err(|_| ControlError::Unavailable)
    }
}
