//! 现有会话执行与归属查询的窄端口，由宿主引擎实现。

use std::{future::Future, pin::Pin};
use crate::{CreateSessionInput, LifecycleError, PromptTurn, SessionScope, SessionSnapshot};

pub type SessionFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, LifecycleError>> + Send + 'a>>;
pub type EventSink<E> = Box<dyn FnMut(E) -> bool + Send>;

/// 保留宿主的强类型请求与内部事件，不擦除权限或执行上下文。
pub trait SessionExecution {
    type Request;
    type Event;

    fn start_prompt(&self, request: Self::Request) -> PromptTurn;
    fn start_prompt_with_runner<R>(&self, request: Self::Request, runner: R) -> PromptTurn
    where
        R: FnOnce(Self::Request, EventSink<Self::Event>) + Send + 'static;
}

/// 仅暴露当前调用者已有的生命周期及归属查询操作。
pub trait SessionQuery {
    fn create_session<'a>(&'a self, input: CreateSessionInput) -> SessionFuture<'a, SessionSnapshot>;
    fn create_visible_task_session<'a>(&'a self, input: CreateSessionInput) -> SessionFuture<'a, SessionSnapshot>;
    fn load_session<'a>(&'a self, scope: SessionScope, session_id: &'a str) -> SessionFuture<'a, SessionSnapshot>;
    #[cfg(not(target_arch = "wasm32"))]
    fn reserve_session<'a>(&'a self, input: CreateSessionInput) -> SessionFuture<'a, SessionSnapshot>;
    #[cfg(not(target_arch = "wasm32"))]
    fn reserve_visible_task_session<'a>(&'a self, scope: SessionScope, parent_id: String) -> SessionFuture<'a, SessionSnapshot>;
    fn fork_visible_task_session<'a>(&'a self, scope: SessionScope, parent_id: &'a str, source_id: &'a str, message_id: Option<&'a str>) -> SessionFuture<'a, session_info::Info>;
    fn visible_tasks_for_parent<'a>(&'a self, scope: SessionScope, parent_id: &'a str) -> SessionFuture<'a, Vec<session_info::Info>>;
    fn validate_session_scope<'a>(&'a self, scope: SessionScope, session_id: &'a str) -> SessionFuture<'a, session_info::Info>;
}
