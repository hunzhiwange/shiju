//! Raw session service assembly and typed HTTP forwarding.
use session_info::Info;
use session_port::{CreateSessionInput, SessionScope, SessionSnapshot, LifecycleError};
use vw_api_transport::TenantScope;
use std::{future::Future, collections::HashMap};
use axum::{Json, extract::{Path, Query, State}, http::HeaderMap};
use crate::gateway::{ApiError, AppState, instance::InstanceQuery};
use super::message_ops::Host;
use super::shared::{UiSessionCreateBody, UiSessionListQuery};
use vw_api_session::session::*;
use crate::session as agent_session;
use crate::snapshot;
pub(super) struct PreparedFork(crate::session::session::TurnFork);
impl gateway_session::session_host::PreparedTurnFork for PreparedFork {
    fn source_directory(&self) -> &str { self.0.source_directory() }
    async fn persist(self) -> Result<Info, session_info::Error> { self.0.persist().await }
}
impl gateway_session::session_host::SessionHost for Host {
    type State = AppState;
    type Fork = PreparedFork;
    async fn with_tenant_scope<T: Send>(tenant: TenantScope, future: impl Future<Output = T> + Send) -> T { crate::session::ui_store::with_tenant_scope(tenant, future).await }
    fn status_list() -> std::collections::HashMap<String, session_port::SessionStatus> { crate::session::status::list() }
    fn is_successful_finish_reason(reason: Option<&str>) -> bool { crate::session::runtime::is_successful_finish_reason(reason) }
    fn first_response_source(content: &str) -> String { crate::session::title::first_response_source(content) }
    fn resolve_scope(tenant: &TenantScope, directory: Option<&str>, project: Option<&str>) -> Option<String> { crate::session::ui_store::resolve_session_scope_id_for_tenant(tenant, directory, project) }
    fn has_committed_first_response(tenant: &TenantScope, project: &str, id: &str) -> anyhow::Result<bool> { crate::session::ui_store::has_committed_first_response(tenant, project, id) }
    fn deletion_candidates(tenant: &TenantScope) -> anyhow::Result<Vec<Info>> { crate::session::ui_store::load_agent_deletion_candidates(tenant) }
    fn ensure_ready(directory: &str) -> Result<(), String> { crate::worktree::environment::ensure_ready(directory).map_err(|error| error.to_string()) }
    async fn create_session(input: CreateSessionInput) -> Result<SessionSnapshot, LifecycleError> { crate::session::runtime::create_session(input).await }
    async fn reserve_session(input: CreateSessionInput) -> Result<SessionSnapshot, LifecycleError> { crate::session::runtime::reserve_session(input).await }
    async fn load_session(scope: SessionScope, id: &str) -> Result<SessionSnapshot, LifecycleError> { crate::session::runtime::load_session(scope, id).await }
    async fn get(id: &str) -> Result<Info, session_info::Error> { crate::session::session::get(id).await }
    async fn get_any(id: &str) -> Result<Info, session_info::Error> { crate::session::session::get_any(id).await }
    async fn children(id: &str) -> Result<Vec<Info>, session_info::Error> { crate::session::session::children(id).await }
    async fn remove(id: &str) -> Result<(), session_info::Error> { crate::session::session::remove(id).await }
    async fn remove_any(id: &str) -> Result<(), session_info::Error> { crate::session::session::remove_any(id).await }
    async fn list_all() -> Result<Vec<Info>, session_info::Error> { crate::session::session::list_all().await }
    async fn list_for_project(project: String) -> Result<Vec<Info>, session_info::Error> { crate::session::session::list_for_project(project).await }
    async fn remove_archived_locked(info: Info) -> Result<(), session_info::Error> { crate::session::session::remove_archived_locked(info).await }
    async fn update(id: &str, action: impl FnOnce(&mut Info) + Send) -> Result<Info, session_info::Error> { crate::session::session::update(id, action).await }
    async fn update_any(id: &str, action: impl FnOnce(&mut Info) + Send) -> Result<Info, session_info::Error> { crate::session::session::update_any(id, action).await }
    async fn fork(id: &str, message: Option<&str>) -> Result<Info, session_info::Error> { crate::session::session::fork(id, message).await }
    async fn refresh_summary(id: &str) -> Result<session_info::Summary, String> { crate::session::summary::refresh_session_diff_summary(id).await.map_err(|error| error.to_string()) }
    async fn todo_get(id: &str) -> Vec<vw_api_session::session::GatewaySessionTodoItem> { crate::session::todo::get(id).await }
    async fn todo_update(session_id: String, todos: Vec<vw_api_session::session::GatewaySessionTodoItem>) -> Result<(), String> { crate::session::todo::update(crate::session::todo::UpdateInput { session_id, todos }).await.map_err(|error| error.to_string()) }
    async fn cleanup_after_message(id: &str, message: &str) -> Result<(), String> { crate::session::revert::cleanup_after_message(id, message).await.map_err(|error| error.to_string()) }
    async fn diff(session_id: String, message_id: Option<String>) -> Vec<snapshot_types::FileDiff> { crate::session::summary::diff(crate::session::summary::DiffInput { session_id, message_id }).await }
    async fn process_session(id: &str, parent: &str, auto: bool) -> Result<session_port::CompactionOutcome, session_info::Error> { crate::session::compaction::process_session(id, parent, auto).await }
    async fn generate_title(session_id: String, content: String, preferred_model: Option<String>, acp_agent: Option<String>) -> Result<String, String> { crate::session::title::generate_from_content(session_id, content, preferred_model, acp_agent).await }
    async fn resolve_full_access(project: &str, id: &str, value: Option<bool>, inherited: bool) -> anyhow::Result<bool> { crate::session::agent_binding::resolve_full_access_scoped(project, id, value, inherited).await }
    async fn invalidate_engine(state: &Self::State, id: &str) -> () { crate::gateway::chat::invalidate_session_query_engine(state, id).await }
    async fn fork_engine(state: &Self::State, source: &str, target: &str) -> anyhow::Result<()> { crate::gateway::chat::fork_session_query_engine(state, source, target).await }
    async fn prepare_fork(id: &str, boundary: &str) -> Result<Self::Fork, session_info::Error> { crate::session::session::TurnFork::prepare(id, boundary).await.map(PreparedFork) }
    async fn create_checkout(name: Option<&str>) -> Result<worktree_core::Info, String> { crate::worktree::create_fork_checkout(name).await.map_err(|error| error.to_string()) }
    async fn remove_checkout(directory: String) -> Result<(), String> { crate::worktree::remove_fork_checkout(directory).await.map_err(|error| error.to_string()) }
    async fn metadata_lock(tenant: &TenantScope) -> tokio::sync::OwnedMutexGuard<()> { crate::session::session::lock_metadata_writes(tenant).await }
}
pub(super) async fn ui_session_list(
    Query(query): Query<UiSessionListQuery>,
    _headers: HeaderMap,
) -> Result<Json<Vec<agent_session::session::Info>>, ApiError> {
    gateway_session::session_ops::ui_session_list::<Host>(Query(query), _headers).await
}
pub(super) async fn ui_session_status(
    Query(query): Query<InstanceQuery>,
    _headers: HeaderMap,
) -> Result<Json<HashMap<String, agent_session::status::Info>>, ApiError> {
    gateway_session::session_ops::ui_session_status::<Host>(Query(gateway_instance::host::InstanceQuery { directory: query.directory }), _headers).await
}
pub(super) async fn ui_session_get(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<agent_session::session::Info>, ApiError> {
    gateway_session::session_ops::ui_session_get::<Host>(Path(session_id), Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers).await
}
pub(super) async fn session_full_access_put(
    path: Path<String>,
    query: Query<InstanceQuery>,
    headers: HeaderMap,
    Json(full_access): Json<bool>,
) -> Result<Json<bool>, ApiError> {
    gateway_session::session_ops::session_full_access_put::<Host>(path, Query(gateway_instance::host::InstanceQuery { directory: query.0.directory }), headers, Json(full_access)).await
}
pub(super) async fn ui_session_create(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<Option<UiSessionCreateBody>>,
) -> Result<Json<agent_session::session::Info>, ApiError> {
    gateway_session::session_ops::ui_session_create::<Host>(Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers, Json(body)).await
}
pub(super) async fn ui_session_patch(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewaySessionPatchBody>,
) -> Result<Json<agent_session::session::Info>, ApiError> {
    gateway_session::session_ops::ui_session_patch::<Host>(Path(session_id), Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers, Json(body)).await
}
pub(super) async fn ui_session_delete(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<bool>, ApiError> {
    gateway_session::session_ops::ui_session_delete::<Host>(State(state), Path(session_id), Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers).await
}
pub(super) async fn session_children(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<agent_session::session::Info>>, ApiError> {
    gateway_session::session_ops::session_children::<Host>(Path(session_id), Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers).await
}
pub(super) async fn session_todo_get(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<vw_api_session::session::GatewaySessionTodoItem>>, ApiError> {
    gateway_session::session_ops::session_todo_get::<Host>(Path(session_id), Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers).await
}
pub(super) async fn session_todo_put(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewaySessionTodoPutBody>,
) -> Result<Json<bool>, ApiError> {
    gateway_session::session_ops::session_todo_put::<Host>(Path(session_id), Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers, Json(body)).await
}
pub(super) async fn session_fork(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<Option<GatewaySessionForkBody>>,
) -> Result<Json<agent_session::session::Info>, ApiError> {
    gateway_session::session_ops::session_fork::<Host>(State(state), Path(session_id), Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers, Json(body)).await
}
pub(super) async fn session_reset(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewaySessionResetBody>,
) -> Result<Json<agent_session::session::Info>, ApiError> {
    gateway_session::session_ops::session_reset::<Host>(State(state), Path(session_id), Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers, Json(body)).await
}
pub(super) async fn session_diff(
    Path(session_id): Path<String>,
    Query(query): Query<GatewaySessionDiffQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<snapshot::FileDiff>>, ApiError> {
    gateway_session::session_ops::session_diff::<Host>(Path(session_id), Query(query), headers).await
}
pub(super) async fn session_summarize(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewaySessionSummarizeBody>,
) -> Result<Json<bool>, ApiError> {
    gateway_session::session_ops::session_summarize::<Host>(Path(session_id), Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers, Json(body)).await
}
pub(super) async fn session_title_generate(
    Path(session_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewaySessionTitleGenerateBody>,
) -> Result<Json<GatewaySessionTitleGenerateResponse>, ApiError> {
    gateway_session::session_ops::session_title_generate::<Host>(Path(session_id), Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers, Json(body)).await
}

#[cfg(test)]
use gateway_session::session_ops::task_session_summary_needs_refresh;
#[cfg(test)]
#[path = "session_ops_tests.rs"]
mod session_ops_tests;
