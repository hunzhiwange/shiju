use super::*;
use vw_api_types::app_providers::create::AppProviderCreate;

pub(super) async fn create(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
    body: Result<Json<AppProviderCreate>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else {
        return error("invalid input");
    };
    match tokio::task::spawn_blocking(move || store(database)?.create(&tenant, app, &id, &request))
        .await
    {
        Ok(Ok(summary)) => (StatusCode::CREATED, Json(summary)).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
