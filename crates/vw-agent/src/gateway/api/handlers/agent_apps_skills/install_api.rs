//! Scoped installation on the policy-selected execution host; never runs hooks.
use std::path::PathBuf;
use axum::{Extension, Json, Router, extract::DefaultBodyLimit,
    http::{HeaderMap, StatusCode}, routing::post};
use vw_api_types::{agent_apps_skills::install_api::*, id::{ProjectScope, TenantScope}};
use crate::global_database::GlobalDatabase;

pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}

pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    router_at(policy, "https://codeload.github.com".into())
}

pub(super) fn router_at(policy: Option<PathBuf>, archives: String) -> Router {
    Router::new().route("/agent-apps/skills/install", post(move |
        Extension(tenant): Extension<TenantScope>, database: Option<Extension<GlobalDatabase>>,
        headers: HeaderMap, Json(request): Json<InstallSkillRequest>,
    | {
        let policy = policy.clone();
        let archives = archives.clone();
        async move { execute(policy, tenant, database, headers, request, archives).await.map(Json) }
    }))
    .layer(DefaultBodyLimit::max(16 * 1024))
    .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
        response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-store"));
        response
    }))
}

async fn execute(policy: Option<PathBuf>, tenant: TenantScope,
    database: Option<Extension<GlobalDatabase>>, headers: HeaderMap, request: InstallSkillRequest, archives: String,
) -> Result<InstallSkillResponse, StatusCode> {
    let project = ProjectScope::new(tenant, request.scope.project_id.clone())
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let file_id = match &request.source {
        SkillInstallSource::Local { file_id, .. } | SkillInstallSource::Archive { file_id } => Some(file_id.as_str()),
        SkillInstallSource::Git { .. } => None,
    };
    // No storage initialization, source lookup or network before authorization.
    let (root, source, principal) = super::super::agent_apps::authorize_skill_install_owner(
        policy.as_deref(), &headers, &project, &request.scope.target_host_id, file_id)?;
    #[cfg(target_os = "macos")]
    {
        use crate::gateway::agent_apps_skills_store::{SkillInstallsStore, SkillReposStore};
        use vw_agent_apps::skills::local_install::validate_directory;
        let db = match database {
            Some(Extension(db)) => db,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        };
        let store = SkillInstallsStore::new(db.clone()).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let state = store.get(&project, &request.scope.target_host_id, &root)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        if state.version != request.expected_version { return Err(StatusCode::CONFLICT); }
        let host = request.scope.target_host_id.clone();

        match request.source {
            SkillInstallSource::Git { repo_id, selection, directory } => {
                validate_directory(&directory).map_err(|_| StatusCode::BAD_REQUEST)?;
                if state.skills.iter().any(|skill| skill.directory.eq_ignore_ascii_case(&directory)) {
                    return Err(StatusCode::CONFLICT);
                }
                let repos = SkillReposStore::new(db).and_then(|store| store.get(&project))
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                let repo = repos.repos.into_iter().find(|repo| format!("{}:{}", repo.owner, repo.name) == repo_id)
                    .ok_or(StatusCode::NOT_FOUND)?;
                if !repo.enabled { return Err(StatusCode::CONFLICT); }
                // Redirects cannot expand the approved GitHub download authority.
                let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none())
                    .timeout(std::time::Duration::from_secs(120)).build()
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                let result = SkillInstallItem { target_host_id: host.clone(), directory: directory.clone(),
                    status: SkillInstallStatus::Pending };
                let job = store.start_repo_install_at(project, host, request.expected_version, root,
                    client, repo, selection, directory, principal, archives).map_err(|_| StatusCode::BAD_REQUEST)?;
                Ok(InstallSkillResponse { scope: request.scope, task_id: job.task_id, results: vec![result] })
            }
            source_kind => {
                let source = source.ok_or(StatusCode::FORBIDDEN)?;
                tokio::task::spawn_blocking(move || {
                    if let SkillInstallSource::Local { directory, .. } = &source_kind {
                        validate_directory(directory).map_err(|_| StatusCode::BAD_REQUEST)?;
                    }
                    let metadata = std::fs::symlink_metadata(&source).map_err(|_| StatusCode::BAD_REQUEST)?;
                    let valid = match &source_kind {
                        SkillInstallSource::Local { .. } => metadata.is_dir(),
                        SkillInstallSource::Archive { .. } => metadata.is_file(),
                        SkillInstallSource::Git { .. } => false,
                    };
                    if !valid { return Err(StatusCode::BAD_REQUEST); }
                    let (job, results) = store.run_file_install(&project, &host, &root, &principal, || {
                        match source_kind {
                            SkillInstallSource::Local { directory, .. } => {
                                validate_directory(&directory)?;
                                store.install_local(&project, &host, request.expected_version,
                                    &root, &source, &directory)?;
                                Ok(vec![SkillInstallItem { target_host_id: host.clone(), directory,
                                    status: SkillInstallStatus::Installed }])
                            }
                            SkillInstallSource::Archive { .. } => {
                                let report = store.install_from_zip(&project, &host, &root, &source)?;
                                let mut results = Vec::new();
                                for skill in report.installed {
                                    results.push(SkillInstallItem { target_host_id: host.clone(),
                                        directory: skill.directory, status: SkillInstallStatus::Installed });
                                }
                                for skill in report.skipped {
                                    results.push(SkillInstallItem { target_host_id: host.clone(),
                                        directory: skill.directory, status: SkillInstallStatus::Skipped });
                                }
                                for skill in report.failed {
                                    results.push(SkillInstallItem { target_host_id: host.clone(),
                                        directory: skill.directory, status: SkillInstallStatus::Failed });
                                }
                                Ok(results)
                            }
                            SkillInstallSource::Git { .. } => anyhow::bail!("invalid file installation source"),
                        }
                    }).map_err(|_| StatusCode::CONFLICT)?;
                    Ok(InstallSkillResponse { scope: request.scope, task_id: job.task_id, results })
                }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = (database, project, request, root, source, principal); Err(StatusCode::NOT_IMPLEMENTED) }
}
