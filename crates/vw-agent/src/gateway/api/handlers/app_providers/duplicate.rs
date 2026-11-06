use super::*;
use vw_api_types::app_providers::duplicate::AppProviderDuplicate;
pub(super) fn router() -> Router {
    Router::new().route(
        "/app-providers/{app}/{id}/duplicate",
        axum::routing::post(duplicate).layer(DefaultBodyLimit::max(16 * 1024)),
    )
}
async fn duplicate(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
    body: Result<Json<AppProviderDuplicate>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else {
        return error("invalid input");
    };
    match tokio::task::spawn_blocking(move || {
        store(database)?.duplicate(&tenant, app, &id, &request)
    })
    .await
    {
        Ok(Ok(summary)) => (StatusCode::CREATED, Json(summary)).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
