//! Authorized update API. All sources and filesystem roots are server-owned.
use axum::{Extension, Json, Router, extract::{Path, Query, DefaultBodyLimit},
    http::{HeaderMap, StatusCode}, routing::{get, post}};
use vw_api_types::{agent_apps_skills::{updates_api::*, repos_api::SkillReposQuery},
    id::{ProjectScope, TenantScope, TaskId}};
use crate::global_database::GlobalDatabase;

#[derive(Clone)]
struct UpdatePolicy(Option<std::path::PathBuf>, String);

pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from))
}

pub(super) fn router_with_policy(policy: Option<std::path::PathBuf>) -> Router {
    router_at(policy, "https://codeload.github.com".into())
}

// Execution-side dependency only; clients cannot override archive URLs.
pub(super) fn router_at(policy: Option<std::path::PathBuf>, archives: String) -> Router {
    Router::new()
        .route("/agent-apps/skills/check-updates", post(check))
        .route("/agent-apps/skills/{id}/update", post(update))
        .route("/agent-apps/skills/update-tasks/{id}", get(status))
        .layer(Extension(UpdatePolicy(policy, archives)))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}

fn authorize(policy: &UpdatePolicy, tenant: TenantScope, headers: &HeaderMap, scope: &SkillReposQuery, write: bool)
    -> Result<(ProjectScope, std::path::PathBuf), StatusCode> {
    let project = ProjectScope::new(tenant, scope.project_id.clone()).map_err(|_| StatusCode::BAD_REQUEST)?;
    let root = if write {
        super::super::agent_apps::authorize_skill_install(policy.0.as_deref(), headers, &project,
            &scope.target_host_id, None)?.0
    } else {
        super::super::agent_apps::authorize_skill_inventory(policy.0.as_deref(), headers, &project,
            &scope.target_host_id)?
    };
    Ok((project, root))
}

#[cfg(target_os = "macos")]
fn store(database: Option<Extension<GlobalDatabase>>)
    -> Result<crate::gateway::agent_apps_skills_store::SkillInstallsStore, StatusCode> {
    let database = match database {
        Some(Extension(db)) => db,
        None => GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    };
    crate::gateway::agent_apps_skills_store::SkillInstallsStore::new(database)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

#[cfg(target_os = "macos")]
fn client() -> Result<reqwest::Client, StatusCode> {
    reqwest::Client::builder().redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(120)).build()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

async fn check(Extension(tenant): Extension<TenantScope>, database: Option<Extension<GlobalDatabase>>,
    Extension(policy): Extension<UpdatePolicy>, headers: HeaderMap, Json(request): Json<CheckSkillUpdatesRequest>,
) -> Result<Json<CheckSkillUpdatesResponse>, StatusCode> {
    let (project, root) = authorize(&policy, tenant, &headers, &request.scope, false)?;
    #[cfg(target_os = "macos")]
    {
        let store = store(database)?;
        // Read one snapshot so returned hashes and version describe the same revision.
        let state = store.get(&project, &request.scope.target_host_id, &root)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let report = vw_agent_apps::skills::update_check::check_at(
            &client()?, &state.ssot_root, &state.skills, &state.repo_sources, &policy.1).await;
        // Native report and public failure codes share the established discovery wire format.
        Ok(Json(CheckSkillUpdatesResponse {
            scope: request.scope, version: state.version,
            updates: serde_json::from_value(serde_json::to_value(&report.updates).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            failures: serde_json::from_value(serde_json::to_value(&report.failures).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            skill_failures: serde_json::from_value(serde_json::to_value(&report.skill_failures).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            without_repository: report.without_repository, unchanged: report.unchanged,
        }))
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = (database, project, root); Err(StatusCode::NOT_IMPLEMENTED) }
}

async fn update(Extension(tenant): Extension<TenantScope>, database: Option<Extension<GlobalDatabase>>,
    Extension(policy): Extension<UpdatePolicy>, headers: HeaderMap, Path(id): Path<String>, Json(request): Json<UpdateSkillRequest>,
) -> Result<Json<SkillUpdateTask>, StatusCode> {
    let (project, root) = authorize(&policy, tenant, &headers, &request.scope, true)?;
    #[cfg(target_os = "macos")]
    {
        vw_agent_apps::skills::local_install::validate_directory(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
        if request.expected_hash.len() != 64 || !request.expected_hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let (_, _, principal) = super::super::agent_apps::authorize_skill_install_owner(
            policy.0.as_deref(), &headers, &project, &request.scope.target_host_id, None)?;
        let store = store(database)?;
        let job = store.start_skill_update_at(project, request.scope.target_host_id.clone(), request.expected_version,
            root, client()?, id, request.expected_hash, policy.1, Some(principal)).map_err(|_| StatusCode::CONFLICT)?;
        Ok(Json(SkillUpdateTask { scope: request.scope, task_id: job.task_id,
            phase: SkillUpdatePhase::Started, directory: None, requires_reconciliation: false,
            version: request.expected_version, content_hash: None }))
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = (database, project, root, id); Err(StatusCode::NOT_IMPLEMENTED) }
}

async fn status(Extension(tenant): Extension<TenantScope>, database: Option<Extension<GlobalDatabase>>,
    Extension(policy): Extension<UpdatePolicy>, headers: HeaderMap, Path(id): Path<String>, Query(scope): Query<SkillReposQuery>,
) -> Result<Json<SkillUpdateTask>, StatusCode> {
    let (project, root) = authorize(&policy, tenant, &headers, &scope, false)?;
    #[cfg(target_os = "macos")]
    {
        let store = store(database)?;
        let (_, principal) = super::super::agent_apps::authorize_skill_job(
            policy.0.as_deref(), &headers, &project, &scope.target_host_id)?;
        let task_id = TaskId::from(id);
        let job = store.owned_job(&project, &scope.target_host_id, &task_id, &principal, &root, false)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.ok_or(StatusCode::NOT_FOUND)?;
        let state = store.get(&project, &scope.target_host_id, &root).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let hash = job.content_hash;
        use vw_agent_apps::skills::jobs::SkillJobPhase as Phase;
        let phase = match job.phase {
            Phase::Started => SkillUpdatePhase::Started, Phase::Staged => SkillUpdatePhase::Staged,
            Phase::Committing => SkillUpdatePhase::Committing, Phase::Completed => SkillUpdatePhase::Completed,
            Phase::Failed => SkillUpdatePhase::Failed, Phase::Interrupted => SkillUpdatePhase::Interrupted,
        };
        Ok(Json(SkillUpdateTask { scope, task_id: job.task_id, phase,
            directory: job.installed_directory, requires_reconciliation: job.requires_reconciliation,
            version: job.update_version.unwrap_or(state.version), content_hash: hash }))
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = (database, project, root, id); Err(StatusCode::NOT_IMPLEMENTED) }
}
