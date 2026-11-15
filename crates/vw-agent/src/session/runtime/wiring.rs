//! 会话端口的宿主工厂接线，保留现有调用签名。

use super::host::NativeSessionEngine;
use super::{CreateSessionInput, LifecycleError, SessionScope, SessionSnapshot, PromptTurn};
use session_port::{SessionExecution, SessionQuery};

pub fn start_prompt(request: crate::session::processor::Request) -> PromptTurn {
    NativeSessionEngine.start_prompt(request)
}

pub(crate) fn start_prompt_with_runner<R>(request: crate::session::processor::Request, runner: R) -> PromptTurn
where
    R: FnOnce(crate::session::processor::Request, super::prompt::EventSink) + Send + 'static,
{
    NativeSessionEngine.start_prompt_with_runner(request, runner)
}

pub async fn create_session(input: CreateSessionInput) -> Result<SessionSnapshot, LifecycleError> {
    NativeSessionEngine.create_session(input).await
}

pub async fn create_visible_task_session(input: CreateSessionInput) -> Result<SessionSnapshot, LifecycleError> {
    NativeSessionEngine.create_visible_task_session(input).await
}

pub async fn load_session(scope: SessionScope, session_id: &str) -> Result<SessionSnapshot, LifecycleError> {
    NativeSessionEngine.load_session(scope, session_id).await
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn reserve_session(input: CreateSessionInput) -> Result<SessionSnapshot, LifecycleError> {
    NativeSessionEngine.reserve_session(input).await
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn reserve_visible_task_session(scope: SessionScope, parent_id: String) -> Result<SessionSnapshot, LifecycleError> {
    NativeSessionEngine.reserve_visible_task_session(scope, parent_id).await
}

pub(crate) async fn fork_visible_task_session(scope: SessionScope, parent_id: &str, source_id: &str, message_id: Option<&str>) -> Result<session_info::Info, LifecycleError> {
    NativeSessionEngine.fork_visible_task_session(scope, parent_id, source_id, message_id).await
}

pub(crate) async fn visible_tasks_for_parent(scope: SessionScope, parent_id: &str) -> Result<Vec<session_info::Info>, LifecycleError> {
    NativeSessionEngine.visible_tasks_for_parent(scope, parent_id).await
}

pub(crate) async fn validate_session_scope(scope: SessionScope, session_id: &str) -> Result<session_info::Info, LifecycleError> {
    NativeSessionEngine.validate_session_scope(scope, session_id).await
}
