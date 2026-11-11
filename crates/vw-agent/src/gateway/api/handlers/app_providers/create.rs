use super::*;
use vw_api_types::app_providers::create::AppProviderCreate;

#[allow(clippy::too_many_arguments)]
pub(super) async fn create(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path((app, id)): Path<(AppId, String)>,
    policy: Option<Extension<super::claude::Policy>>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(query): axum::extract::Query<
        vw_api_types::agent_apps::aggregation::AggregationWriteQuery,
    >,
    #[cfg(target_os = "macos")] control: Option<
        Extension<super::super::app_routing::RoutingControl>,
    >,
    body: Result<
        Json<vw_api_types::app_providers::claude::ClaudeOr<AppProviderCreate>>,
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
                super::claude::WriteKind::Create,
                PublicationInput {
                    query,
                    #[cfg(target_os = "macos")]
                    routing: control.map(|Extension(c)| c),
                },
            )
            .await;
        }
        vw_api_types::app_providers::claude::ClaudeOr::Legacy(request) => request,
    };
    match tokio::task::spawn_blocking(move || store(database)?.create(&tenant, app, &id, &request))
        .await
    {
        Ok(Ok(summary)) => (StatusCode::CREATED, Json(summary)).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
