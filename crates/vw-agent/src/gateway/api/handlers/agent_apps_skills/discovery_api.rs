//! Discovery runs only after project/host authorization and registered-repo lookup.
use std::{collections::BTreeMap, sync::Arc};
use axum::{Extension, Json, Router, extract::DefaultBodyLimit, http::{HeaderMap, StatusCode}, routing::post};
use tokio::sync::Mutex;
use vw_agent_apps::skills::{discovery_cache::SkillDiscoveryCache, repos::SkillRepo};
use vw_api_types::{agent_apps_skills::discovery_api::*, id::{ProjectScope, TenantScope}};
use crate::{global_database::GlobalDatabase, gateway::agent_apps_skills_store::SkillReposStore};

#[derive(Clone)]
struct DiscoveryTransport { policy: Option<std::path::PathBuf>, api: String, archives: String }

// Router-owned state never crosses gateway instances or tenant/project/host boundaries.
type Caches = Arc<Mutex<BTreeMap<String, Arc<Mutex<SkillDiscoveryCache>>>>>;

pub(crate) fn router() -> Router {
    router_at(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from),
        "https://api.github.com".into(), "https://codeload.github.com".into())
}

pub(super) fn router_at(policy: Option<std::path::PathBuf>, api: String, archives: String) -> Router {
    Router::new().route("/agent-apps/skills/discover", post(discover))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(Extension(Caches::default()))
        .layer(Extension(DiscoveryTransport { policy, api, archives }))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}

async fn discover(
    Extension(tenant): Extension<TenantScope>,
    Extension(caches): Extension<Caches>,
    Extension(transport): Extension<DiscoveryTransport>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Json(request): Json<DiscoverSkillsRequest>,
) -> Result<Json<DiscoverSkillsResponse>, StatusCode> {
    let scope = ProjectScope::new(tenant, request.scope.project_id.clone())
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let key = serde_json::to_string(&(scope.tenant_scope(), scope.project_id(),
        &request.scope.target_host_id, &request.repo_id)).map_err(|_| StatusCode::BAD_REQUEST)?;
    let (store, state, scope) = tokio::task::spawn_blocking(move || {
        let policy = transport.policy;
        super::super::agent_apps::authorize_skill_repos(policy.as_deref(), &headers,
            &scope, &request.scope.target_host_id, false)?;
        let db = match database {
            Some(Extension(db)) => db,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        };
        let store = SkillReposStore::new(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let state = store.get(&scope).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        Ok::<_, StatusCode>((store, state, scope))
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)??;
    let repos = select_repositories(&state.repos, request.repo_id.as_deref())?;
    let cache = {
        let mut caches = caches.lock().await;
        // Bound router memory without evicting a cache used by an outstanding request.
        if !caches.contains_key(&key) && caches.len() >= 256 {
            caches.retain(|_, value| Arc::strong_count(value) > 1);
            if caches.len() >= 256 { return Err(StatusCode::SERVICE_UNAVAILABLE); }
        }
        caches.entry(key).or_default().clone()
    };
    let cache = cache.lock().await;
    cache.update_repositories(state.version, repos).map_err(|_| StatusCode::CONFLICT)?;
    let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none())
        .build().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let report = cache.discover_at(&client, request.force_refresh, &transport.api, &transport.archives).await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    // A configuration changed during I/O cannot be presented as its new revision.
    let current = tokio::task::spawn_blocking(move || store.get(&scope)).await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if current.version != state.version { return Err(StatusCode::CONFLICT); }
    project_report(state.version, report).map(Json)
}

pub(super) fn select_repositories(repos: &[SkillRepo], id: Option<&str>) -> Result<Vec<SkillRepo>, StatusCode> {
    match id {
        None => Ok(repos.to_vec()),
        Some(id) => {
            let (owner, name) = id.split_once(':').ok_or(StatusCode::BAD_REQUEST)?;
            let repo = repos.iter().find(|repo| repo.owner.eq_ignore_ascii_case(owner)
                && repo.name.eq_ignore_ascii_case(name)).ok_or(StatusCode::NOT_FOUND)?;
            if !repo.enabled { return Err(StatusCode::CONFLICT); }
            Ok(vec![repo.clone()])
        }
    }
}

pub(super) fn project_report(version: u64, report: vw_agent_apps::skills::discovery::SkillDiscoveryResult)
    -> Result<DiscoverSkillsResponse, StatusCode> {
    // Explicit wire DTOs stay independent of the business crate; all report fields survive.
    let report = serde_json::json!({"version": version, "skills": report.skills, "failures": report.failures});
    serde_json::from_value(report).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
