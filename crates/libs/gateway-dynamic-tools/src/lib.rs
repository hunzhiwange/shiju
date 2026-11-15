#![cfg(not(target_arch = "wasm32"))]
//! Native authenticated dynamic tool host API; not a Codex JSON-RPC transport.
use gateway_types::ApiError;
use session_port::SessionScope;
use session_info::Info;
use std::{future::Future, pin::Pin};
use futures_util::stream::BoxStream;
use serde::Deserialize;

pub type HostFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ApiError>> + Send + 'a>>;
pub type HostCalls = BoxStream<'static, vw_api_tools::DynamicToolCallRequest>;

/// Existing session runtime actions; the host retains all state and authorization policy.
pub trait DynamicToolsHost: Send + Sync + 'static {
    type Policy: Clone + Default + Send + Sync + 'static;
    type Entry: Send + Sync;
    fn resolve_directory(directory: Option<String>, headers: &axum::http::HeaderMap) -> String;
    fn validate_scope(scope: SessionScope, session: &str) -> HostFuture<'_, Info>;
    fn register(scope: SessionScope, info: Info, policy: Self::Policy) -> Result<vw_api_tools::DynamicToolHostBinding, ApiError>;
    fn authenticate(scope: &SessionScope, session: &str, token: &str) -> Result<Self::Entry, ApiError>;
    fn replace(entry: Self::Entry, request: vw_api_tools::ReplaceDynamicToolsRequest) -> Result<u64, ApiError>;
    fn calls(entry: Self::Entry) -> Result<HostCalls, ApiError>;
    fn respond(entry: Self::Entry, call: &str, response: vw_api_tools::DynamicToolResponse) -> Result<(), ApiError>;
}

#[derive(Deserialize)]
struct InstanceQuery { directory: Option<String> }
use axum::{
    Extension, Json, Router,
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post, put},
};
use futures_util::stream;
use std::convert::Infallible;
use {
    vw_api_transport::id::TenantScope,
    vw_api_tools::DynamicToolHostBinding,
    vw_api_tools::DynamicToolResponse,
    vw_api_tools::ReplaceDynamicToolsRequest,
    vw_api_tools::ReplaceDynamicToolsResponse,
};

pub fn router<H: DynamicToolsHost, S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/session/{session_id}/dynamic-tools/hosts", post(register::<H>))
        .route("/session/{session_id}/dynamic-tools", put(replace::<H>))
        .route("/session/{session_id}/dynamic-tools/calls", get(calls::<H>))
        .route("/session/{session_id}/dynamic-tools/calls/{call_id}/response", post(respond::<H>))
}
fn denied(error: impl std::fmt::Display) -> ApiError {
    ApiError { status: StatusCode::FORBIDDEN, message: error.to_string() }
}
async fn scope<H: DynamicToolsHost>(
    tenant: TenantScope,
    query: InstanceQuery,
    headers: &HeaderMap,
    session: &str,
) -> Result<(SessionScope, Info), ApiError> {
    let scope = SessionScope::new(tenant, H::resolve_directory(query.directory, headers))
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let info = H::validate_scope(scope.clone(), session)
        .await
        .map_err(|_| ApiError::not_found("session not found in requested scope"))?;
    Ok((scope, info))
}
fn host<H: DynamicToolsHost>(
    scope: &SessionScope,
    session: &str,
    headers: &HeaderMap,
) -> Result<H::Entry, ApiError> {
    let mut values = headers.get_all("x-vibewindow-tool-host").iter();
    let token = values
        .next()
        .and_then(|value| value.to_str().ok())
        .filter(|token| !token.is_empty())
        .ok_or_else(|| denied("missing dynamic tool host binding"))?;
    if values.next().is_some() {
        return Err(denied("ambiguous dynamic tool host binding"));
    }
    H::authenticate(scope, session, token)
}
async fn register<H: DynamicToolsHost>(
    policy: Option<Extension<H::Policy>>,
    Extension(tenant): Extension<TenantScope>,
    Path(session): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<DynamicToolHostBinding>, ApiError> {
    let (scope, info) = scope::<H>(tenant, query, &headers, &session).await?;
    H::register(scope, info, policy.map(|Extension(policy)| policy).unwrap_or_default())
        .map(Json)
}
async fn replace<H: DynamicToolsHost>(
    Extension(tenant): Extension<TenantScope>,
    Path(session): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(request): Json<ReplaceDynamicToolsRequest>,
) -> Result<Json<ReplaceDynamicToolsResponse>, ApiError> {
    let (scope, _) = scope::<H>(tenant, query, &headers, &session).await?;
    let entry = host::<H>(&scope, &session, &headers)?;
    let version = H::replace(entry, request)?;
    Ok(Json(ReplaceDynamicToolsResponse { version }))
}
async fn calls<H: DynamicToolsHost>(
    Extension(tenant): Extension<TenantScope>,
    Path(session): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let (scope, _) = scope::<H>(tenant, query, &headers, &session).await?;
    let calls = H::calls(host::<H>(&scope, &session, &headers)?)?;
    let stream = stream::unfold(calls, |mut calls| async move {
        let call = futures_util::StreamExt::next(&mut calls).await?;
        let event = Event::default()
            .event("dynamic_tool.call")
            .json_data(call)
            .expect("typed call is JSON serializable");
        Some((Ok(event), calls))
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
async fn respond<H: DynamicToolsHost>(
    Extension(tenant): Extension<TenantScope>,
    Path((session, call)): Path<(String, String)>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(response): Json<DynamicToolResponse>,
) -> Result<StatusCode, ApiError> {
    let (scope, _) = scope::<H>(tenant, query, &headers, &session).await?;
    let entry = host::<H>(&scope, &session, &headers)?;
    H::respond(entry, &call, response)?;
    Ok(StatusCode::NO_CONTENT)
}
