//! 模型工具调用的格式解析与参数归一化。

pub mod deepseek_dsml;
pub mod json;
pub mod minimax_xml;
pub mod tool_call_formats;
mod types;
pub use types::ParsedToolCall;
mod tool_calls;
mod xml_helpers;
pub use tool_calls::{detect_tool_call_parse_issue, parse_structured_tool_calls, parse_tool_calls};
#[cfg(test)]
use deepseek_dsml::contains_deepseek_dsml_markup;
#[cfg(test)]
mod tool_calls_tests;
#[cfg(test)]
mod xml_helpers_tests;

#[cfg(test)]
use json::parse_tool_call_value;
#[cfg(test)]
use tool_call_formats::parse_glm_shortened_body;
#[cfg(test)]
mod tests;
