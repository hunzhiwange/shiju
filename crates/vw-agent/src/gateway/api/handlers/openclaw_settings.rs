//! Fixed OpenClaw sections; provider import and live snapshots retain their owners.
use axum::{extract::{DefaultBodyLimit, Query}, http::{HeaderMap, StatusCode}, routing::get, Extension, Json, Router};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;
use vw_api_types::{id::{ProjectId, ProjectScope, TenantScope}, openclaw_settings::*};
use vw_agent_apps::{openclaw_catalog, openclaw_defaults, openclaw_model, openclaw_env, openclaw_tools, openclaw_health, providers::{openclaw::{self, OpenClawFiles}, live::{commit::{ExecutionHostId, ProviderTransactions}, write_guard::WriteError}}};

#[derive(Clone)]
struct Policy(Option<PathBuf>);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Scope { project_id: ProjectId, target_host_id: String }

pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/openclaw/settings/model", get(read_model).put(write_model))
        .route("/agent-apps/openclaw/settings/catalog", get(read_catalog).put(write_catalog))
        .route("/agent-apps/openclaw/settings/defaults", get(read_defaults).put(write_defaults))
        .route("/agent-apps/openclaw/settings/env", get(read_env).put(write_env))
        .route("/agent-apps/openclaw/settings/tools", get(read_tools).put(write_tools))
        .route("/agent-apps/openclaw/settings/health", get(read_health))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(Extension(Policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))))
}
fn authorize(p: &Policy, t: &TenantScope, h: &HeaderMap, s: &Scope, write: bool) -> Result<super::access::Host, StatusCode> {
    super::access::authorize_openclaw_settings(p.0.as_deref(), h,
        &ProjectScope::new(t.clone(), s.project_id.clone()), &s.target_host_id, write)
}
fn error(e: WriteError) -> StatusCode {
    match e {
        WriteError::Conflict | WriteError::Recovered => StatusCode::CONFLICT,
        WriteError::InvalidResource => StatusCode::UNPROCESSABLE_ENTITY,
        WriteError::Io => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
fn convert<T: serde::de::DeserializeOwned>(value: impl serde::Serialize) -> Result<T, StatusCode> {
    serde_json::from_value(serde_json::to_value(value).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
}
#[derive(Clone, Copy)]
enum Section { Model, Catalog, Defaults, Env, Tools, Health }
fn project(section: Section, bytes: Option<&[u8]>) -> Result<Value, StatusCode> {
    match section {
        Section::Env => openclaw_env::get_env_config(bytes, || Ok(())).map_err(error).map(|env| env.map(|v| Value::Object(v.as_object().clone())).unwrap_or(Value::Null)),
        Section::Tools => convert(openclaw_tools::get_tools_config(bytes).map_err(error)?),
        Section::Health => convert(openclaw_health::scan_openclaw_health_from_bytes(bytes)),
        Section::Model => convert(openclaw_model::get_default_model(bytes).map_err(error)?),
        Section::Catalog => convert(openclaw_catalog::get_model_catalog(bytes).map_err(error)?),
        Section::Defaults => convert(openclaw_defaults::get_agents_defaults(bytes).map_err(error)?),
    }
}
async fn read(p: Policy, t: TenantScope, h: HeaderMap, s: Scope, section: Section) -> Result<Json<Value>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let host = authorize(&p, &t, &h, &s, false)?;
        if matches!(section, Section::Env) {
            super::access::authorize_openclaw_env_read(p.0.as_deref(), &h,
                &ProjectScope::new(t.clone(), s.project_id.clone()), &s.target_host_id)?;
        }
        let bytes = openclaw::read(&host.home).map_err(error)?;
        project(section, bytes.as_deref()).map(Json)
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
enum Edit { Model(OpenClawDefaultModel), Catalog(OpenClawModelCatalog), Defaults(OpenClawAgentsDefaults), Env(OpenClawEnvConfig), Tools(OpenClawToolsConfig) }
async fn write(p: Policy, t: TenantScope, h: HeaderMap, s: Scope, edit: Edit) -> Result<Json<OpenClawWriteOutcome>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let host = authorize(&p, &t, &h, &s, true)?;
        let root = host.credential_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let mut files = OpenClawFiles::open(&host.home).map_err(error)?;
        let tx = ProviderTransactions::new(&root, ExecutionHostId::new(&host.id).map_err(error)?).map_err(error)?;
        if tx.recover(&mut files.guard).map_err(error)?.is_some() { return Err(StatusCode::CONFLICT); }
        let expected = tx.state(vw_api_types::agent_apps::AppId::OpenClaw).map_err(error)?;
        let unchanged = || authorize(&p, &t, &h, &s, true).is_ok();
        let warnings = match edit {
            Edit::Env(env) => {
                let raw = serde_json::to_string(&env).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                let env = openclaw_env::OpenClawEnvConfig::parse(&raw).map_err(error)?;
                openclaw_env::set_env_config(&mut files, &tx, &t, &expected, &env, unchanged).map_err(error)?;
                Vec::new()
            }
            Edit::Tools(tools) => {
                openclaw_tools::set_tools_config(&mut files, &tx, &t, &expected, &convert(tools)?, unchanged).map_err(error)?;
                Vec::new()
            }
            Edit::Model(model) => openclaw_model::set_default_model(&mut files, &tx, &t, &expected, &convert(model)?, unchanged)
                .map_err(error)?.warnings.into_iter().map(|message| OpenClawHealthWarning { code: "configuration_write".into(), message, path: None }).collect(),
            Edit::Catalog(catalog) => {
                let (name, pre) = files.snapshot().map_err(error)?;
                let next = openclaw_catalog::set_model_catalog(pre.as_deref(), &convert(catalog)?).map_err(error)?;
                files.apply(&tx, &t, &expected, name, pre.as_deref(), next, unchanged).map_err(error)?;
                Vec::new()
            }
            Edit::Defaults(defaults) => {
                openclaw_defaults::set_agents_defaults(&mut files, &tx, &t, &expected, &convert(defaults)?, unchanged).map_err(error)?;
                Vec::new()
            }
        };
        Ok(Json(OpenClawWriteOutcome { backup_path: None, warnings }))
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

async fn read_model(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>) -> Result<Json<Value>, StatusCode> {
    read(p, t, h, s, Section::Model).await
}
async fn write_model(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>, Json(b): Json<OpenClawDefaultModel>) -> Result<Json<OpenClawWriteOutcome>, StatusCode> {
    write(p, t, h, s, Edit::Model(b)).await
}

async fn read_catalog(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>) -> Result<Json<Value>, StatusCode> {
    read(p, t, h, s, Section::Catalog).await
}
async fn write_catalog(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>, Json(b): Json<OpenClawModelCatalog>) -> Result<Json<OpenClawWriteOutcome>, StatusCode> {
    write(p, t, h, s, Edit::Catalog(b)).await
}

async fn read_defaults(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>) -> Result<Json<Value>, StatusCode> {
    read(p, t, h, s, Section::Defaults).await
}
async fn write_defaults(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>, Json(b): Json<OpenClawAgentsDefaults>) -> Result<Json<OpenClawWriteOutcome>, StatusCode> {
    write(p, t, h, s, Edit::Defaults(b)).await
}

#[cfg(test)]
#[path = "openclaw_settings_tests.rs"]
mod tests;

async fn read_env(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>) -> Result<Json<Value>, StatusCode> {
    read(p, t, h, s, Section::Env).await
}

async fn write_env(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>, Json(b): Json<OpenClawEnvConfig>) -> Result<Json<OpenClawWriteOutcome>, StatusCode> {
    write(p, t, h, s, Edit::Env(b)).await
}

async fn read_tools(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>) -> Result<Json<Value>, StatusCode> {
    read(p, t, h, s, Section::Tools).await
}

async fn write_tools(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>, Json(b): Json<OpenClawToolsConfig>) -> Result<Json<OpenClawWriteOutcome>, StatusCode> {
    write(p, t, h, s, Edit::Tools(b)).await
}

async fn read_health(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>) -> Result<Json<Value>, StatusCode> {
    read(p, t, h, s, Section::Health).await
}
