//! 工具调用解析模块
//!
//! 本模块负责从大语言模型（LLM）的响应中解析工具调用。
//! 支持多种工具调用格式，包括：
//!
//! - OpenAI 风格的 JSON 响应（包含 `tool_calls` 数组）
//! - XML 风格的标签（如 `<tool_call`、`<toolcall`、`<invoke`）
//! - Markdown 代码块格式（如 ` ```tool_call `、` ```tool <name> `）
//! - Minimax 特有的 XML 格式
//! - GLM 风格的工具调用
//! - Perl/哈希引用风格的工具调用
//! - FunctionCall XML 标签格式
//!
//! # 安全性说明
//!
//! 本模块不会从响应中提取任意 JSON 作为工具调用。
//! 这种限制是为了防止提示注入攻击——恶意内容（如邮件、文件或网页中的 JSON）
//! 可能模仿工具调用。工具调用必须显式地包装在认可的格式中，以确保只有
//! LLM 有意发出的工具调用才会被执行。

use crate::app::agent::providers::ToolCall;
use regex::Regex;
use std::sync::LazyLock;

/// 解析后的工具调用结构体
///
/// 表示从 LLM 响应中提取出的单个工具调用，
/// 包含工具名称、参数以及可选的工具调用标识符。
#[derive(Debug, Clone)]
pub(crate) struct ParsedToolCall {
    /// 工具名称，如 "bash"、"file_read" 等。
    /// 兼容别名在解析阶段可能会收敛为内部 canonical 名称。
    pub(crate) name: String,
    /// 工具参数，以 JSON 值形式存储
    pub(crate) arguments: serde_json::Value,
    /// 工具调用的唯一标识符（可选）
    /// 某些 Provider（如 OpenAI）会为每个工具调用分配唯一 ID
    pub(crate) tool_call_id: Option<String>,
}

/// 从 JSON 值中提取工具文本内容
///
/// 递归地从 JSON 结构中提取 `content` 字段的文本内容。
/// 支持 OpenAI 风格的响应结构，其中内容可能位于：
/// - 顶层的 `content` 字段
/// - `message.content` 路径
/// - `choices[*].message.content` 路径
///
/// # 参数
///
/// * `value` - 要提取内容的 JSON 值引用
///
/// # 返回值
///
/// 如果找到非空的文本内容，返回 `Some(String)`；否则返回 `None`
fn extract_tool_text_from_json_value(value: &serde_json::Value) -> Option<String> {
    // 尝试直接从 content 字段获取文本
    if let Some(content) = value
        .get("content")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        return Some(content.to_string());
    }

    // 尝试从 message 字段递归提取
    if let Some(message) = value.get("message") {
        if let Some(content) = extract_tool_text_from_json_value(message) {
            return Some(content);
        }
    }

    // 尝试从 choices 数组中递归提取
    if let Some(choices) = value.get("choices").and_then(|v| v.as_array()) {
        for choice in choices {
            if let Some(content) = extract_tool_text_from_json_value(choice) {
                return Some(content);
            }
        }
    }

    None
}

const DEEPSEEK_DSML_OPEN_TAG: &str = "<｜DSML｜tool_calls>";
const DEEPSEEK_DSML_CLOSE_TAG: &str = "</｜DSML｜tool_calls>";

fn normalize_deepseek_dsml_body(body: &str) -> Option<String> {
    static PARAMETER_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?is)<｜DSML｜parameter\b([^>]*)>(.*?)</｜DSML｜parameter>").unwrap()
    });
    static NAME_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"\bname\s*=\s*(?:"([^"]+)"|'([^']+)')"#).unwrap()
    });
    static STRING_TRUE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"\bstring\s*=\s*(?:"true"|'true')"#).unwrap());

    let mut normalized = String::with_capacity(body.len());
    let mut last_end = 0;

    for capture in PARAMETER_RE.captures_iter(body) {
        let full_match = capture.get(0)?;
        let attributes = capture.get(1)?.as_str();
        let value = capture.get(2)?.as_str();
        let name_capture = NAME_RE.captures(attributes)?;
        let name = name_capture.get(1).or_else(|| name_capture.get(2))?.as_str().trim();
        if name.is_empty() {
            return None;
        }

        normalized.push_str(&body[last_end..full_match.start()]);
        normalized.push_str("<parameter name=");
        normalized.push_str(&serde_json::to_string(name).ok()?);
        normalized.push('>');
        if STRING_TRUE_RE.is_match(attributes) {
            normalized.push_str(&serde_json::to_string(value).ok()?);
        } else {
            normalized.push_str(value);
        }
        normalized.push_str("</parameter>");
        last_end = full_match.end();
    }

    normalized.push_str(&body[last_end..]);
    let normalized = normalized
        .replace("<｜DSML｜invoke", "<invoke")
        .replace("</｜DSML｜invoke>", "</invoke>");

    if normalized.contains("｜DSML｜") {
        None
    } else {
        Some(normalized)
    }
}

fn parse_deepseek_dsml_calls(response: &str) -> Option<(String, Vec<ParsedToolCall>)> {
    let mut text_parts = Vec::new();
    let mut calls = Vec::new();
    let mut remaining = response;

    while let Some(open_index) = remaining.find(DEEPSEEK_DSML_OPEN_TAG) {
        let before = remaining[..open_index].trim();
        if !before.is_empty() {
            text_parts.push(before.to_string());
        }

        let after_open = &remaining[open_index + DEEPSEEK_DSML_OPEN_TAG.len()..];
        let close_index = after_open.find(DEEPSEEK_DSML_CLOSE_TAG)?;
        let normalized_body = normalize_deepseek_dsml_body(&after_open[..close_index])?;
        let (unexpected_text, mut block_calls) =
            super::minimax_xml::parse_minimax_invoke_calls(&normalized_body)?;
        if !unexpected_text.trim().is_empty() || block_calls.is_empty() {
            return None;
        }
        calls.append(&mut block_calls);
        remaining = &after_open[close_index + DEEPSEEK_DSML_CLOSE_TAG.len()..];
    }

    if calls.is_empty() || remaining.contains("｜DSML｜") {
        return None;
    }
    let after = remaining.trim();
    if !after.is_empty() {
        text_parts.push(after.to_string());
    }

    Some((text_parts.join("\n"), calls))
}

/// 从 LLM 响应中解析工具调用
///
/// 这是对外的主要解析函数，尝试按照多种格式从响应文本中提取工具调用。
/// 解析顺序经过精心设计，优先处理更结构化的格式。
///
/// # 解析策略（按优先级）
///
/// 1. **OpenAI JSON 格式**：尝试将整个响应解析为 JSON，提取 `tool_calls` 数组
/// 2. **Minimax XML 格式**：处理 Minimax Provider 特有的 `<minimax:toolcall>` 格式
/// 3. **DeepSeek DSML 格式**：处理 `<｜DSML｜tool_calls>` 包装的调用
/// 4. **XML 标签格式**：解析 `<tool_call`、`<toolcall`、`<tool-call`、`<invoke` 等标签
/// 5. **Markdown 代码块**：处理 ` ```tool_call ` 格式的代码块
/// 6. **带名称的 Markdown 工具块**：处理 ` ```tool <name> ` 格式
/// 7. **XML 属性风格**：处理 `<invoke name="...">` 等带属性的 XML
/// 8. **Perl 风格**：处理 `TOOL_CALL ... /TOOL_CALL` 格式
/// 9. **FunctionCall 格式**：处理 `<FunctionCall>` XML 标签
/// 10. **GLM 风格**：处理 GLM 模型特有的行式调用格式
///
/// # 参数
///
/// * `response` - LLM 的原始响应字符串
///
/// # 返回值
///
/// 返回一个元组：
/// - 第一个元素是去除工具调用后的纯文本内容
/// - 第二个元素是解析出的工具调用向量
///
/// # 示例
///
/// ```ignore
/// let response = r#"这是一个回复
/// <tool_call name="bash">
/// {"command": "ls"}
/// </tool_call"#;
/// let (text, calls) = parse_tool_calls(response);
/// assert_eq!(calls.len(), 1);
/// assert_eq!(calls[0].name, "shell");
/// ```
pub(crate) fn parse_tool_calls(response: &str) -> (String, Vec<ParsedToolCall>) {
    let mut text_parts = Vec::new();
    let mut calls = Vec::new();
    let mut remaining = response;

    // 策略 1：首先尝试解析为 OpenAI 风格的 JSON 响应
    // 这处理像 Minimax 这样以原生 JSON 格式返回 tool_calls 的 Provider
    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(response.trim()) {
        calls = super::json::parse_tool_calls_from_json_value(&json_value);
        if !calls.is_empty() {
            // 如果找到 tool_calls，提取 content 字段作为文本
            // 某些 Provider 将工具调用包装在 `message` 或 `choices[*].message` 下
            if let Some(content) = extract_tool_text_from_json_value(&json_value) {
                text_parts.push(content);
            }
            return (text_parts.join("\n"), canonicalize_calls(calls));
        }
    }

    // 策略 2：尝试 Minimax 特有的 XML 格式
    if let Some((minimax_text, minimax_calls)) =
        super::minimax_xml::parse_minimax_invoke_calls(response)
    {
        if !minimax_calls.is_empty() {
            return (minimax_text, canonicalize_calls(minimax_calls));
        }
    }

    // 策略 3：尝试 DeepSeek DSML 文本工具调用格式
    if let Some((dsml_text, dsml_calls)) = parse_deepseek_dsml_calls(response) {
        return (dsml_text, canonicalize_calls(dsml_calls));
    }

    // 策略 4：回退到 XML 风格的工具调用标签解析
    while let Some((start, open_tag)) =
        super::xml_helpers::find_first_tag(remaining, super::xml_helpers::tool_call_open_tags())
    {
        // 标签之前的所有内容都是普通文本
        let before = &remaining[..start];
        if !before.trim().is_empty() {
            text_parts.push(before.trim().to_string());
        }

        let Some(close_tag) = super::xml_helpers::matching_tool_call_close_tag(open_tag) else {
            break;
        };

        let after_open = &remaining[start + open_tag.len()..];
        if let Some(close_idx) = after_open.find(close_tag) {
            let inner = &after_open[..close_idx];
            let mut parsed_any = false;

            // 优先尝试 JSON 格式
            let json_values = super::json::extract_json_values(inner);
            for value in json_values {
                let parsed_calls = super::json::parse_tool_calls_from_json_value(&value);
                if !parsed_calls.is_empty() {
                    parsed_any = true;
                    calls.extend(parsed_calls);
                }
            }

            // 如果 JSON 解析失败，尝试 XML 格式（DeepSeek/GLM 风格）
            if !parsed_any {
                if let Some(xml_calls) = super::xml_helpers::parse_xml_tool_calls(inner) {
                    calls.extend(xml_calls);
                    parsed_any = true;
                }
            }

            // 尝试 GLM 简化格式：`shell>uname -a` 或 `shell\ncommand: date`
            if !parsed_any {
                if let Some(glm_call) = super::tool_call_formats::parse_glm_shortened_body(inner) {
                    calls.push(glm_call);
                    parsed_any = true;
                }
            }

            // 如果所有格式都无法解析，记录警告
            if !parsed_any {
                tracing::warn!(
                    "格式错误的工具调用标签：标签体中应为工具调用对象（JSON/XML/GLM 格式）"
                );
            }

            remaining = &after_open[close_idx + close_tag.len()..];
        } else {
            // 未找到匹配的闭合标签 —— 首先尝试跨别名闭合标签
            // 模型有时会混用开/闭标签别名（如 <tool_call...</invoke>）
            let mut resolved = false;
            if let Some((cross_idx, cross_tag)) = super::xml_helpers::find_first_tag(
                after_open,
                super::xml_helpers::tool_call_close_tags(),
            ) {
                let inner = &after_open[..cross_idx];
                let mut parsed_any = false;

                // 尝试 JSON 格式
                let json_values = super::json::extract_json_values(inner);
                for value in json_values {
                    let parsed_calls = super::json::parse_tool_calls_from_json_value(&value);
                    if !parsed_calls.is_empty() {
                        parsed_any = true;
                        calls.extend(parsed_calls);
                    }
                }

                // 尝试 XML 格式
                if !parsed_any {
                    if let Some(xml_calls) = super::xml_helpers::parse_xml_tool_calls(inner) {
                        calls.extend(xml_calls);
                        parsed_any = true;
                    }
                }

                // 尝试 GLM 简化格式
                if !parsed_any {
                    if let Some(glm_call) =
                        super::tool_call_formats::parse_glm_shortened_body(inner)
                    {
                        calls.push(glm_call);
                        parsed_any = true;
                    }
                }

                if parsed_any {
                    remaining = &after_open[cross_idx + cross_tag.len()..];
                    resolved = true;
                }
            }

            if resolved {
                continue;
            }

            // 未通过跨别名闭合标签解决 —— 回退到从未闭合标签中恢复 JSON
            // （通过花括号平衡）
            if let Some(json_end) = super::json::find_json_end(after_open) {
                if let Ok(value) =
                    serde_json::from_str::<serde_json::Value>(&after_open[..json_end])
                {
                    let parsed_calls = super::json::parse_tool_calls_from_json_value(&value);
                    if !parsed_calls.is_empty() {
                        calls.extend(parsed_calls);
                        remaining = super::json::strip_leading_close_tags(&after_open[json_end..]);
                        continue;
                    }
                }
            }

            // 尝试提取第一个 JSON 值（带结束位置）
            if let Some((value, consumed_end)) =
                super::json::extract_first_json_value_with_end(after_open)
            {
                let parsed_calls = super::json::parse_tool_calls_from_json_value(&value);
                if !parsed_calls.is_empty() {
                    calls.extend(parsed_calls);
                    remaining = super::json::strip_leading_close_tags(&after_open[consumed_end..]);
                    continue;
                }
            }

            // 最后的手段：在开放标签后的所有内容上尝试 GLM 简化格式
            // 模型可能输出了 `<tool_callshell>ls` 而没有任何闭合标签
            let glm_input = after_open.trim();
            let glm_input = glm_input.strip_suffix("CTIONS").unwrap_or(glm_input).trim();
            if let Some(glm_call) = super::tool_call_formats::parse_glm_shortened_body(glm_input) {
                calls.push(glm_call);
                remaining = "";
                continue;
            }

            remaining = &remaining[start..];
            break;
        }
    }

    // 策略 5：如果 XML 标签未找到任何结果，尝试带 tool_call 语言的 Markdown 代码块
    // OpenRouter 后的模型有时输出 ```tool_call ... ``` 或混合格式
    // ```tool_call ... </tool_call``` 而不是结构化 API 调用或 XML 标签
    if calls.is_empty() {
        // 用于匹配 Markdown 工具调用代码块的正则表达式
        // 支持：tool_call、tool-call、invoke 等语言标识
        static MD_TOOL_CALL_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"(?s)```(?:tool[_-]?call|invoke)\s*\n(.*?)(?:```|</tool[_-]?call>|</toolcall>|</invoke>|</minimax:toolcall>)",
            )
            .unwrap()
        });
        let mut md_text_parts: Vec<String> = Vec::new();
        let mut last_end = 0;

        for cap in MD_TOOL_CALL_RE.captures_iter(response) {
            let full_match = cap.get(0).unwrap();
            // 提取匹配之前的内容作为文本
            let before = &response[last_end..full_match.start()];
            if !before.trim().is_empty() {
                md_text_parts.push(before.trim().to_string());
            }
            let inner = &cap[1];
            // 从代码块内容中提取 JSON 值并解析工具调用
            let json_values = super::json::extract_json_values(inner);
            for value in json_values {
                let parsed_calls = super::json::parse_tool_calls_from_json_value(&value);
                calls.extend(parsed_calls);
            }
            last_end = full_match.end();
        }

        if !calls.is_empty() {
            // 提取最后一个匹配之后的内容
            let after = &response[last_end..];
            if !after.trim().is_empty() {
                md_text_parts.push(after.trim().to_string());
            }
            text_parts = md_text_parts;
            remaining = "";
        }
    }

    // 策略 6：尝试 ```tool <name> 格式（某些 Provider 如 xAI grok 使用）
    // 示例：```tool file_write\n{"path": "...", "content": "..."}\n```
    if calls.is_empty() {
        // 用于匹配带工具名称的 Markdown 代码块的正则表达式
        static MD_TOOL_NAME_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?s)```tool\s+(\w+)\s*\n(.*?)(?:```|$)").unwrap());
        let mut md_text_parts: Vec<String> = Vec::new();
        let mut last_end = 0;

        for cap in MD_TOOL_NAME_RE.captures_iter(response) {
            let full_match = cap.get(0).unwrap();
            // 提取匹配之前的内容作为文本
            let before = &response[last_end..full_match.start()];
            if !before.trim().is_empty() {
                md_text_parts.push(before.trim().to_string());
            }
            let tool_name = super::tool_call_formats::canonicalize_tool_name(&cap[1]);
            let inner = &cap[2];

            // 尝试将内部内容解析为 JSON 参数
            let json_values = super::json::extract_json_values(inner);
            if json_values.is_empty() {
                // 如果找到工具块但无法解析参数，记录警告
                tracing::warn!(
                    tool_name = %tool_name,
                    inner = %inner.chars().take(100).collect::<String>(),
                    "找到 ```tool <name> 代码块但无法解析 JSON 参数"
                );
            } else {
                for value in json_values {
                    let arguments = if value.is_object() {
                        value
                    } else {
                        // 如果值不是对象，使用空对象作为参数
                        serde_json::Value::Object(serde_json::Map::new())
                    };
                    calls.push(ParsedToolCall {
                        name: tool_name.to_string(),
                        arguments,
                        tool_call_id: None,
                    });
                }
            }
            last_end = full_match.end();
        }

        if !calls.is_empty() {
            // 提取最后一个匹配之后的内容
            let after = &response[last_end..];
            if !after.trim().is_empty() {
                md_text_parts.push(after.trim().to_string());
            }
            text_parts = md_text_parts;
            remaining = "";
        }
    }

    // 策略 7：XML 属性风格工具调用
    // 示例格式：
    // <minimax:toolcall>
    // <invoke name="shell">
    // <parameter name="command">ls</parameter>
    // </invoke>
    // </minimax:toolcall>
    if calls.is_empty() {
        let xml_calls = super::tool_call_formats::parse_xml_attribute_tool_calls(remaining);
        if !xml_calls.is_empty() {
            let mut cleaned_text = remaining.to_string();
            for call in xml_calls {
                calls.push(call);
                // 尝试从文本中移除 XML 部分
                if let Some(start) = cleaned_text.find("<minimax:toolcall>") {
                    if let Some(end) = cleaned_text.find("</minimax:toolcall>") {
                        let end_pos = end + "</minimax:toolcall>".len();
                        if end_pos <= cleaned_text.len() {
                            cleaned_text =
                                format!("{}{}", &cleaned_text[..start], &cleaned_text[end_pos..]);
                        }
                    }
                }
            }
            if !cleaned_text.trim().is_empty() {
                text_parts.push(cleaned_text.trim().to_string());
            }
            remaining = "";
        }
    }

    // 策略 8：Perl/哈希引用风格工具调用
    // 示例格式：
    // TOOL_CALL
    // {tool => "shell", args => {
    //   --command "ls -la"
    //   --description "列出当前目录内容"
    // }}
    // /TOOL_CALL
    if calls.is_empty() {
        let perl_calls = super::tool_call_formats::parse_perl_style_tool_calls(remaining);
        if !perl_calls.is_empty() {
            let mut cleaned_text = remaining.to_string();
            for call in perl_calls {
                calls.push(call);
                // 尝试从文本中移除 TOOL_CALL 块
                while let Some(start) = cleaned_text.find("TOOL_CALL") {
                    if let Some(end) = cleaned_text.find("/TOOL_CALL") {
                        let end_pos = end + "/TOOL_CALL".len();
                        if end_pos <= cleaned_text.len() {
                            cleaned_text =
                                format!("{}{}", &cleaned_text[..start], &cleaned_text[end_pos..]);
                        }
                    } else {
                        break;
                    }
                }
            }
            if !cleaned_text.trim().is_empty() {
                text_parts.push(cleaned_text.trim().to_string());
            }
            remaining = "";
        }
    }

    // 策略 9：FunctionCall XML 格式
    // 示例格式：
    // <FunctionCall>
    // file_read
    // <code>path>/Users/...</code>
    // </FunctionCall>
    if calls.is_empty() {
        let func_calls = super::tool_call_formats::parse_function_call_tool_calls(remaining);
        if !func_calls.is_empty() {
            let mut cleaned_text = remaining.to_string();
            for call in func_calls {
                calls.push(call);
                // 尝试从文本中移除 FunctionCall 块
                while let Some(start) = cleaned_text.find("<FunctionCall>") {
                    if let Some(end) = cleaned_text.find("</FunctionCall>") {
                        let end_pos = end + "</FunctionCall>".len();
                        if end_pos <= cleaned_text.len() {
                            cleaned_text =
                                format!("{}{}", &cleaned_text[..start], &cleaned_text[end_pos..]);
                        }
                    } else {
                        break;
                    }
                }
            }
            if !cleaned_text.trim().is_empty() {
                text_parts.push(cleaned_text.trim().to_string());
            }
            remaining = "";
        }
    }

    // 策略 10：GLM 风格工具调用（browser_open/url>https://..., shell/command>ls 等）
    if calls.is_empty() {
        let glm_calls = super::tool_call_formats::parse_glm_style_tool_calls(remaining);
        if !glm_calls.is_empty() {
            let mut cleaned_text = remaining.to_string();
            for (name, args, raw) in &glm_calls {
                calls.push(ParsedToolCall {
                    name: name.clone(),
                    arguments: args.clone(),
                    tool_call_id: None,
                });
                // 如果有原始文本，从清理后的文本中移除
                if let Some(r) = raw {
                    cleaned_text = cleaned_text.replace(r, "");
                }
            }
            if !cleaned_text.trim().is_empty() {
                text_parts.push(cleaned_text.trim().to_string());
            }
            remaining = "";
        }
    }

    // 安全性说明：我们不会在这里回退到从响应中提取任意 JSON
    // 这将允许提示注入攻击，其中恶意内容（例如在邮件、文件或网页中）
    // 可能包含模仿工具调用的 JSON。工具调用必须显式包装在以下格式之一：
    // 1. 带有 "tool_calls" 数组的 OpenAI 风格 JSON
    // 2. VibeWindow 工具调用标签（<tool_call、<toolcall、<tool-call）
    // 3. 带 tool_call/toolcall/tool-call 语言的 Markdown 代码块
    // 4. 显式的 GLM 行式调用格式（如 `shell/command>...`）
    // 这确保只有 LLM 有意的工具调用才会被执行。

    // 最后一个工具调用之后的剩余文本
    if !remaining.trim().is_empty() {
        text_parts.push(remaining.trim().to_string());
    }

    (text_parts.join("\n"), canonicalize_calls(calls))
}

fn canonicalize_calls(mut calls: Vec<ParsedToolCall>) -> Vec<ParsedToolCall> {
    for call in &mut calls {
        call.name = super::tool_call_formats::canonicalize_tool_name(&call.name).to_string();
    }
    calls
}

/// 检测工具调用解析问题
///
/// 当响应看起来像工具调用负载但未能解析出任何工具调用时，
/// 返回一个描述问题的字符串。这有助于诊断和提示用户/模型修正输出格式。
///
/// # 参数
///
/// * `response` - LLM 的原始响应字符串
/// * `parsed_calls` - 已解析的工具调用数组
///
/// # 返回值
///
/// - 如果已解析出工具调用，返回 `None`（无问题）
/// - 如果响应为空，返回 `None`（无问题）
/// - 如果响应看起来像工具调用但未能解析，返回 `Some(String)` 描述问题
///
/// # 检测的模式
///
/// 函数会检查响应是否包含以下模式：
/// - XML 标签：`<tool_call`、`<toolcall`、`<tool-call`
/// - DeepSeek DSML：`<｜DSML｜tool_calls>`
/// - Markdown 代码块：` ```tool_call `、` ```toolcall `、` ```tool-call `
/// - 带名称的工具块：` ```tool file_`、` ```tool shell ` 等
/// - JSON 格式：`"tool_calls"`
/// - 其他格式：`TOOL_CALL`、`<FunctionCall>`
///
/// # 示例
///
/// ```ignore
/// let response = "<tool_call name=\"shell\">\ninvalid json\n</tool_call";
/// let (text, calls) = parse_tool_calls(response);
/// if let Some(issue) = detect_tool_call_parse_issue(response, &calls) {
///     println!("解析问题: {}", issue);
/// }
/// ```
pub(crate) fn detect_tool_call_parse_issue(
    response: &str,
    parsed_calls: &[ParsedToolCall],
) -> Option<String> {
    // 如果已解析出工具调用，无问题
    if !parsed_calls.is_empty() {
        return None;
    }

    let trimmed = response.trim();
    // 空响应不算问题
    if trimmed.is_empty() {
        return None;
    }

    // 检测响应是否看起来像工具调用负载
    let looks_like_tool_payload = trimmed.contains("<tool_call")
        || trimmed.contains("<toolcall")
        || trimmed.contains("<tool-call")
        || trimmed.contains(DEEPSEEK_DSML_OPEN_TAG)
        || trimmed.contains("```tool_call")
        || trimmed.contains("```toolcall")
        || trimmed.contains("```tool-call")
        || trimmed.contains("```tool file_")
        || trimmed.contains("```tool shell")
        || trimmed.contains("```tool web_")
        || trimmed.contains("```tool memory_")
        || trimmed.contains("```tool ") // 通用 ```tool <name> 模式
        || trimmed.contains("\"tool_calls\"")
        || (trimmed.contains("\"name\"") && trimmed.contains("\"arguments\""))
        || trimmed.contains(":UIButtonType")
        || trimmed.contains('✁')
        || trimmed.contains("TOOL_CALL")
        || trimmed.contains("<FunctionCall>");

    if looks_like_tool_payload {
        Some("响应类似工具调用负载但无法解析出有效的工具调用".into())
    } else {
        None
    }
}

/// 解析结构化的工具调用
///
/// 将 Provider 返回的 `ToolCall` 结构（通常来自 OpenAI 风格的 API）
/// 转换为内部的 `ParsedToolCall` 结构。这个函数处理参数的规范化，
/// 确保不同来源的工具调用具有一致的格式。
///
/// # 参数
///
/// * `tool_calls` - Provider 返回的工具调用切片
///
/// # 返回值
///
/// 返回转换后的 `ParsedToolCall` 向量
///
/// # 处理逻辑
///
/// 1. 复制工具名称
/// 2. 解析参数字符串为 JSON 值（失败时使用空对象）
/// 3. 调用规范化函数处理特定工具的参数格式
/// 4. 保留原始的工具调用 ID
///
/// # 示例
///
/// ```ignore
/// let tool_calls = vec![ToolCall {
///     id: "call_123".to_string(),
///     name: "shell".to_string(),
///     arguments: r#"{"command": "ls"}"#.to_string(),
/// }];
/// let parsed = parse_structured_tool_calls(&tool_calls);
/// assert_eq!(parsed.len(), 1);
/// assert_eq!(parsed[0].name, "shell");
/// assert_eq!(parsed[0].tool_call_id, Some("call_123".to_string()));
/// ```
pub(crate) fn parse_structured_tool_calls(tool_calls: &[ToolCall]) -> Vec<ParsedToolCall> {
    tool_calls
        .iter()
        .map(|call| {
            let name = call.name.clone();
            // 尝试解析参数字符串为 JSON，失败时使用空对象
            let parsed_result = serde_json::from_str::<serde_json::Value>(&call.arguments);
            let raw_string_hint =
                if parsed_result.is_err() && call.arguments.chars().any(char::is_whitespace) {
                    Some(call.arguments.as_str())
                } else {
                    None
                };
            let parsed =
                parsed_result.unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));
            ParsedToolCall {
                name: name.clone(),
                // 规范化工具参数（处理特定工具的参数格式差异）
                arguments: super::tool_call_formats::normalize_tool_arguments(
                    &name,
                    parsed,
                    raw_string_hint,
                ),
                tool_call_id: Some(call.id.clone()),
            }
        })
        .collect()
}
