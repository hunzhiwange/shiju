use super::*;
use vw_api_types::app_providers::universal_sync::UniversalSyncRequest;
pub(super) fn router() -> Router {
    Router::new().route(
        "/universal-providers/{id}/sync",
        axum::routing::post(sync).layer(DefaultBodyLimit::max(4096)),
    )
}
async fn sync(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path(id): Path<String>,
    body: Result<Json<UniversalSyncRequest>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else {
        return error("invalid input");
    };
    match tokio::task::spawn_blocking(move || {
        store(database)?.universal_sync(&tenant, &id, &request)
    })
    .await
    {
        Ok(Ok(results)) => Json(results).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
