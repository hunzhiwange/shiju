//! Execution-host storage confirmation; callers never supply filesystem paths.
use axum::{Extension, Json, Router, extract::{Query, DefaultBodyLimit}, http::{HeaderMap, StatusCode}, routing::{get, post}};
use vw_api_types::{agent_apps_skills::{storage_api::*, repos_api::SkillReposQuery}, id::{ProjectScope, TenantScope}};
use crate::global_database::GlobalDatabase;

pub(crate) fn router() -> Router {
    Router::new()
        .route("/agent-apps/skills/storage", get(read).patch(patch))
        .route("/agent-apps/skills/storage/move", post(move_storage))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}
enum Action { Read, Patch(SkillStoragePatch), Move(SkillStorageMoveRequest) }
async fn read(Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, Query(scope): Query<SkillReposQuery>,
) -> Result<Json<SkillStorageResponse>, StatusCode> {
    execute(tenant, db, headers, scope, Action::Read).await.map(|(state, _)| Json(state))
}
async fn patch(Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, Json(request): Json<SkillStoragePatch>,
) -> Result<Json<SkillStorageResponse>, StatusCode> {
    execute(tenant, db, headers, request.scope.clone(), Action::Patch(request)).await.map(|(state, _)| Json(state))
}
async fn move_storage(Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, Json(request): Json<SkillStorageMoveRequest>,
) -> Result<Json<SkillStorageMoveResponse>, StatusCode> {
    execute(tenant, db, headers, request.scope.clone(), Action::Move(request)).await?
        .1.map(Json).ok_or(StatusCode::INTERNAL_SERVER_ERROR)
}
async fn execute(tenant: TenantScope, db: Option<Extension<GlobalDatabase>>, headers: HeaderMap,
    scope: SkillReposQuery, action: Action,
) -> Result<(SkillStorageResponse, Option<SkillStorageMoveResponse>), StatusCode> {
    tokio::task::spawn_blocking(move || {
        let project = ProjectScope::new(tenant, scope.project_id.clone()).map_err(|_| StatusCode::BAD_REQUEST)?;
        let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
        #[cfg(target_os = "macos")]
        {
            use vw_agent_apps::{apps::discovery::read::resolve_cli_installation,
                skills::{app_paths, bulk_sync::SkillSyncTarget, link_deploy::SyncMethod}};
            use crate::gateway::agent_apps_skills_store::SkillInstallsStore;
            // Both inventory and install authority are required to expose and change roots.
            super::super::agent_apps::authorize_skill_inventory(policy.as_deref(), &headers, &project, &scope.target_host_id)?;
            let (env, runtime) = super::super::agent_apps::authorize_skill_bindings(policy.as_deref(), &headers, &project, &scope.target_host_id)?;
            let (_, principal) = super::super::agent_apps::authorize_skill_job(policy.as_deref(), &headers, &project, &scope.target_host_id)?;
            let db = match db { Some(Extension(db)) => db, None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)? };
            let store = SkillInstallsStore::new(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let mut state = store.storage_state(&project, &scope.target_host_id, &runtime, &env.home)
                .map_err(|_| StatusCode::CONFLICT)?;
            let mut migration = None;
            match action {
                Action::Read => {},
                Action::Patch(request) => {
                    let mode = match request.mode { vw_api_types::agent_apps_skills::bindings_api::SkillBindingMode::Auto => SyncMethod::Auto,
                        vw_api_types::agent_apps_skills::bindings_api::SkillBindingMode::Copy => SyncMethod::Copy,
                        vw_api_types::agent_apps_skills::bindings_api::SkillBindingMode::Symlink => SyncMethod::Symlink };
                    state = store.save_storage_mode(&project, &scope.target_host_id, request.expected_version, &state.ssot_root, mode)
                        .map_err(|_| StatusCode::CONFLICT)?;
                },
                Action::Move(request) => {
                    if request.targets.len() > 64 { return Err(StatusCode::BAD_REQUEST); }
                    if request.expected_version != state.version { return Err(StatusCode::CONFLICT); }
                    let destination = match request.location { SkillStorageLocation::Runtime => runtime.clone(),
                        SkillStorageLocation::Agents => env.home.join(".agents/skills") };
                    let mut targets = Vec::new();
                    for (index, target) in request.targets.iter().enumerate() {
                        if request.targets[..index].contains(target) { return Err(StatusCode::BAD_REQUEST); }
                        if !resolve_cli_installation(target.app_id, &env, &vw_api_types::agent_apps::AppInstallationId(target.installation_id.clone()))
                            .map_err(|_| StatusCode::CONFLICT)? { return Err(StatusCode::NOT_FOUND); }
                        let path = app_paths::resolve(&env.home, target.app_id, None, &std::collections::BTreeMap::new(), &[&state.ssot_root, &destination])
                            .map_err(|_| StatusCode::BAD_REQUEST)?;
                        targets.push(SkillSyncTarget { installation_id: target.installation_id.clone(), path });
                    }
                    let (job, result) = store.run_file_install(&project, &scope.target_host_id, &state.ssot_root, &principal, ||
                        store.migrate_storage(&project, &scope.target_host_id, request.expected_version,
                            &state.ssot_root, &destination, &env.home, &runtime, &targets))
                        .map_err(|_| StatusCode::CONFLICT)?;
                    migration = Some((job.task_id, result));
                    state = store.storage_state(&project, &scope.target_host_id, &runtime, &env.home).map_err(|_| StatusCode::CONFLICT)?;
                },
            }
            use vw_api_types::agent_apps_skills::bindings_api::SkillBindingMode;
            let storage = SkillStorageResponse { scope, version: state.version,
                root: state.ssot_root.to_string_lossy().into_owned(), mode: match state.storage_mode {
                    SyncMethod::Auto => SkillBindingMode::Auto, SyncMethod::Copy => SkillBindingMode::Copy, SyncMethod::Symlink => SkillBindingMode::Symlink } };
            let result = migration.map(|(task_id, result)| SkillStorageMoveResponse { storage: storage.clone(), task_id,
                migrated: result.migrated, skipped: result.skipped, errors: result.errors, switched: result.switched });
            Ok((storage, result))
        }
        #[cfg(not(target_os = "macos"))]
        {
            super::super::agent_apps::authorize_skill_inventory(policy.as_deref(), &headers, &project, &scope.target_host_id)?;
            let _ = (db, action);
            Err(StatusCode::NOT_IMPLEMENTED)
        }
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
