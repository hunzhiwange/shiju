//! Typed raw session operations; routing and orchestration live in gateway-session.
use session_info::Info;
use session_port::{CreateSessionInput, SessionScope, SessionSnapshot, LifecycleError};
use vw_api_transport::TenantScope;
use std::future::Future;
pub trait PreparedTurnFork: Send {
    fn source_directory(&self) -> &str;
    fn persist(self) -> impl Future<Output = Result<Info, session_info::Error>> + Send;
}
pub trait SessionHost: crate::message_host::MessageHost {
    type State: Clone + Send + Sync + 'static;
    type Fork: PreparedTurnFork + 'static;
    fn with_tenant_scope<T: Send>(tenant: TenantScope, future: impl Future<Output = T> + Send) -> impl Future<Output = T> + Send;
    fn status_list() -> std::collections::HashMap<String, session_port::SessionStatus>;
    fn is_successful_finish_reason(reason: Option<&str>) -> bool;
    fn first_response_source(content: &str) -> String;
    fn resolve_scope(tenant: &TenantScope, directory: Option<&str>, project: Option<&str>) -> Option<String>;
    fn has_committed_first_response(tenant: &TenantScope, project: &str, id: &str) -> anyhow::Result<bool>;
    fn deletion_candidates(tenant: &TenantScope) -> anyhow::Result<Vec<Info>>;
    fn ensure_ready(directory: &str) -> Result<(), String>;
    fn create_session(input: CreateSessionInput) -> impl Future<Output = Result<SessionSnapshot, LifecycleError>> + Send;
    fn reserve_session(input: CreateSessionInput) -> impl Future<Output = Result<SessionSnapshot, LifecycleError>> + Send;
    fn load_session(scope: SessionScope, id: &str) -> impl Future<Output = Result<SessionSnapshot, LifecycleError>> + Send;
    fn get(id: &str) -> impl Future<Output = Result<Info, session_info::Error>> + Send;
    fn get_any(id: &str) -> impl Future<Output = Result<Info, session_info::Error>> + Send;
    fn children(id: &str) -> impl Future<Output = Result<Vec<Info>, session_info::Error>> + Send;
    fn remove(id: &str) -> impl Future<Output = Result<(), session_info::Error>> + Send;
    fn remove_any(id: &str) -> impl Future<Output = Result<(), session_info::Error>> + Send;
    fn list_all() -> impl Future<Output = Result<Vec<Info>, session_info::Error>> + Send;
    fn list_for_project(project: String) -> impl Future<Output = Result<Vec<Info>, session_info::Error>> + Send;
    fn remove_archived_locked(info: Info) -> impl Future<Output = Result<(), session_info::Error>> + Send;
    fn update(id: &str, action: impl FnOnce(&mut Info) + Send) -> impl Future<Output = Result<Info, session_info::Error>> + Send;
    fn update_any(id: &str, action: impl FnOnce(&mut Info) + Send) -> impl Future<Output = Result<Info, session_info::Error>> + Send;
    fn fork(id: &str, message: Option<&str>) -> impl Future<Output = Result<Info, session_info::Error>> + Send;
    fn refresh_summary(id: &str) -> impl Future<Output = Result<session_info::Summary, String>> + Send;
    fn todo_get(id: &str) -> impl Future<Output = Vec<vw_api_session::session::GatewaySessionTodoItem>> + Send;
    fn todo_update(session_id: String, todos: Vec<vw_api_session::session::GatewaySessionTodoItem>) -> impl Future<Output = Result<(), String>> + Send;
    fn cleanup_after_message(id: &str, message: &str) -> impl Future<Output = Result<(), String>> + Send;
    fn diff(session_id: String, message_id: Option<String>) -> impl Future<Output = Vec<snapshot_types::FileDiff>> + Send;
    fn process_session(id: &str, parent: &str, auto: bool) -> impl Future<Output = Result<session_port::CompactionOutcome, session_info::Error>> + Send;
    fn generate_title(session_id: String, content: String, preferred_model: Option<String>, acp_agent: Option<String>) -> impl Future<Output = Result<String, String>> + Send;
    fn resolve_full_access(project: &str, id: &str, value: Option<bool>, inherited: bool) -> impl Future<Output = anyhow::Result<bool>> + Send;
    fn invalidate_engine(state: &Self::State, id: &str) -> impl Future<Output = ()> + Send;
    fn fork_engine(state: &Self::State, source: &str, target: &str) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn prepare_fork(id: &str, boundary: &str) -> impl Future<Output = Result<Self::Fork, session_info::Error>> + Send;
    fn create_checkout(name: Option<&str>) -> impl Future<Output = Result<worktree_core::Info, String>> + Send;
    fn remove_checkout(directory: String) -> impl Future<Output = Result<(), String>> + Send;
    fn metadata_lock(tenant: &TenantScope) -> impl Future<Output = tokio::sync::OwnedMutexGuard<()>> + Send;
}
