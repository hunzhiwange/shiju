//! Authenticated catalog-only Desktop mapping operations.
use super::*;
use axum::{
    http::HeaderMap,
    routing::{post, put},
};
use std::path::PathBuf;
use vw_api_types::app_providers::{claude_desktop::*, desktop_mapping::*};
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum FormRequest {
    Scoped(vw_api_types::agent_apps::aggregation::AggregationQuery),
    Legacy(ClaudeDesktopTarget),
}
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum MappingRequest {
    Scoped(vw_api_types::agent_apps::aggregation::DesktopMappingChange),
    Legacy(DesktopMappingWrite),
}
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    let router =
        router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from));
    match GlobalDatabase::for_home(&crate::global::paths().home) {
        Ok(database) => router.layer(Extension(database)),
        Err(_) => router,
    }
}
pub(in super::super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/claude-desktop/{id}/form/read", post(read_form))
        .route("/app-providers/claude-desktop/model-mapping/defaults", post(defaults))
        .route("/app-providers/claude-desktop/providers/import-claude", post(import))
        .route(
            "/app-providers/claude-desktop/{id}/model-mapping",
            put(save).layer(DefaultBodyLimit::max(256 * 1024)),
        )
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
    #[cfg(target_os = "macos")] control: Option<
        Extension<super::super::app_routing::RoutingControl>,
    >,
    body: Result<Json<MappingRequest>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else {
        return super::super::aggregation_desktop::failure(
            StatusCode::BAD_REQUEST,
            "invalid_request",
        );
    };
    let request = match request {
        MappingRequest::Scoped(request) => {
            #[cfg(target_os = "macos")]
            return super::super::aggregation_desktop::finish(
                tokio::task::spawn_blocking(move || {
                    super::super::aggregation_desktop::save_mapping(
                        policy.0.as_deref(),
                        tenant,
                        db,
                        headers,
                        id,
                        request,
                        control,
                    )
                })
                .await,
            );
            #[cfg(not(target_os = "macos"))]
            {
                let _ = request;
                return super::super::aggregation_desktop::failure(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "unsupported_target",
                );
            }
        }
        MappingRequest::Legacy(request) => request,
    };
    finish(
        tokio::task::spawn_blocking(move || {
            legacy_unbound(&policy, &tenant, &headers, &request.target_host_id, true)?;
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

async fn read_form(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<FormRequest>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else {
        return super::super::aggregation_desktop::failure(
            StatusCode::BAD_REQUEST,
            "invalid_request",
        );
    };
    let request = match request {
        FormRequest::Scoped(request) => {
            return super::super::aggregation_desktop::finish(
                tokio::task::spawn_blocking(move || {
                    super::super::aggregation_desktop::read_form(
                        policy.0.as_deref(),
                        tenant,
                        db,
                        headers,
                        id,
                        request,
                    )
                })
                .await,
            );
        }
        FormRequest::Legacy(request) => request,
    };
    finish(
        tokio::task::spawn_blocking(move || {
            legacy_unbound(&policy, &tenant, &headers, &request.target_host_id, false)?;
            super::super::agent_apps::provider_editor_context(
                policy.0.as_deref(),
                &headers,
                &tenant,
                &request.target_host_id,
                false,
            )?;
            let db = match db {
                Some(Extension(db)) => db,
                None => GlobalDatabase::for_home(&crate::global::paths().home)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            };
            let store = AppProviderStore::new(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let row = store
                .claude_desktop_record(&tenant, &id)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .ok_or(StatusCode::NOT_FOUND)?;
            let view = vw_agent_apps::providers::desktop_mapping::form_view(&row)
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            Ok(Json(view).into_response())
        })
        .await,
    )
}

fn legacy_unbound(
    policy: &Policy,
    tenant: &TenantScope,
    headers: &HeaderMap,
    host: &str,
    write: bool,
) -> Result<(), StatusCode> {
    let (_, root) = super::super::agent_apps::provider_editor_context(
        policy.0.as_deref(),
        headers,
        tenant,
        host,
        write,
    )?;
    #[cfg(target_os = "macos")]
    {
        use vw_agent_apps::providers::live::commit::{ExecutionHostId, ProviderTransactions};
        if let Some(tx) = ProviderTransactions::open_existing(
            &root,
            ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?,
        )
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        {
            let app = vw_api_types::agent_apps::AppId::ClaudeDesktop;
            if tx.state(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.project_scope.is_some()
                || tx
                    .pending(app)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                    .is_some_and(|pending| pending.target.project_scope.is_some())
            {
                return Err(StatusCode::CONFLICT);
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = root;
    Ok(())
}
