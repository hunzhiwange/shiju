use super::*;
use vw_api_types::app_providers::update::AppProviderUpdate;
pub(super) async fn update(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
    body: Result<Json<AppProviderUpdate>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else {
        return error("invalid input");
    };
    match tokio::task::spawn_blocking(move || store(database)?.update(&tenant, app, &id, &request))
        .await
    {
        Ok(Ok(summary)) => Json(summary).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
