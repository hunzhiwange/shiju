//! 会话执行边界共享的协议契约。

mod events;
pub mod llm_raw;
mod prompt;
pub mod ui_types;

pub use events::*;
pub use prompt::{PromptEvent, PromptTermination};

#[cfg(test)]
mod llm_raw_tests;

mod lifecycle;
mod snapshot;
pub use lifecycle::{SessionScope, CreateSessionInput, SessionSnapshot, LifecycleError};
pub use snapshot::PersistedDynamicTools;

mod toolset;
mod turn;
pub use toolset::{SessionCatalog, SessionConnections, SessionToolset};
pub use turn::{PromptCleanup, PromptCancelHandle, PromptTurn};

mod ports;
pub use ports::{EventSink, SessionExecution, SessionFuture, SessionQuery};

pub mod child_agent;
pub mod task_session;
pub mod chat_stream;
