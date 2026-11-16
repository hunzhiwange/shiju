use crate::session::llm;
use serde_json::Value;
pub(crate) use session_engine::processor_clock::now_ms;
pub(crate) fn tool_calls_to_assistant_message(
    tool_calls: &[llm::ToolCall],
    reasoning_content: &str,
) -> Value { session_engine::runner_messages::tool_calls_to_assistant_message(tool_calls, reasoning_content) }
pub(crate) fn tool_result_to_message(tool_call_id: &str, content: &str) -> Value { session_engine::runner_messages::tool_result_to_message(tool_call_id, content) }
pub(crate) fn assistant_message_with_reasoning(content: &str, reasoning_content: &str) -> Value { session_engine::runner_messages::assistant_message_with_reasoning(content, reasoning_content) }

#[cfg(test)]
#[path = "../../../../../libs/session-engine/tests/host_suites/message_format_tests.rs"]
mod message_format_tests;
