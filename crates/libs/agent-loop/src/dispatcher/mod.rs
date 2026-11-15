//! 工具调度器模块
//!
//! 本模块提供了工具调用（Tool Calls）的解析、格式化和调度功能。
//! 支持两种工具调用格式：
//! - **XML 格式**：使用 `<tool_call_result>` XML 标签包装工具调用
//! - **原生格式**：使用 Provider 原生的工具调用 API（如 OpenAI 的 function calling）
//!
//! ## 核心组件
//!
//! - [`ToolDispatcher`]：工具调度器 trait，定义统一的工具调用接口
//! - [`XmlToolDispatcher`]：基于 XML 标签的工具调度器实现
//! - [`NativeToolDispatcher`]：基于 Provider 原生 API 的工具调度器实现
//! - [`ParsedToolCall`]：解析后的工具调用数据结构
//! - [`ToolExecutionResult`]：工具执行结果数据结构
//!
//! ## 典型工作流
//!
//! 1. Provider 返回响应（可能包含工具调用）
//! 2. 调度器解析响应，提取文本内容和工具调用
//! 3. 执行工具并生成结果
//! 4. 调度器格式化工具结果，准备下一轮对话

use model_types::{ChatMessage, ChatResponse, ConversationMessage, ToolResultMessage};
use tool_types::{Tool, ToolSpec};
use serde_json::Value;
use std::fmt::Write;

/// 解析后的工具调用
///
/// 表示从 AI 响应中解析出的单个工具调用请求。
/// 包含工具名称、参数以及可选的工具调用 ID（用于原生 API 模式）。
///
/// # 字段说明
///
/// - `name`：要调用的工具名称
/// - `arguments`：传递给工具的参数（JSON 格式）
/// - `tool_call_id`：工具调用的唯一标识符（原生 API 模式使用，XML 模式为 None）
///
/// # 示例
///
/// ```ignore
/// let call = ParsedToolCall {
///     name: "get_weather".to_string(),
///     arguments: serde_json::json!({"city": "北京"}),
///     tool_call_id: Some("call_123".to_string()),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ParsedToolCall {
    /// 工具名称
    pub name: String,
    /// 工具参数（JSON 格式）
    pub arguments: Value,
    /// 工具调用 ID（原生 API 模式使用）
    pub tool_call_id: Option<String>,
}

/// 工具执行结果
///
/// 表示工具执行完成后的输出结果。
/// 包含工具名称、输出内容、执行状态以及可选的调用 ID。
///
/// # 字段说明
///
/// - `name`：被执行的工具名称
/// - `output`：工具执行的输出内容（成功或错误信息）
/// - `success`：执行是否成功
/// - `tool_call_id`：对应的工具调用 ID（原生 API 模式使用）
///
/// # 示例
///
/// ```ignore
/// let result = ToolExecutionResult {
///     name: "get_weather".to_string(),
///     output: "北京今天晴，气温 25°C".to_string(),
///     success: true,
///     tool_call_id: Some("call_123".to_string()),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    /// 工具名称
    pub name: String,
    /// 工具输出内容
    pub output: String,
    /// 执行是否成功
    pub success: bool,
    /// 工具调用 ID
    pub tool_call_id: Option<String>,
}

/// 工具调度器 trait
///
/// 定义了工具调用的解析、格式化和消息转换接口。
/// 不同的实现支持不同的工具调用格式（XML 或原生 API）。
///
/// # 实现者
///
/// - [`XmlToolDispatcher`]：基于 XML 标签的工具调用格式
/// - [`NativeToolDispatcher`]：基于 Provider 原生 API 的工具调用格式
///
/// # 线程安全
///
/// 所有实现必须满足 `Send + Sync`，以支持多线程环境。
///
/// # 示例
///
/// ```ignore
/// fn process_response(dispatcher: &dyn ToolDispatcher, response: &ChatResponse) {
///     let (text, tool_calls) = dispatcher.parse_response(response);
///     // 执行工具调用...
///     let result_msg = dispatcher.format_results(&results);
/// }
/// ```
pub trait ToolDispatcher: Send + Sync {
    /// 解析 AI 响应，提取文本内容和工具调用
    ///
    /// # 参数
    ///
    /// - `response`：Provider 返回的聊天响应
    ///
    /// # 返回值
    ///
    /// 返回元组 `(文本内容, 工具调用列表)`：
    /// - 文本内容：AI 响应中的纯文本部分
    /// - 工具调用列表：解析出的所有工具调用请求
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let (text, calls) = dispatcher.parse_response(&response);
    /// println!("AI 说了: {}", text);
    /// for call in calls {
    ///     println!("调用工具: {}", call.name);
    /// }
    /// ```
    fn parse_response(&self, response: &ChatResponse) -> (String, Vec<ParsedToolCall>);

    /// 格式化工具执行结果为对话消息
    ///
    /// 将工具执行结果转换为可发送给 AI 的消息格式。
    ///
    /// # 参数
    ///
    /// - `results`：工具执行结果列表
    ///
    /// # 返回值
    ///
    /// 返回格式化后的对话消息，不同实现可能返回不同格式：
    /// - XML 格式：返回包含 XML 标签的用户消息
    /// - 原生格式：返回 `ToolResults` 类型的消息
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let results = vec![ToolExecutionResult { /* ... */ }];
    /// let msg = dispatcher.format_results(&results);
    /// // 将 msg 添加到对话历史
    /// ```
    fn format_results(&self, results: &[ToolExecutionResult]) -> ConversationMessage;

    /// 生成工具使用提示指令
    ///
    /// 生成包含工具使用协议和可用工具列表的提示文本。
    /// 用于指导 AI 如何正确调用工具。
    ///
    /// # 参数
    ///
    /// - `tools`：可用工具列表
    ///
    /// # 返回值
    ///
    /// 返回提示指令字符串。对于原生 API 模式，通常返回空字符串，
    /// 因为工具调用由 Provider 原生支持，不需要额外的提示。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let instructions = dispatcher.prompt_instructions(&tools);
    /// // 将 instructions 添加到系统提示中
    /// ```
    fn prompt_instructions(&self, tools: &[Box<dyn Tool>]) -> String;

    /// 将对话历史转换为 Provider 消息格式
    ///
    /// 将内部的对话消息格式转换为特定 Provider 所需的消息格式。
    ///
    /// # 参数
    ///
    /// - `history`：对话历史记录
    ///
    /// # 返回值
    ///
    /// 返回转换后的 Provider 消息列表。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let provider_msgs = dispatcher.to_provider_messages(&history);
    /// // 将 provider_msgs 发送给 Provider
    /// ```
    fn to_provider_messages(&self, history: &[ConversationMessage]) -> Vec<ChatMessage>;

    /// 判断是否应该发送工具规范
    ///
    /// # 返回值
    ///
    /// - `true`：需要向 Provider 发送工具定义（原生 API 模式）
    /// - `false`：不需要发送工具定义，工具信息已嵌入提示中（XML 模式）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// if dispatcher.should_send_tool_specs() {
    ///     // 在请求中包含工具定义
    /// }
    /// ```
    fn should_send_tool_specs(&self) -> bool;
}

/// XML 格式工具调度器
///
/// 使用 XML 标签格式来包装工具调用请求和结果。
/// 适用于不支持原生 function calling 的 Provider 或需要自定义工具格式的场景。
///
/// # 工具调用格式
///
/// 工具调用使用 `<tool_call_result>` 标签包装 JSON 对象：
///
/// ```xml
/// <tool_call_result>
/// {"name": "get_weather", "arguments": {"city": "北京"}}
/// </tool_call_result>
/// ```
///
/// # 兼容性
///
/// 自动兼容多种标签变体：
/// - `<toolcall>...</toolcall>`
/// - `<tool-call>...</tool-call>`
/// - `<invoke>...</invoke>`
/// - `<tool_call_result>...</tool_call_result>`
///
/// # 示例
///
/// ```ignore
/// let dispatcher = XmlToolDispatcher::default();
/// let (text, calls) = dispatcher.parse_response(&response);
/// ```
#[derive(Default)]
pub struct XmlToolDispatcher;

impl XmlToolDispatcher {
    /// 解析 XML 格式的工具调用
    ///
    /// 从响应文本中提取纯文本内容和嵌入的 XML 工具调用。
    /// 自动规范化多种标签变体以保持解析一致性。
    ///
    /// # 参数
    ///
    /// - `response`：AI 响应文本
    ///
    /// # 返回值
    ///
    /// 返回元组 `(文本内容, 工具调用列表)`
    ///
    /// # 处理流程
    ///
    /// 1. 规范化标签变体（将 `<toolcall>`、`<tool-call>`、`<invoke>` 转换为统一格式）
    /// 2. 扫描文本，提取标签前后的文本内容
    /// 3. 解析标签内的 JSON 对象，提取工具名称和参数
    /// 4. 跳过格式错误的调用并记录警告
    fn parse_xml_tool_calls(response: &str) -> (String, Vec<ParsedToolCall>) {
        let mut text_parts = Vec::new();
        let mut calls = Vec::new();

        // 规范化标签变体，将不同的标签格式统一转换为 <tool_call_result> 格式
        // 系统其他部分接受 <toolcall>、<tool-call> 和 <invoke>，这里统一处理
        let normalized = response
            .replace("<tool_call>", "<tool_call_result>")
            .replace("</tool_call>", "</tool_call_result>")
            .replace("<toolcall>", "<tool_call_result>")
            .replace("</toolcall>", "</tool_call_result>")
            .replace("<tool-call>", "<tool_call_result>")
            .replace("</tool-call>", "</tool_call_result>")
            .replace("<invoke>", "<tool_call_result>")
            .replace("</invoke>", "</tool_call_result>");

        let mut remaining = normalized.as_str();

        // 循环扫描文本中的所有工具调用标签
        while let Some(start) = remaining.find("<tool_call_result>") {
            // 提取标签前的文本内容
            let before = &remaining[..start];
            if !before.trim().is_empty() {
                text_parts.push(before.trim().to_string());
            }

            // 查找标签结束位置
            if let Some(end) = remaining[start..].find("</tool_call_result>") {
                // 提取标签内的 JSON 内容
                let inner = &remaining[start + 18..start + end];
                match serde_json::from_str::<Value>(inner.trim()) {
                    Ok(parsed) => {
                        // 提取工具名称
                        let name =
                            parsed.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                        if name.is_empty() {
                            // 工具名称为空，跳过此调用
                            remaining = &remaining[start + end + 19..];
                            continue;
                        }
                        // 提取工具参数，如果不存在则使用空对象
                        let arguments = parsed
                            .get("arguments")
                            .cloned()
                            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
                        calls.push(ParsedToolCall { name, arguments, tool_call_id: None });
                    }
                    Err(e) => {
                        // JSON 解析失败，记录警告
                        tracing::warn!("格式错误的 <tool_call_result> JSON: {e}");
                    }
                }
                // 继续处理剩余文本
                remaining = &remaining[start + end + 19..];
            } else {
                // 没有找到结束标签，退出循环
                break;
            }
        }

        // 处理最后剩余的文本内容
        if !remaining.trim().is_empty() {
            text_parts.push(remaining.trim().to_string());
        }

        (text_parts.join("\n"), calls)
    }

    /// 生成工具规范列表
    ///
    /// 从工具集合中提取工具规范信息。
    ///
    /// # 参数
    ///
    /// - `tools`：工具集合
    ///
    /// # 返回值
    ///
    /// 返回工具规范列表
    pub fn tool_specs(tools: &[Box<dyn Tool>]) -> Vec<ToolSpec> {
        tools.iter().map(|tool| tool.spec()).collect()
    }
}

impl ToolDispatcher for XmlToolDispatcher {
    /// 解析 XML 格式的响应
    ///
    /// 从 ChatResponse 中提取文本并解析 XML 工具调用。
    fn parse_response(&self, response: &ChatResponse) -> (String, Vec<ParsedToolCall>) {
        let text = response.text_or_empty();
        Self::parse_xml_tool_calls(text)
    }

    /// 格式化工具执行结果为 XML 格式的对话消息
    ///
    /// 将工具执行结果包装为 XML 格式的用户消息。
    fn format_results(&self, results: &[ToolExecutionResult]) -> ConversationMessage {
        let mut content = String::new();
        for result in results {
            // 根据执行状态确定状态标签
            let status = if result.success { "ok" } else { "error" };
            // 格式化为 XML 工具结果标签
            let _ = writeln!(
                content,
                "<tool_result name=\"{}\" status=\"{}\">\n{}\n</tool_result>",
                result.name, status, result.output
            );
        }
        ConversationMessage::Chat(ChatMessage::user(format!("[Tool results]\n{content}")))
    }

    /// 生成工具使用提示指令
    ///
    /// 生成包含工具使用协议和可用工具列表的提示文本。
    fn prompt_instructions(&self, tools: &[Box<dyn Tool>]) -> String {
        let mut instructions = String::new();
        // 添加工具使用协议标题
        instructions.push_str("## Tool Use Protocol\n\n");
        // 添加工具调用格式说明
        instructions
            .push_str("To use a tool, wrap a JSON object in <tool_call></tool_call> tags:\n\n");
        instructions.push_str(
            "```\n<tool_call>\n{\"name\": \"tool_name\", \"arguments\": {\"param\": \"value\"}}\n</tool_call>\n```\n\n",
        );
        // 添加可用工具列表标题
        instructions.push_str("### Available Tools\n\n");

        // 遍历所有工具，添加工具名称、描述和参数说明
        for tool in tools {
            let spec = tool.spec();
            let _ = writeln!(
                instructions,
                "- **{}**: {}\n  Parameters: `{}`",
                spec.id, spec.description, spec.input_schema
            );
        }

        instructions
    }

    /// 将对话历史转换为 Provider 消息格式（XML 模式）
    ///
    /// 将各种对话消息类型转换为 Provider 可理解的格式。
    fn to_provider_messages(&self, history: &[ConversationMessage]) -> Vec<ChatMessage> {
        history
            .iter()
            .flat_map(|msg| match msg {
                // 普通聊天消息直接传递
                ConversationMessage::Chat(chat) => vec![chat.clone()],
                // 助手的工具调用消息转换为助手消息
                ConversationMessage::AssistantToolCalls { text, .. } => {
                    vec![ChatMessage::assistant(text.clone().unwrap_or_default())]
                }
                // 工具结果消息格式化为用户消息
                ConversationMessage::ToolResults(results) => {
                    let mut content = String::new();
                    for result in results {
                        let _ = writeln!(
                            content,
                            "<tool_result id=\"{}\">\n{}\n</tool_result>",
                            result.tool_call_id, result.content
                        );
                    }
                    vec![ChatMessage::user(format!("[Tool results]\n{content}"))]
                }
            })
            .collect()
    }

    /// XML 模式不需要发送工具规范
    ///
    /// 工具信息已嵌入提示文本中，不需要单独发送工具定义。
    fn should_send_tool_specs(&self) -> bool {
        false
    }
}

/// 原生格式工具调度器
///
/// 使用 Provider 原生的工具调用 API（如 OpenAI 的 function calling）。
/// 适用于支持原生工具调用的 Provider。
///
/// # 特点
///
/// - 直接使用 Provider 返回的工具调用结构
/// - 工具调用带有唯一 ID，支持并行调用
/// - 无需额外的提示指令
/// - 需要在请求中发送工具定义
///
/// # 示例
///
/// ```ignore
/// let dispatcher = NativeToolDispatcher;
/// let (text, calls) = dispatcher.parse_response(&response);
/// ```
pub struct NativeToolDispatcher;

impl ToolDispatcher for NativeToolDispatcher {
    /// 解析原生格式的响应
    ///
    /// 从 ChatResponse 中提取文本和原生工具调用。
    fn parse_response(&self, response: &ChatResponse) -> (String, Vec<ParsedToolCall>) {
        let text = response.text.clone().unwrap_or_default();
        // 将原生工具调用转换为统一的 ParsedToolCall 格式
        let calls = response
            .tool_calls
            .iter()
            .map(|tc| ParsedToolCall {
                name: tc.name.clone(),
                // 解析参数 JSON 字符串，失败时使用空对象
                arguments: serde_json::from_str(&tc.arguments).unwrap_or_else(|e| {
                    tracing::warn!(
                        tool = %tc.name,
                        error = %e,
                        "解析原生工具调用参数为 JSON 失败；使用空对象作为默认值"
                    );
                    Value::Object(serde_json::Map::new())
                }),
                tool_call_id: Some(tc.id.clone()),
            })
            .collect();
        (text, calls)
    }

    /// 格式化工具执行结果为原生格式的对话消息
    ///
    /// 将工具执行结果转换为 ToolResults 类型的消息。
    fn format_results(&self, results: &[ToolExecutionResult]) -> ConversationMessage {
        let messages = results
            .iter()
            .map(|result| ToolResultMessage {
                // 如果没有 tool_call_id，使用 "unknown" 作为默认值
                tool_call_id: result.tool_call_id.clone().unwrap_or_else(|| "unknown".to_string()),
                content: result.output.clone(),
            })
            .collect();
        ConversationMessage::ToolResults(messages)
    }

    /// 原生模式不需要额外的提示指令
    ///
    /// Provider 原生支持工具调用，不需要在提示中说明使用方法。
    fn prompt_instructions(&self, _tools: &[Box<dyn Tool>]) -> String {
        String::new()
    }

    /// 将对话历史转换为 Provider 消息格式（原生模式）
    ///
    /// 将各种对话消息类型转换为 Provider 原生支持的格式。
    fn to_provider_messages(&self, history: &[ConversationMessage]) -> Vec<ChatMessage> {
        history
            .iter()
            .flat_map(|msg| match msg {
                // 普通聊天消息直接传递
                ConversationMessage::Chat(chat) => vec![chat.clone()],
                // 助手的工具调用消息转换为包含工具调用信息的助手消息
                ConversationMessage::AssistantToolCalls { text, tool_calls, reasoning_content } => {
                    // 构建包含内容和工具调用的 JSON 载荷
                    let mut payload = serde_json::json!({
                        "content": text,
                        "tool_calls": tool_calls,
                    });
                    // 如果存在推理内容，添加到载荷中
                    if let Some(rc) = reasoning_content {
                        payload["reasoning_content"] = serde_json::json!(rc);
                    }
                    vec![ChatMessage::assistant(payload.to_string())]
                }
                // 工具结果消息转换为工具类型的消息
                ConversationMessage::ToolResults(results) => results
                    .iter()
                    .map(|result| {
                        ChatMessage::tool(
                            serde_json::json!({
                                "tool_call_id": result.tool_call_id,
                                "content": result.content,
                            })
                            .to_string(),
                        )
                    })
                    .collect(),
            })
            .collect()
    }

    /// 原生模式需要发送工具规范
    ///
    /// Provider 需要工具定义来处理工具调用。
    fn should_send_tool_specs(&self) -> bool {
        true
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
