//! 工具调用解析模块
//!
//! 本模块负责从不同格式的模型输出中解析工具调用（Tool Calls）。
//! 由于不同的 LLM 提供商返回工具调用的格式各异，本模块提供了统一的解析接口，
//! 支持多种常见的工具调用格式。
//!
//! # 支持的格式
//!
//! - **JSON 格式**：标准 JSON 工具调用，包括 OpenAI 兼容格式
//! - **XML 格式**：基于 XML 标签的工具调用（如 Claude 的工具调用格式）
//! - **MiniMax XML 格式**：MiniMax 特有的 XML invoke 调用格式
//! - **GLM 格式**：智谱 GLM 模型的特殊工具调用格式
//! - **Perl 风格格式**：某些模型使用的 Perl 风格参数格式
//! - **XML 属性格式**：将参数放在 XML 属性中的格式
//!
//! # 模块结构
//!
//! - [`json`]：JSON 格式解析工具，提供 JSON 值提取、规范化等功能
//! - [`minimax_xml`]：MiniMax XML 格式解析器
//! - [`tool_call_formats`]：多种工具调用格式的解析器集合
//! - [`tool_calls`]：核心工具调用解析逻辑和结果类型
//! - [`xml_helpers`]：XML 解析辅助函数

mod deepseek_dsml;
mod json;
mod minimax_xml;
mod tool_call_formats;
mod tool_calls;
mod xml_helpers;

pub(crate) use deepseek_dsml::contains_deepseek_dsml_markup;

// JSON 解析相关函数的重导出
// 包括：JSON 值提取、工具调用签名计算、参数解析等
pub use json::extract_json_values;
pub(crate) use json::tool_call_signature;
#[cfg(test)]
pub(crate) use json::{
    parse_arguments_value, parse_tool_call_value, parse_tool_calls_from_json_value,
};

// MiniMax XML 格式解析器的重导出

// 多种工具调用格式解析器的重导出
// 支持：GLM、Perl 风格、XML 属性、函数调用等多种格式
#[cfg(test)]
pub(crate) use tool_call_formats::{
    default_param_for_tool, parse_glm_shortened_body, parse_glm_style_tool_calls,
    parse_perl_style_tool_calls,
};

// 核心工具调用解析类型和函数的重导出
// ParsedToolCall 是解析结果的核心类型
pub(crate) use tool_calls::{
    ParsedToolCall, detect_tool_call_parse_issue, parse_structured_tool_calls, parse_tool_calls,
};

// XML 解析辅助函数的重导出
// 仅对父模块（loop_）可见

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
#[cfg(test)]
#[path = "tool_calls_tests.rs"]
mod tool_calls_tests;
#[cfg(test)]
#[path = "xml_helpers_tests.rs"]
mod xml_helpers_tests;
