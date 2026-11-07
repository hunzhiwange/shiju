use super::*;
use vw_api_types::app_providers::basic::ProviderBasicWrite;
pub(super) fn router() -> Router {
    Router::new().route(
        "/app-providers/{app}/{id}/basic",
        get(read).put(write).layer(DefaultBodyLimit::max(16 * 1024)),
    )
}
async fn read(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
) -> Response {
    match tokio::task::spawn_blocking(move || store(database)?.basic(&tenant, app, &id)).await {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(e)) => error(&e),
        Err(_) => error("application provider storage unavailable"),
    }
}
async fn write(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
    body: Result<Json<ProviderBasicWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(value)) = body else { return error("invalid input") };
    match tokio::task::spawn_blocking(move || {
        store(database)?.write_basic(&tenant, app, &id, &value)
    })
    .await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(e)) => error(&e),
        Err(_) => error("application provider storage unavailable"),
    }
}
