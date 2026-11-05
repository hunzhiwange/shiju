//! 历史记录构建模块
//!
//! 本模块负责处理代理对话历史的构建和格式化，主要用于与 AI 模型提供商的交互。
//! 提供了将工具注册表转换为 OpenAI 兼容格式、构建助手消息历史记录等功能。
//!
//! # 主要功能
//!
//! - **工具格式转换**：将内部工具注册表转换为 OpenAI 函数调用格式
//! - **历史记录构建**：为原生工具调用 API 构建结构化的消息历史
//! - **记忆键生成**：生成用于自动保存的唯一记忆标识符
//!
//! # 使用场景
//!
//! 此模块在代理循环中被广泛使用，用于：
//! 1. 准备发送给 LLM 提供商的工具定义
//! 2. 格式化助手的响应（包含工具调用和推理内容）
//! 3. 在 OpenRouter 等提供商中重建原生的工具调用消息结构

use super::super::parsing::ParsedToolCall;
use crate::app::agent::providers::ToolCall;
use crate::app::agent::tools::Tool;
use uuid::Uuid;

#[cfg(test)]
#[path = "history_tests.rs"]
mod history_tests;

/// 生成自动保存记忆的唯一键
///
/// 为自动保存的记忆项生成一个唯一的标识符，通过组合前缀和 UUID 实现。
/// 这确保了每次保存的记忆项都有全局唯一的键。
///
/// # 参数
///
/// * `prefix` - 记忆键的前缀，通常用于标识记忆的类型或来源
///
/// # 返回值
///
/// 返回格式为 `{prefix}_{uuid}` 的唯一字符串标识符
///
/// # 示例
///
/// ```ignore
/// let key = autosave_memory_key("session");
/// // 可能返回: "session_550e8400-e29b-41d4-a716-446655440000"
/// ```
pub fn autosave_memory_key(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4())
}

/// 将工具注册表转换为 OpenAI 函数调用格式
///
/// 此函数遍历工具注册表中的所有工具，并将它们转换为符合 OpenAI API 规范的
/// 函数调用格式。这个格式被支持原生工具调用的 AI 提供商广泛使用。
///
/// # 参数
///
/// * `tools_registry` - 工具实现的动态分发引用切片，包含所有可用工具
///
/// # 返回值
///
/// 返回一个 JSON 值向量，每个元素代表一个工具的定义，格式如下：
/// ```json
/// {
///     "type": "function",
///     "function": {
///         "name": "工具名称",
///         "description": "工具描述",
///         "parameters": { /* JSON Schema 格式的参数定义 */ }
///     }
/// }
/// ```
///
/// # 示例
///
/// ```ignore
/// let tools: Vec<Box<dyn Tool>> = vec![/* ... */];
/// let formatted = tools_to_openai_format(&tools);
/// // 可用于 OpenAI API 的 tools 参数
/// ```
pub(crate) fn tools_to_openai_format(tools_registry: &[Box<dyn Tool>]) -> Vec<serde_json::Value> {
    tools_registry
        .iter()
        .map(|tool| {
            let spec = tool.spec();
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": spec.id,
                    "description": spec.description,
                    "parameters": spec.input_schema
                }
            })
        })
        .collect()
}

/// 为原生工具调用 API 构建助手历史记录条目
///
/// 构建一个 JSON 格式的助手消息历史记录，用于支持原生工具调用的 AI API。
/// 这个格式被 OpenRouter 提供商中的 `convert_messages` 函数解析，
/// 以重建带有结构化 `tool_calls` 的 `NativeMessage`。
///
/// # 参数
///
/// * `text` - 助手的文本响应内容，可能为空
/// * `tool_calls` - 工具调用数组，包含工具的 ID、名称和参数
/// * `reasoning_content` - 可选的推理内容（思维链），某些模型（如 DeepSeek）支持
///
/// # 返回值
///
/// 返回 JSON 字符串，格式如下：
/// ```json
/// {
///     "content": "文本内容或 null",
///     "tool_calls": [
///         {
///             "id": "工具调用ID",
///             "name": "工具名称",
///             "arguments": "参数JSON字符串"
///         }
///     ],
///     "reasoning_content": "可选的推理内容"
/// }
/// ```
///
/// # 处理逻辑
///
/// 1. 将所有工具调用转换为 JSON 格式
/// 2. 处理文本内容：空文本转换为 null，否则去除首尾空白
/// 3. 如果提供了推理内容，添加到输出对象中
/// 4. 返回完整的 JSON 字符串表示
///
/// # 示例
///
/// ```ignore
/// let tool_calls = vec![ToolCall {
///     id: "call_123".to_string(),
///     name: "get_weather".to_string(),
///     arguments: r#"{"location": "Beijing"}"#.to_string(),
/// }];
/// let history = build_native_assistant_history(
///     "Let me check the weather for you.",
///     &tool_calls,
///     None
/// );
/// ```
pub(crate) fn build_native_assistant_history(
    text: &str,
    tool_calls: &[ToolCall],
    reasoning_content: Option<&str>,
) -> String {
    // 将工具调用数组转换为 JSON 格式
    let calls_json: Vec<serde_json::Value> = tool_calls
        .iter()
        .map(|tc| {
            let mut call = serde_json::json!({
                "id": tc.id,
                "name": tc.name,
                "arguments": tc.arguments,
            });
            if tc.kind != crate::providers::ToolCallKind::Function {
                call["kind"] = serde_json::json!(tc.kind);
            }
            if let Some(namespace) = &tc.namespace {
                call["namespace"] = serde_json::json!(namespace);
            }
            call
        })
        .collect();

    // 处理文本内容：空文本使用 null，非空文本去除首尾空白
    let content = if text.trim().is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(text.trim().to_string())
    };

    // 构建基础的历史记录对象
    let mut obj = serde_json::json!({
        "content": content,
        "tool_calls": calls_json,
    });

    // 如果存在推理内容，添加到对象中
    if let Some(rc) = reasoning_content {
        obj.as_object_mut()
            .unwrap()
            .insert("reasoning_content".to_string(), serde_json::Value::String(rc.to_string()));
    }

    obj.to_string()
}

/// 从已解析的工具调用构建助手历史记录条目
///
/// 与 `build_native_assistant_history` 类似，但接受 `ParsedToolCall` 类型作为输入。
/// 这个版本用于处理已经过解析和验证的工具调用数据。
///
/// # 参数
///
/// * `text` - 助手的文本响应内容，可能为空
/// * `tool_calls` - 已解析的工具调用数组，参数已经是结构化数据
/// * `reasoning_content` - 可选的推理内容
///
/// # 返回值
///
/// 返回 `Option<String>`：
/// - `Some(String)` - 成功构建的 JSON 字符串
/// - `None` - 如果任何工具调用缺少必需的 ID 字段
///
/// # 与 build_native_assistant_history 的区别
///
/// - 接受 `ParsedToolCall` 而不是 `ToolCall`
/// - 参数是已解析的 JSON 值，需要序列化为字符串
/// - 返回 `Option<String>` 以处理缺少 ID 的情况
///
/// # 处理逻辑
///
/// 1. 遍历工具调用，为每个调用构建 JSON 对象
/// 2. 如果任何调用缺少 ID（tool_call_id 为 None），整个函数返回 None
/// 3. 将参数对象序列化为 JSON 字符串
/// 4. 构建完整的历史记录对象，包含内容、工具调用和可选的推理内容
///
/// # 示例
///
/// ```ignore
/// let parsed_calls = vec![ParsedToolCall {
///     tool_call_id: Some("call_123".to_string()),
///     name: "search".to_string(),
///     arguments: json!({"query": "Rust programming"}),
/// }];
/// let history = build_native_assistant_history_from_parsed_calls(
///     "Searching for information...",
///     &parsed_calls,
///     Some("User wants to learn Rust")
/// );
/// assert!(history.is_some());
/// ```
pub(crate) fn build_native_assistant_history_from_parsed_calls(
    text: &str,
    tool_calls: &[ParsedToolCall],
    reasoning_content: Option<&str>,
) -> Option<String> {
    // 将已解析的工具调用转换为 JSON 格式
    // 使用 Option 处理可能缺少 tool_call_id 的情况
    let calls_json = tool_calls
        .iter()
        .map(|tc| {
            Some(serde_json::json!({
                // 如果 ID 不存在，整个映射返回 None
                "id": tc.tool_call_id.clone()?,
                "name": tc.name,
                // 将参数对象序列化为 JSON 字符串，失败时使用空对象
                "arguments": serde_json::to_string(&tc.arguments).unwrap_or_else(|_| "{}".to_string()),
            }))
        })
        .collect::<Option<Vec<_>>>()?; // 任何 None 会导致整个结果为 None

    // 处理文本内容：空文本使用 null，非空文本去除首尾空白
    let content = if text.trim().is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(text.trim().to_string())
    };

    // 构建基础的历史记录对象
    let mut obj = serde_json::json!({
        "content": content,
        "tool_calls": calls_json,
    });

    // 如果存在推理内容，添加到对象中
    if let Some(rc) = reasoning_content {
        obj.as_object_mut()
            .unwrap()
            .insert("reasoning_content".to_string(), serde_json::Value::String(rc.to_string()));
    }

    Some(obj.to_string())
}
