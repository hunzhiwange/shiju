use super::*;
use vw_api_types::app_providers::icons::AppProviderIconChange;
pub(super) fn router() -> Router {
    Router::new().route(
        "/app-providers/{app}/{id}/icon",
        get(read_icon).put(write_icon).layer(DefaultBodyLimit::max(1024)),
    )
}
async fn read_icon(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
) -> Response {
    match tokio::task::spawn_blocking(move || store(database)?.get_icon(&tenant, app, &id)).await {
        Ok(Ok(icon)) => Json(icon).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
async fn write_icon(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
    body: Result<Json<AppProviderIconChange>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else {
        return error("invalid input");
    };
    match tokio::task::spawn_blocking(move || {
        store(database)?.set_icon(&tenant, app, &id, &request)
    })
    .await
    {
        Ok(Ok(icon)) => Json(icon).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
