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
pub use deepseek_dsml::contains_deepseek_dsml_markup;
#[cfg(test)]
mod tool_calls_tests;
#[cfg(test)]
mod xml_helpers_tests;



#[cfg(test)]
mod tests;

pub use json::{extract_json_values, tool_call_signature, parse_arguments_value, parse_tool_call_value, parse_tool_calls_from_json_value};
pub use tool_call_formats::{default_param_for_tool, parse_glm_shortened_body, parse_glm_style_tool_calls, parse_perl_style_tool_calls};
