//! Read-only host state and saved direct-client contract decisions. No proxy startup.
use axum::{
    Extension, Json, Router,
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use std::path::PathBuf;
use vw_api_types::{agent_apps::AppId, app_providers::mode_state::*, id::TenantScope};
#[derive(Clone)]
struct PolicyFile(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/{app}/mode/state", get(read))
        .layer(Extension(PolicyFile(path)))
}
async fn read(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<crate::global_database::GlobalDatabase>>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    Query(query): Query<ModeStateQuery>,
) -> Result<Response, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let (_, root) = super::super::agent_apps::provider_preview_context(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &query.target_host_id,
        )?;
        read_authorized(root.as_deref(), &query, &tenant, app, database)
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
#[cfg(target_os = "macos")]
fn read_authorized(
    root: Option<&std::path::Path>,
    query: &ModeStateQuery,
    tenant: &TenantScope,
    app: AppId,
    database: Option<Extension<crate::global_database::GlobalDatabase>>,
) -> Result<Response, StatusCode> {
    use vw_agent_apps::providers::{
        live::commit::{ExecutionHostId, ProviderTransactions},
        mode_state::{self, ClientConfiguration, ModeStateError},
    };
    let map_error = |e| match e {
        ModeStateError::Ownership => StatusCode::FORBIDDEN,
        ModeStateError::RouteUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    };
    let mut state = Default::default();
    if let Some(root) = root
        && let Some(tx) = ProviderTransactions::open_existing(
            root,
            ExecutionHostId::new(&query.target_host_id).map_err(|_| StatusCode::BAD_REQUEST)?,
        )
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    {
        if tx.pending(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.is_some() {
            return Err(StatusCode::CONFLICT);
        }
        state = tx.state(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    }
    let mut status = mode_state::observe(&state, tenant).map_err(map_error)?;
    if let Some(id) = &query.provider_id {
        if query.mode == ProviderMode::Proxy {
            return Err(map_error(ModeStateError::RouteUnavailable));
        }
        vw_agent_apps::providers::validate_identity(id).map_err(|_| StatusCode::BAD_REQUEST)?;
        let database = database.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let row =
            crate::app_providers::AppProviderStore::preview_record(&database.0, tenant, app, id)
                .map_err(|e| {
                    if e == "application provider not found" {
                        StatusCode::NOT_FOUND
                    } else {
                        StatusCode::SERVICE_UNAVAILABLE
                    }
                })?;
        let configuration = ClientConfiguration::saved(
            app,
            row.get("settingsConfig").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
            row.get("category").and_then(serde_json::Value::as_str) == Some("official"),
        )
        .map_err(map_error)?;
        status.decision = Some(mode_state::rewrite_decision(&state, query.mode, &configuration));
    }
    Ok(Json(status).into_response())
}
#[cfg(not(target_os = "macos"))]
fn read_authorized(
    _root: Option<&std::path::Path>,
    _query: &ModeStateQuery,
    _tenant: &TenantScope,
    _app: AppId,
    _database: Option<Extension<crate::global_database::GlobalDatabase>>,
) -> Result<Response, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}
