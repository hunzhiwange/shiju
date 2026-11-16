//! 解析恢复测试模块
//!
//! 本模块包含针对代理循环中解析功能的边缘情况测试。
//! 主要测试在各种异常输入情况下，解析器是否能够优雅地恢复并返回合理的结果。
//!
//! # 测试范围
//!
//! - 工具调用参数的空值处理
//! - 仅包含空白字符的工具名称处理
//! - 空字符串参数的处理
//! - 无效 JSON 字符串的参数处理
//! - 缺失参数的处理
//!
//! # 设计原则
//!
//! 这些测试遵循"快速失败"原则，确保解析器在遇到异常输入时：
//! 1. 不会崩溃或 panic
//! 2. 返回合理的默认值（通常是空对象或 None）
//! 3. 保持系统稳定性

use crate::parsing::json::{parse_arguments_value, parse_tool_call_value};

// ═══════════════════════════════════════════════════════════════════════
// 恢复测试 - 工具调用解析边缘情况
// ═══════════════════════════════════════════════════════════════════════

/// 测试解析参数值时对 null 值的处理
///
/// # 测试场景
///
/// 当工具调用的参数为 JSON null 值时，解析器应将其原样返回为 `Value::Null`。
/// 这确保了 null 值能够被正确传递到后续处理流程，而不是被转换为空对象或其他默认值。
///
/// # 预期行为
///
/// - 输入：`serde_json::json!(null)`
/// - 输出：`Value::Null`
#[test]
fn parse_arguments_value_handles_null() {
    let value = serde_json::json!(null);
    let result = parse_arguments_value(Some(&value));
    assert!(result.is_null());
}

/// 测试解析工具调用时对仅含空白字符名称的处理
///
/// # 测试场景
///
/// 当工具调用的名称字段仅包含空白字符（如空格、制表符等）时，
/// 解析器应将其识别为无效工具名称并返回 `None`。
/// 这防止了带有空名称的工具调用被错误地传递到执行层。
///
/// # 预期行为
///
/// - 输入：`{"function": {"name": "   ", "arguments": {}}}`
/// - 输出：`None`
#[test]
fn parse_tool_calls_handles_whitespace_only_name() {
    let value = serde_json::json!({"function": {"name": "   ", "arguments": {}}});
    let result = parse_tool_call_value(&value);
    assert!(result.is_none());
}

/// 测试解析工具调用时对空字符串参数的处理
///
/// # 测试场景
///
/// 当工具调用的参数字段为空字符串（`""`）时，
/// 解析器应仍然能够成功解析工具调用，并将名称正确提取。
/// 空字符串参数通常被视为有效的"无参数"调用。
///
/// # 预期行为
///
/// - 输入：`{"name": "test", "arguments": ""}`
/// - 输出：有效的 `ToolCall`，名称为 "test"
#[test]
fn parse_tool_calls_handles_empty_string_arguments() {
    let value = serde_json::json!({"name": "test", "arguments": ""});
    let result = parse_tool_call_value(&value);
    assert!(result.is_some());
    assert_eq!(result.unwrap().name, "test");
}

// ═══════════════════════════════════════════════════════════════════════
// 恢复测试 - 参数解析
// ═══════════════════════════════════════════════════════════════════════

/// 测试解析参数值时对无效 JSON 字符串的处理
///
/// # 测试场景
///
/// 当工具调用的参数是一个字符串，但其内容不是有效的 JSON 时，
/// 解析器应返回一个空对象 `{}` 作为安全的默认值。
/// 这确保了即使参数格式错误，系统也能继续运行而不会崩溃。
///
/// # 预期行为
///
/// - 输入：`String("not valid json")`
/// - 输出：空对象 `{}`
#[test]
fn parse_arguments_value_handles_invalid_json_string() {
    let value = serde_json::Value::String("not valid json".to_string());
    let result = parse_arguments_value(Some(&value));
    assert!(result.is_object());
    assert!(result.as_object().unwrap().is_empty());
}

/// 测试解析参数值时对缺失参数的处理
///
/// # 测试场景
///
/// 当工具调用完全没有提供参数字段（即 `None`）时，
/// 解析器应返回一个空对象 `{}` 作为安全的默认值。
/// 这确保了无参数的工具调用能够被正确处理。
///
/// # 预期行为
///
/// - 输入：`None`
/// - 输出：空对象 `{}`
#[test]
fn parse_arguments_value_handles_none() {
    let result = parse_arguments_value(None);
    assert!(result.is_object());
    assert!(result.as_object().unwrap().is_empty());
}
