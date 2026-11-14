//! Scoped price and display operations over the host-owned usage database.
use std::{collections::BTreeMap, path::PathBuf, sync::{Arc, Mutex}};
use axum::{Extension, Json, Router, extract::{Path, Query}, http::{HeaderMap, StatusCode}, routing::{get, post}};
use serde::Deserialize;
use serde_json::{Value, json};
use vw_agent_apps::{usage as u, apps::lifecycle::operations::{Operation, OperationFailure, Phase}};
use vw_api_types::{TenantScope, id::ModelId};
use super::super::external_sessions::usage_scan::{self as host, Intent};

#[derive(Clone)]
struct Policy(Option<PathBuf>);
type Response = Result<Json<Value>, StatusCode>;
fn unavailable<T>(_: T) -> StatusCode { StatusCode::SERVICE_UNAVAILABLE }
fn domain(error: String) -> StatusCode {
    if error.contains("storage") || error.contains("saved") { StatusCode::SERVICE_UNAVAILABLE }
    else { StatusCode::BAD_REQUEST }
}
fn output(value: impl serde::Serialize) -> Response {
    serde_json::to_value(value).map(Json).map_err(unavailable)
}
pub(super) fn router() -> Router {
    Router::new()
        .route("/usage/prices", get(list).post(create))
        .route("/usage/prices/{id}", axum::routing::put(replace).delete(delete))
        .route("/usage/prices/import", post(import))
        .route("/usage/models-dev/candidates", get(candidates))
        .route("/usage/models-dev/config", get(config).put(save_config))
        .route("/usage/models-dev/sync", post(sync))
        .route("/usage/rates", get(display))
        .route("/usage/rates/refresh", post(refresh))
        .route("/usage/display", get(display).put(save_display))
        .layer(Extension(Policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))))
}
fn authorize(p: &Policy, t: TenantScope, h: &HeaderMap, i: &Intent, write: bool) -> Result<u::scan_jobs::ScanOwner, StatusCode> {
    let owner = host::authorized_owner(p.0.as_deref(), t, h, i)?;
    if write {
        super::super::agent_apps::authorize_external_history_maintenance(p.0.as_deref(),h,&owner.scope,&owner.target_host)?;
    }
    Ok(owner)
}
fn connection() -> Result<rusqlite::Connection, StatusCode> {
    let db = crate::usage_bridge::context::database().map_err(unavailable)?;
    let c = db.connect_sqlite().map_err(unavailable)?;
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
enum Read { Prices, Config, Display }
async fn read(p: Policy,t: TenantScope,h: HeaderMap,i: Intent,kind: Read) -> Response {
    tokio::task::spawn_blocking(move || {
        let o = authorize(&p,t,&h,&i,false)?;
        let c = connection()?;
        match kind {
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
read_handler!(config,Config);
read_handler!(display,Display);
enum Write { Price(u::pricing::ModelPricingInfo,Option<String>), Delete(String), Import(ImportBody), Config(u::models_sync::ModelsDevSyncConfig), Display(DisplayBody) }
async fn write(p: Policy,t: TenantScope,h: HeaderMap,i: Intent,kind: Write) -> Response {
    tokio::task::spawn_blocking(move || {
        let o = authorize(&p,t,&h,&i,true)?;
        let mut c = connection()?;
        match kind {
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
    output(u::models_selection::flatten_models(&vw_provider_resolver::models::snapshot().await.providers))
}
async fn refresh(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Query(i): Query<Intent>) -> Response {
    let o = tokio::task::spawn_blocking(move || authorize(&p,t,&h,&i,true)).await.map_err(unavailable)??;
    let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build().map_err(unavailable)?;
    let result = u::exchange::fetch_usd_rates(&client,chrono::Utc::now().timestamp_millis()).await;
    tokio::task::spawn_blocking(move || {
        let s = u::exchange_store::save_refresh_result(&mut connection()?,&o.scope,&o.target_host,result).map_err(unavailable)?;
        output(json!({"display":s.resolve(chrono::Utc::now().timestamp_millis()),"settings":s}))
    }).await.map_err(unavailable)?
}
async fn sync(Extension(p): Extension<Policy>,Extension(t): Extension<TenantScope>,h: HeaderMap,Query(i): Query<Intent>) -> Result<(StatusCode,Json<Value>),StatusCode> {
    let o = tokio::task::spawn_blocking(move || authorize(&p,t,&h,&i,true)).await.map_err(unavailable)??;
    let sync = tokio::task::spawn_blocking({ let o=o.clone(); move || {
        u::models_sync::ModelsSync::new(Arc::new(Mutex::new(connection()?)),o.scope,o.target_host).map_err(unavailable)
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
            result = sync.sync(true) => {
                let result = result.map_err(|_| OperationFailure::Failed)?;
                *saved_result.lock().map_err(|_| OperationFailure::Failed)? = Some(json!(result));
                Ok(())
            },
        }
    }).map_err(unavailable)?;
    jobs.insert(id.clone(),super::scan::Job {owner:o,operation,result:result_slot});
    Ok((StatusCode::ACCEPTED,Json(json!({"task_id":id,"status":"queued"}))))
}
