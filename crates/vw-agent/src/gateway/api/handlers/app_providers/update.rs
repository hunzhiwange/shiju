use super::*;
use vw_api_types::app_providers::update::AppProviderUpdate;
pub(super) async fn update(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
    policy: Option<Extension<super::claude::Policy>>,
    headers: axum::http::HeaderMap,
    body: Result<
        Json<vw_api_types::app_providers::claude::ClaudeOr<AppProviderUpdate>>,
        JsonRejection,
    >,
) -> Response {
    let Ok(Json(request)) = body else {
        return error("invalid input");
    };
    let request = match request {
        vw_api_types::app_providers::claude::ClaudeOr::Claude(request) => {
            if app != AppId::Claude {
                return StatusCode::BAD_REQUEST.into_response();
            }
            return super::claude::write(
                tenant,
                database,
                headers,
                policy,
                id,
                request,
                super::claude::WriteKind::Update,
            )
            .await;
        }
        vw_api_types::app_providers::claude::ClaudeOr::Legacy(request) => request,
    };
    match tokio::task::spawn_blocking(move || store(database)?.update(&tenant, app, &id, &request))
        .await
    {
        Ok(Ok(summary)) => Json(summary).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
