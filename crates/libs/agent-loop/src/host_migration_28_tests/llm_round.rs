//! 测试单轮模型调用的工具解析、多模态降级、钩子、用量与取消处理。
//! 脚本化供应商、OCR 与钩子夹具记录实际输入，以验证降级路径而非仅验证返回文本。

use super::*;
use agent_hooks::HookHandler;
use agent_multimodal::ocr::{ImageTextExtractor, SessionImageTextExtractor};
use agent_observability::{Observer, ObserverEvent};
use model_types::{ChatMessage, ChatRequest, Provider, ToolCall};
use tool_types::ToolSpec;
use crate::parsing::{ParsedToolCall, detect_tool_call_parse_issue, parse_structured_tool_calls, parse_tool_calls};
use crate::core::llm_round::LlmRoundResult as RoundResult;
type LlmRoundResult = RoundResult<ParsedToolCall>;
use agent_observability::traits::ObserverMetric;
use model_types::{ChatResponse, ProviderCapabilities, TokenUsage};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[allow(clippy::too_many_arguments)]
// 集中装配单轮依赖，个别场景只替换供应商能力、OCR 或取消条件。
async fn run_test_round(
    provider: &TestProvider,
    observer: &RecordingObserver,
    history: &[ChatMessage],
    use_native_tools: bool,
    tool_specs: &[ToolSpec],
    cancellation_token: Option<&CancellationToken>,
    iteration: usize,
) -> Result<LlmRoundResult> {
    run_llm_round(
        provider,
        history,
        observer,
        "testcov-provider",
        "testcov-model",
        0.2,
        "testcov-channel",
        &config_tools::MultimodalConfig::default(),
        cancellation_token,
        None,
        tool_specs,
        use_native_tools,
        "turn-testcov-0088",
        iteration,
    )
    .await
}

struct TestProvider {
    result: Mutex<Option<Result<ChatResponse>>>,
    capabilities: ProviderCapabilities,
    calls: Mutex<usize>,
    last_tools_len: Mutex<Option<usize>>,
    last_messages: Mutex<Vec<ChatMessage>>,
    delay: Option<Duration>,
}

struct RecordingLlmInputHook {
    calls: Arc<AtomicUsize>,
    models: Arc<Mutex<Vec<String>>>,
}

#[derive(Default)]
struct StubImageTextExtractor {
    calls: AtomicUsize,
}

impl StubImageTextExtractor {
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ImageTextExtractor for StubImageTextExtractor {
    async fn supports_image_input(&self, _model_ref: &str) -> Result<bool> {
        Ok(true)
    }

    async fn extract_text(&self, _model_ref: &str, _image_data_uri: &str) -> Result<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok("visible OCR text".to_string())
    }
}

#[async_trait]
impl HookHandler for RecordingLlmInputHook {
    fn name(&self) -> &str {
        "recording-llm-input-testcov-0088"
    }

    async fn on_llm_input(&self, messages: &[ChatMessage], model: &str) {
        assert_eq!(messages.len(), 1);
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.models.lock().expect("models lock should not be poisoned").push(model.to_string());
    }
}

impl TestProvider {
    fn with_response(response: ChatResponse) -> Self {
        Self::with_result(Ok(response), None, ProviderCapabilities::default())
    }

    fn with_delay(delay: Duration, response: ChatResponse) -> Self {
        Self::with_result(Ok(response), Some(delay), ProviderCapabilities::default())
    }

    fn with_error(message: &'static str) -> Self {
        Self::with_result(Err(anyhow::anyhow!(message)), None, ProviderCapabilities::default())
    }

    fn with_result(
        result: Result<ChatResponse>,
        delay: Option<Duration>,
        capabilities: ProviderCapabilities,
    ) -> Self {
        Self {
            result: Mutex::new(Some(result)),
            capabilities,
            calls: Mutex::new(0),
            last_tools_len: Mutex::new(None),
            last_messages: Mutex::new(Vec::new()),
            delay,
        }
    }

    fn calls(&self) -> usize {
        *self.calls.lock().expect("calls lock should not be poisoned")
    }

    fn last_tools_len(&self) -> Option<usize> {
        *self.last_tools_len.lock().expect("tools lock should not be poisoned")
    }

    fn last_messages(&self) -> Vec<ChatMessage> {
        self.last_messages.lock().expect("messages lock should not be poisoned").clone()
    }
}

#[async_trait]
impl Provider for TestProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        self.capabilities.clone()
    }

    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> Result<String> {
        Ok(String::new())
    }

    async fn chat(
        &self,
        request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> Result<ChatResponse> {
        if let Some(delay) = self.delay {
            tokio::time::sleep(delay).await;
        }

        *self.calls.lock().expect("calls lock should not be poisoned") += 1;
        *self.last_tools_len.lock().expect("tools lock should not be poisoned") =
            request.tools.map(|tools| tools.len());
        *self.last_messages.lock().expect("messages lock should not be poisoned") =
            request.messages.to_vec();

        self.result
            .lock()
            .expect("result lock should not be poisoned")
            .take()
            .expect("test provider response should be configured")
    }
}

#[derive(Default)]
struct RecordingObserver {
    events: Arc<Mutex<Vec<ObserverEvent>>>,
}

impl RecordingObserver {
    fn events(&self) -> Vec<ObserverEvent> {
        self.events.lock().expect("events lock should not be poisoned").clone()
    }
}

impl Observer for RecordingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        self.events.lock().expect("events lock should not be poisoned").push(event.clone());
    }

    fn record_metric(&self, _metric: &ObserverMetric) {}

    fn name(&self) -> &str {
        "recording-testcov-0088"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_llm_round(
    provider: &dyn Provider,
    history: &[ChatMessage],
    observer: &dyn Observer,
    provider_name: &str,
    model: &str,
    temperature: f64,
    channel_name: &str,
    multimodal_config: &config_tools::MultimodalConfig,
    cancellation_token: Option<&CancellationToken>,
    hooks: Option<&agent_hooks::HookRunner>,
    tool_specs: &[ToolSpec],
    use_native_tools: bool,
    turn_id: &str,
    iteration: usize,
) -> Result<LlmRoundResult> {
    let extractor = SessionImageTextExtractor::new();
    run_llm_round_with_extractor(
        provider,
        history,
        observer,
        provider_name,
        model,
        temperature,
        channel_name,
        multimodal_config,
        cancellation_token,
        hooks,
        tool_specs,
        use_native_tools,
        turn_id,
        iteration,
        &extractor,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_llm_round_with_extractor(
    provider: &dyn Provider,
    history: &[ChatMessage],
    observer: &dyn Observer,
    provider_name: &str,
    model: &str,
    temperature: f64,
    channel_name: &str,
    multimodal_config: &config_tools::MultimodalConfig,
    cancellation_token: Option<&CancellationToken>,
    hooks: Option<&agent_hooks::HookRunner>,
    tool_specs: &[ToolSpec],
    use_native_tools: bool,
    turn_id: &str,
    iteration: usize,
    extractor: &dyn ImageTextExtractor,
) -> Result<LlmRoundResult> {
    agent_loop::core::llm_round::run_llm_round_with_extractor(&services(), provider, history, observer, provider_name, model, temperature, channel_name, multimodal_config, cancellation_token, hooks, tool_specs, use_native_tools, turn_id, iteration, extractor).await
}

/// 为文本回退解析出的工具调用补齐稳定 id。
///
/// 参数 `iteration` 用于保证同一轮内 id 可读且跨轮不冲突。
#[cfg(test)]
fn assign_fallback_tool_call_ids(calls: &mut [ParsedToolCall], iteration: usize) {
    agent_loop::core::llm_round::assign_fallback_tool_call_ids(calls, iteration, |call| &mut call.tool_call_id);
}

/// 构造写入会话历史的助手消息内容。
///
/// 返回值会保留 provider 原生工具调用或从文本解析出的工具调用信息，确保后续
/// tool result 能与模型上下文正确配对。
#[cfg(test)]
fn build_assistant_history(text: &str, parsed: &[ParsedToolCall], native: &[ToolCall], reasoning: Option<&str>, use_native: bool) -> String {
    agent_loop::core::llm_round::build_assistant_history(&services(), text, parsed, native, reasoning, use_native)
}

fn services() -> crate::core::llm_round::LlmServices<ParsedToolCall> {
    crate::core::llm_round::LlmServices {
        parse_structured: parse_structured_tool_calls, parse_text: parse_tool_calls,
        detect_issue: detect_tool_call_parse_issue, call_id_mut: |call| &mut call.tool_call_id,
        history_parsed: |text, calls, reasoning| crate::core::history::build_native_assistant_history_from_parsed_calls(text, calls, reasoning, |call| crate::core::tool_execution::CallView { name: &call.name, arguments: &call.arguments, tool_call_id: call.tool_call_id.as_deref() }),
        history_native: crate::core::history::build_native_assistant_history,
        scrub_credentials: agent_util::credentials::scrub_credentials,
        token_usage: plugin_hooks::hook_model::token_usage,
        emit_model: |_, _| Box::pin(async {}),
    }
}

mod tests;
