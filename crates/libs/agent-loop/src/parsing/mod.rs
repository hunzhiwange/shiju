//! 模型工具调用的格式解析与参数归一化。

pub mod deepseek_dsml;
pub mod json;
pub mod minimax_xml;
pub mod tool_call_formats;
mod types;
pub use types::ParsedToolCall;

#[cfg(test)]
use json::parse_tool_call_value;
#[cfg(test)]
use tool_call_formats::parse_glm_shortened_body;
#[cfg(test)]
mod tests;
