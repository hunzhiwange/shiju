//! 结构化工具调用解析测试模块
//!
//! 本模块包含针对工具调用解析函数的单元测试，主要测试以下功能：
//! - 从 JSON 值中解析工具调用的各种边界情况
//! - 处理空数组、缺失字段、顶层数组等异常格式
//! - 从字符串载荷中恢复结构化的工具调用参数
//!
//! 这些测试确保解析逻辑在非标准输入下仍能优雅地恢复或正确处理。

use super::*;

/// 测试解析空工具调用数组
///
/// 验证当输入 JSON 包含空的 tool_calls 数组时，
/// 解析函数应返回空的工具调用向量，而不是错误或 None。
///
/// 测试场景：
/// - 输入：`{"tool_calls": []}`
/// - 期望输出：空的 Vec
#[test]
fn parse_tool_calls_from_json_value_handles_empty_array() {
    let value = serde_json::json!({"tool_calls": []});
    let result = parse_tool_calls_from_json_value(&value);

    assert!(result.is_empty());
}

/// 测试处理缺失 tool_calls 字段的情况
///
/// 验证当输入 JSON 缺少 tool_calls 字段但有其他工具调用字段时，
/// 解析函数应能够通过降级路径正确提取工具调用。
///
/// 测试场景：
/// - 输入：`{"name": "test", "arguments": {}}`
/// - 期望输出：包含一个工具调用的 Vec
#[test]
fn parse_tool_calls_from_json_value_handles_missing_tool_calls() {
    let value = serde_json::json!({"name": "test", "arguments": {}});
    let result = parse_tool_calls_from_json_value(&value);

    assert_eq!(result.len(), 1);
}

/// 测试处理顶层工具调用数组
///
/// 验证当输入 JSON 直接是工具调用数组时，
/// 解析函数应能够识别并正确解析所有工具调用。
///
/// 测试场景：
/// - 输入：顶层 JSON 数组，包含多个工具调用对象
/// - 期望输出：包含所有工具调用的 Vec
#[test]
fn parse_tool_calls_from_json_value_handles_top_level_array() {
    let value = serde_json::json!([
        {"name": "tool_a", "arguments": {}},
        {"name": "tool_b", "arguments": {}}
    ]);
    let result = parse_tool_calls_from_json_value(&value);

    assert_eq!(result.len(), 2);
}

/// 测试从字符串载荷中恢复 bash 命令
///
/// 验证当工具调用的 arguments 字段是字符串而非 JSON 对象时，
/// 解析函数应能够将其转换为结构化的参数格式。
///
/// 测试场景：
/// - 输入：bash 工具调用，arguments 为字符串 "ls -la"
/// - 期望输出：解析后的工具调用保留当前 `bash` 工具 ID，并包含结构化的 command 参数
///
/// 这是容错机制的重要测试，确保即使模型返回非标准格式，
/// 系统也能正确处理。
#[test]
fn parse_structured_tool_calls_recovers_bash_command_from_string_payload() {
    let calls = vec![ToolCall {
        kind: Default::default(),
        namespace: None,
        id: "call_1".to_string(),
        name: "bash".to_string(),
        arguments: "ls -la".to_string(),
    }];

    let parsed = parse_structured_tool_calls(&calls);

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].name, "bash");

    let command = parsed[0].arguments.get("command").and_then(|v| v.as_str());

    assert_eq!(command, Some("ls -la"));
}
