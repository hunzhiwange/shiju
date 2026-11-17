//! Host composition for system prompts.
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub use llm_runtime::messages::has_tool_calls;
#[cfg(any(test, target_arch = "wasm32"))]
pub use llm_runtime::messages::{build_chat_messages, resolve_tools, has_tool_calls};
pub fn build_system_messages(input: &super::types::StreamInput, is_codex: bool) -> Vec<String> {
    super::prompt::ensure_host();
    llm_runtime::messages::build_system_messages(input, is_codex)
}
#[cfg(test)]
#[path = "../../../../libs/llm-runtime/src/messages_tests.rs"]
mod messages_tests;

// These contracts require the real session prompt renderer installed by ensure_host.
#[cfg(test)]
#[path = "messages_system_tests.rs"]
mod messages_system_tests;
