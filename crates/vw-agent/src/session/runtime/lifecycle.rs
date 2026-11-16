//! Shared lifecycle services bound to metadata IO.
use super::types::*;
use crate::session::session_io::NativeSessionIo;
pub async fn create_session(input: CreateSessionInput) -> Result<SessionSnapshot, LifecycleError> { session_engine::runtime::lifecycle::create_session::<NativeSessionIo>(input).await }
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn reserve_session(
    input: CreateSessionInput,
) -> Result<SessionSnapshot, LifecycleError> { session_engine::runtime::lifecycle::reserve_session::<NativeSessionIo>(input).await }
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn reserve_visible_task_session(
    scope: SessionScope,
    parent_id: String,
) -> Result<SessionSnapshot, LifecycleError> { session_engine::runtime::lifecycle::reserve_visible_task_session::<NativeSessionIo>(scope, parent_id).await }
pub async fn create_visible_task_session(
    input: CreateSessionInput,
) -> Result<SessionSnapshot, LifecycleError> { session_engine::runtime::lifecycle::create_visible_task_session::<NativeSessionIo>(input).await }
pub(crate) async fn fork_visible_task_session(
    scope: SessionScope,
    parent_id: &str,
    source_id: &str,
    message_id: Option<&str>,
) -> Result<session_info::Info, LifecycleError> { session_engine::runtime::lifecycle::fork_visible_task_session::<NativeSessionIo>(scope, parent_id, source_id, message_id).await }
pub(crate) async fn visible_tasks_for_parent(
    scope: SessionScope,
    parent_id: &str,
) -> Result<Vec<session_info::Info>, LifecycleError> { session_engine::runtime::lifecycle::visible_tasks_for_parent::<NativeSessionIo>(scope, parent_id).await }
pub async fn load_session(
    scope: SessionScope,
    session_id: &str,
) -> Result<SessionSnapshot, LifecycleError> { session_engine::runtime::lifecycle::load_session::<NativeSessionIo>(scope, session_id).await }
pub(crate) async fn validate_session_scope(
    scope: SessionScope,
    session_id: &str,
) -> Result<session_info::Info, LifecycleError> { session_engine::runtime::lifecycle::validate_session_scope::<NativeSessionIo>(scope, session_id).await }
