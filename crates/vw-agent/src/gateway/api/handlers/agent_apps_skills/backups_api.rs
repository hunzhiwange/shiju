//! Scoped backup operations; saved metadata never authorizes a destination.
use axum::{Extension, Json, Router, extract::{Path, Query, DefaultBodyLimit},
    http::{HeaderMap, StatusCode}, routing::{get, post}};
use vw_api_types::{agent_apps_skills::{backups_api::*, repos_api::SkillReposQuery},
    id::{ProjectScope, TenantScope}};
use crate::global_database::GlobalDatabase;

#[derive(Clone)]
struct BackupPolicy(Option<std::path::PathBuf>);
pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from))
}
pub(super) fn router_with_policy(policy: Option<std::path::PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/skills/backups", get(list))
        .route("/agent-apps/skills/backups/{id}", axum::routing::delete(delete))
        .route("/agent-apps/skills/backups/{id}/restore", post(restore))
        .layer(Extension(BackupPolicy(policy)))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}
#[derive(serde::Serialize)]
#[serde(untagged)]
enum Response { List(SkillBackupsResponse), Delete(DeleteSkillBackupResponse), Restore(RestoreSkillBackupResponse) }
enum Operation { List, Delete(String), Restore(String, u64, Vec<vw_api_types::agent_apps_skills::bindings_api::SkillBindingTarget>) }
async fn list(Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
    Extension(policy): Extension<BackupPolicy>, headers: HeaderMap, Query(scope): Query<SkillReposQuery>,
) -> Result<Json<Response>, StatusCode> { execute(tenant, db, policy, headers, scope, Operation::List).await }
async fn delete(Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
    Extension(policy): Extension<BackupPolicy>, headers: HeaderMap, Path(id): Path<String>,
    Json(request): Json<DeleteSkillBackupRequest>,
) -> Result<Json<Response>, StatusCode> { execute(tenant, db, policy, headers, request.scope, Operation::Delete(id)).await }
async fn restore(Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
    Extension(policy): Extension<BackupPolicy>, headers: HeaderMap, Path(id): Path<String>,
    Json(request): Json<RestoreSkillBackupRequest>,
) -> Result<Json<Response>, StatusCode> {
    execute(tenant, db, policy, headers, request.scope,
        Operation::Restore(id, request.expected_version, request.apps)).await
}
async fn execute(tenant: TenantScope, db: Option<Extension<GlobalDatabase>>, policy: BackupPolicy,
    headers: HeaderMap, scope: SkillReposQuery, operation: Operation,
) -> Result<Json<Response>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let project = ProjectScope::new(tenant, scope.project_id.clone()).map_err(|_| StatusCode::BAD_REQUEST)?;
        #[cfg(target_os = "macos")]
        {
            use vw_agent_apps::{skills::{backup_catalog, app_paths, bulk_sync::SkillInstallTarget},
                apps::discovery::read::resolve_cli_installation};
            let (env, root) = super::super::agent_apps::authorize_skill_bindings(
                policy.0.as_deref(), &headers, &project, &scope.target_host_id)?;
            let id = match &operation { Operation::List => None,
                Operation::Delete(id) | Operation::Restore(id, _, _) => Some(id) };
            if let Some(id) = id {
                if !id.starts_with("skill-uninstall-") || id.len() > 255
                    || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
            let backups = backup_root(&root, &project, &scope.target_host_id)?;
            let entries = match backup_catalog::list_backups(&backups) {
                Ok(entries) => entries,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
                Err(_) => return Err(StatusCode::CONFLICT),
            };
            match operation {
                Operation::List => Ok(Json(Response::List(SkillBackupsResponse { scope,
                    backups: entries.into_iter().map(|entry| SkillBackup { backup_id: entry.backup_id,
                        directory: entry.skill.directory, name: entry.skill.name, created_at: entry.created_at }).collect() }))),
                Operation::Delete(id) => {
                    if !entries.iter().any(|entry| entry.backup_id == id) { return Err(StatusCode::NOT_FOUND); }
                    backup_catalog::delete_backup(&backups, &id).map_err(|_| StatusCode::CONFLICT)?;
                    Ok(Json(Response::Delete(DeleteSkillBackupResponse { scope, backup_id: id })))
                }
                Operation::Restore(id, version, apps) => {
                    let entry = entries.iter().find(|entry| entry.backup_id == id).ok_or(StatusCode::NOT_FOUND)?;
                    if apps.len() > 64 { return Err(StatusCode::BAD_REQUEST); }
                    let mut targets = std::collections::BTreeMap::new();
                    for (index, target) in apps.iter().enumerate() {
                        if apps[..index].contains(target) { return Err(StatusCode::BAD_REQUEST); }
                        SkillInstallTarget { installation_id: target.installation_id.clone(), app_id: target.app_id }
                            .validate().map_err(|_| StatusCode::BAD_REQUEST)?;
                        if !resolve_cli_installation(target.app_id, &env,
                            &vw_api_types::agent_apps::AppInstallationId(target.installation_id.clone()))
                            .map_err(|_| StatusCode::CONFLICT)? { return Err(StatusCode::NOT_FOUND); }
                        let path = app_paths::resolve(&env.home, target.app_id, None,
                            &std::collections::BTreeMap::new(), &[&root]).map_err(|_| StatusCode::BAD_REQUEST)?;
                        let key = serde_json::to_string(&(target.installation_id.clone(), target.app_id, entry.skill.directory.clone()))
                            .map_err(|_| StatusCode::BAD_REQUEST)?;
                        targets.insert(key, path);
                    }
                    let db = match db { Some(Extension(db)) => db,
                        None => GlobalDatabase::for_home(&crate::global::paths().home).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)? };
                    let store = crate::gateway::agent_apps_skills_store::SkillInstallsStore::new(db)
                        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                    let (state, result) = store.restore_skill_backup(&project, &scope.target_host_id,
                        version, &root, &backups, &id, &targets).map_err(|_| StatusCode::CONFLICT)?;
                    Ok(Json(Response::Restore(RestoreSkillBackupResponse { scope, backup_id: id,
                        directory: result.skill.directory, version: state.version })))
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            super::super::agent_apps::authorize_skill_install(policy.0.as_deref(), &headers,
                &project, &scope.target_host_id, None)?;
            let _ = (db, operation);
            Err(StatusCode::NOT_IMPLEMENTED)
        }
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

/// Include the complete resource scope even when policies share one physical SSOT.
#[cfg(target_os = "macos")]
pub(super) fn backup_root(root: &std::path::Path, project: &ProjectScope, host: &str)
    -> Result<std::path::PathBuf, StatusCode> {
    use sha2::{Digest, Sha256};
    let identity = serde_json::to_vec(&(project.platform_id(), project.company_id(), project.project_id(), host))
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let physical = std::fs::canonicalize(root).map_err(|_| StatusCode::CONFLICT)?;
    let mut name = physical.file_name().ok_or(StatusCode::CONFLICT)?.to_os_string();
    name.push(format!(".backups-{}", hex::encode(Sha256::digest(identity))));
    Ok(physical.with_file_name(name))
}
