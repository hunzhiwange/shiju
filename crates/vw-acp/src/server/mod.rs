//! ACP Agent 侧协议适配器。

mod agent;
mod backend;
mod content;
mod events;
mod state;
#[cfg(test)]
mod acceptance_tests;

pub use agent::VibeWindowAgent;
pub use backend::{
    BackendCancelHandle, BackendError, BackendHistoryEntry, BackendPermissionOption,
    BackendPermissionOptionKind, BackendPermissionRequest, BackendPrompt, BackendRole,
    BackendSession, SessionBackend,
};
pub use content::{PromptContent, prompt_content};
pub use events::{
    BackendEvent, BackendPlanEntry, BackendPlanStatus, BackendTermination, BackendToolStatus,
    SessionUpdateSink,
};
pub use state::SessionBinding;
pub use agent::serve_with_io;

#[cfg(test)]
#[path = "agent_tests.rs"]
mod agent_tests;

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod prompt_tests;

#[cfg(test)]
#[path = "transport_tests.rs"]
mod transport_tests;
