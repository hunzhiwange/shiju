//! 边缘情况解析测试模块
//!
//! 本模块包含针对 `parse_tool_calls` 函数的健壮性和边缘情况测试。
//! 这些测试确保解析器能够正确处理各种异常输入、格式错误以及边界条件，
//! 从而防止系统在面对非标准输入时崩溃或产生不可预期的行为。
//!
//! # 测试覆盖范围
//!
//! - 空输入和纯空白字符输入
//! - 嵌套 XML 标签处理
//! - 截断的 JSON 数据
//! - 空的 JSON 对象
//! - 孤立的闭合标签
//! - 超大参数值
//! - 特殊字符处理
//! - 嵌入文本中的原始 JSON（不应被提取）
//! - 混合格式内容
//!
//! # 关联问题
//!
//! 这些测试旨在防止以下模式相关的问题：
//! - Issue #746, #418, #777, #848

use crate::parsing::parse_tool_calls;

// ════════════════════════════════════════════════════════════════════════════
// TG4 (内联): parse_tool_calls 健壮性测试 — 格式错误/边缘情况输入
// 防止: 模式 4 问题 #746, #418, #777, #848
// ════════════════════════════════════════════════════════════════════════════

/// 测试空输入返回空结果
///
/// 验证当输入为空字符串时，`parse_tool_calls` 函数应返回：
/// - 空的工具调用列表
/// - 空的文本内容
///
/// # 预期行为
///
/// 空输入不应导致 panic 或错误，而应优雅地返回空结果。
#[test]
fn parse_tool_calls_empty_input_returns_empty() {
    let (text, calls) = parse_tool_calls("");
    // 断言：空输入应产生空的工具调用列表
    assert!(calls.is_empty(), "empty input should produce no tool calls");
    // 断言：空输入应产生空的文本内容
    assert!(text.is_empty(), "empty input should produce no text");
}

/// 测试纯空白字符输入返回空的工具调用列表
///
/// 验证当输入仅包含空白字符（空格、换行、制表符）时，
/// 不应提取任何工具调用，且返回的文本应为空或仅包含空白。
///
/// # 预期行为
///
/// 纯空白输入应被正确处理，不产生任何工具调用。
#[test]
fn parse_tool_calls_whitespace_only_returns_empty_calls() {
    let (text, calls) = parse_tool_calls("   \n\t  ");
    // 断言：纯空白输入不应产生工具调用
    assert!(calls.is_empty());
    // 断言：返回的文本应为空或仅包含可修剪的空白字符
    assert!(text.is_empty() || text.trim().is_empty());
}

/// 测试嵌套 XML 标签的处理
///
/// 验证当工具调用标签被双重包装时，解析器仍能正确提取内部的工具调用。
///
/// # 测试场景
///
/// 输入包含双重嵌套的 `<tool_call 标签，内层才是真正的工具调用。
///
/// # 预期行为
///
/// 应至少提取出一个有效的工具调用。
#[test]
fn parse_tool_calls_nested_xml_tags_handled() {
    // 构造双重包装的工具调用 XML
    // 外层和内层都有 <tool_call 标签，解析器应能提取内层调用
    let response =
        r#"<tool_call<tool_call{"name":"echo","arguments":{"msg":"hi"}}</tool_call</tool_call>"#;
    let (_text, calls) = parse_tool_calls(response);
    // 断言：嵌套标签应至少解析出一个工具调用
    assert!(!calls.is_empty(), "nested XML tags should still yield at least one tool call");
}

/// 测试截断的 JSON 不会导致 panic
///
/// 验证当工具调用标签内的 JSON 不完整时，解析器能优雅处理而不会崩溃。
///
/// # 测试场景
///
/// JSON 对象在中间被截断，缺少闭合的括号和引号。
///
/// # 预期行为
///
/// 不应触发 panic；函数应安全返回，可能返回空结果或部分解析结果。
#[test]
fn parse_tool_calls_truncated_json_no_panic() {
    // 构造包含不完整 JSON 的输入
    // JSON 在 "ls" 之后被截断，缺少闭合的括号和标签
    let response = r#"<tool_call{"name":"shell","arguments":{"command":"ls"</tool_call>"#;
    let (_text, _calls) = parse_tool_calls(response);
    // 此测试的重点是确保不发生 panic
    // 如果代码执行到这里，说明解析器安全处理了截断的 JSON
}

/// 测试空 JSON 对象不产生工具调用
///
/// 验证当工具调用标签内仅包含空的 JSON 对象 `{}` 时，
/// 不会产生有效的工具调用。
///
/// # 预期行为
///
/// 空的 JSON 对象没有 `name` 字段，因此不应产生工具调用。
#[test]
fn parse_tool_calls_empty_json_object_in_tag() {
    let response = "<tool_call{}</tool_call>";
    let (_text, calls) = parse_tool_calls(response);
    // 断言：空的 JSON 对象不应产生工具调用
    assert!(calls.is_empty(), "empty JSON object should not produce a tool call");
}

/// 测试仅存在闭合标签时返回文本
///
/// 验证当输入包含孤立的闭合标签（缺少开始标签）时，
/// 标签周围的文本应被保留，且不产生工具调用。
///
/// # 预期行为
///
/// 孤立的闭合标签不应产生工具调用；
/// 标签周围的文本内容应被完整保留。
#[test]
fn parse_tool_calls_closing_tag_only_returns_text() {
    let response = "Some text </tool_call> more text";
    let (text, calls) = parse_tool_calls(response);
    // 断言：孤立的闭合标签不应产生工具调用
    assert!(calls.is_empty(), "closing tag only should not produce calls");
    // 断言：标签周围的文本应被保留
    assert!(!text.is_empty(), "text around orphaned closing tag should be preserved");
}

/// 测试超大参数不会导致 panic
///
/// 验证当工具调用的参数包含非常大的值（如 100,000 个字符）时，
/// 解析器仍能正常工作而不崩溃或超时。
///
/// # 测试场景
///
/// 构造一个包含 100,000 个字符的参数值的工具调用。
///
/// # 预期行为
///
/// - 应成功解析出工具调用
/// - 工具调用的 `name` 字段应为 "echo"
/// - 参数应被完整提取
#[test]
fn parse_tool_calls_very_large_arguments_no_panic() {
    // 构造一个包含 100,000 个字符的超大参数
    let large_arg = "x".repeat(100_000);
    let response = format!(
        r#"<tool_call{{"name":"echo","arguments":{{"message":"{}"}}}}</tool_call>"#,
        large_arg
    );
    let (_text, calls) = parse_tool_calls(&response);
    // 断言：超大参数应被成功解析
    assert_eq!(calls.len(), 1, "large arguments should still parse");
    // 断言：工具名称应正确提取
    assert_eq!(calls[0].name, "echo");
}

/// 测试参数中的特殊字符处理
///
/// 验证当工具调用参数包含特殊字符时，解析器能正确处理。
///
/// # 测试的特殊字符
///
/// - 双引号 `"`
/// - 尖括号 `<>`
/// - 和号 `&`
/// - 单引号 `'`
/// - 换行符 `\n`
/// - 制表符 `\t`
///
/// # 预期行为
///
/// 应成功解析工具调用，且工具名称正确。
#[test]
fn parse_tool_calls_special_characters_in_arguments() {
    // 构造包含多种特殊字符的参数
    let response = r#"<tool_call{"name":"echo","arguments":{"message":"hello \"world\" <>&'\n\t"}}</tool_call>"#;
    let (_text, calls) = parse_tool_calls(response);
    // 断言：应成功解析出一个工具调用
    assert_eq!(calls.len(), 1);
    // 断言：工具名称应为 "echo"
    assert_eq!(calls[0].name, "echo");
}

/// 测试嵌入文本中的原始 JSON 不被提取
///
/// 验证当普通文本中包含 JSON 格式数据但没有工具调用标签时，
/// 这些 JSON 不应被错误地提取为工具调用。
///
/// # 测试场景
///
/// 文本中包含有效的工具调用 JSON 结构，但没有任何 XML 标签包裹。
///
/// # 预期行为
///
/// 没有 `<tool_call` 标签包裹的 JSON 不应被提取为工具调用。
#[test]
fn parse_tool_calls_text_with_embedded_json_not_extracted() {
    // 构造包含 JSON 但没有标签的纯文本
    // 这种情况下的 JSON 只是普通文本内容，不应被解析为工具调用
    let response = r#"Here is some data: {"name":"echo","arguments":{"message":"hi"}} end."#;
    let (_text, calls) = parse_tool_calls(response);
    // 断言：没有标签包裹的 JSON 不应被提取
    assert!(calls.is_empty(), "raw JSON in text without tags should not be extracted");
}

/// 测试混合格式内容的解析
///
/// 验证当输入包含普通文本和工具调用的混合内容时，
/// 解析器能正确提取工具调用并保留文本内容。
///
/// # 测试场景
///
/// 输入包含：
/// 1. 工具调用前的说明文本
/// 2. 格式化的工具调用（带有换行和缩进）
/// 3. 工具调用后的结果说明文本
///
/// # 预期行为
///
/// - 应提取出一个工具调用
/// - 工具名称应收敛为内部 canonical 名称 `shell`
/// - 工具调用前的文本应被保留
#[test]
fn parse_tool_calls_multiple_formats_mixed() {
    // 构造混合内容：文本 + 工具调用 + 文本
    let response = r#"I'll help you with that.

<tool_call>
{"name":"bash","arguments":{"command":"echo hello"}}
</tool_call>

Let me check the result."#;
    let (text, calls) = parse_tool_calls(response);
    // 断言：应从混合内容中提取出一个工具调用
    assert_eq!(calls.len(), 1, "should extract one tool call from mixed content");
    // 断言：工具名称应收敛为内部 canonical 名称 shell
    assert_eq!(calls[0].name, "shell");
    // 断言：工具调用前的文本应被保留
    assert!(text.contains("help you"), "text before tool call should be preserved");
}
