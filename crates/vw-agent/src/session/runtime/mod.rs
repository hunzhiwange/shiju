//! 原生会话运行时边界。

mod lifecycle;
mod prompt;
mod types;

pub use lifecycle::{create_session, create_visible_task_session, load_session};
pub(crate) use lifecycle::visible_tasks_for_parent;
pub(crate) use lifecycle::fork_visible_task_session;
pub use prompt::start_prompt;
pub(crate) use prompt::start_prompt_with_runner;
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
