//! Opaque, expiring selections bind discovery to tenant/project/host and content.
use axum::{Extension, Json, Router, extract::{Query, DefaultBodyLimit},
    http::{HeaderMap, StatusCode}, routing::{get, post}};
use vw_api_types::{agent_apps_skills::import_api::*, id::{ProjectScope, TenantScope}};
use crate::global_database::GlobalDatabase;

pub(crate) fn router() -> Router {
    Router::new()
        .route("/agent-apps/skills/unmanaged", get(scan))
        .route("/agent-apps/skills/import", post(import))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}

async fn scan(Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, Query(query): Query<UnmanagedSkillsQuery>,
) -> Result<Json<UnmanagedSkillsResponse>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let project = ProjectScope::new(tenant, query.project_id.clone())
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
        let (home, root) = super::super::agent_apps::authorize_skill_import(
            policy.as_deref(), &headers, &project, &query.target_host_id)?;
        #[cfg(target_os = "macos")]
        { native::scan(project, db, query, home, root).map(Json) }
        #[cfg(not(target_os = "macos"))]
        { let _ = (db, query, home, root); Err(StatusCode::NOT_IMPLEMENTED) }
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

async fn import(Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, Json(request): Json<ImportSkillRequest>,
) -> Result<Json<ImportSkillResponse>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let project = ProjectScope::new(tenant, request.scope.project_id.clone())
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
        let (home, root) = super::super::agent_apps::authorize_skill_import(
            policy.as_deref(), &headers, &project, &request.scope.target_host_id)?;
        #[cfg(target_os = "macos")]
        { native::import(project, db, request, home, root).map(Json) }
        #[cfg(not(target_os = "macos"))]
        { let _ = (db, request, home, root); Err(StatusCode::NOT_IMPLEMENTED) }
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use std::{collections::BTreeMap, path::PathBuf, sync::{LazyLock, Mutex}, time::{Duration, Instant}};
    use vw_agent_apps::skills::{app_paths::{self, AppSkillsPath}, unmanaged_import::UnmanagedSelection,
        unmanaged_scan::scan_unmanaged};
    use vw_api_types::agent_apps::AppId;
    use crate::gateway::agent_apps_skills_store::SkillInstallsStore;

    #[derive(Clone)]
    struct Selection {
        scope: String,
        home: PathBuf,
        root: PathBuf,
        version: u64,
        targets: Vec<AppSkillsPath>,
        selection: UnmanagedSelection,
        created: Instant,
    }
    static SELECTIONS: LazyLock<Mutex<BTreeMap<String, Selection>>> =
        LazyLock::new(|| Mutex::new(BTreeMap::new()));

    fn scope_key(project: &ProjectScope, host: &str) -> Result<String, StatusCode> {
        serde_json::to_string(&(project.tenant_scope(), project.project_id(), host))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
    fn store(db: Option<Extension<GlobalDatabase>>) -> Result<SkillInstallsStore, StatusCode> {
        let db = match db {
            Some(Extension(db)) => db,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        };
        SkillInstallsStore::new(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
    }
    pub(super) fn scan(project: ProjectScope, db: Option<Extension<GlobalDatabase>>,
        query: UnmanagedSkillsQuery, home: PathBuf, root: PathBuf,
    ) -> Result<UnmanagedSkillsResponse, StatusCode> {
        if query.apps.is_empty() || query.apps.len() > 256 { return Err(StatusCode::BAD_REQUEST); }
        let mut targets = Vec::new();
        for app in query.apps.split(',') {
            let app: AppId = serde_json::from_value(serde_json::Value::String(app.to_owned()))
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            if targets.iter().any(|target: &AppSkillsPath| target.app_id == app) {
                return Err(StatusCode::BAD_REQUEST);
            }
            targets.push(app_paths::resolve(&home, app, None, &BTreeMap::new(), &[&root])
                .map_err(|_| StatusCode::BAD_REQUEST)?);
        }
        let state = store(db)?.get(&project, &query.target_host_id, &root)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let rows = scan_unmanaged(&home, &root, &state.skills, &targets)
            .map_err(|_| StatusCode::CONFLICT)?;
        let scope = scope_key(&project, &query.target_host_id)?;
        let mut cache = SELECTIONS.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        cache.retain(|_, item| item.created.elapsed() < Duration::from_secs(600));
        let candidates = rows.iter().filter(|row| !row.conflict && !row.external_link
            && row.content_hash.is_some()).count();
        if cache.len() + candidates > 4096 { return Err(StatusCode::SERVICE_UNAVAILABLE); }
        let mut skills = Vec::new();
        for row in rows {
            let selection_id = if !row.conflict && !row.external_link {
                row.content_hash.as_ref().map(|hash| {
                    let id = uuid::Uuid::new_v4().to_string();
                    cache.insert(id.clone(), Selection { scope: scope.clone(), home: home.clone(),
                        root: root.clone(), version: state.version, targets: targets.clone(),
                        selection: UnmanagedSelection { directory: row.directory.clone(),
                            content_hash: hash.clone(), sources: row.found_in.clone() },
                        created: Instant::now() });
                    id
                })
            } else { None };
            skills.push(UnmanagedSkillItem { selection_id, directory: row.directory, name: row.name,
                content_hash: row.content_hash, apps: row.found_in.iter().map(|s| s.app_id).collect(),
                managed: row.managed, conflict: row.conflict, external_link: row.external_link });
        }
        Ok(UnmanagedSkillsResponse { scope: vw_api_types::agent_apps_skills::repos_api::SkillReposQuery { project_id: query.project_id, target_host_id: query.target_host_id }, version: state.version, skills })
    }
    pub(super) fn import(project: ProjectScope, db: Option<Extension<GlobalDatabase>>,
        request: ImportSkillRequest, home: PathBuf, root: PathBuf,
    ) -> Result<ImportSkillResponse, StatusCode> {
        let mut snapshot = SELECTIONS.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .get(&request.selection_id).cloned().ok_or(StatusCode::CONFLICT)?;
        if snapshot.created.elapsed() >= Duration::from_secs(600)
            || snapshot.scope != scope_key(&project, &request.scope.target_host_id)?
            || snapshot.home != home || snapshot.root != root
            || snapshot.version != request.expected_version {
            return Err(StatusCode::CONFLICT);
        }
        if request.apps.is_empty() || request.apps.iter().enumerate().any(|(i, app)|
            request.apps[..i].contains(app) || !snapshot.selection.sources.iter().any(|s| s.app_id == *app)) {
            return Err(StatusCode::BAD_REQUEST);
        }
        snapshot.selection.sources.retain(|source| request.apps.contains(&source.app_id));
        snapshot.targets.retain(|target| request.apps.contains(&target.app_id));
        let state = store(db)?.import_unmanaged(&project, &request.scope.target_host_id,
            request.expected_version, &home, &root, &snapshot.targets, &snapshot.selection)
            .map_err(|_| StatusCode::CONFLICT)?;
        Ok(ImportSkillResponse { scope: request.scope, version: state.version,
            directory: snapshot.selection.directory })
    }
}
