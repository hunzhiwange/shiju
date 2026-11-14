use std::path::PathBuf;
use axum::{Extension, Json, Router, extract::{Path, Query}, http::{HeaderMap, StatusCode}, routing::{get, post}};
use vw_api_types::{agent_apps_skills::{jobs_api::*, repos_api::SkillReposQuery}, id::{ProjectScope, TaskId, TenantScope}};
use crate::global_database::GlobalDatabase;

pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}

pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    let cancel_policy = policy.clone();
    Router::new()
        .route("/agent-apps/skills/jobs/{id}", get(move |
            Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
            headers: HeaderMap, Path(id): Path<TaskId>, Query(scope): Query<SkillReposQuery>,
        | { let policy = policy.clone(); async move { execute(policy, tenant, db, headers, id, scope, false).await.map(Json) } }))
        .route("/agent-apps/skills/jobs/{id}/cancel", post(move |
            Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
            headers: HeaderMap, Path(id): Path<TaskId>, Query(scope): Query<SkillReposQuery>,
        | { let policy = cancel_policy.clone(); async move { execute(policy, tenant, db, headers, id, scope, true).await.map(Json) } }))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}

async fn execute(policy: Option<PathBuf>, tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>, headers: HeaderMap, id: TaskId,
    scope: SkillReposQuery, cancel: bool,
) -> Result<SkillJobResponse, StatusCode> {
    let project = ProjectScope::new(tenant, scope.project_id.clone()).map_err(|_| StatusCode::BAD_REQUEST)?;
    let (root, principal) = super::super::agent_apps::authorize_skill_job(
        policy.as_deref(), &headers, &project, &scope.target_host_id)?;
    #[cfg(target_os = "macos")]
    {
        tokio::task::spawn_blocking(move || {
            use crate::gateway::agent_apps_skills_store::SkillInstallsStore;
            use vw_agent_apps::skills::jobs::SkillJobPhase as Phase;
            let db = match db {
                Some(Extension(db)) => db,
                None => GlobalDatabase::for_home(&crate::global::paths().home)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            };
            let job = SkillInstallsStore::new(db).and_then(|store| store.owned_job(
                &project, &scope.target_host_id, &id, &principal, &root, cancel))
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.ok_or(StatusCode::NOT_FOUND)?;
            let phase = match job.phase {
                Phase::Started => SkillJobPhase::Started,
                Phase::Staged => SkillJobPhase::Staged,
                Phase::Committing => SkillJobPhase::Committing,
                Phase::Completed => SkillJobPhase::Completed,
                Phase::Failed => SkillJobPhase::Failed,
                Phase::Interrupted => SkillJobPhase::Interrupted,
            };
            Ok(SkillJobResponse { scope, task_id: job.task_id, phase,
                installed_directory: job.installed_directory,
                requires_reconciliation: job.requires_reconciliation })
        }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = (db, project, root, principal, id, scope, cancel); Err(StatusCode::NOT_IMPLEMENTED) }
}
