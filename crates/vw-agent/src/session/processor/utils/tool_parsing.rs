//! 文本工具调用解析逻辑，负责识别用户消息中的斜杠工具调用并校验工具名。

pub(crate) use session_engine::tool_parsing::{parse_tool_at, is_valid_tool_name, query_has_any_tool_calls_with_allowed};
pub(crate) fn query_has_any_tool_calls(query: &str) -> bool { super::super::helpers::query_has_any_tool_calls(query) }
