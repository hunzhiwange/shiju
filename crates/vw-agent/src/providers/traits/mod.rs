//! 模型提供方的通用对话类型、流式事件与调用契约。
//!
//! 上层会话通过这些类型屏蔽具体供应商协议差异。

use crate::app::agent::tools::ToolSpec;
use async_trait::async_trait;
use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

/// 对话中的单条角色消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    /// 构造系统角色消息。
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".into(), content: content.into() }
    }

    /// 构造用户角色消息。
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".into(), content: content.into() }
    }

    /// 构造助手角色消息。
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: "assistant".into(), content: content.into() }
    }

    /// 构造工具角色消息。
    pub fn tool(content: impl Into<String>) -> Self {
        Self { role: "tool".into(), content: content.into() }
    }
}

/// 模型请求执行的结构化工具调用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// 单次模型 API 响应报告的原始令牌计数。
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cached_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
}

/// 可同时包含文本、工具调用和推理透传内容的模型响应。
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// 响应文本；仅含工具调用时可以为空。
    pub text: Option<String>,
    /// 模型请求的工具调用。
    pub tool_calls: Vec<ToolCall>,
    /// 提供方可用时返回的令牌用量。
    pub usage: Option<TokenUsage>,
    /// 推理模型返回的原始思考内容。
    ///
    /// 该字段保持不透明透传，以满足要求在后续工具调用历史中回传推理内容的提供方。
    pub reasoning_content: Option<String>,
}

impl ChatResponse {
    /// 判断模型是否请求了至少一个工具调用。
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }

    /// 返回文本内容；缺失时返回空字符串切片。
    pub fn text_or_empty(&self) -> &str {
        self.text.as_deref().unwrap_or("")
    }
}

/// 提供方对话调用的借用式请求载荷。
#[derive(Debug, Clone, Copy)]
pub struct ChatRequest<'a> {
    pub messages: &'a [ChatMessage],
    pub tools: Option<&'a [ToolSpec]>,
}

/// 回传给模型的单个工具执行结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultMessage {
    pub tool_call_id: String,
    pub content: String,
}

/// 包含普通消息和工具交互的多轮对话记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ConversationMessage {
    /// Regular chat message (system, user, assistant).
    Chat(ChatMessage),
    /// Tool calls from the assistant (stored for history fidelity).
    AssistantToolCalls {
        text: Option<String>,
        tool_calls: Vec<ToolCall>,
        /// Raw reasoning content from thinking models, preserved for round-trip
        /// fidelity with provider APIs that require it.
        reasoning_content: Option<String>,
    },
    /// Results of tool executions, fed back to the LLM.
    ToolResults(Vec<ToolResultMessage>),
}

/// 流式响应中的一个内容分块。
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// Text delta for this chunk.
    pub delta: String,
    /// Whether this is the final chunk.
    pub is_final: bool,
    /// Approximate token count for this chunk (estimated).
    pub token_count: usize,
}

impl StreamChunk {
    /// 创建非终止文本增量。
    pub fn delta(text: impl Into<String>) -> Self {
        Self { delta: text.into(), is_final: false, token_count: 0 }
    }

    /// 创建不携带文本的终止分块。
    pub fn final_chunk() -> Self {
        Self { delta: String::new(), is_final: true, token_count: 0 }
    }

    /// 创建以错误消息为内容的终止分块。
    pub fn error(message: impl Into<String>) -> Self {
        Self { delta: message.into(), is_final: true, token_count: 0 }
    }

    /// 按约每四字节一个令牌粗略估算当前分块用量。
    pub fn with_token_estimate(mut self) -> Self {
        self.token_count = self.delta.len().div_ceil(4);
        self
    }
}

/// 流式对话请求选项。
#[derive(Debug, Clone, Copy, Default)]
pub struct StreamOptions {
    /// Whether to enable streaming (default: true).
    pub enabled: bool,
    /// Whether to include token counts in chunks.
    pub count_tokens: bool,
}

impl StreamOptions {
    /// 以显式流式开关创建选项。
    pub fn new(enabled: bool) -> Self {
        Self { enabled, count_tokens: false }
    }

    /// 启用分块令牌估算。
    pub fn with_token_count(mut self) -> Self {
        self.count_tokens = true;
        self
    }
}

/// 流式操作的统一结果类型。
pub type StreamResult<T> = std::result::Result<T, StreamError>;

/// 流式传输、解析或提供方执行阶段的错误。
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("HTTP error: {0}")]
    Http(reqwest::Error),

    #[error("JSON parse error: {0}")]
    Json(serde_json::Error),

    #[error("Invalid SSE format: {0}")]
    InvalidSse(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// 请求能力不受支持时返回的结构化错误。
#[derive(Debug, Clone, thiserror::Error)]
#[error("provider_capability_error provider={provider} capability={capability} message={message}")]
pub struct ProviderCapabilityError {
    pub provider: String,
    pub capability: String,
    pub message: String,
}

/// 提供方能力声明，用于选择工具调用模式和请求格式。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderCapabilities {
    /// Whether the provider supports native tool calling via API primitives.
    ///
    /// When `true`, the provider can convert tool definitions to API-native
    /// formats (e.g., Gemini's functionDeclarations, Anthropic's input_schema).
    ///
    /// When `false`, tools must be injected via system prompt as text.
    pub native_tool_calling: bool,
    /// Whether the provider supports vision / image inputs.
    pub vision: bool,
}

/// 各模型提供方原生工具定义的载荷格式。
///
/// 统一 `ToolSpec` 由具体提供方转换为这些格式；不支持原生工具时使用提示词回退。
#[derive(Debug, Clone)]
pub enum ToolsPayload {
    /// Gemini API format (functionDeclarations).
    Gemini { function_declarations: Vec<serde_json::Value> },
    /// Anthropic Messages API format (tools with input_schema).
    Anthropic { tools: Vec<serde_json::Value> },
    /// OpenAI Chat Completions API format (tools with function).
    OpenAI { tools: Vec<serde_json::Value> },
    /// Prompt-guided fallback (tools injected as text in system prompt).
    PromptGuided { instructions: String },
}

/// 屏蔽原生与 WASM 平台线程安全约束差异的提供方边界。
#[cfg(not(target_arch = "wasm32"))]
pub trait ProviderBounds: Send + Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync> ProviderBounds for T {}

#[cfg(target_arch = "wasm32")]
pub trait ProviderBounds {}
#[cfg(target_arch = "wasm32")]
impl<T> ProviderBounds for T {}

/// 模型提供方的统一同步、流式与工具调用契约。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Provider: ProviderBounds {
    /// 查询提供方能力；默认不声明任何可选能力。
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::default()
    }

    /// 将统一工具规格转换为提供方载荷；默认生成提示词引导格式。
    fn convert_tools(&self, tools: &[ToolSpec]) -> ToolsPayload {
        ToolsPayload::PromptGuided { instructions: build_tool_instructions_text(tools) }
    }

    /// 执行不含显式系统提示的单轮对话。
    async fn simple_chat(
        &self,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        self.chat_with_system(None, message, model, temperature).await
    }

    /// 执行可带系统提示的单轮对话。
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String>;

    /// 执行多轮对话；默认提取系统消息和最后一条用户消息转交单轮接口。
    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let system = messages.iter().find(|m| m.role == "system").map(|m| m.content.as_str());
        let last_user =
            messages.iter().rfind(|m| m.role == "user").map(|m| m.content.as_str()).unwrap_or("");
        self.chat_with_system(system, last_user, model, temperature).await
    }

    /// 供代理循环调用的结构化对话接口。
    ///
    /// 提供方不支持原生工具时，默认把工具说明注入系统提示且拒绝不一致的原生载荷。
    async fn chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        // If tools are provided but provider doesn't support native tools,
        // inject tool instructions into system prompt as fallback.
        if let Some(tools) = request.tools {
            if !tools.is_empty() && !self.supports_native_tools() {
                let tool_instructions = match self.convert_tools(tools) {
                    ToolsPayload::PromptGuided { instructions } => instructions,
                    payload => {
                        anyhow::bail!(
                            "Provider returned non-prompt-guided tools payload ({payload:?}) while supports_native_tools() is false"
                        )
                    }
                };
                let mut modified_messages = request.messages.to_vec();

                // Inject tool instructions into an existing system message.
                // If none exists, prepend one to the conversation.
                if let Some(system_message) =
                    modified_messages.iter_mut().find(|m| m.role == "system")
                {
                    if !system_message.content.is_empty() {
                        system_message.content.push_str("\n\n");
                    }
                    system_message.content.push_str(&tool_instructions);
                } else {
                    modified_messages.insert(0, ChatMessage::system(tool_instructions));
                }

                let text = self.chat_with_history(&modified_messages, model, temperature).await?;
                return Ok(ChatResponse {
                    text: Some(text),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                });
            }
        }

        let text = self.chat_with_history(request.messages, model, temperature).await?;
        Ok(ChatResponse {
            text: Some(text),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        })
    }

    /// 判断提供方是否支持 API 原生工具调用。
    fn supports_native_tools(&self) -> bool {
        self.capabilities().native_tool_calling
    }

    /// 判断提供方是否支持多模态图像输入。
    fn supports_vision(&self) -> bool {
        self.capabilities().vision
    }

    /// 预热连接池；默认无操作，持有 HTTP 客户端的提供方可覆盖。
    async fn warmup(&self) -> anyhow::Result<()> {
        Ok(())
    }

    /// 携带原生工具定义执行对话；默认回退到历史对话且不生成工具调用。
    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        _tools: &[serde_json::Value],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        let text = self.chat_with_history(messages, model, temperature).await?;
        Ok(ChatResponse {
            text: Some(text),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        })
    }

    /// 判断提供方是否支持流式响应；默认返回 `false`。
    fn supports_streaming(&self) -> bool {
        false
    }

    /// 发起可带系统提示的流式对话；默认返回空流表示不支持。
    fn stream_chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
        _options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        // Default: return an empty stream (not supported)
        stream::empty().boxed()
    }

    /// 发起带历史的流式对话；默认提取系统消息和最后一条用户消息转交基础流接口。
    fn stream_chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
        options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        let system = messages.iter().find(|m| m.role == "system").map(|m| m.content.as_str());
        let last_user =
            messages.iter().rfind(|m| m.role == "user").map(|m| m.content.as_str()).unwrap_or("");
        self.stream_chat_with_system(system, last_user, model, temperature, options)
    }
}

/// 构建提示词引导式工具调用说明。
///
/// 当提供方不支持原生工具调用时，用 XML 风格标签描述可用工具及调用协议。
pub fn build_tool_instructions_text(tools: &[ToolSpec]) -> String {
    let mut instructions = String::new();

    instructions.push_str("## Tool Use Protocol\n\n");
    instructions.push_str("To use a tool, wrap a JSON object in <tool_call></tool_call> tags:\n\n");
    instructions.push_str("<tool_call>\n");
    instructions.push_str(r#"{"name": "tool_name", "arguments": {"param": "value"}}"#);
    instructions.push_str("\n</tool_call>\n\n");
    instructions.push_str("You may use multiple tool calls in a single response. ");
    instructions.push_str("After tool execution, results appear in <tool_result> tags. ");
    instructions
        .push_str("Continue reasoning with the results until you can give a final answer.\n\n");
    instructions.push_str("### Available Tools\n\n");

    for tool in tools {
        writeln!(&mut instructions, "**{}**: {}", tool.id, tool.description)
            .expect("writing to String cannot fail");

        let parameters =
            serde_json::to_string(&tool.input_schema).unwrap_or_else(|_| "{}".to_string());
        writeln!(&mut instructions, "Parameters: `{parameters}`")
            .expect("writing to String cannot fail");
        instructions.push('\n');
    }

    instructions
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
