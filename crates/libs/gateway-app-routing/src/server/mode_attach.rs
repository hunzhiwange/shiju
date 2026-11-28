//! Reuse the native mode transaction and the listener's single credential owner.
use super::lifecycle_api::{ControlError, RoutingControl};
use axum::{
    Extension, Json,
    extract::{Path, rejection::JsonRejection},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use {
    vw_api_app_identity::AppId,
    vw_api_app_routing::{RoutingAttachment, RoutingModeRequest},
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::TenantScope,
};

pub async fn attachment(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path((host, project, app)): Path<(String, ProjectId, AppId)>,
    headers: HeaderMap,
) -> Result<Json<RoutingAttachment>, ControlError> {
    use vw_agent_apps::providers::{
        live::commit::{ExecutionHostId, ProviderTransactions},
        mode_state,
    };
    use vw_api_transport::ProjectScope;

    let project = ProjectScope::new(tenant.clone(), project)
        .map_err(|_| ControlError::InvalidConfig)?;
    let (_, root) = RoutingControl::authorize(
        control,
        headers,
        tenant.clone(),
        host.clone(),
        Some((project.project_id().clone(), app)),
        false,
    )
    .await?;
    if !vw_agent_apps::routing::supports_app(app) {
        return Err(ControlError::UnsupportedApp);
    }
    tokio::task::spawn_blocking(move || {
        let root = root.ok_or(ControlError::Unavailable)?;
        let host = ExecutionHostId::new(&host).map_err(|_| ControlError::InvalidConfig)?;
        let Some(tx) = ProviderTransactions::open_existing(&root, host)
            .map_err(|_| ControlError::Unavailable)?
        else {
            return Ok(Json(RoutingAttachment { attached: false }));
        };
        let state = tx.state(app).map_err(|_| ControlError::Unavailable)?;
        if state.project_scope.as_ref().is_some_and(|owner| owner != &project) {
            return Err(ControlError::Forbidden);
        }
        if let Some(pending) = tx.pending(app).map_err(|_| ControlError::Unavailable)? {
            if pending.tenant != tenant
                || pending.target.project_scope.as_ref().is_some_and(|owner| owner != &project)
            {
                return Err(ControlError::Forbidden);
            }
            return Err(ControlError::TransitionInProgress);
        }
        let status = mode_state::observe(&state, &tenant).map_err(|_| ControlError::Forbidden)?;
        Ok(Json(RoutingAttachment { attached: status.attached }))
    })
    .await
    .map_err(|_| ControlError::Unavailable)?
}

pub async fn attach_mode(
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
            use {
    vw_api_transport::error::ApiErrorBody,
    vw_api_transport::error::ApiErrorDetail,
    vw_api_transport::id::ProjectScope,
};
            let apply = || {
                let project = ProjectScope::new(tenant.clone(), project)
                    .map_err(|_| StatusCode::BAD_REQUEST)?;
                let (root, _) = control.authorize_mode(&headers, &project, &host, app)?;
                let authorization = app_access_service::access::authorize_provider_editor(
                    Some(&control.policy), &headers, &tenant, &host, true,
                )?;
                let home = authorization.host.home;
                let provider_root = authorization.host.credential_root
                    .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
                if root != provider_root {
                    return Err(StatusCode::CONFLICT);
                }
                let scope = vw_api_app_failover::FailoverQueueScope {
                    project,
                    target_host_id: host,
                    app_id: app,
                };
                app_routing_services::provider_publication::apply(
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
