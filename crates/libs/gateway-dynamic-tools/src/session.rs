//! Composition adapter for the existing session-owned dynamic tool runtime.
use crate::{DynamicToolsHost, HostCalls, HostFuture};
use gateway_types::ApiError;
use agent_gateway::instance::{InstanceQuery, resolve_directory};
use session_store::dynamic_tool_store::PersistentToolPolicy;
use session_port::SessionScope;
use session_engine::{session_io::SessionIo, runtime::dynamic_hosts::{SessionHost, HostRegistrationConflict, HostTools}};
use axum::http::{HeaderMap, StatusCode};
use std::sync::Arc;
use vw_api_tools::{DynamicToolHostBinding, DynamicToolResponse, ReplaceDynamicToolsRequest};

pub struct SessionDynamicToolsHost<I, T>(std::marker::PhantomData<fn() -> (I, T)>);
impl<I: SessionIo, T: HostTools + Send + Sync + 'static> DynamicToolsHost for SessionDynamicToolsHost<I, T> {
    type Policy = PersistentToolPolicy;
    type Entry = Arc<SessionHost>;
    fn resolve_directory(directory: Option<String>, headers: &HeaderMap) -> String {
        resolve_directory(&InstanceQuery { directory }, headers)
    }
    fn validate_scope(scope: SessionScope, session: &str) -> HostFuture<'_, session_info::Info> {
        Box::pin(async move {
            session_engine::runtime::lifecycle::validate_session_scope::<I>(scope, session).await
                .map_err(|_| ApiError::not_found("session not found in requested scope"))
        })
    }
    fn register(scope: SessionScope, info: session_info::Info, policy: Self::Policy) -> Result<DynamicToolHostBinding, ApiError> {
        SessionHost::register(scope, info, policy).map_err(|error| match error.downcast_ref::<HostRegistrationConflict>() {
            Some(conflict) => ApiError { status: StatusCode::CONFLICT, message: conflict.to_string() },
            None => ApiError::internal("failed to restore dynamic tool catalog"),
        })
    }
    fn authenticate(scope: &SessionScope, session: &str, token: &str) -> Result<Self::Entry, ApiError> {
        SessionHost::authenticate(scope, session, token).map_err(denied)
    }
    fn replace(entry: Self::Entry, request: ReplaceDynamicToolsRequest) -> Result<u64, ApiError> {
        entry.replace::<T>(request).map_err(|error| ApiError::bad_request(error.to_string()))
    }
    fn calls(entry: Self::Entry) -> Result<HostCalls, ApiError> {
        let calls = entry.calls().map_err(denied)?;
        Ok(Box::pin(futures_util::stream::unfold(calls, |mut calls| async move {
            let call = calls.recv().await?;
            Some((call, calls))
        })))
    }
    fn respond(entry: Self::Entry, call: &str, response: DynamicToolResponse) -> Result<(), ApiError> {
        entry.host.respond(entry.host.owner(), call, response).map_err(denied)
    }
}
fn denied(error: impl std::fmt::Display) -> ApiError {
    ApiError { status: StatusCode::FORBIDDEN, message: error.to_string() }
}
