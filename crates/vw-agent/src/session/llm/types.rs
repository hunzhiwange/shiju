//! Canonical LLM types and error conversion exports.
pub use llm_runtime::types::{AgentInfo, StreamInput, PromptStreamEvent};
pub use llm_runtime::{Error, StreamEvent};
pub use model_types::{ToolCall, ToolCallKind};
pub(crate) use llm_runtime::catalog_error::catalog_error_string;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use llm_runtime::replay_safety::prevent_error_replay;
