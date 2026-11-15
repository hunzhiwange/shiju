//! JSON 解析工具模块
//!
//! 本模块提供了一系列用于解析 LLM（大语言模型）返回的工具调用数据的辅助函数。
//! 主要功能包括：
//!
//! - 解析工具调用的参数值、ID 和名称
//! - 规范化 JSON 数据结构用于生成工具调用签名
//! - 从各种格式的 JSON 值中提取工具调用列表
//! - 从文本流中提取和定位 JSON 对象
//!
//! 这些函数支持处理多种不同格式的 LLM 响应，确保工具调用能够被正确识别和解析。

use crate::app::agent::agent::loop_::parsing::ParsedToolCall;

#[cfg(test)]
#[path = "json_tests.rs"]
mod json_tests;

/// 解析工具调用的参数值
///
/// 将可能以字符串形式编码的 JSON 参数转换为 JSON 值对象。
/// 这个函数处理 LLM 可能返回的三种参数格式：
///
/// 1. 字符串形式的 JSON（需要反序列化）
/// 2. 已经是 JSON 对象的值（直接返回）
/// 3. 空值（返回空对象）
///
/// # 参数
///
/// - `raw`: 可选的 JSON 值引用，可能是字符串或对象
///
/// # 返回值
///
/// 返回解析后的 `serde_json::Value`。如果解析失败或输入为空，返回空对象 `{}`
///
/// # 示例
///
/// ```ignore
/// let string_arg = Some(&serde_json::json!("{\"key\": \"value\"}"));
/// let parsed = parse_arguments_value(string_arg);
/// assert_eq!(parsed, serde_json::json!({"key": "value"}));
///
/// let obj_arg = Some(&serde_json::json!({"key": "value"}));
/// let parsed = parse_arguments_value(obj_arg);
/// assert_eq!(parsed, serde_json::json!({"key": "value"}));
///
/// let none_arg: Option<&serde_json::Value> = None;
/// let parsed = parse_arguments_value(none_arg);
/// assert_eq!(parsed, serde_json::json!({}));
/// ```
pub(crate) fn parse_arguments_value(raw: Option<&serde_json::Value>) -> serde_json::Value {
    match raw {
        // 如果参数是字符串形式，尝试解析为 JSON
        Some(serde_json::Value::String(s)) => serde_json::from_str::<serde_json::Value>(s)
            .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new())),
        // 如果已经是 JSON 值，直接克隆
        Some(value) => value.clone(),
        // 如果没有参数，返回空对象
        None => serde_json::Value::Object(serde_json::Map::new()),
    }
}

/// 解析工具调用的 ID
///
/// 从多个可能的位置提取工具调用的唯一标识符。
/// LLM 可能使用不同的字段名来存储工具调用 ID，此函数按优先级依次尝试：
///
/// 1. `function.id` - 嵌套在 function 对象中的 ID
/// 2. 根对象的 `id` 字段
/// 3. 根对象的 `tool_call_id` 字段
/// 4. 根对象的 `call_id` 字段
///
/// # 参数
///
/// - `root`: 根 JSON 值
/// - `function`: 可选的 function 对象引用
///
/// # 返回值
///
/// 返回 `Option<String>`，包含找到的第一个非空 ID（去除首尾空白）。
/// 如果所有位置都找不到有效 ID，返回 `None`
///
/// # 示例
///
/// ```ignore
/// let json = serde_json::json!({"function": {"id": "call_123"}, "id": "root_456"});
/// let id = parse_tool_call_id(&json, json.get("function"));
/// assert_eq!(id, Some("call_123".to_string()));
/// ```
pub(crate) fn parse_tool_call_id(
    root: &serde_json::Value,
    function: Option<&serde_json::Value>,
) -> Option<String> {
    function
        .and_then(|func| func.get("id"))
        .or_else(|| root.get("id"))
        .or_else(|| root.get("tool_call_id"))
        .or_else(|| root.get("call_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(ToString::to_string)
}

/// 规范化 JSON 结构用于工具签名计算
///
/// 递归地对 JSON 对象的键进行排序，生成确定性的 JSON 结构。
/// 这对于生成稳定的工具调用签名非常重要，确保相同内容不同顺序的
/// JSON 对象能够产生相同的签名。
///
/// # 参数
///
/// - `value`: 要规范化的 JSON 值
///
/// # 返回值
///
/// 返回键已排序的规范化 JSON 值。对于对象，键按字母顺序排序；
/// 对于数组，递归处理每个元素；对于原始值，直接克隆返回
///
/// # 示例
///
/// ```ignore
/// let json = serde_json::json!({"z": 1, "a": 2});
/// let canonical = canonicalize_json_for_tool_signature(&json);
/// assert_eq!(canonical, serde_json::json!({"a": 2, "z": 1}));
/// ```
pub(crate) fn canonicalize_json_for_tool_signature(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            // 收集所有键并排序
            let mut keys: Vec<String> = map.keys().cloned().collect();
            keys.sort_unstable();

            // 按排序后的顺序重建对象
            let mut ordered = serde_json::Map::new();
            for key in keys {
                if let Some(child) = map.get(&key) {
                    // 递归规范化子元素
                    ordered.insert(key, canonicalize_json_for_tool_signature(child));
                }
            }
            serde_json::Value::Object(ordered)
        }
        serde_json::Value::Array(items) => serde_json::Value::Array(
            // 递归处理数组中的每个元素
            items.iter().map(canonicalize_json_for_tool_signature).collect(),
        ),
        // 原始值直接克隆
        _ => value.clone(),
    }
}

/// 生成工具调用的签名字符串
///
/// 根据工具名称和参数生成一个规范化的签名元组，用于唯一标识工具调用。
/// 签名包括：
/// - 小写化的工具名称（去除首尾空白）
/// - 规范化并序列化为字符串的参数 JSON
///
/// # 参数
///
/// - `name`: 工具名称
/// - `arguments`: 工具参数的 JSON 值
///
/// # 返回值
///
/// 返回元组 `(工具名称, 参数JSON字符串)`，其中名称已转为小写并去除空白
///
/// # 示例
///
/// ```ignore
/// let name = "GetWeather";
/// let args = serde_json::json!({"city": "Beijing"});
/// let (name, args_str) = tool_call_signature(name, &args);
/// assert_eq!(name, "getweather");
/// assert_eq!(args_str, r#"{"city":"Beijing"}"#);
/// ```
pub(crate) fn tool_call_signature(name: &str, arguments: &serde_json::Value) -> (String, String) {
    let canonical_args = canonicalize_json_for_tool_signature(arguments);
    let args_json = serde_json::to_string(&canonical_args).unwrap_or_else(|_| "{}".to_string());
    (name.trim().to_ascii_lowercase(), args_json)
}

/// 从单个 JSON 值解析工具调用
///
/// 尝试从给定的 JSON 值中提取工具调用信息。支持两种格式：
///
/// 1. 嵌套格式：包含 `function` 对象，其中有 `name` 和 `arguments`
/// 2. 扁平格式：直接在根对象中包含 `name` 和 `arguments`
///
/// # 参数
///
/// - `value`: 包含工具调用信息的 JSON 值
///
/// # 返回值
///
/// 如果成功解析出有效的工具调用（名称非空），返回 `Some(ParsedToolCall)`；
/// 否则返回 `None`
///
/// # 示例
///
/// ```ignore
/// let nested = serde_json::json!({
///     "function": {
///         "name": "search",
///         "arguments": "{\"query\": \"rust\"}"
///     },
///     "id": "call_123"
/// });
/// let parsed = parse_tool_call_value(&nested);
/// assert!(parsed.is_some());
/// ```
pub(crate) fn parse_tool_call_value(value: &serde_json::Value) -> Option<ParsedToolCall> {
    // 首先尝试解析嵌套的 function 格式
    if let Some(function) = value.get("function") {
        let tool_call_id = parse_tool_call_id(value, Some(function));
        let name = function.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();

        // 如果名称有效，解析参数并返回
        if !name.is_empty() {
            let raw_arguments = function.get("arguments").or_else(|| function.get("parameters"));
            let arguments = super::tool_call_formats::normalize_tool_arguments(
                &name,
                parse_arguments_value(raw_arguments),
                super::tool_call_formats::raw_string_argument_hint(raw_arguments),
            );
            return Some(ParsedToolCall { name, arguments, tool_call_id });
        }
    }

    // 回退到扁平格式：直接从根对象提取
    let tool_call_id = parse_tool_call_id(value, None);
    let name = value.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();

    // 如果名称为空，无法构成有效的工具调用
    if name.is_empty() {
        return None;
    }

    // 解析参数并返回工具调用
    let raw_arguments = value.get("arguments").or_else(|| value.get("parameters"));
    let arguments = super::tool_call_formats::normalize_tool_arguments(
        &name,
        parse_arguments_value(raw_arguments),
        super::tool_call_formats::raw_string_argument_hint(raw_arguments),
    );
    Some(ParsedToolCall { name, arguments, tool_call_id })
}

/// 从 JSON 值中解析所有工具调用
///
/// 递归地从各种可能的 JSON 结构中提取工具调用列表。支持的格式包括：
///
/// 1. 包含 `tool_calls` 数组的对象
/// 2. 包含 `message` 字段的嵌套结构
/// 3. OpenAI 风格的 `choices` 数组
/// 4. 直接的工具调用数组
/// 5. 单个工具调用对象
///
/// # 参数
///
/// - `value`: 可能包含工具调用的 JSON 值
///
/// # 返回值
///
/// 返回解析出的所有有效工具调用的向量。如果没有找到工具调用，返回空向量
///
/// # 示例
///
/// ```ignore
/// let json = serde_json::json!({
///     "tool_calls": [
///         {"function": {"name": "search", "arguments": "{}"}},
///         {"function": {"name": "read", "arguments": "{}"}}
///     ]
/// });
/// let calls = parse_tool_calls_from_json_value(&json);
/// assert_eq!(calls.len(), 2);
/// ```
pub(crate) fn parse_tool_calls_from_json_value(value: &serde_json::Value) -> Vec<ParsedToolCall> {
    let mut calls = Vec::new();

    // 尝试从 tool_calls 数组中提取
    if let Some(tool_calls) = value.get("tool_calls").and_then(|v| v.as_array()) {
        for call in tool_calls {
            if let Some(parsed) = parse_tool_call_value(call) {
                calls.push(parsed);
            }
        }

        // 如果成功提取到工具调用，直接返回
        if !calls.is_empty() {
            return calls;
        }
    }

    // 尝试从嵌套的 message 字段中递归提取
    if let Some(message) = value.get("message") {
        let nested = parse_tool_calls_from_json_value(message);
        if !nested.is_empty() {
            return nested;
        }
    }

    // 尝试从 OpenAI 风格的 choices 数组中提取
    if let Some(choices) = value.get("choices").and_then(|v| v.as_array()) {
        for choice in choices {
            if let Some(message) = choice.get("message") {
                let nested = parse_tool_calls_from_json_value(message);
                if !nested.is_empty() {
                    calls.extend(nested);
                }
            }
        }
        if !calls.is_empty() {
            return calls;
        }
    }

    // 尝试将整个值作为数组处理
    if let Some(array) = value.as_array() {
        for item in array {
            if let Some(parsed) = parse_tool_call_value(item) {
                calls.push(parsed);
            }
        }
        return calls;
    }

    // 最后尝试将整个值作为单个工具调用对象处理
    if let Some(parsed) = parse_tool_call_value(value) {
        calls.push(parsed);
    }

    calls
}

/// 提取文本中第一个 JSON 值及其结束位置
///
/// 从输入字符串的开头（跳过前导空白）查找第一个有效的 JSON 对象或数组，
/// 并返回该 JSON 值及其在原始字符串中的结束字节位置。
///
/// # 参数
///
/// - `input`: 包含 JSON 数据的输入字符串
///
/// # 返回值
///
/// 如果找到有效的 JSON，返回 `Some((JSON值, 结束位置))`；
/// 否则返回 `None`。结束位置是相对于原始输入字符串的字节偏移量
///
/// # 示例
///
/// ```ignore
/// let input = r#"  {"key": "value"} some text"#;
/// let result = extract_first_json_value_with_end(input);
/// assert!(result.is_some());
/// let (value, end_pos) = result.unwrap();
/// assert_eq!(value, serde_json::json!({"key": "value"}));
/// ```
pub(crate) fn extract_first_json_value_with_end(input: &str) -> Option<(serde_json::Value, usize)> {
    let trimmed = input.trim_start();
    // 计算被去除的前导空白字节数
    let trim_offset = input.len().saturating_sub(trimmed.len());

    // 扫描字符串，查找 JSON 对象或数组的起始位置
    for (byte_idx, ch) in trimmed.char_indices() {
        // 只处理对象 `{` 或数组 `[` 的起始字符
        if ch != '{' && ch != '[' {
            continue;
        }

        let slice = &trimmed[byte_idx..];
        // 使用流式反序列化器尝试解析 JSON
        let mut stream = serde_json::Deserializer::from_str(slice).into_iter::<serde_json::Value>();

        if let Some(Ok(value)) = stream.next() {
            let consumed = stream.byte_offset();
            // 只有成功消费了字节才返回结果
            if consumed > 0 {
                // 返回原始字符串中的绝对位置
                return Some((value, trim_offset + byte_idx + consumed));
            }
        }
    }

    None
}

/// 去除字符串前导的 XML 闭合标签
///
/// 重复删除字符串开头的所有 XML 闭合标签（如 `</tag>`），
/// 直到遇到非闭合标签内容或字符串结束。这在处理 LLM 生成的内容时很有用，
/// 因为有时会在 JSON 前产生多余的闭合标签。
///
/// # 参数
///
/// - `input`: 可能包含前导闭合标签的输入字符串
///
/// # 返回值
///
/// 返回去除前导闭合标签后的字符串切片。
/// 如果在去除过程中遇到不完整的标签，返回空字符串
///
/// # 示例
///
/// ```ignore
/// let input = "</tool></invoke>{\"data\": \"value\"}";
/// let result = strip_leading_close_tags(input);
/// assert_eq!(result, "{\"data\": \"value\"}");
/// ```
pub(crate) fn strip_leading_close_tags(mut input: &str) -> &str {
    loop {
        let trimmed = input.trim_start();

        // 如果不是以闭合标签开头，返回当前结果
        if !trimmed.starts_with("</") {
            return trimmed;
        }

        // 查找闭合标签的结束符 `>`
        let Some(close_end) = trimmed.find('>') else {
            // 标签不完整，返回空字符串
            return "";
        };

        // 移除这个闭合标签，继续循环检查
        input = &trimmed[close_end + 1..];
    }
}

pub use json_extract::extract_json_values;

/// 通过跟踪括号平衡查找 JSON 对象的结束位置
///
/// 手动扫描字符串，通过跟踪花括号的嵌套深度来确定 JSON 对象的结束位置。
/// 此函数正确处理字符串内的转义字符和嵌套引号。
///
/// # 参数
///
/// - `input`: 可能以 JSON 对象开头的输入字符串
///
/// # 返回值
///
/// 如果找到完整且平衡的 JSON 对象，返回 `Some(结束位置)`，该位置是
/// 相对于原始输入字符串的字节偏移量；如果输入不是以 `{` 开头或
/// 括号不平衡，返回 `None`
///
/// # 示例
///
/// ```ignore
/// let input = r#"{"key": "value with \"quote\""} more"#;
/// let end = find_json_end(input);
/// assert!(end.is_some());
/// ```
pub(crate) fn find_json_end(input: &str) -> Option<usize> {
    let trimmed = input.trim_start();
    let offset = input.len() - trimmed.len();

    // 必须以对象起始符开头
    if !trimmed.starts_with('{') {
        return None;
    }

    let mut depth = 0; // 花括号嵌套深度
    let mut in_string = false; // 是否在字符串字面量内
    let mut escape_next = false; // 下一个字符是否被转义

    for (i, ch) in trimmed.char_indices() {
        // 处理转义字符
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            // 字符串内的反斜杠表示转义
            '\\' if in_string => escape_next = true,
            // 双引号切换字符串状态
            '"' => in_string = !in_string,
            // 对象起始符增加深度（非字符串内）
            '{' if !in_string => depth += 1,
            // 对象结束符减少深度（非字符串内）
            '}' if !in_string => {
                depth -= 1;
                // 深度归零表示找到完整对象
                if depth == 0 {
                    return Some(offset + i + ch.len_utf8());
                }
            }
            _ => {}
        }
    }

    // 未找到匹配的结束符
    None
}
