//! 原解析类型与新历史构建规则的绑定。
use super::super::parsing::ParsedToolCall;
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub use agent_loop::core::history::{autosave_memory_key, build_native_assistant_history};
#[cfg(any(test, target_arch = "wasm32"))]
pub use agent_loop::core::history::{autosave_memory_key, tools_to_openai_format, build_native_assistant_history};
pub(crate) fn build_native_assistant_history_from_parsed_calls(text: &str, calls: &[ParsedToolCall], reasoning: Option<&str>) -> Option<String> {
    agent_loop::core::history::build_native_assistant_history_from_parsed_calls(text, calls, reasoning, |call| agent_loop::core::tool_execution::CallView { name: &call.name, arguments: &call.arguments, tool_call_id: call.tool_call_id.as_deref() })
}
