//! 工具调用解析协议测试模块
//!
//! 本模块包含针对工具调用解析和问题检测功能的单元测试。
//! 主要测试以下场景：
//! - 空工具调用数组的处理
//! - 格式错误的工具调用载荷检测
//! - 正常文本的解析行为

use crate::parsing::{detect_tool_call_parse_issue, parse_tool_calls};

/// 测试空工具调用数组的处理行为
///
/// 此测试验证当 JSON 响应中包含空的 `tool_calls` 数组时，
/// 解析器的处理方式。当工具调用数组为空时，整个 JSON 响应
/// 应该作为普通文本返回，而不是尝试解析为工具调用。
///
/// # 测试场景
/// 输入是一个 JSON 对象，包含 `content` 字段和空的 `tool_calls` 数组
///
/// # 断言
/// - 解析后的文本应包含原始 JSON 中的 content 内容
/// - 工具调用列表应为空
#[test]
fn parse_tool_calls_handles_empty_tool_calls_array() {
    // 构造包含空工具调用数组的 JSON 响应
    let response = r#"{"content": "Hello", "tool_calls": []}"#;

    // 解析响应
    let (text, calls) = parse_tool_calls(response);

    // 验证：当 tool_calls 为空时，整个 JSON 作为文本返回
    // 应包含原始 content 字段的内容
    assert!(text.contains("Hello"));
    // 验证：不产生任何工具调用
    assert!(calls.is_empty());
}

/// 测试格式错误的工具调用载荷检测
///
/// 此测试验证 `detect_tool_call_parse_issue` 函数能够正确识别
/// 格式错误的工具调用载荷。未闭合的 JSON 标签应该被标记为
/// 需要诊断的问题。
///
/// # 测试场景
/// 输入包含一个未闭合的工具调用 JSON 标签，缺少结束符号
///
/// # 断言
/// - 应该返回 Some 问题标识
/// - 格式错误的载荷必须被标记以供诊断
#[test]
fn detect_tool_call_parse_issue_flags_malformed_payloads() {
    // 构造格式错误的工具调用（未闭合的 JSON）
    let response = ":UIButtonType{\"name\":\"bash\",\"arguments\":{\"command\":\"pwd\"}✁";

    // 检测问题，传入空的工具调用列表
    let issue = detect_tool_call_parse_issue(response, &[]);

    // 验证：格式错误的载荷应被检测出来
    assert!(issue.is_some(), "malformed tool payload should be flagged for diagnostics");
}

/// 测试正常文本不触发误报
///
/// 此测试验证 `detect_tool_call_parse_issue` 函数不会对
/// 正常的文本内容产生误报。当响应中不包含任何工具调用
/// 相关的内容时，不应该被标记为有解析问题。
///
/// # 测试场景
/// 输入是简单的自然语言文本，不包含任何工具调用语法
///
/// # 断言
/// - 应该返回 None，表示未检测到问题
#[test]
fn detect_tool_call_parse_issue_ignores_normal_text() {
    // 使用普通文本，不含工具调用语法
    let issue = detect_tool_call_parse_issue("Thanks, done.", &[]);

    // 验证：普通文本不应触发问题检测
    assert!(issue.is_none());
}
