//! 原生会话端口的具体处理器与元数据存储组合。


// Shell cleanup is owned by session-engine::runtime::prompt.

/// 会话服务已由 session-engine 持有，这里只绑定具体处理器。
pub(super) struct NativeSessionEngine;

impl session_port::SessionExecution for NativeSessionEngine {
    type Request = crate::session::processor::Request;
    type Event = crate::session::processor::StreamEvent;

    fn start_prompt(&self, request: Self::Request) -> session_port::PromptTurn {
        super::prompt::start_prompt_with_runner(request, |request, emit| {
            crate::session::processor::run(request, emit)
        })
    }

    fn start_prompt_with_runner<R>(&self, request: Self::Request, runner: R) -> session_port::PromptTurn
    where
        R: FnOnce(Self::Request, session_port::EventSink<Self::Event>) + Send + 'static,
    {
        super::prompt::start_prompt_with_runner(request, runner)
    }
}

use session_port::{CreateSessionInput, SessionFuture, SessionScope, SessionSnapshot};

impl session_port::SessionQuery for NativeSessionEngine {
    fn create_session<'a>(&'a self, input: CreateSessionInput) -> SessionFuture<'a, SessionSnapshot> {
        Box::pin(super::lifecycle::create_session(input))
    }

    fn create_visible_task_session<'a>(&'a self, input: CreateSessionInput) -> SessionFuture<'a, SessionSnapshot> {
        Box::pin(super::lifecycle::create_visible_task_session(input))
    }

    fn load_session<'a>(&'a self, scope: SessionScope, session_id: &'a str) -> SessionFuture<'a, SessionSnapshot> {
        Box::pin(super::lifecycle::load_session(scope, session_id))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn reserve_session<'a>(&'a self, input: CreateSessionInput) -> SessionFuture<'a, SessionSnapshot> {
        Box::pin(super::lifecycle::reserve_session(input))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn reserve_visible_task_session<'a>(&'a self, scope: SessionScope, parent_id: String) -> SessionFuture<'a, SessionSnapshot> {
        Box::pin(super::lifecycle::reserve_visible_task_session(scope, parent_id))
    }

    fn fork_visible_task_session<'a>(&'a self, scope: SessionScope, parent_id: &'a str, source_id: &'a str, message_id: Option<&'a str>) -> SessionFuture<'a, session_info::Info> {
        Box::pin(super::lifecycle::fork_visible_task_session(scope, parent_id, source_id, message_id))
    }

    fn visible_tasks_for_parent<'a>(&'a self, scope: SessionScope, parent_id: &'a str) -> SessionFuture<'a, Vec<session_info::Info>> {
        Box::pin(super::lifecycle::visible_tasks_for_parent(scope, parent_id))
    }

    fn validate_session_scope<'a>(&'a self, scope: SessionScope, session_id: &'a str) -> SessionFuture<'a, session_info::Info> {
        Box::pin(super::lifecycle::validate_session_scope(scope, session_id))
    }
}
