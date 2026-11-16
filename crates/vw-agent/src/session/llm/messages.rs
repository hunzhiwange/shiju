//! Host composition for system prompts.
pub use llm_runtime::messages::{build_chat_messages, resolve_tools, has_tool_calls};
#[cfg(test)]
use llm_runtime::messages::ensure_openai_tool_call_sequence;
pub fn build_system_messages(input: &super::types::StreamInput, is_codex: bool) -> Vec<String> {
    super::prompt::ensure_host();
    llm_runtime::messages::build_system_messages(input, is_codex)
}
#[cfg(test)]
#[path = "../../../../libs/llm-runtime/src/messages_tests.rs"]
mod messages_tests;
