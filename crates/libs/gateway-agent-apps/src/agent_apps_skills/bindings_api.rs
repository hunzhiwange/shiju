//! Non-atomic distribution preserves every pair's outcome.
use axum::{Extension, Json, Router, extract::{Path, DefaultBodyLimit},
    http::{HeaderMap, StatusCode}, routing::{post, get}};
use {
    vw_api_app_skills::bindings_api::*,
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
};
use agent_database::GlobalDatabase;

#[cfg(target_os = "macos")]
pub(super) fn authorize_target(
    target: &SkillBindingTarget,
    env: &vw_agent_apps::apps::discovery::read::LocalReadEnvironment,
) -> Result<(), StatusCode> {
    use vw_agent_apps::{
        apps::discovery::read::resolve_cli_installation, skills::bulk_sync::SkillInstallTarget,
    };
    SkillInstallTarget { installation_id: target.installation_id.clone(), app_id: target.app_id }
        .validate().map_err(|_| StatusCode::BAD_REQUEST)?;
    if target.installation_id == DEFAULT_SKILL_DIRECTORY
        || resolve_cli_installation(target.app_id, env,
            &vw_api_app_identity::AppInstallationId(target.installation_id.clone()))
            .map_err(|_| StatusCode::CONFLICT)?
    {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub fn router() -> Router {
    Router::new()
        .route("/agent-apps/skills/binding-targets", get(snapshot).put(preferences))
        .route("/agent-apps/skills/{id}/bindings", post(bind))
        .route("/agent-apps/skills/resync", post(resync))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}
async fn bind(Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, Path(id): Path<String>, Json(request): Json<SkillBindingsRequest>,
) -> Result<Json<SkillBindingsResponse>, StatusCode> {
    execute(tenant, db, headers, request.scope, request.targets,
        Some((id, request.enabled, request.mode))).await.map(Json)
}
async fn resync(Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, Json(request): Json<SkillResyncRequest>,
) -> Result<Json<SkillBindingsResponse>, StatusCode> {
    execute(tenant, db, headers, request.scope, request.targets, None).await.map(Json)
}
async fn execute(tenant: TenantScope, db: Option<Extension<GlobalDatabase>>, headers: HeaderMap,
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    targets: Vec<SkillBindingTarget>, binding: Option<(String, bool, SkillBindingMode)>,
) -> Result<SkillBindingsResponse, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let project = ProjectScope::new(tenant, scope.project_id.clone())
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
        #[cfg(target_os = "macos")]
        {
            use vw_agent_apps::skills::{app_paths,
                bulk_sync::{SkillSyncTarget, SkillSyncStatus, prepare_default_directory},
                link_deploy::SyncMethod, local_install::validate_directory};
            use app_settings_store::agent_apps_skills_store::SkillInstallsStore;
            let (env, root) = app_access_service::access::authorize_skill_bindings(
                policy.as_deref(), &headers, &project, &scope.target_host_id)?;
            if targets.is_empty() || targets.len() > 64 { return Err(StatusCode::BAD_REQUEST); }
            if let Some((id, _, _)) = &binding {
                validate_directory(id).map_err(|_| StatusCode::BAD_REQUEST)?;
            }
            // Revalidate every installation before any storage or publication work.
            let mut resolved = Vec::new();
            for (index, target) in targets.iter().enumerate() {
                if targets[..index].contains(target) { return Err(StatusCode::BAD_REQUEST); }
                authorize_target(target, &env)?;
                let path = app_paths::resolve(&env.home, target.app_id, None,
                    &std::collections::BTreeMap::new(), &[&root])
                    .map_err(|_| StatusCode::BAD_REQUEST)?;
                resolved.push(SkillSyncTarget { installation_id: target.installation_id.clone(), path });
            }
            let db = match db {
                Some(Extension(db)) => db,
                None => GlobalDatabase::for_home(&agent_paths::paths().home)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            };
            let store = SkillInstallsStore::new(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let result = match binding {
                Some((id, enabled, mode)) => {
                    if enabled && resolved.iter().any(|target| target.installation_id == DEFAULT_SKILL_DIRECTORY) {
                        let state = store.get(&project, &scope.target_host_id, &root)
                            .map_err(|_| StatusCode::CONFLICT)?;
                        if !state.skills.iter().any(|skill| skill.directory == id) {
                            return Err(StatusCode::NOT_FOUND);
                        }
                        for target in &resolved {
                            if target.installation_id == DEFAULT_SKILL_DIRECTORY {
                                prepare_default_directory(&env.home, &target.path)
                                    .map_err(|_| StatusCode::CONFLICT)?;
                            }
                        }
                    }
                    let method = match mode { SkillBindingMode::Auto => SyncMethod::Auto,
                        SkillBindingMode::Symlink => SyncMethod::Symlink, SkillBindingMode::Copy => SyncMethod::Copy };
                    store.bulk_toggle(&project, &scope.target_host_id, &root, &[id], &resolved, enabled, method)
                }
                None => store.resync_all_apps(&project, &scope.target_host_id, &root, &resolved),
            }.map_err(|_| StatusCode::CONFLICT)?;
            Ok(SkillBindingsResponse { scope, results: result.items.into_iter().map(|item| SkillBindingItem {
                target: SkillBindingTarget { installation_id: item.target.installation_id, app_id: item.target.app_id },
                directory: item.directory,
                status: match item.status { SkillSyncStatus::Enabled => SkillBindingStatus::Enabled,
                    SkillSyncStatus::Disabled => SkillBindingStatus::Disabled, SkillSyncStatus::Failed => SkillBindingStatus::Failed },
            }).collect() })
        }
        #[cfg(not(target_os = "macos"))]
        {
            app_access_service::access::authorize_skill_install(policy.as_deref(), &headers,
                &project, &scope.target_host_id, None)?;
            let _ = (db, targets, binding);
            Err(StatusCode::NOT_IMPLEMENTED)
        }
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

async fn snapshot(Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, axum::extract::Query(scope): axum::extract::Query<vw_api_app_skills::repos_api::SkillReposQuery>,
) -> Result<Json<SkillBindingSnapshot>, StatusCode> {
    binding_settings(tenant, db, headers, scope, None).await.map(Json)
}
async fn preferences(Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, Json(request): Json<SkillBindingPreferences>,
) -> Result<Json<SkillBindingSnapshot>, StatusCode> {
    binding_settings(tenant, db, headers, request.scope, Some((request.expected_version, request.targets))).await.map(Json)
}
async fn binding_settings(tenant: TenantScope, db: Option<Extension<GlobalDatabase>>, headers: HeaderMap,
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    change: Option<(u64, Vec<SkillBindingTarget>)>,
) -> Result<SkillBindingSnapshot, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let project = ProjectScope::new(tenant, scope.project_id.clone()).map_err(|_| StatusCode::BAD_REQUEST)?;
        let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
        #[cfg(target_os = "macos")]
        {
            let (env, root) = app_access_service::access::authorize_skill_bindings(
                policy.as_deref(), &headers, &project, &scope.target_host_id)?;
            use vw_agent_apps::skills::{app_paths::{distribution, SkillDistribution}, bulk_sync::SkillInstallTarget};
            use app_settings_store::agent_apps_skills_store::SkillInstallsStore;
            let db = match db { Some(Extension(db)) => db, None => GlobalDatabase::for_home(&agent_paths::paths().home)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)? };
            let store = SkillInstallsStore::new(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let state = if let Some((version, targets)) = change {
                for target in &targets {
                    authorize_target(target, &env)?;
                }
                store.save_install_targets(&project, &scope.target_host_id, version, &root,
                    targets.into_iter().map(|t| SkillInstallTarget { installation_id: t.installation_id, app_id: t.app_id }).collect())
            } else { store.get(&project, &scope.target_host_id, &root) }.map_err(|_| StatusCode::CONFLICT)?;
            use vw_api_app_identity::AppId::*;
            let capabilities = [Claude, ClaudeDesktop, Codex, Gemini, GrokBuild, OpenCode, OpenClaw, Hermes, Pi, Mcode, Kimi, Dsh, Agy, Omp]
                .into_iter().map(|app_id| {
                    let (supported, reason) = match distribution(app_id) {
                        SkillDistribution::NoDirectorySync => (false, "此应用不支持技能目录同步"),
                        SkillDistribution::PathOnly => (false, "仅提供路径解析，尚无技能安装入口"),
                        SkillDistribution::NativeDirectory => (true, "启用状态由执行主机原生目录决定"),
                        SkillDistribution::ManagedBinding => (true, "受管应用绑定"),
                    };
                    SkillBindingCapability { app_id, supported, reason: reason.into() }
                }).collect();
            let managed_bindings = state.copy_bindings.iter().filter_map(|(key, binding)| {
                let (installation_id, app_id, directory) =
                    serde_json::from_str::<(String, vw_api_app_identity::AppId, String)>(key).ok()?;
                if directory != binding.directory
                    || distribution(app_id) != SkillDistribution::ManagedBinding
                {
                    return None;
                }
                Some(ManagedSkillBinding { target: SkillBindingTarget { installation_id, app_id }, directory })
            }).collect();
            let mut native_bindings = Vec::new();
            for app_id in [Pi, Omp] {
                use vw_agent_apps::skills::app_paths;
                let Ok(path) = app_paths::resolve(&env.home, app_id, None, &Default::default(), &[&root]) else {
                    continue;
                };
                for skill in &state.skills {
                    if let Ok(enabled) = app_paths::native_enabled(&env.home, &path, &skill.directory) {
                        native_bindings.push(NativeSkillBinding {
                            app_id, directory: skill.directory.clone(), enabled,
                        });
                    }
                }
            }
            Ok(SkillBindingSnapshot { scope, version: state.version, capabilities,
                targets: state.install_targets.into_iter().map(|t| SkillBindingTarget { installation_id: t.installation_id, app_id: t.app_id }).collect(),
                managed_bindings: Some(managed_bindings),
                native_bindings,
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            app_access_service::access::authorize_skill_install(policy.as_deref(), &headers, &project, &scope.target_host_id, None)?;
            let _ = (db, change); Err(StatusCode::NOT_IMPLEMENTED)
        }
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
