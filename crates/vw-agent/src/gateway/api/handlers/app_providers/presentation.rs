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
    headers: axum::http::HeaderMap,
    axum::extract::Query(query): axum::extract::Query<
        vw_api_types::agent_apps::aggregation::AggregationWriteQuery,
    >,
    #[cfg(target_os = "macos")] control: Option<
        Extension<super::super::app_routing::RoutingControl>,
    >,
    body: Result<Json<AppProviderPresentation>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else { return error("invalid input") };
    match tokio::task::spawn_blocking(move || -> Result<_, Response> {
        let store = store(database).map_err(|message| error(&message))?;
        #[cfg(target_os = "macos")]
        let store = if let Some(host) = query.target_host_id.clone() {
            let input = PublicationInput { query, routing: control.map(|Extension(c)| c) };
            let headers = input.headers(headers).map_err(IntoResponse::into_response)?;
            let policy =
                std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
            publication_store(store, policy.as_deref(), &headers, &tenant, &host, app, input)
                .map_err(IntoResponse::into_response)?
        } else {
            store
        };
        #[cfg(not(target_os = "macos"))]
        let _ = (headers, query);
        store.set_presentation(&tenant, app, &id, &request).map_err(|message| {
            #[cfg(target_os = "macos")]
            if let Some(status) = store.publication_error() {
                return status.into_response();
            }
            error(&message)
        })
    })
    .await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(response)) => response,
        Err(_) => error("application provider storage unavailable"),
    }
}
