//! Native authenticated dynamic tool host API; not a Codex JSON-RPC transport.
use crate::gateway::{
    ApiError,
    instance::{InstanceQuery, resolve_directory},
};
use crate::session::dynamic_tool_store::PersistentToolPolicy;
use crate::session::runtime::{
    SessionScope,
    dynamic_hosts::{HostRegistrationConflict, SessionHost},
    validate_session_scope,
};
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
use vw_api_types::{
    id::TenantScope,
    tools::{
        DynamicToolHostBinding, DynamicToolResponse, ReplaceDynamicToolsRequest,
        ReplaceDynamicToolsResponse,
    },
};

pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/session/{session_id}/dynamic-tools/hosts", post(register))
        .route("/session/{session_id}/dynamic-tools", put(replace))
        .route("/session/{session_id}/dynamic-tools/calls", get(calls))
        .route("/session/{session_id}/dynamic-tools/calls/{call_id}/response", post(respond))
}
fn denied(error: impl std::fmt::Display) -> ApiError {
    ApiError { status: StatusCode::FORBIDDEN, message: error.to_string() }
}
async fn scope(
    tenant: TenantScope,
    query: InstanceQuery,
    headers: &HeaderMap,
    session: &str,
) -> Result<(SessionScope, crate::session::session::Info), ApiError> {
    let scope = SessionScope::new(tenant, resolve_directory(&query, headers))
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let info = validate_session_scope(scope.clone(), session)
        .await
        .map_err(|_| ApiError::not_found("session not found in requested scope"))?;
    Ok((scope, info))
}
fn host(
    scope: &SessionScope,
    session: &str,
    headers: &HeaderMap,
) -> Result<std::sync::Arc<SessionHost>, ApiError> {
    let mut values = headers.get_all("x-vibewindow-tool-host").iter();
    let token = values
        .next()
        .and_then(|value| value.to_str().ok())
        .filter(|token| !token.is_empty())
        .ok_or_else(|| denied("missing dynamic tool host binding"))?;
    if values.next().is_some() {
        return Err(denied("ambiguous dynamic tool host binding"));
    }
    SessionHost::authenticate(scope, session, token).map_err(denied)
}
async fn register(
    policy: Option<Extension<PersistentToolPolicy>>,
    Extension(tenant): Extension<TenantScope>,
    Path(session): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<DynamicToolHostBinding>, ApiError> {
    let (scope, info) = scope(tenant, query, &headers, &session).await?;
    SessionHost::register(scope, info, policy.map(|Extension(policy)| policy).unwrap_or_default())
        .map(Json)
        .map_err(|error| match error.downcast_ref::<HostRegistrationConflict>() {
            Some(conflict) => {
                ApiError { status: StatusCode::CONFLICT, message: conflict.to_string() }
            }
            None => ApiError::internal("failed to restore dynamic tool catalog"),
        })
}
async fn replace(
    Extension(tenant): Extension<TenantScope>,
    Path(session): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(request): Json<ReplaceDynamicToolsRequest>,
) -> Result<Json<ReplaceDynamicToolsResponse>, ApiError> {
    let (scope, _) = scope(tenant, query, &headers, &session).await?;
    let entry = host(&scope, &session, &headers)?;
    let version =
        entry.replace(request).map_err(|error| ApiError::bad_request(error.to_string()))?;
    Ok(Json(ReplaceDynamicToolsResponse { version }))
}
async fn calls(
    Extension(tenant): Extension<TenantScope>,
    Path(session): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let (scope, _) = scope(tenant, query, &headers, &session).await?;
    let calls = host(&scope, &session, &headers)?.calls().map_err(denied)?;
    let stream = stream::unfold(calls, |mut calls| async move {
        let call = calls.recv().await?;
        let event = Event::default()
            .event("dynamic_tool.call")
            .json_data(call)
            .expect("typed call is JSON serializable");
        Some((Ok(event), calls))
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
async fn respond(
    Extension(tenant): Extension<TenantScope>,
    Path((session, call)): Path<(String, String)>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(response): Json<DynamicToolResponse>,
) -> Result<StatusCode, ApiError> {
    let (scope, _) = scope(tenant, query, &headers, &session).await?;
    let entry = host(&scope, &session, &headers)?;
    entry.host.respond(entry.host.owner(), &call, response).map_err(denied)?;
    Ok(StatusCode::NO_CONTENT)
}
