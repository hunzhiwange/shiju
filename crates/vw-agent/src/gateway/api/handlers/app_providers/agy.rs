//! Agy catalog writes do not activate a global provider or touch native settings.
use super::*;
use axum::{
    http::HeaderMap,
    routing::{post, put},
};
use std::path::PathBuf;
use vw_api_types::app_providers::agy::*;
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(in crate::gateway::api::handlers) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/agy/{id}/config", put(save))
        .route("/app-providers/agy/{id}/switch", post(switch))
        .layer(DefaultBodyLimit::max(32768))
        .layer(Extension(Policy(path)))
}
async fn save(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(b): Json<AgyWrite>,
) -> Response {
    match tokio::task::spawn_blocking(move || -> Result<AgySaved, StatusCode> {
        super::super::agent_apps::provider_editor_context(
            p.0.as_deref(),
            &h,
            &t,
            &b.target_host_id,
            true,
        )?;
        // Validation is pure; persistence cannot be interpreted as a running terminal.
        if !cfg!(target_os = "macos") {
            return Err(StatusCode::NOT_IMPLEMENTED);
        }
        #[cfg(target_os = "macos")]
        vw_agent_apps::providers::agy::validate(&b.profile)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        store(db)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .agy_save(&t, &id, &b.profile)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        Ok(AgySaved { catalog_saved: true })
    })
    .await
    {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(s)) => s.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
async fn switch(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    h: HeaderMap,
    Path(_id): Path<String>,
    Json(b): Json<AgyTarget>,
) -> Response {
    match tokio::task::spawn_blocking(move || {
        super::super::agent_apps::provider_editor_context(
            p.0.as_deref(),
            &h,
            &t,
            &b.target_host_id,
            true,
        )
    })
    .await
    {
        Ok(Ok(_)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({"code":"agy_switch_unsupported"})),
        )
            .into_response(),
        Ok(Err(s)) => s.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
