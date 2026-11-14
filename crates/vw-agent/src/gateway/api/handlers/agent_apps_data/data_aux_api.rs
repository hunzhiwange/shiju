//! Authorized portable downloads and host-local backup policy. No client paths.
use axum::{Extension, Json, Router, http::{HeaderMap, StatusCode, header}, routing::post, response::{IntoResponse, Response}};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex}, time::{Instant, Duration}};
use crate::{global_database::GlobalDatabase, gateway::settings_store::GatewaySettingsStore};
use vw_api_types::{agent_apps::{data_management::*, AppUserPreferences}, id::TenantScope};
use vw_agent_apps::data_management::{export, safety_snapshot::SafetySnapshotScope};
#[derive(Clone)]
struct State { policy: Option<PathBuf>, downloads: Arc<Mutex<HashMap<String, Download>>> }
struct Download { target: DataManagementTarget, key: String, created: Instant, bytes: Vec<u8> }
#[derive(Deserialize)]
#[serde(rename_all="camelCase", deny_unknown_fields)]
struct Input { target: DataManagementTarget, #[serde(default)] transfer_id: Option<String>, #[serde(default)] category_id: Option<String>, #[serde(default)] confirmed: bool, #[serde(default)] settings: Option<Settings> }
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase", deny_unknown_fields)]
pub(super) struct Settings { pub interval_hours: u32, pub retain_count: u32 }
impl Default for Settings { fn default() -> Self { Self { interval_hours: 24, retain_count: 10 } } }
pub(super) fn load_settings(db: &GlobalDatabase, target: &DataManagementTarget) -> Result<Option<Settings>, StatusCode> {
    let key = serde_json::to_string(target).map_err(|_| StatusCode::BAD_REQUEST)?;
    GatewaySettingsStore::read_existing(db, target.scope.tenant_scope(), "data_backup_policy", &key)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.map(|s| serde_json::from_str(&s).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)).transpose()
}
pub(crate) fn router() -> Router { router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from)) }
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new().route("/agent-apps/data/export", post(create))
        .route("/agent-apps/data/export/download", post(download))
        .route("/agent-apps/data/locations/list", post(locations))
        .route("/agent-apps/data/locations/delete", post(clean))
        .route("/agent-apps/data/locations/resolve", post(resolve))
        .route("/agent-apps/data/policy", post(settings))
        .layer(Extension(State { policy, downloads: Arc::new(Mutex::new(HashMap::new())) }))
        .layer(axum::extract::DefaultBodyLimit::max(16*1024))
        .layer(axum::middleware::map_response(async |mut r: Response| { r.headers_mut().insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-store")); r }))
}
fn authorize(state: &State, tenant: &TenantScope, headers: &HeaderMap, target: &DataManagementTarget) -> Result<PathBuf, StatusCode> {
    if target.scope.tenant_scope() != tenant { return Err(StatusCode::FORBIDDEN); }
    super::super::agent_apps::authorize_data_backups(state.policy.as_deref(), headers, target)
}
fn scope(target: &DataManagementTarget) -> SafetySnapshotScope { SafetySnapshotScope { project: target.scope.clone(), host_id: target.target_host_id.clone(), principal_id: target.principal_id.clone() } }
fn database(db: Option<Extension<GlobalDatabase>>) -> Result<GlobalDatabase, StatusCode> { db.map(|Extension(db)| db).map(Ok).unwrap_or_else(|| GlobalDatabase::for_home(&crate::global::paths().home).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)) }
async fn create(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>, headers: HeaderMap, Json(input): Json<ExportDataRequest>) -> Result<Json<Value>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        authorize(&state, &tenant, &headers, &input.target)?;
        super::super::agent_apps::authorize_data_export(state.policy.as_deref(),&headers,&input.target)?;
        if input.format != DataArchiveFormat::NativePackage || input.selections.len()!=1 || input.selections[0].category!=DataCategory::Preferences { return Err(StatusCode::NOT_IMPLEMENTED); }
        if !matches!(input.selections[0].entries, DataEntrySelection::All) || input.idempotency_key.is_empty() || input.idempotency_key.len()>256 { return Err(StatusCode::BAD_REQUEST); }
        let mut cache = state.downloads.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        cache.retain(|_, d| d.created.elapsed()<Duration::from_secs(300));
        if let Some((id, _))=cache.iter().find(|(_, d)| d.target==input.target && d.key==input.idempotency_key) { return Ok(Json(json!({"transferId":id,"expiresInSeconds":300}))); }
        if cache.len()>=2 { return Err(StatusCode::TOO_MANY_REQUESTS); }
        let db=database(db)?;
        let value=GatewaySettingsStore::read_existing(&db, &tenant, "agent_apps_user", &input.target.principal_id).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let user: AppUserPreferences=value.map(|v| serde_json::from_str(&v)).transpose().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.unwrap_or_else(vw_agent_apps::apps::preferences::default_user_preferences);
        // Only normalized portable preference fields; host settings are never read.
        let record=json!({"appOrder":user.app_order,"visibleApps":user.visible_apps});
        let dir=tempfile::tempdir().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let path=dir.path().canonicalize().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let receipt=export::export_portable_package(&path, &scope(&input.target), &[DataCategory::Preferences], |_, _| Ok(vec![record.clone()])).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let bytes=std::fs::read(path.join(format!("{}.shiju-package",receipt.id))).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        export::verify_portable_package(&bytes,&receipt.digest).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let id=uuid::Uuid::new_v4().to_string();
        cache.insert(id.clone(), Download { target:input.target, key:input.idempotency_key, created:Instant::now(), bytes });
        Ok(Json(json!({"transferId":id,"expiresInSeconds":300})))
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn download(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Json(input): Json<Input>) -> Result<Response, StatusCode> {
    authorize(&state,&tenant,&headers,&input.target)?;
    super::super::agent_apps::authorize_data_export(state.policy.as_deref(),&headers,&input.target)?;
    let mut cache=state.downloads.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    cache.retain(|_,d| d.created.elapsed()<Duration::from_secs(300));
    let d=cache.get(input.transfer_id.as_deref().ok_or(StatusCode::BAD_REQUEST)?).filter(|d|d.target==input.target).ok_or(StatusCode::NOT_FOUND)?;
    Ok(([(header::CONTENT_TYPE,"application/zip"),(header::CONTENT_DISPOSITION,"attachment; filename=export.shiju-package")],d.bytes.clone()).into_response())
}
async fn settings(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, db: Option<Extension<GlobalDatabase>>, headers: HeaderMap, Json(input): Json<Input>) -> Result<Json<Value>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        authorize(&state,&tenant,&headers,&input.target)?;
        let db=database(db)?;
        if let Some(settings)=input.settings {
            if settings.retain_count==0 { return Err(StatusCode::BAD_REQUEST); }
            let key=serde_json::to_string(&input.target).map_err(|_| StatusCode::BAD_REQUEST)?;
            GatewaySettingsStore::new(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.upsert(&tenant,"data_backup_policy",&key,&serde_json::to_string(&settings).map_err(|_| StatusCode::BAD_REQUEST)?).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            Ok(Json(json!(settings)))
        } else { Ok(Json(json!(load_settings(&db,&input.target)?.unwrap_or_default()))) }
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn locations(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Json(input): Json<Input>) -> Result<Json<Value>, StatusCode> { category(state,tenant,headers,input,false).await }
async fn clean(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Json(input): Json<Input>) -> Result<Json<Value>, StatusCode> { category(state,tenant,headers,input,true).await }
async fn category(state:State, tenant:TenantScope, headers:HeaderMap, input:Input, delete:bool) -> Result<Json<Value>,StatusCode> {
    tokio::task::spawn_blocking(move || {
        let root=authorize(&state,&tenant,&headers,&input.target)?;
        // Category roots must be explicitly registered by the administrator.
        let (app,device,environment)=super::super::agent_apps::data_location_roots(state.policy.as_deref(),&headers,&input.target)?;
        let scope=scope(&input.target);
        let roots=vw_agent_apps::data_management::locations::BackupLocationRoots { scope:&scope,local_backups:&root,app:&app,device:&device,environment:&environment };
        if delete {
            if !input.confirmed { return Err(StatusCode::BAD_REQUEST); }
            let report=vw_agent_apps::data_management::location_delete::delete_location(&roots,&scope,input.category_id.as_deref().ok_or(StatusCode::BAD_REQUEST)?).map_err(|e| { use vw_agent_apps::data_management::location_delete::LocationDeleteError::*; match e { UnknownCategory|DatabaseNotDeletable=>StatusCode::BAD_REQUEST, OwnerRequired=>StatusCode::NOT_IMPLEMENTED, _=>StatusCode::CONFLICT } })?;
            Ok(Json(json!({"freedBytes":report.freed_bytes,"undeletedCount":report.undeleted.len()})))
        } else { Ok(Json(json!(vw_agent_apps::data_management::locations::list_locations(&roots,&scope).map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?))) }
    }).await.map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?
}

async fn resolve(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Json(input): Json<Input>) -> Result<Json<Value>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let root = authorize(&state, &tenant, &headers, &input.target)?;
        let (app, device, environment) = super::super::agent_apps::data_location_roots(state.policy.as_deref(), &headers, &input.target)?;
        let scope = scope(&input.target);
        let roots = vw_agent_apps::data_management::locations::BackupLocationRoots { scope: &scope, local_backups: &root, app: &app, device: &device, environment: &environment };
        let path = vw_agent_apps::data_management::locations::location_path(&roots, &scope, input.category_id.as_deref().ok_or(StatusCode::BAD_REQUEST)?)
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        Ok(Json(json!(path)))
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
