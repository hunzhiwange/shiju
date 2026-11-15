//! 工具调用标签移除测试模块
//!
//! 本模块包含针对 `strip_tool_call_tags` 函数的全面测试用例，验证该函数能够正确
//! 从消息文本中移除各种格式的工具调用标签。
//!
//! # 测试覆盖范围
//!
//! ## 支持的标签格式
//!
//! 函数需要处理多种工具调用标签格式：
//! - `<tool>...</tool>` - 标准工具调用标签
//! - `<toolcall>...</toolcall>` - 工具调用别名标签
//! - `<tool-call>...</tool-call>` - 带连字符的工具调用标签
//! - `<invoke>...</invoke>` - 调用标签
//! - `erte...ttri` - 非标准格式的工具调用标记
//!
//! ## 测试场景
//!
//! - 单个标签移除
//! - 多个标签移除
//! - 混合标签格式
//! - 未闭合标签处理
//! - JSON 内容移除
//! - 额外换行符清理
//! - 空输入处理
//!
//! # 设计原则
//!
//! 这些测试确保工具调用标签不会泄露到最终用户可见的消息中，同时保持
//! 原始文本的其他部分完整无损。

use super::*;

/// 测试移除标准 `<tool>` 标签
///
/// # 测试目的
///
/// 验证函数能够正确移除标准的 `<tool>...</tool>` 标签及其内部的 JSON 内容。
///
/// # 测试场景
///
/// - 输入包含 `<tool>` 标签，内部包含工具名称和参数的 JSON
/// - 预期输出应为移除标签后的纯文本，保留前后的普通文本
///
/// # 示例
///
/// ```text
/// 输入: "Hello <tool>{"name":"shell","arguments":{"command":"ls"}}</tool> world"
/// 输出: "Hello  world"
/// ```
#[test]
fn strip_tool_call_tags_removes_standard_tags() {
    let input = "Hello <tool>{\"name\":\"shell\",\"arguments\":{\"command\":\"ls\"}}</tool> world";
    let result = strip_tool_call_tags(input);
    assert_eq!(result, "Hello  world");
}

/// 测试移除 `<toolcall>` 别名标签
///
/// # 测试目的
///
/// 验证函数能够识别并移除 `<toolcall>...</toolcall>` 标签格式，
/// 这是工具调用标签的一个常见别名变体。
///
/// # 测试场景
///
/// - 输入包含 `<toolcall>` 标签及其 JSON 内容
/// - 预期输出应为标签被完全移除后的文本
///
/// # 示例
///
/// ```text
/// 输入: "Hello <toolcall>{"name":"shell",...}</toolcall> world"
/// 输出: "Hello  world"
/// ```
#[test]
fn strip_tool_call_tags_removes_alias_tags() {
    let input =
        "Hello <toolcall>{\"name\":\"shell\",\"arguments\":{\"command\":\"ls\"}}</toolcall> world";
    let result = strip_tool_call_tags(input);
    assert_eq!(result, "Hello  world");
}

/// 测试移除带连字符的 `<tool-call>` 标签
///
/// # 测试目的
///
/// 验证函数能够处理使用连字符分隔的 `<tool-call>...</tool-call>` 标签格式。
///
/// # 测试场景
///
/// - 输入包含带连字符的 `<tool-call>` 标签
/// - 预期输出应为标签及其内容被移除后的文本
///
/// # 示例
///
/// ```text
/// 输入: "Hello <tool-call>{"name":"shell",...}</tool-call> world"
/// 输出: "Hello  world"
/// ```
#[test]
fn strip_tool_call_tags_removes_dash_tags() {
    let input = "Hello <tool-call>{\"name\":\"shell\",\"arguments\":{\"command\":\"ls\"}}</tool-call> world";
    let result = strip_tool_call_tags(input);
    assert_eq!(result, "Hello  world");
}

/// 测试移除非标准格式的工具调用标记
///
/// # 测试目的
///
/// 验证函数能够识别并移除非标准格式的工具调用标记（如 `erte...ttri`），
/// 这些可能是某些特定场景下使用的自定义标记格式。
///
/// # 测试场景
///
/// - 输入包含 `erte` 开头和 `ttri` 结尾的非标准标记
/// - 标记内部包含 JSON 格式的工具调用数据
/// - 预期输出应为标记被移除后的文本
///
/// # 示例
///
/// ```text
/// 输入: "Hello erte{"name":"shell",...}ttri world"
/// 输出: "Hello  world"
/// ```
#[test]
fn strip_tool_call_tags_removes_tool_call_tags() {
    let input = "Hello erte{\"name\":\"shell\",\"arguments\":{\"command\":\"ls\"}}ttri world";
    let result = strip_tool_call_tags(input);
    assert_eq!(result, "Hello  world");
}

/// 测试移除 `<invoke>` 调用标签
///
/// # 测试目的
///
/// 验证函数能够处理 `<invoke>...</invoke>` 标签格式，
/// 这是另一种常见的工具调用表示方式。
///
/// # 测试场景
///
/// - 输入包含 `<invoke>` 标签及其 JSON 内容
/// - 预期输出应为标签及其内容被完全移除
///
/// # 示例
///
/// ```text
/// 输入: "Hello <invoke>{"name":"shell",...}</invoke> world"
/// 输出: "Hello  world"
/// ```
#[test]
fn strip_tool_call_tags_removes_invoke_tags() {
    let input =
        "Hello <invoke>{\"name\":\"shell\",\"arguments\":{\"command\":\"date\"}}</invoke> world";
    let result = strip_tool_call_tags(input);
    assert_eq!(result, "Hello  world");
}

/// 测试处理多个相同格式的标签
///
/// # 测试目的
///
/// 验证函数能够在同一段文本中正确移除多个 `<tool>` 标签，
/// 而不会产生遗漏或错误处理。
///
/// # 测试场景
///
/// - 输入包含两个独立的 `<tool>` 标签
/// - 预期输出应为所有标签都被移除，保留中间和两端的普通文本
///
/// # 示例
///
/// ```text
/// 输入: "Start <tool>a</tool> middle <tool>b</tool> end"
/// 输出: "Start  middle  end"
/// ```
#[test]
fn strip_tool_call_tags_handles_multiple_tags() {
    let input = "Start <tool>a</tool> middle <tool>b</tool> end";
    let result = strip_tool_call_tags(input);
    assert_eq!(result, "Start  middle  end");
}

/// 测试处理混合格式的多个标签
///
/// # 测试目的
///
/// 验证函数能够在同一段文本中正确处理不同格式的工具调用标签，
/// 包括 `<tool>`、`<toolcall>` 和 `<tool-call>` 等多种格式。
///
/// # 测试场景
///
/// - 输入包含三种不同格式的工具调用标签
/// - 预期输出应为所有格式的标签都被正确移除
///
/// # 示例
///
/// ```text
/// 输入: "A <tool>a</tool> B <toolcall>b</toolcall> C <tool-call>c</tool-call> D"
/// 输出: "A  B  C  D"
/// ```
#[test]
fn strip_tool_call_tags_handles_mixed_tags() {
    let input = "A <tool>a</tool> B <toolcall>b</toolcall> C <tool-call>c</tool-call> D";
    let result = strip_tool_call_tags(input);
    assert_eq!(result, "A  B  C  D");
}

/// 测试保留不包含标签的普通文本
///
/// # 测试目的
///
/// 验证函数对不包含任何工具调用标签的纯文本保持原样，
/// 不会进行不必要的修改或处理。
///
/// # 测试场景
///
/// - 输入为普通的文本消息，不包含任何特殊标签
/// - 预期输出应与输入完全相同
///
/// # 示例
///
/// ```text
/// 输入: "Hello world! This is a test."
/// 输出: "Hello world! This is a test."
/// ```
#[test]
fn strip_tool_call_tags_preserves_normal_text() {
    let input = "Hello world! This is a test.";
    let result = strip_tool_call_tags(input);
    assert_eq!(result, "Hello world! This is a test.");
}

/// 测试处理未闭合的标签
///
/// # 测试目的
///
/// 验证函数对格式不完整（缺少闭合标签）的情况能够安全处理，
/// 不会引发错误或产生意外的输出。
///
/// # 测试场景
///
/// - 输入包含起始标签但没有对应的闭合标签
/// - 预期输出应保持原样，不做修改
///
/// # 设计考虑
///
/// 这种情况可能出现在消息被截断或格式错误的场景中，
/// 函数应当采取保守策略，保留原始内容而非尝试猜测意图。
///
/// # 示例
///
/// ```text
/// 输入: "Hello <tool>world"
/// 输出: "Hello <tool>world"
/// ```
#[test]
fn strip_tool_call_tags_handles_unclosed_tags() {
    let input = "Hello <tool>world";
    let result = strip_tool_call_tags(input);
    assert_eq!(result, "Hello <tool>world");
}

/// 测试处理未闭合的非标准工具调用标记（带 JSON 内容）
///
/// # 测试目的
///
/// 验证函数对包含 JSON 内容但不完整的非标准工具调用标记的处理能力，
/// 确保能够清理不完整的标记及其内容。
///
/// # 测试场景
///
/// - 输入包含 `erte` 开头但缺少 `ttri` 结尾的非标准标记
/// - 标记内部包含完整的 JSON 工具调用数据
/// - 预期输出应为移除不完整标记及其内容后的文本
///
/// # 示例
///
/// ```text
/// 输入: "Status:\nerte\n{"name":"shell","arguments":{"command":"uptime"}}"
/// 输出: "Status:"
/// ```
#[test]
fn strip_tool_call_tags_handles_unclosed_tool_call_with_json() {
    let input = "Status:\nerte\n{\"name\":\"shell\",\"arguments\":{\"command\":\"uptime\"}}";
    let result = strip_tool_call_tags(input);
    assert_eq!(result, "Status:");
}

/// 测试处理标签配对不匹配的情况
///
/// # 测试目的
///
/// 验证函数对非标准标记 `erte...ttri` 的处理能力，
/// 即使格式看起来不匹配标准的 XML 样式标签。
///
/// # 测试场景
///
/// - 输入只包含 `erte...ttri` 标记及其 JSON 内容
/// - 预期输出应为空字符串（标记及其内容被完全移除）
///
/// # 示例
///
/// ```text
/// 输入: "erte{"name":"shell",...}ttri"
/// 输出: ""
/// ```
#[test]
fn strip_tool_call_tags_handles_mismatched_close_tag() {
    let input = "erte{\"name\":\"shell\",\"arguments\":{\"command\":\"uptime\"}}ttri";
    let result = strip_tool_call_tags(input);
    assert_eq!(result, "");
}

/// 测试清理标签周围的额外换行符
///
/// # 测试目的
///
/// 验证函数在移除标签时能够同时清理标签前后多余的换行符，
/// 避免在最终输出中留下过多的空白行。
///
/// # 测试场景
///
/// - 输入包含标签，且标签前后有多个换行符
/// - 预期输出应保留合理的换行结构，移除标签相关的过多空白
///
/// # 示例
///
/// ```text
/// 输入: "Hello\n\n<tool>\ntest\n</tool>\n\n\nworld"
/// 输出: "Hello\n\nworld"
/// ```
#[test]
fn strip_tool_call_tags_cleans_extra_newlines() {
    let input = "Hello\n\n<tool>\ntest\n</tool>\n\n\nworld";
    let result = strip_tool_call_tags(input);
    assert_eq!(result, "Hello\n\nworld");
}

/// 测试处理空输入字符串
///
/// # 测试目的
///
/// 验证函数对空字符串的边界情况处理正确，
/// 不会引发错误或产生意外的输出。
///
/// # 测试场景
///
/// - 输入为空字符串
/// - 预期输出也应为空字符串
///
/// # 示例
///
/// ```text
/// 输入: ""
/// 输出: ""
/// ```
#[test]
fn strip_tool_call_tags_handles_empty_input() {
    let input = "";
    let result = strip_tool_call_tags(input);
    assert_eq!(result, "");
}

/// 测试处理仅包含标签的输入
///
/// # 测试目的
///
/// 验证当输入完全由工具调用标签组成时，函数能够正确返回空字符串。
///
/// # 测试场景
///
/// - 输入只包含一个完整的工具调用标签
/// - 预期输出应为空字符串（标签及其内容被完全移除）
///
/// # 示例
///
/// ```text
/// 输入: "<tool>{"name":"test"}</tool>"
/// 输出: ""
/// ```
#[test]
fn strip_tool_call_tags_handles_only_tags() {
    let input = "<tool>{\"name\":\"test\"}</tool>";
    let result = strip_tool_call_tags(input);
    assert_eq!(result, "");
}
