//! Authorization precedes storage. Frozen snippets never resolve or open native config files.
use super::*;
use axum::{extract::Query, http::HeaderMap};
use std::path::PathBuf;
use vw_api_types::app_providers::snippets::*;
#[derive(Clone)]
struct SnippetPolicy(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/{app}/config/snippet", get(read).put(write))
        .route("/app-providers/{app}/config/snippet/extract", axum::routing::post(extract))
        .route("/app-providers/{app}/config/snippet/preview", axum::routing::post(preview))
        .layer(DefaultBodyLimit::max(128 * 1024))
        .layer(Extension(SnippetPolicy(path)))
}
fn response(result: Result<ConfigSnippet, String>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(message) => error(&message),
    }
}
async fn read(
    Extension(policy): Extension<SnippetPolicy>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    query: Result<Query<SnippetRead>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, StatusCode> {
    let Query(request) = query.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        super::super::agent_apps::authorize_provider_config_access(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &request.target_host_id,
            false,
        )?;
        Ok(response(store(database).and_then(|store| store.snippet_get(&tenant, app))))
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn write(
    Extension(policy): Extension<SnippetPolicy>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    body: Result<Json<SnippetWrite>, JsonRejection>,
) -> Result<Response, StatusCode> {
    let Json(request) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        super::super::agent_apps::authorize_provider_config_access(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &request.target_host_id,
            true,
        )?;
        Ok(response(
            store(database).and_then(|store| store.snippet_save(&tenant, app, &request.content)),
        ))
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn preview(
    Extension(policy): Extension<SnippetPolicy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    body: Result<Json<SnippetWrite>, JsonRejection>,
) -> Result<Response, StatusCode> {
    let Json(request) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        super::super::agent_apps::authorize_provider_config_access(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &request.target_host_id,
            false,
        )?;
        Ok(response(vw_agent_apps::providers::snippets::normalize(app, &request.content)))
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn extract(
    Extension(policy): Extension<SnippetPolicy>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    body: Result<Json<SnippetExtract>, JsonRejection>,
) -> Result<Response, StatusCode> {
    let Json(request) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        super::super::agent_apps::authorize_provider_config_access(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &request.target_host_id,
            false,
        )?;
        Ok(response(match request.source {
            SnippetSource::Draft { settings } => {
                vw_agent_apps::providers::snippets::extract(app, &settings)
            }
            SnippetSource::Saved { provider_id } => store(database)
                .and_then(|store| store.snippet_extract_saved(&tenant, app, &provider_id)),
        }))
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
