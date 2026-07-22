//! MiniMax XML 格式工具调用解析器
//!
//! 本模块负责解析 MiniMax 模型特有的 XML 格式工具调用响应。
//! MiniMax 模型使用基于属性 XML 标签的格式来表达工具调用，
//! 而非标准的 JSON 函数调用格式。
//!
//! # 格式说明
//!
//! MiniMax 的工具调用格式如下：
//!
//! ```xml
//! <invoke name="工具名称">
//!     <parameter name="参数名">参数值</parameter>
//! </invoke>
//! ```
//!
//! 例如：
//!
//! ```xml
//! <invoke name="bash">
//!     <parameter name="command">pwd</parameter>
//! </invoke>
//! ```
//!
//! # 解析策略
//!
//! 1. 使用正则表达式匹配 `<invoke>` 标签及其内容
//! 2. 从 invoke 标签中提取 `name` 属性作为工具名称
//! 3. 解析内部的 `<parameter>` 标签提取参数键值对
//! 4. 将参数转换为 JSON 对象格式，以便与其他解析器保持一致
//!
//! # 模块关系
//!
//! 该模块是 `parsing` 模块的一部分，与 `json.rs` 模块协作，
//! 后者负责从参数值中提取 JSON 内容。

use crate::app::agent::agent::loop_::parsing::ParsedToolCall;
use regex::Regex;
use std::sync::LazyLock;

#[cfg(test)]
#[path = "minimax_xml_tests.rs"]
mod minimax_xml_tests;

/// MiniMax invoke 标签正则表达式
///
/// 用于匹配 MiniMax 格式的工具调用标签。
///
/// # 正则表达式详解
///
/// - `(?is)` - 启用单行模式（`.` 匹配换行符）和大小写不敏感模式
/// - `<invoke\b` - 匹配 invoke 标签开头（`\b` 确保是完整单词）
/// - `[^>]*` - 匹配除 `>` 外的任意字符（标签内的其他属性）
/// - `\bname\s*=\s*` - 匹配 name 属性名及等号（允许空白）
/// - `(?:\"([^\"]+)\"|'([^']+)')` - 匹配双引号或单引号包裹的属性值
///   - 第一个捕获组：双引号内的值
///   - 第二个捕获组：单引号内的值
/// - `[^>]*>` - 匹配标签的剩余部分和闭合的 `>`
/// - `(.*?)` - 第三个捕获组：标签内容（非贪婪匹配）
/// - `</invoke>` - 匹配闭合标签
///
/// # 匹配示例
///
/// 输入：`<invoke name="bash"><parameter name="command">pwd</parameter></invoke>`
/// - 捕获组 0：完整匹配
/// - 捕获组 1 或 2：工具名称 "bash"
/// - 捕获组 3：标签内容 `<parameter name="command">pwd</parameter>`
static MINIMAX_INVOKE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<invoke\b[^>]*\bname\s*=\s*(?:\"([^\"]+)\"|'([^']+)')[^>]*>(.*?)</invoke>"#)
        .unwrap()
});

/// MiniMax parameter 标签正则表达式
///
/// 用于匹配 MiniMax 格式的参数标签，从 invoke 标签内容中提取参数。
///
/// # 正则表达式详解
///
/// - `(?is)` - 启用单行模式和大小写不敏感模式
/// - `<parameter\b` - 匹配 parameter 标签开头
/// - `[^>]*` - 匹配标签内的其他属性
/// - `\bname\s*=\s*` - 匹配 name 属性名及等号
/// - `(?:\"([^\"]+)\"|'([^']+)')` - 匹配双引号或单引号包裹的属性名
///   - 第一个捕获组：双引号内的参数名
///   - 第二个捕获组：单引号内的参数名
/// - `[^>]*>` - 匹配标签的剩余部分和闭合的 `>`
/// - `(.*?)` - 第三个捕获组：参数值（非贪婪匹配）
/// - `</parameter>` - 匹配闭合标签
///
/// # 匹配示例
///
/// 输入：`<parameter name="command">pwd</parameter>`
/// - 捕获组 1 或 2：参数名 "command"
/// - 捕获组 3：参数值 "pwd"
static MINIMAX_PARAMETER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?is)<parameter\b[^>]*\bname\s*=\s*(?:\"([^\"]+)\"|'([^']+)')[^>]*>(.*?)</parameter>"#,
    )
    .unwrap()
});

/// 解析 MiniMax 风格的 XML 工具调用
///
/// 该函数从 MiniMax 模型的响应文本中提取工具调用信息。
/// 它会解析带有属性的 invoke/parameter 标签，并将其转换为
/// 统一的 `ParsedToolCall` 结构。
///
/// # 参数
///
/// * `response` - MiniMax 模型的原始响应文本，可能包含一个或多个
///   `<invoke>` 标签以及普通文本内容。
///
/// # 返回值
///
/// 返回 `Option<(String, Vec<ParsedToolCall>)>`：
/// - `Some((text, calls))` - 解析成功时返回元组：
///   - `text`：去除工具调用后的纯文本内容，已清理 MiniMax 特有的标签标记
///   - `calls`：解析出的工具调用列表
/// - `None` - 响应中未找到任何工具调用
///
/// # 解析逻辑
///
/// ## 第一阶段：遍历匹配 invoke 标签
///
/// 1. 使用 `MINIMAX_INVOKE_RE` 正则表达式查找所有 `<invoke>` 标签
/// 2. 对于每个匹配，记录标签前的文本内容（非空白部分）
/// 3. 从捕获组中提取工具名称（支持双引号或单引号格式）
///
/// ## 第二阶段：解析参数
///
/// 1. 使用 `MINIMAX_PARAMETER_RE` 从 invoke 标签内容中提取参数
/// 2. 将参数值尝试解析为 JSON，失败则保持为字符串
/// 3. 参数存入 JSON Map 中，键为参数名，值为参数值
///
/// ## 第三阶段：处理无参数情况
///
/// 如果未找到任何 parameter 标签，函数会尝试以下备选方案：
/// 1. 尝试将整个 invoke 标签内容解析为 JSON 对象
/// 2. 如果是 JSON 对象，直接作为参数使用
/// 3. 如果是其他 JSON 类型，包装在 "value" 键下
/// 4. 如果非 JSON 且非空，包装在 "content" 键下
///
/// ## 第四阶段：清理文本
///
/// 最终文本会移除以下 MiniMax 特有的标记：
/// - `<minimax:tool_call>` 和 `</minimax:tool_call>`
/// - `<minimax:toolcall>` 和 `</minimax:toolcall>`
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::agent::loop_::parsing::minimax_xml::parse_minimax_invoke_calls;
///
/// let response = r#"这是一段说明文字。<invoke name="bash"><parameter name="command">pwd</parameter></invoke>执行后的结果。"#;
///
/// if let Some((text, calls)) = parse_minimax_invoke_calls(response) {
///     println!("文本: {}", text);
///     println!("工具调用数量: {}", calls.len());
///     for call in calls {
///         println!("  工具名: {}", call.name);
///         println!("  参数: {}", call.arguments);
///     }
/// }
/// ```
///
/// # 兼容性说明
///
/// - 支持双引号和单引号包裹的属性值
/// - 标签内的空白会被自动处理
/// - 参数值中的 JSON 内容会被自动识别和解析
/// - 工具调用 ID 字段（`tool_call_id`）在 MiniMax 格式中不可用，始终为 `None`
pub(crate) fn parse_minimax_invoke_calls(response: &str) -> Option<(String, Vec<ParsedToolCall>)> {
    // 存储解析出的工具调用
    let mut calls = Vec::new();
    // 存储工具调用之间的文本片段
    let mut text_parts = Vec::new();
    // 记录上次匹配结束的位置，用于提取工具调用之间的文本
    let mut last_end = 0usize;

    // 遍历所有匹配的 invoke 标签
    for cap in MINIMAX_INVOKE_RE.captures_iter(response) {
        // 获取完整匹配的范围，用于计算文本位置
        let full_match = cap.get(0).expect("regex captures include the full match");

        // 提取当前 invoke 标签之前的文本内容
        // 这部分文本是上次匹配结束到当前匹配开始之间的内容
        let before = response[last_end..full_match.start()].trim();
        if !before.is_empty() {
            text_parts.push(before.to_string());
        }

        // 从捕获组中提取工具名称
        // 捕获组 1 是双引号格式，捕获组 2 是单引号格式
        let name =
            cap.get(1).or_else(|| cap.get(2)).map(|m| m.as_str().trim()).filter(|v| !v.is_empty());
        // 获取 invoke 标签的内部内容（用于后续解析参数）
        let body = cap.get(3).map(|m| m.as_str()).unwrap_or("").trim();
        // 更新最后匹配位置，为下一次迭代做准备
        last_end = full_match.end();

        // 如果无法提取工具名称，跳过此匹配
        let Some(name) = name else {
            continue;
        };

        // 解析参数，存储为 JSON Map
        let mut args = serde_json::Map::new();

        // 遍历 invoke 标签内容中的所有 parameter 标签
        for param_cap in MINIMAX_PARAMETER_RE.captures_iter(body) {
            // 提取参数名称（支持双引号或单引号格式）
            let key = param_cap
                .get(1)
                .or_else(|| param_cap.get(2))
                .map(|m| m.as_str().trim())
                .unwrap_or_default();

            // 跳过空参数名
            if key.is_empty() {
                continue;
            }

            // 提取参数值
            let value = param_cap.get(3).map(|m| m.as_str().trim()).unwrap_or_default();

            // 跳过空参数值
            if value.is_empty() {
                continue;
            }

            // 尝试从参数值中提取 JSON 内容
            // 这允许参数值本身是 JSON 对象、数组或其他类型
            let parsed = super::json::extract_json_values(value).into_iter().next();

            // 将参数插入到 args Map 中
            // 如果无法解析为 JSON，则作为字符串处理
            args.insert(
                key.to_string(),
                parsed.unwrap_or_else(|| serde_json::Value::String(value.to_string())),
            );
        }

        // 处理未找到任何 parameter 标签的情况
        if args.is_empty() {
            // 尝试将整个 invoke 标签内容解析为 JSON
            if let Some(first_json) = super::json::extract_json_values(body).into_iter().next() {
                match first_json {
                    // 如果是 JSON 对象，直接作为参数使用
                    serde_json::Value::Object(obj) => args = obj,
                    // 如果是其他 JSON 类型（字符串、数字、数组等），
                    // 包装在 "value" 键下以保持一致性
                    other => {
                        args.insert("value".to_string(), other);
                    }
                }
            } else if !body.is_empty() {
                // 如果内容不是 JSON 但非空，将其作为字符串包装在 "content" 键下
                args.insert("content".to_string(), serde_json::Value::String(body.to_string()));
            }
        }

        // 构建解析后的工具调用并添加到列表
        calls.push(ParsedToolCall {
            name: name.to_string(),
            arguments: serde_json::Value::Object(args),
            // MiniMax 格式不包含工具调用 ID
            tool_call_id: None,
        });
    }

    // 如果没有解析到任何工具调用，返回 None
    if calls.is_empty() {
        return None;
    }

    // 提取最后一个工具调用之后的文本内容
    let after = response[last_end..].trim();
    if !after.is_empty() {
        text_parts.push(after.to_string());
    }

    // 构建最终文本
    // 1. 用换行符连接所有文本片段
    // 2. 移除 MiniMax 特有的工具调用标记（多种格式变体）
    // 3. 清理首尾空白
    let text = text_parts
        .join("\n")
        .replace("<minimax:tool_call>", "")
        .replace("</minimax:tool_call>", "")
        .replace("<minimax:toolcall>", "")
        .replace("</minimax:toolcall>", "")
        .trim()
        .to_string();

    // 返回清理后的文本和解析出的工具调用列表
    Some((text, calls))
}
