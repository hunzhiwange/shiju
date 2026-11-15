//! 会话执行契约的宿主类型绑定。

pub use session_port::{
    CreateSessionInput, LifecycleError, PromptCancelHandle, PromptEvent, PromptTermination,
    PromptTurn, SessionScope, SessionSnapshot,
};

pub type SessionToolset = session_port::SessionToolset<
    crate::tools::catalog::ToolCatalogSnapshot,
    crate::plugins::McpSessionConnections,
>;
