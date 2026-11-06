use super::*;
use vw_api_types::app_providers::endpoints::AppProviderEndpointChange;
pub(super) fn router() -> Router {
    Router::new().route(
        "/app-providers/{app}/{id}/endpoints",
        get(list).post(write).layer(DefaultBodyLimit::max(16 * 1024)),
    )
}
async fn list(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
) -> Response {
    match tokio::task::spawn_blocking(move || store(database)?.list_endpoints(&tenant, app, &id))
        .await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
async fn write(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
    body: Result<Json<AppProviderEndpointChange>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else {
        return error("invalid input");
    };
    match tokio::task::spawn_blocking(move || {
        store(database)?.change_endpoint(&tenant, app, &id, &request)
    })
    .await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
