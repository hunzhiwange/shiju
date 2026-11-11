//! Claude Code form routes: authorization precedes record or native file access.
use super::*;

pub(super) async fn read(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<ClaudeTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(req)) = body else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let result = tokio::task::spawn_blocking(move || -> Result<ClaudeFormView, StatusCode> {
        #[cfg(target_os = "macos")]
        {
            super::super::super::agent_apps::provider_editor_context(
                policy.0.as_deref(),
                &headers,
                &tenant,
                &req.target_host_id,
                false,
            )?;
            vw_agent_apps::providers::validate_identity(&id)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            let store = store(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let row = store
                .claude_record(&tenant, &id)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .ok_or(StatusCode::NOT_FOUND)?;
            vw_agent_apps::providers::claude::form::view(&row)
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (policy, tenant, db, headers, id, req);
            Err(StatusCode::NOT_IMPLEMENTED)
        }
    })
    .await;
    match result {
        Ok(Ok(view)) => Json(view).into_response(),
        Ok(Err(status)) => status.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn write(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<
        vw_api_types::agent_apps::aggregation::AggregationWriteQuery,
    >,
    #[cfg(target_os = "macos")] control: Option<
        Extension<super::super::super::app_routing::RoutingControl>,
    >,
    Path(id): Path<String>,
    method: axum::http::Method,
    body: Result<Json<ClaudeFormWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(req)) = body else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let kind =
        if method == axum::http::Method::POST { WriteKind::Create } else { WriteKind::Update };
    write_input_scoped(
        tenant,
        db,
        headers,
        id,
        WriteInput::Form(req),
        kind,
        policy,
        PublicationInput {
            query,
            #[cfg(target_os = "macos")]
            routing: control.map(|Extension(c)| c),
        },
    )
    .await
}
