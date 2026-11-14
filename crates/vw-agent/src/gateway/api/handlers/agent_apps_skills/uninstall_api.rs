//! Backup-first removal through the existing scoped settings transaction.
use axum::{Extension, Json, Router, extract::{Path, DefaultBodyLimit},
    http::{HeaderMap, StatusCode}, routing::delete};
use vw_api_types::{agent_apps_skills::uninstall_api::*, id::{ProjectScope, TenantScope}};
use crate::global_database::GlobalDatabase;

#[derive(Clone)]
struct UninstallPolicy(Option<std::path::PathBuf>);

pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from))
}

pub(super) fn router_with_policy(policy: Option<std::path::PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/skills/{id}", delete(uninstall))
        .layer(Extension(UninstallPolicy(policy)))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}

async fn uninstall(Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
    Extension(policy): Extension<UninstallPolicy>, headers: HeaderMap, Path(id): Path<String>,
    Json(request): Json<UninstallSkillRequest>,
) -> Result<Json<UninstallSkillResponse>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let project = ProjectScope::new(tenant, request.scope.project_id.clone())
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        #[cfg(target_os = "macos")]
        {
            use vw_agent_apps::{apps::discovery::read::resolve_cli_installation,
                skills::{app_paths, local_install::validate_directory}};
            use crate::gateway::agent_apps_skills_store::SkillInstallsStore;
            let (env, root) = super::super::agent_apps::authorize_skill_bindings(
                policy.0.as_deref(), &headers, &project, &request.scope.target_host_id)?;
            validate_directory(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
            let db = match db {
                Some(Extension(db)) => db,
                None => GlobalDatabase::for_home(&crate::global::paths().home)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            };
            let store = SkillInstallsStore::new(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let state = store.get(&project, &request.scope.target_host_id, &root)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            if !state.skills.iter().any(|skill| skill.directory == id) {
                return Err(StatusCode::NOT_FOUND);
            }
            if state.version != request.expected_version { return Err(StatusCode::CONFLICT); }
            let mut targets = std::collections::BTreeMap::new();
            for (key, binding) in &state.copy_bindings {
                if binding.directory != id { continue; }
                let Ok((installation, app, directory)) = serde_json::from_str::<(
                    String, vw_api_types::agent_apps::AppId, String)>(key) else { continue; };
                if directory != id { continue; }
                // Unresolvable installations remain preserved by the native owner.
                if !resolve_cli_installation(app, &env,
                    &vw_api_types::agent_apps::AppInstallationId(installation)).unwrap_or(false) {
                    continue;
                }
                if let Ok(path) = app_paths::resolve(&env.home, app, None,
                    &std::collections::BTreeMap::new(), &[&root]) {
                    targets.insert(key.clone(), path);
                }
            }
            // Derive a sibling from the authorized physical SSOT; no request path
            // can redirect backups. The native owner rejects links and overlaps.
            let backups = super::backups_api::backup_root(&root, &project, &request.scope.target_host_id)?;
            match std::fs::create_dir(&backups) {
                Ok(()) => {},
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {},
                Err(_) => return Err(StatusCode::CONFLICT),
            }
            let (state, result) = store.uninstall_skill(&project, &request.scope.target_host_id,
                request.expected_version, &root, &backups, &id, &targets)
                .map_err(|_| StatusCode::CONFLICT)?;
            Ok(Json(UninstallSkillResponse { scope: request.scope, version: state.version,
                backup_id: result.backup_id, backup_path: result.backup_path,
                preserved_paths: result.preserved_paths, cleanup_incomplete: result.cleanup_incomplete }))
        }
        #[cfg(not(target_os = "macos"))]
        {
            super::super::agent_apps::authorize_skill_install(policy.0.as_deref(), &headers,
                &project, &request.scope.target_host_id, None)?;
            let _ = (db, id);
            Err(StatusCode::NOT_IMPLEMENTED)
        }
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
