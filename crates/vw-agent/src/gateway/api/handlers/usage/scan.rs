//! Named host operations. All target access follows authenticated scope checks.
use std::{collections::HashMap, path::PathBuf, sync::{Mutex, OnceLock}};
use axum::{Extension, Json, Router, extract::{Path, Query}, http::{HeaderMap, StatusCode}, routing::{get, post}};
use serde::Deserialize;
use serde_json::{Value, json};
use vw_agent_apps::{usage as u, apps::lifecycle::operations::{Operation, OperationFailure, Phase}};
use vw_api_types::{TenantScope, id::TaskId};
use super::super::external_sessions::usage_scan::{self as host, Intent};
use u::scan_jobs::ScanOwner;

#[derive(Clone)]
struct Policy(Option<PathBuf>);
struct Job { owner: ScanOwner, operation: Operation }
static JOBS: OnceLock<Mutex<HashMap<String, Job>>> = OnceLock::new();
fn jobs() -> &'static Mutex<HashMap<String, Job>> { JOBS.get_or_init(Default::default) }
type Response = Result<Json<Value>, StatusCode>;
fn unavailable<T>(_: T) -> StatusCode { StatusCode::SERVICE_UNAVAILABLE }

pub(super) fn router() -> Router {
    Router::new()
        .route("/usage/scans/last", get(last))
        .route("/usage/jobs/{id}", get(job))
        .route("/usage/jobs/{id}/cancel", post(cancel))
        .route("/usage/settings", get(settings).put(save_settings))
        .route("/usage/maintenance/codex-rebuild", post(rebuild))
        .route("/usage/maintenance/rollup", post(rollup))
        .route("/usage/maintenance/backfill", post(backfill))
        .layer(Extension(Policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))))
}
fn owner(policy: &Policy, tenant: TenantScope, headers: &HeaderMap, intent: &Intent) -> Result<ScanOwner, StatusCode> {
    host::authorized_owner(policy.0.as_deref(), tenant, headers, intent)
}
async fn last(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(i): Query<Intent>) -> Response {
    tokio::task::spawn_blocking(move || {
        let o = owner(&p,t,&h,&i)?;
        let schedules = host::schedules().lock().map_err(unavailable)?;
        let timestamp = schedules.get(&o).map(|s| s.last_complete_scan()).transpose().map_err(unavailable)?.flatten();
        Ok(Json(json!({"last_complete_scan": timestamp})))
    }).await.map_err(unavailable)?
}
async fn job(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Path(id): Path<String>, Query(i): Query<Intent>) -> Response {
    lookup(p,t,h,i,id,false).await
}
async fn cancel(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Path(id): Path<String>, Json(i): Json<Intent>) -> Response {
    lookup(p,t,h,i,id,true).await
}
async fn lookup(p: Policy,t: TenantScope,h: HeaderMap,i: Intent,id: String,cancel: bool) -> Response {
    tokio::task::spawn_blocking(move || {
        let o = owner(&p,t,&h,&i)?;
        let map = jobs().lock().map_err(unavailable)?;
        if let Some(j) = map.get(&id).filter(|j| j.owner == o) {
            super::super::agent_apps::authorize_external_history_maintenance(p.0.as_deref(),&h,&o.scope,&o.target_host)?;
            if cancel { j.operation.cancel(); }
            let progress = j.operation.snapshot();
            return Ok(Json(json!({"task_id":id,"status":progress.status,"phase":format!("{:?}",progress.phase)})));
        }
        drop(map);
        let schedules = host::schedules().lock().map_err(unavailable)?;
        let schedule = schedules.get(&o).ok_or(StatusCode::NOT_FOUND)?;
        let id = TaskId::from(id);
        let result = if cancel { schedule.cancel(&id) } else { schedule.job(&id) };
        Ok(Json(serde_json::to_value(result.map_err(|_| StatusCode::NOT_FOUND)?).map_err(unavailable)?))
    }).await.map_err(unavailable)?
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsUpdate { project_id: vw_api_types::id::ProjectId, target_host: String, app: String, expected_revision: u64, settings: u::source_settings::UsageSourceSettings }
async fn settings(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(i): Query<Intent>) -> Response {
    tokio::task::spawn_blocking(move || {
        let o = owner(&p,t,&h,&i)?;
        let db = crate::usage_bridge::context::database().map_err(unavailable)?;
        let connection = db.connect_sqlite().map_err(unavailable)?;
        u::source_settings::initialize_source_settings(&connection).map_err(unavailable)?;
        let mut rows = Vec::new();
        for (_,app,_) in host::ADAPTERS {
            let state = u::source_settings::load_source_settings(&connection,&o.scope,&o.target_host,app).map_err(unavailable)?;
            rows.push(json!({"app":app,"settings":state.settings,"revision":state.settings_revision}));
        }
        Ok(Json(json!(rows)))
    }).await.map_err(unavailable)?
}
async fn save_settings(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Json(r): Json<SettingsUpdate>) -> Response {
    tokio::task::spawn_blocking(move || {
        let i = Intent {project_id:r.project_id,target_host:r.target_host,active:None,paused:None};
        let o = owner(&p,t,&h,&i)?;
        // Writing settings requires the explicit maintenance grant as well.
        super::super::agent_apps::authorize_external_history_maintenance(p.0.as_deref(),&h,&o.scope,&o.target_host)?;
        if !host::ADAPTERS.iter().any(|(_,app,_)| *app == r.app) { return Err(StatusCode::BAD_REQUEST); }
        let db = crate::usage_bridge::context::database().map_err(unavailable)?;
        let mut c = db.connect_sqlite().map_err(unavailable)?;
        u::source_settings::initialize_source_settings(&c).map_err(unavailable)?;
        let s = u::source_settings::save_source_settings(&mut c,&o.scope,&o.target_host,&r.app,r.expected_revision,r.settings).map_err(|e| match e {
            u::source_settings::SourceSettingsError::RevisionConflict => StatusCode::CONFLICT,
            u::source_settings::SourceSettingsError::InvalidInput => StatusCode::BAD_REQUEST,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        })?;
        Ok(Json(json!({"settings":s.settings,"revision":s.settings_revision})))
    }).await.map_err(unavailable)?
}
#[derive(Clone, Copy)]
enum Maintenance { Rebuild, Rollup, Backfill }
macro_rules! submit_handler { ($name:ident,$kind:ident) => {
    async fn $name(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Json(i): Json<Intent>) -> Result<(StatusCode,Json<Value>),StatusCode> {
        submit(p,t,h,i,Maintenance::$kind).await
    }
}; }
submit_handler!(rebuild,Rebuild);
submit_handler!(rollup,Rollup);
submit_handler!(backfill,Backfill);
async fn submit(p: Policy,t: TenantScope,h: HeaderMap,i: Intent,kind: Maintenance) -> Result<(StatusCode,Json<Value>),StatusCode> {
    let (o,home,instances) = tokio::task::spawn_blocking(move || {
        let o = owner(&p,t,&h,&i)?;
        let (_,home,instances,_) = super::super::agent_apps::authorize_external_history_maintenance(p.0.as_deref(),&h,&o.scope,&o.target_host)?;
        Ok::<_,StatusCode>((o,home,instances))
    }).await.map_err(unavailable)??;
    let id = uuid::Uuid::new_v4().to_string();
    let operation = Operation::new();
    let worker = operation.clone();
    let mut map = jobs().lock().map_err(unavailable)?;
    if map.len() >= 64 {
        let terminal = map.iter().find(|(_,j)| j.operation.snapshot().phase == Phase::Terminal).map(|(id,_)| id.clone());
        if let Some(id) = terminal { map.remove(&id); }
        else { return Err(StatusCode::TOO_MANY_REQUESTS); }
    }
    if map.values().any(|j| j.owner.scope == o.scope && j.owner.target_host == o.target_host && j.operation.snapshot().phase != Phase::Terminal) { return Err(StatusCode::CONFLICT); }
    map.insert(id.clone(),Job {owner:o.clone(),operation:operation.clone()});
    if operation.start_task(async move {
        tokio::task::spawn_blocking(move || {
            let result = execute(&o,&home,instances,kind,&worker);
            if worker.is_cancelled() { Err(OperationFailure::Cancelled) }
            else { result.map_err(|_| OperationFailure::Failed) }
        }).await.map_err(|_| OperationFailure::Failed)?
    }).is_err() { map.remove(&id); return Err(StatusCode::SERVICE_UNAVAILABLE); }
    Ok((StatusCode::ACCEPTED,Json(json!({"task_id":id,"status":"queued"}))))
}
fn execute(o: &ScanOwner, home: &std::path::Path, instances: Vec<super::super::agent_apps::ExternalHistoryInstance>, kind: Maintenance, operation: &Operation) -> anyhow::Result<()> {
    anyhow::ensure!(!operation.is_cancelled(),"cancelled");
    let db = crate::usage_bridge::context::database()?;
    let mut c = db.connect_sqlite()?;
    u::store::initialize_usage_store(&mut c)?;
    u::source_settings::initialize_source_settings(&c)?;
    u::cursors::initialize_usage_cursors(&c)?;
    match kind {
        Maintenance::Rollup => { u::retention::rollup_and_prune(&mut c,&o.scope,&o.target_host,chrono::Utc::now().timestamp(),|| operation.is_cancelled())?; }
        Maintenance::Backfill => {
            u::pricing::initialize_pricing_store(&c).map_err(anyhow::Error::msg)?;
            u::pricing_batch::backfill_unknown_costs(&mut c,&o.scope,&o.target_host,128,|| operation.is_cancelled()).map_err(anyhow::Error::msg)?;
        }
        Maintenance::Rebuild => {
            let settings = u::source_settings::load_source_settings(&c,&o.scope,&o.target_host,"codex")?;
            anyhow::ensure!(settings.settings.scan_enabled,"scan disabled");
            let project = crate::project::store::ProjectStore::new(db)?.get(&o.scope)?.ok_or_else(|| anyhow::anyhow!("project unavailable"))?;
            anyhow::ensure!(instances.iter().any(|i| i.provider == vw_api_types::external_session::SessionProvider::Codex),"Codex source not authorized");
            // Rebuild replaces all native Codex rows for this host/scope. Reject
            // partial instance authority before entering that destructive transaction.
            let scope_key = serde_json::to_string(&(&o.scope,&o.target_host))?;
            let mut statement = c.prepare("SELECT identity_key FROM usage_source_identities WHERE scope_key=?1")?;
            let identities = statement.query_map([scope_key],|r| r.get::<_,String>(0))?.collect::<Result<Vec<_>,_>>()?;
            drop(statement);
            for identity in identities {
                let identity: u::store::UsageSourceIdentity = serde_json::from_str(&identity)?;
                if identity.app == "codex" && identity.kind == vw_api_types::usage::UsageSourceKind::Native {
                    anyhow::ensure!(instances.iter().any(|i| i.provider == vw_api_types::external_session::SessionProvider::Codex && i.instance_id == identity.instance_id),"partial Codex authority");
                }
            }
            let instances = instances.into_iter().map(|i| (home.to_path_buf(),i)).collect::<Vec<_>>();
            let files = host::discover(&o.scope,&o.target_host,vw_api_types::external_session::SessionProvider::Codex,&instances,&project.worktree)?;
            let mut lineage = u::codex_lineage::CodexLineage::new(files.iter().map(|f| f.path.clone()).collect())?;
            u::codex_rebuild::rebuild_codex_usage(&mut c,&o.scope,&o.target_host,&mut lineage,operation,|writer,lineage| {
                for file in files {
                    let name = file.path.file_name().ok_or(u::codex_rebuild::CodexRebuildError::InvalidInput)?.to_string_lossy().into_owned();
                    let key = u::cursors::UsageCursorKey {target_host:o.target_host.clone(),app:"codex".into(),instance_id:file.identity.instance_id,file_identity:name};
                    u::snapshot_scan::replay_codex(writer,lineage,&key,&file.path)?;
                }
                Ok(())
            })?;
        }
    }
    Ok(())
}
