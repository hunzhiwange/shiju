//! 工具调用解析入口的领域库转发。
pub(crate) use agent_loop::parsing::{
    ParsedToolCall, detect_tool_call_parse_issue, parse_structured_tool_calls, parse_tool_calls,
};
