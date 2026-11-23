//! Host-local backup policy and storage locations.
use axum::{Extension, Json, Router, http::{HeaderMap, StatusCode, header}, routing::post, response::Response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;
use {agent_database::GlobalDatabase, app_settings_store::settings_store::GatewaySettingsStore};
use {
    vw_api_app_operations::data_management::*,
    vw_api_transport::id::TenantScope,
};
use vw_agent_apps::data_management::safety_snapshot::SafetySnapshotScope;
#[derive(Clone)]
struct State { policy: Option<PathBuf> }
#[derive(Deserialize)]
#[serde(rename_all="camelCase", deny_unknown_fields)]
struct Input { target: DataManagementTarget, #[serde(default)] category_id: Option<String>, #[serde(default)] confirmed: bool, #[serde(default)] settings: Option<Settings> }
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase", deny_unknown_fields)]
pub(super) struct Settings { pub interval_hours: u32, pub retain_count: u32 }
impl Default for Settings { fn default() -> Self { Self { interval_hours: 24, retain_count: 10 } } }
pub(super) fn load_settings(db: &GlobalDatabase, target: &DataManagementTarget) -> Result<Option<Settings>, StatusCode> {
    let key = serde_json::to_string(target).map_err(|_| StatusCode::BAD_REQUEST)?;
    GatewaySettingsStore::read_existing(db, target.scope.tenant_scope(), "data_backup_policy", &key)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.map(|s| serde_json::from_str(&s).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)).transpose()
}
pub fn router() -> Router { router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from)) }
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/data/locations/list", post(locations))
        .route("/agent-apps/data/locations/delete", post(clean))
        .route("/agent-apps/data/locations/resolve", post(resolve))
        .route("/agent-apps/data/policy", post(settings))
        .layer(Extension(State { policy }))
        .layer(axum::extract::DefaultBodyLimit::max(16*1024))
        .layer(axum::middleware::map_response(async |mut r: Response| { r.headers_mut().insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-store")); r }))
}
fn authorize(state: &State, tenant: &TenantScope, headers: &HeaderMap, target: &DataManagementTarget) -> Result<PathBuf, StatusCode> {
    if target.scope.tenant_scope() != tenant { return Err(StatusCode::FORBIDDEN); }
    crate::access::authorize_data_backups(state.policy.as_deref(), headers, target)
}
fn scope(target: &DataManagementTarget) -> SafetySnapshotScope { SafetySnapshotScope { project: target.scope.clone(), host_id: target.target_host_id.clone(), principal_id: target.principal_id.clone() } }
fn database(db: Option<Extension<GlobalDatabase>>) -> Result<GlobalDatabase, StatusCode> { db.map(|Extension(db)| db).map(Ok).unwrap_or_else(|| GlobalDatabase::for_home(&agent_paths::paths().home).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)) }
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
        let (app,device,environment)=crate::access::data_location_roots(state.policy.as_deref(),&headers,&input.target)?;
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
        let (app, device, environment) = crate::access::data_location_roots(state.policy.as_deref(), &headers, &input.target)?;
        let scope = scope(&input.target);
        let roots = vw_agent_apps::data_management::locations::BackupLocationRoots { scope: &scope, local_backups: &root, app: &app, device: &device, environment: &environment };
        let path = vw_agent_apps::data_management::locations::location_path(&roots, &scope, input.category_id.as_deref().ok_or(StatusCode::BAD_REQUEST)?)
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        Ok(Json(json!(path)))
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
