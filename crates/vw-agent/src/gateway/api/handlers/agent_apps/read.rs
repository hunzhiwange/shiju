//! Authenticated, explicitly granted target-host reads.
use super::access;
use axum::{
    Extension, Json, Router,
    extract::Query,
    http::{HeaderMap, StatusCode},
    routing::get,
};
use serde::Deserialize;
use std::path::PathBuf;
use vw_api_types::{
    agent_apps::{AppCatalogResponse, AppId, AppReadResponse},
    id::TenantScope,
};

#[derive(Clone)]
struct PolicyFile(Option<PathBuf>);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadQuery {
    target_host_id: String,
    app_id: Option<AppId>,
}

pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}

/// Same file-backed production path, with explicit path injection for embedding/testing.
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/catalog", get(catalog))
        .route("/agent-apps/installations", get(read))
        .route("/agent-apps/diagnostics", get(read))
        .route("/agent-apps/versions", get(read))
        .layer(Extension(PolicyFile(path)))
}

async fn authorized_host(
    policy: PolicyFile,
    headers: HeaderMap,
    tenant: TenantScope,
    target: String,
) -> Result<access::Host, StatusCode> {
    tokio::task::spawn_blocking(move || {
        access::authorize(policy.0.as_deref(), &headers, &tenant, &target)
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

async fn catalog(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(query): Query<ReadQuery>,
) -> Result<Json<AppCatalogResponse>, StatusCode> {
    authorized_host(policy, headers, tenant, query.target_host_id).await?;
    Ok(Json(AppCatalogResponse {
        apps: vw_agent_apps::apps::catalog::catalog().into_iter().map(|c| c.metadata).collect(),
    }))
}

async fn read(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(query): Query<ReadQuery>,
) -> Result<Json<AppReadResponse>, StatusCode> {
    let host = authorized_host(policy, headers, tenant, query.target_host_id).await?;
    let app = query.app_id.ok_or(StatusCode::BAD_REQUEST)?;
    #[cfg(not(unix))]
    {
        let _ = (host, app);
        Err(StatusCode::NOT_IMPLEMENTED)
    }
    #[cfg(unix)]
    tokio::task::spawn_blocking(move || {
        vw_agent_apps::apps::discovery::read::read(
            app,
            &vw_agent_apps::apps::discovery::read::LocalReadEnvironment {
                host_id: host.id,
                home: host.home,
                search_paths: host.search_paths,
                applications: host.applications,
            },
        )
        .map(Json)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
