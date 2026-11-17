//! Scoped price and display operations over the host-owned usage database.
use std::{collections::BTreeMap, path::PathBuf, sync::{Arc, Mutex}};
use axum::{Extension, Json, Router, extract::{Path, Query}, http::{HeaderMap, StatusCode}, routing::{get, post}};
use serde::Deserialize;
use serde_json::{Value, json};
use crate::usage as u;
use app_lifecycle::operations::{Operation, OperationFailure, Phase};
use {
    vw_api_transport::TenantScope,
    vw_api_transport::id::ModelId,
};
use crate::host::Intent;

#[derive(Clone)]
struct Policy(Option<PathBuf>, crate::Host);
type Response = Result<Json<Value>, StatusCode>;
fn unavailable<T>(_: T) -> StatusCode { StatusCode::SERVICE_UNAVAILABLE }
fn domain(error: String) -> StatusCode {
    if error.contains("storage") || error.contains("saved") { StatusCode::SERVICE_UNAVAILABLE }
    else { StatusCode::BAD_REQUEST }
}
fn output(value: impl serde::Serialize) -> Response {
    serde_json::to_value(value).map(Json).map_err(unavailable)
}
pub(super) fn router(host: crate::Host) -> Router {
    Router::new()
        .route("/usage/prices", get(list).post(create))
        .route("/usage/prices/{id}", axum::routing::put(replace).delete(delete))
        .route("/usage/prices/import", post(import))
        .route("/usage/prices/billing-defaults", get(billing).put(save_billing))
        .route("/usage/prices/storage", get(storage_info))
        .route("/usage/prices/storage/open", post(open_storage))
        .route("/usage/models-dev/candidates", get(candidates))
        .route("/usage/models-dev/config", get(config).put(save_config))
        .route("/usage/models-dev/sync", post(sync))
        .route("/usage/models-dev/sync/auto", post(sync_auto))
        .route("/usage/rates", get(display))
        .route("/usage/rates/refresh", post(refresh))
        .route("/usage/display", get(display).put(save_display))
        .layer(Extension(Policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from), host)))
}
fn authorize(p: &Policy, t: TenantScope, h: &HeaderMap, i: &Intent, write: bool) -> Result<u::scan_jobs::ScanOwner, StatusCode> {
    let owner = (p.1.authorized_owner)(p.0.as_deref(), t, h, i)?;
    if write {
        (p.1.authorize_maintenance)(p.0.as_deref(),h,&owner.scope,&owner.target_host)?;
    }
    Ok(owner)
}
fn connection(host: crate::Host) -> Result<rusqlite::Connection, StatusCode> {
    let db = (host.database)().map_err(unavailable)?;
    let c = db.connect_sqlite().map_err(unavailable)?;
    u::pricing::initialize_pricing_store(&c).map_err(unavailable)?;
    u::models_sync::initialize_models_sync(&c).map_err(unavailable)?;
    u::exchange_store::initialize_exchange_store(&c).map_err(unavailable)?;
    Ok(c)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PriceBody { price: u::pricing::ModelPricingInfo }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportBody { source: String, prices: Vec<u::pricing::ModelPricingInfo> }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DisplayBody {
    currency: Option<u::exchange::Currency>, mode: Option<u::exchange::RateMode>,
    #[serde(default)] manual_rates: BTreeMap<String,f64>,
}
#[derive(Clone, Copy)]
enum Read { Prices, Config, Display, Billing }
async fn read(p: Policy,t: TenantScope,h: HeaderMap,i: Intent,kind: Read) -> Response {
    tokio::task::spawn_blocking(move || {
        let o = authorize(&p,t,&h,&i,false)?;
        let c = connection(p.1)?;
        match kind {
            Read::Billing => output(u::pricing::load_billing_defaults(&c,&o.scope,&o.target_host).map_err(domain)?),
            Read::Prices => output(u::pricing::list_model_pricing(&c,&o.scope,&o.target_host).map_err(domain)?),
            Read::Config => output(u::models_sync::get_models_sync_config(&c,&o.scope,&o.target_host).map_err(domain)?),
            Read::Display => {
                let s = u::exchange_store::load_exchange_settings(&c,&o.scope,&o.target_host).map_err(domain)?;
                output(json!({"display":s.resolve(chrono::Utc::now().timestamp_millis()),"settings":s}))
            }
        }
    }).await.map_err(unavailable)?
}
macro_rules! read_handler { ($name:ident,$kind:ident) => {
    async fn $name(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Query(i): Query<Intent>) -> Response { read(p,t,h,i,Read::$kind).await }
}; }
read_handler!(list,Prices);
read_handler!(billing,Billing);
read_handler!(config,Config);
read_handler!(display,Display);
enum Write { Billing(u::pricing::BillingDefaults), Price(u::pricing::ModelPricingInfo,Option<String>), Delete(String), Import(ImportBody), Config(u::models_sync::ModelsDevSyncConfig), Display(DisplayBody) }
async fn write(p: Policy,t: TenantScope,h: HeaderMap,i: Intent,kind: Write) -> Response {
    tokio::task::spawn_blocking(move || {
        let o = authorize(&p,t,&h,&i,true)?;
        let mut c = connection(p.1)?;
        match kind {
            Write::Billing(defaults) => output(u::pricing::save_billing_defaults(&mut c,&o.scope,&o.target_host,defaults).map_err(domain)?),
            Write::Price(price,id) => {
                if id.is_some_and(|id| id != price.model_id.as_ref()) { return Err(StatusCode::BAD_REQUEST); }
                output(u::pricing::save_model_pricing(&mut c,&o.scope,&o.target_host,price).map_err(domain)?)
            }
            Write::Delete(id) => {
                if !u::pricing::delete_model_pricing(&mut c,&o.scope,&o.target_host,&ModelId::from(id)).map_err(domain)? { return Err(StatusCode::NOT_FOUND); }
                output(json!({"deleted":true}))
            }
            Write::Import(r) => {
                if r.prices.is_empty() || r.prices.len()>256 || r.source.len()>256 { return Err(StatusCode::BAD_REQUEST); }
                output(json!({"changed":u::pricing_batch::import_model_pricing(&mut c,&o.scope,&o.target_host,&r.source,r.prices).map_err(domain)?}))
            }
            Write::Config(config) => {
                let selection = &config.selection;
                if selection.selected_model_keys.len()>4096 || selection.excluded_common_model_keys.len()>4096
                    || selection.selected_model_keys.iter().chain(&selection.excluded_common_model_keys).any(|k| k.is_empty() || k.len()>512) { return Err(StatusCode::BAD_REQUEST); }
                output(u::models_sync::save_models_sync_config(&mut c,&o.scope,&o.target_host,config).map_err(domain)?)
            }
            Write::Display(r) => output(u::exchange_store::save_display_patch(&mut c,&o.scope,&o.target_host,u::exchange_store::DisplayPatch {currency:r.currency,mode:r.mode,manual_rates:r.manual_rates}).map_err(domain)?),
        }
    }).await.map_err(unavailable)?
}
async fn create(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Query(i): Query<Intent>,Json(r): Json<PriceBody>) -> Response { write(p,t,h,i,Write::Price(r.price,None)).await }
async fn replace(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Path(id): Path<String>,Query(i): Query<Intent>,Json(r): Json<PriceBody>) -> Response { write(p,t,h,i,Write::Price(r.price,Some(id))).await }
async fn delete(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Path(id): Path<String>,Query(i): Query<Intent>) -> Response { write(p,t,h,i,Write::Delete(id)).await }
async fn import(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Query(i): Query<Intent>,Json(r): Json<ImportBody>) -> Response { write(p,t,h,i,Write::Import(r)).await }
async fn save_config(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Query(i): Query<Intent>,Json(r): Json<u::models_sync::ModelsDevSyncConfig>) -> Response { write(p,t,h,i,Write::Config(r)).await }
async fn save_display(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Query(i): Query<Intent>,Json(r): Json<DisplayBody>) -> Response { write(p,t,h,i,Write::Display(r)).await }
async fn candidates(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Query(i): Query<Intent>) -> Response {
    tokio::task::spawn_blocking(move || authorize(&p,t,&h,&i,false)).await.map_err(unavailable)??;
    output(u::models_selection::flatten_models(&provider_model_cache::snapshot().await.providers))
}
async fn refresh(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Query(i): Query<Intent>) -> Response {
    let host_context = p.1;
    let o = tokio::task::spawn_blocking(move || authorize(&p,t,&h,&i,true)).await.map_err(unavailable)??;
    let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build().map_err(unavailable)?;
    let result = u::exchange::fetch_usd_rates(&client,chrono::Utc::now().timestamp_millis()).await;
    tokio::task::spawn_blocking(move || {
        let s = u::exchange_store::save_refresh_result(&mut connection(host_context)?,&o.scope,&o.target_host,result).map_err(unavailable)?;
        output(json!({"display":s.resolve(chrono::Utc::now().timestamp_millis()),"settings":s}))
    }).await.map_err(unavailable)?
}
async fn sync(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Query(i): Query<Intent>) -> Result<(StatusCode,Json<Value>),StatusCode> {
    sync_request(p, t, h, i, true).await
}
async fn sync_auto(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Query(i): Query<Intent>) -> Result<(StatusCode,Json<Value>),StatusCode> {
    sync_request(p, t, h, i, false).await
}
async fn sync_request(p: Policy, t: TenantScope, h: HeaderMap, i: Intent, force: bool) -> Result<(StatusCode,Json<Value>),StatusCode> {
    let host_context = p.1;
    let o = tokio::task::spawn_blocking(move || authorize(&p,t,&h,&i,true)).await.map_err(unavailable)??;
    let sync = tokio::task::spawn_blocking({ let o=o.clone(); move || {
        u::models_sync::ModelsSync::new(Arc::new(Mutex::new(connection(host_context)?)),o.scope,o.target_host).map_err(unavailable)
    }}).await.map_err(unavailable)??;
    let mut jobs = super::scan::jobs().lock().map_err(unavailable)?;
    if jobs.values().any(|j| j.owner.scope==o.scope && j.owner.target_host==o.target_host && j.operation.snapshot().phase!=Phase::Terminal) { return Err(StatusCode::CONFLICT); }
    if jobs.len()>=64 {
        let terminal = jobs.iter().find(|(_,j)| j.operation.snapshot().phase==Phase::Terminal).map(|(id,_)| id.clone()).ok_or(StatusCode::TOO_MANY_REQUESTS)?;
        jobs.remove(&terminal);
    }
    let id=uuid::Uuid::new_v4().to_string();
    let operation=Operation::new();
    let worker=operation.clone();
    let result_slot=Arc::new(Mutex::new(None));
    let saved_result=result_slot.clone();
    operation.start_task(async move {
        // Dropping the fetch future on cancellation prevents a later price commit.
        tokio::select! {
            biased;
            _ = worker.cancelled() => Err(OperationFailure::Cancelled),
            result = sync.sync(force) => {
                let result = result.map_err(|_| OperationFailure::Failed)?;
                *saved_result.lock().map_err(|_| OperationFailure::Failed)? = Some(json!(result));
                Ok(())
            },
        }
    }).map_err(unavailable)?;
    jobs.insert(id.clone(),super::scan::Job {owner:o,operation,result:result_slot});
    Ok((StatusCode::ACCEPTED,Json(json!({"task_id":id,"status":"queued"}))))
}

async fn save_billing(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Query(i): Query<Intent>,Json(r): Json<u::pricing::BillingDefaults>) -> Response { write(p,t,h,i,Write::Billing(r)).await }

async fn storage_info(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Query(i): Query<Intent>) -> Response { storage_action(p,t,h,i,false).await }
async fn open_storage(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Query(i): Query<Intent>) -> Response { storage_action(p,t,h,i,true).await }
async fn storage_action(p: Policy,t: TenantScope,h: HeaderMap,i: Intent,open: bool) -> Response {
    tokio::task::spawn_blocking(move || {
        // Paths are resolved only after local executor/project maintenance authorization.
        // The request never supplies a path, and the desktop never opens this path itself.
        authorize(&p,t,&h,&i,true)?;
        let db = (p.1.database)().map_err(unavailable)?;
        let path = db.sqlite_path().map_err(|_| StatusCode::NOT_IMPLEMENTED)?.canonicalize().map_err(unavailable)?;
        let directory = path.parent().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let can_open = cfg!(target_os = "macos") || cfg!(target_os = "windows")
            || (cfg!(target_os = "linux") && (std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()));
        if open {
            if !can_open { return Err(StatusCode::NOT_IMPLEMENTED); }
            let mut command = if cfg!(target_os = "macos") { std::process::Command::new("/usr/bin/open") }
                else if cfg!(target_os = "windows") { std::process::Command::new("explorer.exe") }
                else { std::process::Command::new("xdg-open") };
            let status = command.arg(directory).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().map_err(unavailable)?;
            if !status.success() { return Err(StatusCode::SERVICE_UNAVAILABLE); }
        }
        output(json!({"path":path.to_string_lossy(),"can_open":can_open,"reason":if can_open { None } else { Some("执行主机没有可用的桌面目录打开能力") }}))
    }).await.map_err(unavailable)?
}
