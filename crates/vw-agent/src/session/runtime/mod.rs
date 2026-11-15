//! 原生会话运行时边界。

mod host;
mod lifecycle;
mod prompt;
mod types;
mod wiring;

pub(crate) use wiring::fork_visible_task_session;
pub(crate) use wiring::visible_tasks_for_parent;
pub use wiring::{create_session, create_visible_task_session, load_session};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use wiring::{reserve_session, reserve_visible_task_session};
pub use wiring::start_prompt;
pub(crate) use wiring::start_prompt_with_runner;
pub use types::{
    CreateSessionInput, LifecycleError, PromptCancelHandle, PromptEvent, PromptTermination,
    PromptTurn, SessionScope, SessionSnapshot, SessionToolset,
};

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod lifecycle_tests;

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod prompt_tests;

pub mod dynamic_calls;
#[cfg(test)]
mod dynamic_calls_tests;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod dynamic_hosts;
pub(crate) use wiring::validate_session_scope;

pub(crate) fn is_successful_finish_reason(reason: Option<&str>) -> bool {
    !matches!(
        reason,
        Some(
            "tool_calls"
                | "tool_use"
                | "content_filter"
                | "error"
                | "cancelled"
                | "waiting_for_input"
                | "requires_action"
                | "refusal"
                | "max_turn_requests"
                | "length"
                | "max_tokens"
        )
    )
}
