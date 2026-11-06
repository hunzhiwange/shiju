use super::*;
use vw_api_types::app_providers::presentation::AppProviderPresentation;
pub(super) fn router() -> Router {
    Router::new().route("/app-providers/{app}/presentation/entries", get(list)).route(
        "/app-providers/{app}/{id}/presentation",
        axum::routing::put(write).layer(DefaultBodyLimit::max(16 * 1024)),
    )
}
async fn list(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path(app): Path<AppId>,
) -> Response {
    match tokio::task::spawn_blocking(move || store(database)?.list_presentation(&tenant, app))
        .await
    {
        Ok(Ok(rows)) => Json(rows).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
async fn write(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
    body: Result<Json<AppProviderPresentation>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else {
        return error("invalid input");
    };
    match tokio::task::spawn_blocking(move || {
        store(database)?.set_presentation(&tenant, app, &id, &request)
    })
    .await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
