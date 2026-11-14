use axum::{Extension, Json, Router, extract::Query, http::{HeaderMap, StatusCode}, routing::get};
use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex}, time::{Duration, Instant}};
use vw_api_types::{ProjectScope, TenantScope, external_session::*};
use vw_agent_apps::sessions::catalog::{Catalog, CatalogQuery};

struct Snapshot {
    project: ProjectScope,
    principal: String,
    host: String,
    bindings: Vec<(SessionProvider, String, PathBuf)>,
    catalog: Catalog,
    groups: Vec<SessionCatalogGroup>,
    truncated: bool,
    created: Instant,
}
#[derive(Clone, Default)]
pub(super) struct Snapshots(Arc<Mutex<HashMap<String, Snapshot>>>);
#[derive(Clone)]
pub(super) struct Policy(pub(super) Option<PathBuf>);
pub(in super::super) fn router() -> Router {
    Router::new()
        .merge(super::usage_scan::router())
        .route("/external-sessions/markdown", axum::routing::post(super::markdown::read))
        .route("/external-sessions/compression/query", axum::routing::post(super::compression::read))
        .route("/external-sessions/compression", axum::routing::post(super::compression::write))
        .route("/external-sessions/history-options/query", axum::routing::post(super::maintenance::history_settings::read))
        .route("/external-sessions/history-options", axum::routing::post(super::maintenance::history_settings::write))
        .route("/external-sessions/history-backups/query", axum::routing::post(super::maintenance::history_settings::backups))
        .route("/external-sessions/maintenance", axum::routing::post(super::maintenance::submit))
        .route("/external-sessions/maintenance/query", axum::routing::post(super::maintenance::query))
        .route("/external-sessions/maintenance/cancel", axum::routing::post(super::maintenance::cancel))
        .layer(Extension(super::maintenance::Jobs::default()))
        .route("/external-sessions/resume/command", axum::routing::post(super::resume::preview))
        .route("/external-sessions/resume", axum::routing::post(super::resume::resume))
        .route("/external-sessions/catalog", get(list))
        .route("/external-sessions/delete", axum::routing::post(super::delete::single))
        .route("/external-sessions/delete-batch", axum::routing::post(super::delete::batch))
        .route("/external-sessions/images", axum::routing::post(super::images::read))
        .route("/external-sessions/content", axum::routing::post(super::content::read))
        .route("/external-sessions/messages", axum::routing::post(super::messages::read))
        .route("/external-sessions/stream", axum::routing::post(super::stream::read))
        .route("/external-sessions/stream/cancel", axum::routing::post(super::stream::cancel))
        .layer(Extension(super::stream::Reads::default()))
        .layer(Extension(Snapshots::default()))
        .layer(Extension(Policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))))
}
async fn list(
    Extension(policy): Extension<Policy>, Extension(snapshots): Extension<Snapshots>,
    Extension(tenant): Extension<TenantScope>, headers: HeaderMap,
    Query(request): Query<SessionCatalogQuery>,
) -> Result<Json<SessionCatalogPage>, StatusCode> {
    tokio::task::spawn_blocking(move || execute(policy, snapshots, tenant, headers, request))
        .await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
fn execute(policy: Policy, snapshots: Snapshots, tenant: TenantScope, headers: HeaderMap,
    request: SessionCatalogQuery) -> Result<Json<SessionCatalogPage>, StatusCode> {
    let project = ProjectScope::new(tenant, request.project_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let (principal, home, instances) = super::super::agent_apps::authorize_external_history(
        policy.0.as_deref(), &headers, &project, &request.target_host_id)?;
    let instances: Vec<_> = instances.into_iter().filter(|i|
        request.provider.is_none_or(|p| p == i.provider)
        && request.instance_id.as_ref().is_none_or(|id| id == &i.instance_id)).collect();
    if instances.is_empty() { return Err(StatusCode::FORBIDDEN); }
    let bindings: Vec<_> = instances.iter().map(|i| (i.provider, i.instance_id.clone(), i.root.clone())).collect();
    let query = CatalogQuery { provider: request.provider, target_host: Some(request.target_host_id.clone()),
        instance_id: request.instance_id, project_dir: request.project_dir, search: request.search, page_size: request.page_size };
    if query.page_size == 0 || query.page_size > 200 || query.search.len() > 4096 { return Err(StatusCode::BAD_REQUEST); }
    let mut store = snapshots.0.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    store.retain(|_, s| s.created.elapsed() < Duration::from_secs(300));
    let (key, cursor) = match request.cursor.as_deref() {
        Some(token) => {
            let (key, cursor) = token.split_once(':').ok_or(StatusCode::GONE)?;
            (key.to_owned(), Some(cursor))
        }
        None => {
            if store.len() >= 128 { return Err(StatusCode::TOO_MANY_REQUESTS); }
            let mut batches = Vec::new();
            let mut groups: Vec<SessionCatalogGroup> = Vec::new();
            let mut truncated = false;
            for instance in instances {
                let identity = ExternalSessionIdentity { scope: project.clone(), target_host: request.target_host_id.clone(),
                    instance_id: instance.instance_id, provider: instance.provider, session_id: ExternalSessionId("scan".into()),
                    source: SessionSource::File { relative_path: "scan".into() } };
                let (sessions, limited) = super::scan::scan(&instance.root, &home, &identity)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                truncated |= limited;
                for s in &sessions {
                    if let Some(group) = groups.iter_mut().find(|g| g.provider == s.identity.provider
                        && g.instance_id == s.identity.instance_id && g.project_dir == s.project_dir) { group.count += 1; }
                    else { groups.push(SessionCatalogGroup { provider: s.identity.provider, instance_id: s.identity.instance_id.clone(), project_dir: s.project_dir.clone(), count: 1 }); }
                }
                batches.push(Ok(sessions));
            }
            let catalog = Catalog::from_batches(project.clone(), batches).map_err(error_status)?;
            let key = uuid::Uuid::new_v4().to_string();
            store.insert(key.clone(), Snapshot { project: project.clone(), principal: principal.clone(), host: request.target_host_id.clone(),
                bindings: bindings.clone(), catalog, groups, truncated, created: Instant::now() });
            (key, None)
        }
    };
    let snapshot = store.get_mut(&key).ok_or(StatusCode::GONE)?;
    if snapshot.project != project || snapshot.principal != principal || snapshot.host != request.target_host_id
        || snapshot.bindings != bindings { return Err(StatusCode::FORBIDDEN); }
    let page = snapshot.catalog.page(&project, &query, cursor).map_err(error_status)?;
    Ok(Json(SessionCatalogPage { sessions: page.sessions, total: page.total,
        next_cursor: page.next_cursor.map(|c| format!("{key}:{c}")), groups: snapshot.groups.clone(), truncated: snapshot.truncated }))
}
fn error_status(code: ExternalSessionErrorCode) -> StatusCode {
    match code {
        ExternalSessionErrorCode::Forbidden => StatusCode::FORBIDDEN,
        ExternalSessionErrorCode::InvalidInput => StatusCode::BAD_REQUEST,
        ExternalSessionErrorCode::CursorExpired => StatusCode::GONE,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

impl Snapshots {
    /// A deletion attempt may partially mutate native metadata before returning
    /// an error. Its invalidation event expires every affected directory cursor.
    pub(super) fn deletion_attempted(&self, identity: &ExternalSessionIdentity) -> Result<(), StatusCode> {
        self.0.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.retain(|_, snapshot|
            !(snapshot.project == identity.scope && snapshot.host == identity.target_host
              && snapshot.bindings.iter().any(|(provider, instance, _)|
                  *provider == identity.provider && instance == &identity.instance_id)));
        Ok(())
    }
}
