//! 通过本地 gateway 会话流式能力实现的 LLM provider。
//!
//! `SessionLlmProvider` 把通用 provider trait 的请求转换为 gateway chat stream 请求。
//! 原生目标直接走 `vw_gateway_client`；wasm 目标不支持本地 gateway 流式调用，因此显式
//! 返回错误，避免表现为半可用的 provider。

use crate::app::agent::multimodal::DISABLE_OCR_OPTION;
use crate::app::agent::providers::traits::{
    ChatMessage, ChatRequest, ChatResponse, Provider, ProviderCapabilities, StreamChunk,
    StreamOptions, TokenUsage, ToolsPayload,
};
use async_trait::async_trait;
use futures_util::{StreamExt, stream};

/// 基于当前会话 gateway 的 LLM provider。
pub struct SessionLlmProvider {
    support_vision: bool,
}

impl SessionLlmProvider {
    /// 创建新的会话 LLM provider。
    pub fn new() -> Self {
        Self { support_vision: false }
    }

    /// 按运行时选项创建会话 LLM provider。
    pub fn with_options(options: &crate::app::agent::providers::ProviderRuntimeOptions) -> Self {
        // gateway/aisdk 路径已支持多模态；未显式关闭时默认放开 vision，避免图片标记被本地误拒。
        Self { support_vision: options.model_support_vision.unwrap_or(true) }
    }

    /// 创建并显式声明是否支持视觉输入。
    pub fn with_vision(support_vision: bool) -> Self {
        Self { support_vision }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn gateway_endpoint() -> vw_gateway_client::GatewayEndpoint {
        let config = crate::app::agent::config::get_blocking();
        vw_gateway_client::GatewayEndpoint::new(config.gateway.host, config.gateway.port)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn build_messages(system_prompt: Option<&str>, message: &str) -> Vec<serde_json::Value> {
        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            if !sys.trim().is_empty() {
                messages.push(serde_json::json!({"role":"system","content": sys}));
            }
        }
        messages.push(serde_json::json!({"role":"user","content": message}));
        messages
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn build_history_messages(messages: &[ChatMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .filter_map(|message| match message.role.as_str() {
                "system" | "user" | "assistant" => {
                    Some(serde_json::json!({"role": message.role, "content": message.content}))
                }
                "tool" => {
                    Some(serde_json::json!({"role": "tool", "content": message.content}))
                }
                other => {
                    // gateway 只接受明确支持的角色；跳过未知角色比伪造角色更安全。
                    tracing::warn!(role = %other, "session llm provider skipped unsupported history role");
                    None
                }
            })
            .collect()
    }

    fn extract_system_and_last_user<'a>(messages: &'a [ChatMessage]) -> (Option<&'a str>, &'a str) {
        let system = messages.iter().find(|m| m.role == "system").map(|m| m.content.as_str());
        let last_user =
            messages.iter().rfind(|m| m.role == "user").map(|m| m.content.as_str()).unwrap_or("");
        (system, last_user)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn parse_usage(value: Option<&serde_json::Value>) -> Option<TokenUsage> {
        let usage: vw_gateway_client::GatewayChatUsage =
            serde_json::from_value(value?.clone()).ok()?;

        Some(TokenUsage {
            input_tokens: u64::try_from(usage.input_tokens).ok(),
            output_tokens: u64::try_from(usage.output_tokens).ok(),
            cached_tokens: u64::try_from(usage.cached_tokens).ok(),
            reasoning_tokens: u64::try_from(usage.reasoning_tokens).ok(),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn gateway_chat_request(
        messages: Vec<serde_json::Value>,
        model: &str,
    ) -> vw_gateway_client::GatewayChatStreamRequest {
        vw_gateway_client::GatewayChatStreamRequest {
            session_id: None,
            messages,
            system: None,
            model: Some(model.to_string()),
            reasoning_effort: None,
            agent: None,
            allowed_tools: None,
            acp_agent: None,
            acp_allowed_tools: None,
            // Provider 调用只负责一次 LLM 推理；工具编排由外层调用方负责。
            options: Some(serde_json::json!({
                "disable_tools": true,
                (DISABLE_OCR_OPTION): true,
            })),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn stream_gateway_chat_blocking(
        messages: Vec<serde_json::Value>,
        model: &str,
    ) -> anyhow::Result<(String, Option<TokenUsage>)> {
        let endpoint = Self::gateway_endpoint();
        let mut output = String::new();
        let mut stream_error = None;
        let mut usage = None;

        vw_gateway_client::GatewayClient::stream_chat_blocking(
            &endpoint,
            None,
            &Self::gateway_chat_request(messages, model),
            |event| {
                match event {
                    vw_gateway_client::GatewayChatStreamEvent::Delta(delta) => {
                        output.push_str(&delta)
                    }
                    vw_gateway_client::GatewayChatStreamEvent::Done {
                        usage: done_usage, ..
                    } => {
                        // Done 事件是用量统计的唯一可靠来源，收到后即可停止阻塞流消费。
                        usage = Self::parse_usage(done_usage.as_ref());
                        return false;
                    }
                    vw_gateway_client::GatewayChatStreamEvent::Error(error) => {
                        stream_error = Some(error);
                        return false;
                    }
                    vw_gateway_client::GatewayChatStreamEvent::Other(_) => {}
                }
                true
            },
        )
        .map_err(anyhow::Error::msg)?;

        if let Some(error) = stream_error {
            anyhow::bail!(error);
        }

        Ok((output, usage))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for SessionLlmProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities { native_tool_calling: false, vision: self.support_vision }
    }

    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let messages = Self::build_messages(system_prompt, message);
            let model = model.to_string();
            let (text, _) = tokio::task::spawn_blocking(move || {
                Self::stream_gateway_chat_blocking(messages, &model)
            })
            .await
            .map_err(|error| anyhow::anyhow!("gateway chat task failed: {error}"))??;
            return Ok(text);
        }

        #[cfg(target_arch = "wasm32")]
        anyhow::bail!("SessionLlmProvider requires gateway streaming on non-wasm targets")
    }

    async fn chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        let messages = if let Some(tools) = request.tools {
            if !tools.is_empty() && !self.supports_native_tools() {
                // 当前 provider 不声明原生工具调用能力，因此把工具定义降级为系统提示。
                let tool_instructions = match self.convert_tools(tools) {
                    ToolsPayload::PromptGuided { instructions } => instructions,
                    payload => {
                        anyhow::bail!(
                            "Provider returned non-prompt-guided tools payload ({payload:?}) while supports_native_tools() is false"
                        )
                    }
                };
                let mut modified_messages = request.messages.to_vec();
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
                modified_messages
            } else {
                request.messages.to_vec()
            }
        } else {
            request.messages.to_vec()
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let history_messages = Self::build_history_messages(&messages);
            let model = model.to_string();
            let (text, usage) = tokio::task::spawn_blocking(move || {
                Self::stream_gateway_chat_blocking(history_messages, &model)
            })
            .await
            .map_err(|error| anyhow::anyhow!("gateway chat task failed: {error}"))??;
            return Ok(ChatResponse {
                text: Some(text),
                tool_calls: Vec::new(),
                usage,
                reasoning_content: None,
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            let text = self.chat_with_history(&messages, model, temperature).await?;
            Ok(ChatResponse {
                text: Some(text),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            })
        }
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (text, _) =
                Self::stream_gateway_chat_blocking(Self::build_history_messages(messages), model)?;
            return Ok(text);
        }

        #[cfg(target_arch = "wasm32")]
        {
            let (system, last_user) = Self::extract_system_and_last_user(messages);
            self.chat_with_system(system, last_user, model, temperature).await
        }
    }

    fn stream_chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        _temperature: f64,
        options: StreamOptions,
    ) -> stream::BoxStream<'static, crate::app::agent::providers::traits::StreamResult<StreamChunk>>
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let endpoint = Self::gateway_endpoint();
            let model = model.to_string();
            let messages = Self::build_messages(system_prompt, message);

            let enable_count = options.count_tokens;
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<StreamChunk>();
            std::thread::spawn(move || {
                // gateway client 是阻塞接口，放入独立线程并用 channel 桥接回异步 stream。
                let result = vw_gateway_client::GatewayClient::stream_chat_blocking(
                    &endpoint,
                    None,
                    &Self::gateway_chat_request(messages, &model),
                    |event| {
                        match event {
                            vw_gateway_client::GatewayChatStreamEvent::Delta(delta) => {
                                let chunk = if enable_count {
                                    // 未提供真实 token 增量时，用估算值维持调用方的流式计数体验。
                                    StreamChunk::delta(delta).with_token_estimate()
                                } else {
                                    StreamChunk::delta(delta)
                                };
                                let _ = tx.send(chunk);
                            }
                            vw_gateway_client::GatewayChatStreamEvent::Done { .. } => {
                                let _ = tx.send(StreamChunk::final_chunk());
                                return false;
                            }
                            vw_gateway_client::GatewayChatStreamEvent::Error(error) => {
                                let _ = tx.send(StreamChunk::error(error));
                                return false;
                            }
                            vw_gateway_client::GatewayChatStreamEvent::Other(_) => {}
                        }
                        true
                    },
                );

                if let Err(err) = result {
                    let _ = tx.send(StreamChunk::error(err));
                }
            });

            let s = tokio_stream::wrappers::UnboundedReceiverStream::new(rx).map(Ok);
            return s.boxed();
        }

        #[cfg(target_arch = "wasm32")]
        {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<StreamChunk>();
            let _ = tx.send(StreamChunk::error(
                "SessionLlmProvider requires gateway streaming on non-wasm targets",
            ));
            let s = tokio_stream::wrappers::UnboundedReceiverStream::new(rx).map(Ok);
            s.boxed()
        }
    }

    fn stream_chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        _temperature: f64,
        options: StreamOptions,
    ) -> stream::BoxStream<'static, crate::app::agent::providers::traits::StreamResult<StreamChunk>>
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let endpoint = Self::gateway_endpoint();
            let model = model.to_string();
            let messages = Self::build_history_messages(messages);
            let enable_count = options.count_tokens;
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<StreamChunk>();

            std::thread::spawn(move || {
                // 与 system/message 入口保持相同桥接方式，确保历史消息流式路径行为一致。
                let result = vw_gateway_client::GatewayClient::stream_chat_blocking(
                    &endpoint,
                    None,
                    &Self::gateway_chat_request(messages, &model),
                    |event| {
                        match event {
                            vw_gateway_client::GatewayChatStreamEvent::Delta(delta) => {
                                let chunk = if enable_count {
                                    StreamChunk::delta(delta).with_token_estimate()
                                } else {
                                    StreamChunk::delta(delta)
                                };
                                let _ = tx.send(chunk);
                            }
                            vw_gateway_client::GatewayChatStreamEvent::Done { .. } => {
                                let _ = tx.send(StreamChunk::final_chunk());
                                return false;
                            }
                            vw_gateway_client::GatewayChatStreamEvent::Error(error) => {
                                let _ = tx.send(StreamChunk::error(error));
                                return false;
                            }
                            vw_gateway_client::GatewayChatStreamEvent::Other(_) => {}
                        }
                        true
                    },
                );

                if let Err(err) = result {
                    let _ = tx.send(StreamChunk::error(err));
                }
            });

            let s = tokio_stream::wrappers::UnboundedReceiverStream::new(rx).map(Ok);
            return s.boxed();
        }

        #[cfg(target_arch = "wasm32")]
        {
            let (system, last_user) = Self::extract_system_and_last_user(messages);
            self.stream_chat_with_system(system, last_user, model, temperature, options)
        }
    }
}

#[cfg(test)]
#[path = "session_llm_tests.rs"]
mod session_llm_tests;
