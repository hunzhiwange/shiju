//! JSON 提取功能恢复性测试模块
//!
//! 本模块提供 `extract_json_values` 函数的单元测试，验证其在各种边界条件和异常输入场景下的行为。
//! 主要测试目标包括：
//! - 空输入和纯空白字符的处理
//! - 多个 JSON 对象的连续提取
//! - JSON 数组和对象的混合提取
//!
//! 这些测试属于恢复性测试（Recovery Tests），旨在确保解析器在面对不规范或边缘情况时能够安全、稳定地运行。

use crate::parsing::json::extract_json_values;

/// 测试空字符串输入的处理
///
/// # 测试场景
/// 当输入为空字符串时，应返回空的向量，而不是抛出错误或返回无效数据。
///
/// # 预期行为
/// - 输入：空字符串 `""`
/// - 输出：空的 `Vec<String>`
/// - 验证点：结果向量长度应为 0
#[test]
fn extract_json_values_handles_empty_string() {
    let result = extract_json_values("");
    assert!(result.is_empty());
}

/// 测试仅包含空白字符的输入处理
///
/// # 测试场景
/// 当输入仅包含空白字符（空格、换行符、制表符等）时，应返回空的向量。
/// 这是一种常见的边界情况，通常出现在日志清理或用户输入预处理之后。
///
/// # 预期行为
/// - 输入：包含多种空白字符的字符串 `"   \n\t  "`
/// - 输出：空的 `Vec<String>`
/// - 验证点：结果向量长度应为 0，不应尝试解析任何内容
#[test]
fn extract_json_values_handles_whitespace_only() {
    let result = extract_json_values("   \n\t  ");
    assert!(result.is_empty());
}

/// 测试连续多个 JSON 对象的提取
///
/// # 测试场景
/// 当输入包含多个紧邻的 JSON 对象（无分隔符）时，应能够逐一提取所有对象。
/// 这种情况常见于流式响应或日志文件中，多个 JSON 记录直接拼接在一起。
///
/// # 输入示例
/// ```json
/// {"a": 1}{"b": 2}{"c": 3}
/// ```
///
/// # 预期行为
/// - 输入：三个连续的 JSON 对象
/// - 输出：包含三个元素的 `Vec<String>`
/// - 验证点：结果向量长度应为 3，每个元素应为有效的 JSON 字符串
#[test]
fn extract_json_values_handles_multiple_objects() {
    let input = r#"{"a": 1}{"b": 2}{"c": 3}"#;
    let result = extract_json_values(input);
    assert_eq!(result.len(), 3);
}

/// 测试 JSON 数组与对象的混合提取
///
/// # 测试场景
/// 当输入同时包含 JSON 数组和对象时，应能够正确识别并提取所有 JSON 值。
/// 这验证了解析器对不同 JSON 类型（对象 `{}` 和数组 `[]`）的识别能力。
///
/// # 输入示例
/// ```json
/// [1, 2, 3]{"key": "value"}
/// ```
///
/// # 预期行为
/// - 输入：一个 JSON 数组后跟一个 JSON 对象
/// - 输出：包含两个元素的 `Vec<String>`
/// - 验证点：结果向量长度应为 2，分别对应数组和对时象
#[test]
fn extract_json_values_handles_arrays() {
    let input = r#"[1, 2, 3]{"key": "value"}"#;
    let result = extract_json_values(input);
    assert_eq!(result.len(), 2);
}
