//! Authenticated project repository management, backed by the settings transaction.
use axum::{Extension, Json, Router, extract::{Path, Query, DefaultBodyLimit},
    http::{HeaderMap, StatusCode}, routing::{get, delete}};
use vw_api_types::{agent_apps_skills::repos_api::*, id::{ProjectScope, TenantScope}};
use vw_agent_apps::skills::repos::{SkillRepo, RepoError};
use crate::{global_database::GlobalDatabase, gateway::agent_apps_skills_store::{
    SkillReposStore, SkillReposState, SkillReposVersionConflict}};

pub(crate) fn router() -> Router {
    Router::new()
        .route("/agent-apps/skills/repos", get(read).post(save))
        .route("/agent-apps/skills/repos/{id}", delete(remove))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}

async fn read(Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>, headers: HeaderMap,
    Query(scope): Query<SkillReposQuery>,
) -> Result<Json<SkillReposResponse>, StatusCode> {
    execute(tenant, database, headers, scope, None).await
}

async fn save(Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>, headers: HeaderMap,
    Json(request): Json<SaveSkillRepoRequest>,
) -> Result<Json<SkillReposResponse>, StatusCode> {
    execute(tenant, database, headers, request.scope,
        Some(Change::Save(request.expected_version, request.repo))).await
}

async fn remove(Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>, headers: HeaderMap,
    Path(id): Path<String>, Json(request): Json<RemoveSkillRepoRequest>,
) -> Result<Json<SkillReposResponse>, StatusCode> {
    // A colon separates validated GitHub owner/name components in a single segment.
    let (owner, name) = id.split_once(':').ok_or(StatusCode::BAD_REQUEST)?;
    execute(tenant, database, headers, request.scope,
        Some(Change::Remove(request.expected_version, owner.into(), name.into()))).await
}

enum Change { Save(u64, SkillRepoEntry), Remove(u64, String, String) }

async fn execute(tenant: TenantScope, database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, scope: SkillReposQuery, change: Option<Change>,
) -> Result<Json<SkillReposResponse>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let project = ProjectScope::new(tenant, scope.project_id.clone())
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
        super::super::agent_apps::authorize_skill_repos(policy.as_deref(), &headers,
            &project, &scope.target_host_id, change.is_some())?;
        let db = match database {
            Some(Extension(db)) => db,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        };
        let store = SkillReposStore::new(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let state: SkillReposState = match change {
            None => store.get(&project),
            Some(Change::Save(version, repo)) => store.save(&project, version, SkillRepo {
                owner: repo.owner, name: repo.name, branch: repo.branch, enabled: repo.enabled,
            }),
            Some(Change::Remove(version, owner, name)) => store.remove(&project, version, &owner, &name),
        }.map_err(storage_error)?;
        Ok(Json(SkillReposResponse { scope, version: state.version,
            repos: state.repos.into_iter().map(|repo| SkillRepoEntry {
                owner: repo.owner, name: repo.name, branch: repo.branch, enabled: repo.enabled,
            }).collect() }))
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

fn storage_error(error: anyhow::Error) -> StatusCode {
    if error.downcast_ref::<SkillReposVersionConflict>().is_some() { StatusCode::CONFLICT }
    else if error.downcast_ref::<RepoError>().is_some() { StatusCode::BAD_REQUEST }
    else { StatusCode::SERVICE_UNAVAILABLE }
}
