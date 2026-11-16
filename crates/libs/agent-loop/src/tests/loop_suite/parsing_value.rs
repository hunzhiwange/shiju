//! 工具调用值解析测试模块
//!
//! 本模块包含针对 `parse_tool_call_value` 函数的单元测试，
//! 验证该函数在各种输入格式和边缘情况下的解析行为。
//!
//! # 测试范围
//!
//! - 缺失必需字段的处理
//! - 顶层字段与嵌套字段的兼容性
//! - 字段别名的支持（如 `parameters` vs `arguments`，`cmd` vs `command`）
//! - 原始字符串参数的恢复
//! - 工具调用 ID 别名的保留

use crate::parsing::json::parse_tool_call_value;

/// 测试缺失 name 字段时的处理
///
/// 当输入的 JSON 值中缺少 `name` 字段（无论是顶层还是嵌套在 `function` 中）时，
/// 解析函数应返回 `None`，表示无法解析为有效的工具调用。
///
/// # 测试场景
///
/// - 输入: `{"function": {"arguments": {}}}`
/// - 缺失: `name` 字段
/// - 预期: 返回 `None`
#[test]
fn parse_tool_call_value_handles_missing_name_field() {
    let value = serde_json::json!({"function": {"arguments": {}}});
    let result = parse_tool_call_value(&value);
    assert!(result.is_none());
}

/// 测试顶层 name 字段的解析（非 OpenAI 标准格式）
///
/// 某些模型可能将 `name` 字段放在顶层而非嵌套在 `function` 对象中，
/// 解析函数应能兼容这种非标准格式。
///
/// # 测试场景
///
/// - 输入: `{"name": "test_tool", "arguments": {}}`
/// - 格式: 非标准顶层结构
/// - 预期: 成功解析，name 为 "test_tool"
#[test]
fn parse_tool_call_value_handles_top_level_name() {
    let value = serde_json::json!({"name": "test_tool", "arguments": {}});
    let result = parse_tool_call_value(&value);
    assert!(result.is_some());
    assert_eq!(result.unwrap().name, "test_tool");
}

/// 测试顶层 `parameters` 字段别名
///
/// 某些工具调用格式使用 `parameters` 而非 `arguments` 作为参数字段名，
/// 解析函数应能识别并处理此别名。
///
/// # 测试场景
///
/// - 输入: `{"name": "schedule", "parameters": {"action": "create", "message": "test"}}`
/// - 字段别名: `parameters` 替代 `arguments`
/// - 预期: 成功解析，参数正确映射到 `arguments` 字段
#[test]
fn parse_tool_call_value_accepts_top_level_parameters_alias() {
    let value = serde_json::json!({
        "name": "schedule",
        "parameters": {"action": "create", "message": "test"}
    });
    let result = parse_tool_call_value(&value).expect("tool call should parse");
    assert_eq!(result.name, "schedule");
    assert_eq!(result.arguments.get("action").and_then(|v| v.as_str()), Some("create"));
}

/// 测试嵌套在 `function` 中的 `parameters` 字段别名
///
/// 验证在标准 OpenAI 格式（`function` 嵌套结构）中，
/// 解析函数能够识别 `parameters` 作为 `arguments` 的别名。
///
/// # 测试场景
///
/// - 输入: `{"function": {"name": "bash", "parameters": {"command": "date"}}}`
/// - 格式: 标准 OpenAI 嵌套结构
/// - 字段别名: `parameters` 替代 `arguments`
/// - 预期: 成功解析，并保留当前工具名 `bash`，参数中包含 `command`
#[test]
fn parse_tool_call_value_accepts_function_parameters_alias() {
    let value = serde_json::json!({
        "function": {
            "name": "bash",
            "parameters": {"command": "date"}
        }
    });
    let result = parse_tool_call_value(&value).expect("tool call should parse");
    assert_eq!(result.name, "bash");
    assert_eq!(result.arguments.get("command").and_then(|v| v.as_str()), Some("date"));
}

/// 测试从原始字符串参数中恢复 bash 命令
///
/// 某些模型可能直接将 bash 命令作为字符串值传递给 `arguments` 字段，
/// 而非预期的 JSON 对象格式。解析函数应能识别这种格式并恢复命令。
///
/// # 测试场景
///
/// - 输入: `{"name": "bash", "arguments": "uname -a"}`
/// - 异常格式: `arguments` 为字符串而非对象
/// - 预期: 成功恢复，并保留当前工具名 `bash`
#[test]
fn parse_tool_call_value_recovers_bash_command_from_raw_string_arguments() {
    let value = serde_json::json!({
        "name": "bash",
        "arguments": "uname -a"
    });
    let result = parse_tool_call_value(&value).expect("tool call should parse");
    assert_eq!(result.name, "bash");
    assert_eq!(result.arguments.get("command").and_then(|v| v.as_str()), Some("uname -a"));
}

/// 测试从 `cmd` 别名恢复 bash 命令
///
/// 某些工具调用可能使用 `cmd` 而非 `command` 作为命令参数的键名，
/// 解析函数应能识别并映射此别名到标准 `command` 字段。
///
/// # 测试场景
///
/// - 输入: `{"function": {"name": "bash", "arguments": {"cmd": "pwd"}}}`
/// - 字段别名: `cmd` 替代 `command`
/// - 预期: 成功解析，并保留当前工具名 `bash`
#[test]
fn parse_tool_call_value_recovers_bash_command_from_cmd_alias() {
    let value = serde_json::json!({
        "function": {
            "name": "bash",
            "arguments": {"cmd": "pwd"}
        }
    });
    let result = parse_tool_call_value(&value).expect("tool call should parse");
    assert_eq!(result.name, "bash");
    assert_eq!(result.arguments.get("command").and_then(|v| v.as_str()), Some("pwd"));
}

/// 测试工具调用 ID 别名的保留
///
/// 工具调用可能使用不同的 ID 字段名（如 `call_id` 而非标准的 `id`），
/// 解析函数应能识别这些别名并保留在解析结果中。
///
/// # 测试场景
///
/// - 输入: `{"call_id": "legacy_1", "function": {"name": "bash", "arguments": {"command": "date"}}}`
/// - 字段别名: `call_id` 替代 `id`
/// - 预期: 成功解析，ID 值保留在 `tool_call_id` 字段中
#[test]
fn parse_tool_call_value_preserves_tool_call_id_aliases() {
    let value = serde_json::json!({
        "call_id": "legacy_1",
        "function": {
            "name": "bash",
            "arguments": {"command": "date"}
        }
    });
    let result = parse_tool_call_value(&value).expect("tool call should parse");
    assert_eq!(result.tool_call_id.as_deref(), Some("legacy_1"));
}
