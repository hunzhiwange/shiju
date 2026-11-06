use super::*;
use vw_api_types::app_providers::universal::UniversalProviderWrite;
pub(super) fn router() -> Router {
    Router::new().route("/universal-providers", get(list)).route(
        "/universal-providers/{id}",
        get(read).put(save).delete(delete).layer(DefaultBodyLimit::max(128 * 1024)),
    )
}
fn response<T: serde::Serialize>(
    result: Result<Result<T, String>, tokio::task::JoinError>,
) -> Response {
    match result {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(message)) if message == "invalid stored universal provider" => {
            error("invalid stored application provider profile")
        }
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
async fn list(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
) -> Response {
    response(tokio::task::spawn_blocking(move || store(database)?.universal_list(&tenant)).await)
}
async fn read(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path(id): Path<String>,
) -> Response {
    response(
        tokio::task::spawn_blocking(move || store(database)?.universal_get(&tenant, &id)).await,
    )
}
async fn save(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path(id): Path<String>,
    body: Result<Json<UniversalProviderWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else {
        return error("invalid input");
    };
    response(
        tokio::task::spawn_blocking(move || {
            store(database)?.universal_save(&tenant, &id, &request)
        })
        .await,
    )
}
async fn delete(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path(id): Path<String>,
) -> Response {
    response(
        tokio::task::spawn_blocking(move || store(database)?.universal_delete(&tenant, &id)).await,
    )
}
