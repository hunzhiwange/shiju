//! Authenticated catalog-only Desktop mapping operations.
use super::*;
use axum::{
    http::HeaderMap,
    routing::{post, put},
};
use std::path::PathBuf;
use vw_api_types::app_providers::{claude_desktop::*, desktop_mapping::*};
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/claude-desktop/model-mapping/defaults", post(defaults))
        .route("/app-providers/claude-desktop/providers/import-claude", post(import))
        .route("/app-providers/claude-desktop/{id}/model-mapping", put(save))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(Extension(Policy(path)))
}
async fn defaults(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    body: Result<Json<ClaudeDesktopTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    if let Err(s) = super::super::agent_apps::provider_editor_context(
        policy.0.as_deref(),
        &headers,
        &tenant,
        &request.target_host_id,
        false,
    ) {
        return s.into_response();
    }
    Json(vw_agent_apps::providers::desktop_mapping::defaults()).into_response()
}
async fn import(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    body: Result<Json<ClaudeDesktopTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    finish(
        tokio::task::spawn_blocking(move || {
            let store = authorized_store(&policy, &tenant, db, &headers, &request.target_host_id)?;
            let imported =
                store.desktop_import_claude(&tenant).map_err(|_| StatusCode::CONFLICT)?;
            Ok(Json(DesktopImportResult { imported }).into_response())
        })
        .await,
    )
}
async fn save(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<DesktopMappingWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    finish(
        tokio::task::spawn_blocking(move || {
            let store = authorized_store(&policy, &tenant, db, &headers, &request.target_host_id)?;
            store
                .desktop_mapping_save(&tenant, &id, &request.mapping)
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            Ok(Json(ClaudeDesktopResult {
                catalog_saved: true,
                live: ClaudeDesktopLiveResult::NotAttempted,
            })
            .into_response())
        })
        .await,
    )
}
fn finish(result: Result<Result<Response, StatusCode>, tokio::task::JoinError>) -> Response {
    match result {
        Ok(Ok(r)) => r,
        Ok(Err(s)) => s.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
fn authorized_store(
    policy: &Policy,
    tenant: &TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: &HeaderMap,
    host: &str,
) -> Result<AppProviderStore, StatusCode> {
    super::super::agent_apps::provider_editor_context(
        policy.0.as_deref(),
        headers,
        tenant,
        host,
        true,
    )?;
    let db = match db {
        Some(Extension(db)) => db,
        None => GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    };
    AppProviderStore::new(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
