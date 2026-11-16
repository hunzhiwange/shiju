//! 原生会话运行时边界。

mod lifecycle;
mod prompt;
mod types;

pub(crate) use lifecycle::fork_visible_task_session;
pub(crate) use lifecycle::visible_tasks_for_parent;
pub use lifecycle::{create_session, create_visible_task_session, load_session};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use lifecycle::{reserve_session, reserve_visible_task_session};
pub use prompt::start_prompt;
pub(crate) use prompt::start_prompt_with_runner;
pub use types::{
    CreateSessionInput, LifecycleError, PromptCancelHandle, PromptEvent, PromptTermination,
    PromptTurn, SessionScope, SessionSnapshot, SessionToolset,
};

// Real NativeSessionIo persistence, tenant isolation and visible-task wiring contracts.
// Directory-only SessionScope validation lives in session-port::lifecycle tests.
#[cfg(test)]
#[path = "../../../../libs/session-engine/tests/host_suites/runtime/lifecycle_tests.rs"]
mod lifecycle_tests;

#[cfg(test)]
#[path = "../../../../libs/session-engine/tests/host_suites/runtime/prompt_tests.rs"]
mod prompt_tests;

pub mod dynamic_calls;
// Dynamic callback tests live with the shared implementation.

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod dynamic_hosts;
pub(crate) use lifecycle::validate_session_scope;

pub(crate) use session_engine::runtime::is_successful_finish_reason;
