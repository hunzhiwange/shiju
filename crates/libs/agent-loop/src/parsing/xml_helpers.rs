//! XML 辅助解析模块
//!
//! 本模块提供用于解析 XML 格式工具调用的一系列辅助函数和常量。
//! 支持多种 XML 标签格式，包括标准 XML 标签和特殊分隔符格式。
//!
//! # 主要功能
//!
//! - 提取 XML 标签对（`<tag>...</tag>`）
//! - 解析 XML 格式的工具调用
//! - 识别和匹配工具调用的开始/结束标签
//!
//! # 支持的标签格式
//!
//! 本模块支持以下工具调用标签格式：
//! - `{{-- ... --}}` 特殊分隔符格式
//! - `<toolcall>...</toolcall>` 标准工具调用标签
//! - `<tool-call>...</tool-call>` 连字符格式
//! - `<invoke>...</invoke>` 调用格式
//! - `<minimax:tool_call>...</minimax:tool_call>` 带命名空间格式

use super::ParsedToolCall;
use regex::Regex;
use std::sync::LazyLock;

/// 检查给定标签名是否为 XML 元标签（非工具调用标签）
///
/// 元标签是用于表示思维过程、分析、推理等元信息的标签，
/// 而非实际的工具调用。这些标签在解析工具调用时应被跳过。
///
/// # 参数
///
/// * `tag` - 待检查的标签名称
///
/// # 返回值
///
/// 如果标签是元标签则返回 `true`，否则返回 `false`
///
/// # 支持的元标签
///
/// - `tool_call` / `toolcall` / `tool-call`：工具调用包装标签
/// - `invoke`：调用包装标签
/// - `thinking` / `thought`：思考过程标签
/// - `analysis` / `reasoning` / `reflection`：分析推理标签
pub(crate) fn is_xml_meta_tag(tag: &str) -> bool {
    let normalized = tag.to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "tool_call"
            | "toolcall"
            | "tool-call"
            | "invoke"
            | "thinking"
            | "thought"
            | "analysis"
            | "reasoning"
            | "reflection"
    )
}

/// XML 开始标签正则表达式
///
/// 用于匹配 XML 开始标签（如 `<tag_name>`）。
/// 标签名必须以字母或下划线开头，可包含字母、数字、下划线和连字符。
/// 此正则表达式不使用反向引用，以提高性能和可维护性。
static XML_OPEN_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<([a-zA-Z_][a-zA-Z0-9_-]*)>").unwrap());

/// 从输入字符串中提取所有 XML 标签对
///
/// 遍历输入字符串，提取所有匹配的 `<tag>...</tag>` 标签对，
/// 返回标签名和内部内容的列表。
///
/// # 参数
///
/// * `input` - 待解析的输入字符串
///
/// # 返回值
///
/// 返回一个元组向量，每个元组包含：
/// - 标签名称（字符串切片）
/// - 标签内部内容（字符串切片，已去除首尾空白）
///
/// # 算法说明
///
/// 该函数使用非正则表达式方式查找闭合标签，避免使用反向引用，
/// 从而提高解析效率和可靠性。
///
/// # 示例
///
/// ```ignore
/// let input = "<foo>content1</foo><bar>content2</bar>";
/// let pairs = extract_xml_pairs(input);
/// // 返回: [("foo", "content1"), ("bar", "content2")]
/// ```
pub(crate) fn extract_xml_pairs(input: &str) -> Vec<(&str, &str)> {
    let mut results = Vec::new();
    let mut search_start = 0;

    // 循环查找所有开始标签
    while let Some(open_cap) = XML_OPEN_TAG_RE.captures(&input[search_start..]) {
        let full_open = open_cap.get(0).unwrap();
        let tag_name = open_cap.get(1).unwrap().as_str();
        let open_end = search_start + full_open.end();

        // 构建对应的闭合标签并查找其位置
        let closing_tag = format!("</{tag_name}>");
        if let Some(close_pos) = input[open_end..].find(&closing_tag) {
            // 提取标签内部内容，去除首尾空白
            let inner = &input[open_end..open_end + close_pos];
            results.push((tag_name, inner.trim()));
            // 更新搜索起点为闭合标签之后
            search_start = open_end + close_pos + closing_tag.len();
        } else {
            // 未找到闭合标签，跳过当前位置继续搜索
            search_start = open_end;
        }
    }
    results
}

/// 解析 XML 格式的工具调用
///
/// 从 XML 内容中解析出工具调用列表。支持两种参数格式：
/// 1. 嵌套 XML 标签参数：`<tool><arg1>value1</arg1></tool>`
/// 2. JSON 参数载荷：`<tool>{"arg1": "value1"}</tool>`
///
/// # 参数
///
/// * `xml_content` - 待解析的 XML 内容字符串
///
/// # 返回值
///
/// - 如果成功解析到工具调用，返回 `Some(Vec<ParsedToolCall>)`
/// - 如果内容不是有效的 XML 格式或未找到工具调用，返回 `None`
///
/// # 解析规则
///
/// 1. 跳过元标签（如 `thinking`、`analysis` 等）
/// 2. 优先尝试解析 JSON 参数
/// 3. 如果不是 JSON，则解析嵌套 XML 标签作为参数
/// 4. 如果没有嵌套标签，将整个内容作为 `content` 参数
///
/// # 示例
///
/// ```ignore
/// // JSON 参数格式
/// let xml = r#"<shell>{"command":"pwd"}</shell>"#;
/// let calls = parse_xml_tool_calls(xml);
///
/// // 嵌套标签格式
/// let xml = r#"<memory_recall><query>search term</query></memory_recall>"#;
/// let calls = parse_xml_tool_calls(xml);
/// ```
pub(crate) fn parse_xml_tool_calls(xml_content: &str) -> Option<Vec<ParsedToolCall>> {
    let mut calls = Vec::new();
    let trimmed = xml_content.trim();

    // 快速检查：必须以 '<' 开头且包含 '>'
    if !trimmed.starts_with('<') || !trimmed.contains('>') {
        return None;
    }

    // 遍历所有 XML 标签对
    for (tool_name_str, inner_content) in extract_xml_pairs(trimmed) {
        let tool_name = tool_name_str.to_string();

        // 跳过元标签
        if is_xml_meta_tag(&tool_name) {
            continue;
        }

        // 跳过空内容
        if inner_content.is_empty() {
            continue;
        }

        let mut args = serde_json::Map::new();

        // 优先尝试解析 JSON 参数
        if let Some(first_json) = super::json::extract_json_values(inner_content).into_iter().next()
        {
            match first_json {
                // JSON 对象：直接作为参数映射
                serde_json::Value::Object(object_args) => {
                    args = object_args;
                }
                // 其他 JSON 值：包装为 "value" 参数
                other => {
                    args.insert("value".to_string(), other);
                }
            }
        } else {
            // 非 JSON：解析嵌套 XML 标签作为参数
            for (key_str, value) in extract_xml_pairs(inner_content) {
                let key = key_str.to_string();

                // 跳过元标签作为参数名
                if is_xml_meta_tag(&key) {
                    continue;
                }

                // 添加非空值参数
                if !value.is_empty() {
                    args.insert(key, serde_json::Value::String(value.to_string()));
                }
            }

            // 如果没有嵌套标签参数，将整个内容作为 "content" 参数
            if args.is_empty() {
                args.insert(
                    "content".to_string(),
                    serde_json::Value::String(inner_content.to_string()),
                );
            }
        }

        // 构建解析后的工具调用
        calls.push(ParsedToolCall {
            name: tool_name,
            arguments: serde_json::Value::Object(args),
            tool_call_id: None,
        });
    }

    if calls.is_empty() { None } else { Some(calls) }
}

/// 工具调用开始标签列表
///
/// 定义所有支持的工具调用开始标签格式。
/// 数组索引与 `TOOL_CALL_CLOSE_TAGS` 一一对应。
const TOOL_CALL_OPEN_TAGS: [&str; 7] = [
    "<tool_call>",         // 特殊分隔符格式
    "<tool_call",          // 兼容旧式 `<tool_call{...}</tool_call>` 格式
    "<toolcall>",          // 标准工具调用标签
    "<tool-call>",         // 连字符格式
    "<invoke>",            // 调用格式
    "<minimax:tool_call>", // MiniMax 带命名空间格式（下划线）
    "<minimax:toolcall>",  // MiniMax 带命名空间格式（无分隔）
];

/// 工具调用结束标签列表
///
/// 定义所有支持的工具调用结束标签格式。
/// 数组索引与 `TOOL_CALL_OPEN_TAGS` 一一对应。
const TOOL_CALL_CLOSE_TAGS: [&str; 7] = [
    "</tool_call>",         // 特殊分隔符格式
    "</tool_call>",         // 兼容旧式 `<tool_call{...}</tool_call>` 格式
    "</toolcall>",          // 标准工具调用标签
    "</tool-call>",         // 连字符格式
    "</invoke>",            // 调用格式
    "</minimax:tool_call>", // MiniMax 带命名空间格式（下划线）
    "</minimax:toolcall>",  // MiniMax 带命名空间格式（无分隔）
];

/// 在字符串中查找第一个出现的标签
///
/// 遍历标签列表，找出在目标字符串中最早出现的标签。
///
/// # 参数
///
/// * `haystack` - 待搜索的目标字符串
/// * `tags` - 标签列表
///
/// # 返回值
///
/// - 如果找到任何标签，返回 `Some((位置, 标签))`，其中位置是该标签在字符串中的起始索引
/// - 如果没有找到任何标签，返回 `None`
///
/// # 示例
///
/// ```ignore
/// let tags = ["<toolcall>", "<invoke>"];
/// let result = find_first_tag("text <toolcall> content <invoke>", &tags);
/// // 返回: Some((5, "<toolcall>"))
/// ```
pub(crate) fn find_first_tag<'a>(haystack: &str, tags: &'a [&'a str]) -> Option<(usize, &'a str)> {
    tags.iter()
        .filter_map(|tag| haystack.find(tag).map(|idx| (idx, *tag)))
        .min_by_key(|(idx, _)| *idx)
}

/// 获取工具调用开始标签对应的结束标签
///
/// 根据开始标签返回其匹配的结束标签。
///
/// # 参数
///
/// * `open_tag` - 工具调用的开始标签
///
/// # 返回值
///
/// - 如果开始标签是支持的格式，返回 `Some(对应的结束标签)`
/// - 如果开始标签不被识别，返回 `None`
///
/// # 支持的标签对
///
/// | 开始标签 | 结束标签 |
/// |---------|---------|
/// | `Λ` | `Λ` |
/// | `<toolcall>` | `</toolcall>` |
/// | `<tool-call>` | `</tool-call>` |
/// | `<invoke>` | `</invoke>` |
/// | `<minimax:tool_call>` | `</minimax:tool_call>` |
/// | `<minimax:toolcall>` | `</minimax:toolcall>` |
pub(crate) fn matching_tool_call_close_tag(open_tag: &str) -> Option<&'static str> {
    match open_tag {
        "<tool_call>" => Some("</tool_call>"),
        "<tool_call" => Some("</tool_call>"),
        "<toolcall>" => Some("</toolcall>"),
        "<tool-call>" => Some("</tool-call>"),
        "<invoke>" => Some("</invoke>"),
        "<minimax:tool_call>" => Some("</minimax:tool_call>"),
        "<minimax:toolcall>" => Some("</minimax:toolcall>"),
        _ => None,
    }
}

/// 获取所有工具调用开始标签的静态引用
///
/// 返回支持的开始标签数组，用于迭代匹配。
///
/// # 返回值
///
/// 返回包含所有支持的开始标签的静态切片引用
pub(super) fn tool_call_open_tags() -> &'static [&'static str] {
    &TOOL_CALL_OPEN_TAGS
}

/// 获取所有工具调用结束标签的静态引用
///
/// 返回支持的结束标签数组，用于迭代匹配。
///
/// # 返回值
///
/// 返回包含所有支持的结束标签的静态切片引用
pub(super) fn tool_call_close_tags() -> &'static [&'static str] {
    &TOOL_CALL_CLOSE_TAGS
}
