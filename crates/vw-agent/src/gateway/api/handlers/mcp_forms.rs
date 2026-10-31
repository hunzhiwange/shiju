//! A short-lived, tenant-bound host lease for desktop MCP forms.
//! The random host ID is held only by the UI, never by the model or MCP server.
use crate::{gateway::ApiError, plugins::McpFormSession, session::ui_store};
use axum::{Json, Router, routing::post};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use vw_api_types::{
    id::TenantScope,
    mcp_form::{
        McpFormHostOpen, McpFormHostOpened, McpFormHostRef, McpFormHostReply, McpFormRequest,
    },
};

const LEASE_IDLE: Duration = Duration::from_secs(20);
struct Host {
    tenant: TenantScope,
    lease: McpFormSession,
    _receiver: tokio::sync::broadcast::Receiver<crate::session::ui_types::McpFormEvent>,
    touched: Instant,
}
fn hosts() -> &'static Mutex<HashMap<String, Host>> {
    static HOSTS: OnceLock<Mutex<HashMap<String, Host>>> = OnceLock::new();
    HOSTS.get_or_init(Default::default)
}
pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/mcp/forms/open", post(open))
        .route("/mcp/forms/poll", post(poll))
        .route("/mcp/forms/reply", post(reply))
        .route("/mcp/forms/close", post(close))
        .layer(axum::extract::DefaultBodyLimit::max(196_608))
}
async fn open(
    headers: axum::http::HeaderMap,
    Json(body): Json<McpFormHostOpen>,
) -> Result<Json<McpFormHostOpened>, ApiError> {
    if body.session_id.trim().is_empty() || body.session_id.len() > 256 {
        return Err(ApiError::bad_request("invalid MCP form session"));
    }
    let directory = crate::gateway::instance::resolve_directory(
        &crate::gateway::instance::InstanceQuery { directory: body.workspace },
        &headers,
    );
    let workspace = Path::new(&directory)
        .canonicalize()
        .map_err(|_| ApiError::bad_request("MCP form workspace is unavailable"))?;
    if !workspace.is_dir() {
        return Err(ApiError::bad_request("MCP form workspace must be a directory"));
    }
    let tenant = ui_store::current_tenant_scope();
    let host_id = uuid::Uuid::new_v4().to_string();
    {
        let mut entries = hosts().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        entries.retain(|_, host| host.touched.elapsed() < LEASE_IDLE);
        if entries.values().filter(|h| h.tenant == tenant).count() >= 64 {
            return Err(ApiError::bad_request("MCP form host limit reached"));
        }
        let lease = McpFormSession::register(&body.session_id, &workspace).map_err(|_| {
            ApiError::bad_request("MCP form handler is already installed or unavailable")
        })?;
        let receiver = lease.subscribe();
        entries.insert(
            host_id.clone(),
            Host { tenant, lease, _receiver: receiver, touched: Instant::now() },
        );
    }
    let id = host_id.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(LEASE_IDLE).await;
            let mut entries = hosts().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if entries.get(&id).is_none_or(|host| host.touched.elapsed() >= LEASE_IDLE) {
                entries.remove(&id);
                break;
            }
        }
    });
    Ok(Json(McpFormHostOpened { host_id }))
}
fn with_host<T>(
    body: &McpFormHostRef,
    f: impl FnOnce(&Host) -> Result<T, ApiError>,
) -> Result<T, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let mut entries = hosts().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let host = entries
        .get_mut(&body.host_id)
        .filter(|host| host.tenant == tenant && host.touched.elapsed() < LEASE_IDLE)
        .ok_or_else(|| ApiError::not_found("MCP form host is unavailable"))?;
    host.touched = Instant::now();
    f(host)
}
async fn poll(Json(body): Json<McpFormHostRef>) -> Result<Json<Vec<McpFormRequest>>, ApiError> {
    with_host(&body, |host| Ok(Json(host.lease.pending())))
}
async fn reply(Json(body): Json<McpFormHostReply>) -> Result<Json<bool>, ApiError> {
    with_host(&McpFormHostRef { host_id: body.host_id }, |host| {
        host.lease.respond(&body.request, body.response).map_err(|_| {
            ApiError::bad_request("MCP form input is invalid or the request has expired")
        })?;
        Ok(Json(true))
    })
}
async fn close(Json(body): Json<McpFormHostRef>) -> Result<Json<bool>, ApiError> {
    with_host(&body, |_| Ok(()))?;
    hosts().lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&body.host_id);
    Ok(Json(true))
}
#[cfg(test)]
#[path = "mcp_forms_tests.rs"]
mod tests;
