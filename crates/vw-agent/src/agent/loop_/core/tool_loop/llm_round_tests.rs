//! 测试单轮模型调用的工具解析、多模态降级、钩子、用量与取消处理。
//! 脚本化供应商、OCR 与钩子夹具记录实际输入，以验证降级路径而非仅验证返回文本。

use super::*;
use crate::hooks::{HookHandler, HookRunner};
use crate::multimodal::ocr::ImageTextExtractor;
use crate::observability::traits::ObserverMetric;
use crate::providers::traits::{ChatResponse, ProviderCapabilities, TokenUsage};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[test]
fn fallback_tool_call_ids_are_stable_per_iteration() {
    let mut calls = vec![
        ParsedToolCall {
            name: "shell".to_string(),
            arguments: serde_json::json!({}),
            tool_call_id: None,
        },
        ParsedToolCall {
            name: "grep".to_string(),
            arguments: serde_json::json!({}),
            tool_call_id: Some("existing".to_string()),
        },
    ];

    assign_fallback_tool_call_ids(&mut calls, 2);

    assert_eq!(calls[0].tool_call_id.as_deref(), Some("fallback_3_1"));
    assert_eq!(calls[1].tool_call_id.as_deref(), Some("existing"));
}

#[test]
fn assistant_history_handles_plain_text_and_missing_native_ids() {
    let history = build_assistant_history("plain response", &[], &[], None, false);

    assert_eq!(history, "plain response");

    let calls = vec![ParsedToolCall {
        name: "shell".to_string(),
        arguments: serde_json::json!({ "command": "pwd" }),
        tool_call_id: None,
    }];

    let history = build_assistant_history("plain response", &calls, &[], Some("reasoning"), true);

    assert_eq!(history, "plain response");
}

#[tokio::test]
async fn run_llm_round_plain_success_records_usage_and_text() {
    let provider = TestProvider::with_response(ChatResponse {
        text: Some("testcov-0088 plain response".to_string()),
        tool_calls: Vec::new(),
        usage: Some(TokenUsage {
            input_tokens: Some(11),
            output_tokens: Some(7),
            cached_tokens: Some(3),
            reasoning_tokens: Some(2),
        }),
        reasoning_content: None,
    });
    let observer = RecordingObserver::default();
    let history = vec![ChatMessage::user("hello")];
    let token = CancellationToken::new();

    let result = run_test_round(&provider, &observer, &history, false, &[], Some(&token), 0)
        .await
        .expect("plain response should succeed");

    assert_eq!(result.response_text, "testcov-0088 plain response");
    assert_eq!(result.display_text, "testcov-0088 plain response");
    assert_eq!(result.assistant_history_content, "testcov-0088 plain response");
    assert!(result.tool_calls.is_empty());
    assert!(result.native_tool_calls.is_empty());
    assert!(!result.parse_issue_detected);
    assert_eq!(result.duration_secs, 0);

    let events = observer.events();
    assert!(matches!(
        &events[0],
        ObserverEvent::LlmRequest { provider, model, messages_count }
            if provider == "testcov-provider" && model == "testcov-model" && *messages_count == 1
    ));
    assert!(matches!(
        &events[1],
        ObserverEvent::LlmResponse {
            success: true,
            error_message: None,
            input_tokens: Some(11),
            output_tokens: Some(7),
            cached_tokens: Some(3),
            reasoning_tokens: Some(2),
            ..
        }
    ));
}

#[tokio::test]
async fn run_llm_round_accepts_image_markers_for_vision_provider() {
    let provider = TestProvider::with_result(
        Ok(ChatResponse {
            text: Some("vision accepted".to_string()),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        }),
        None,
        ProviderCapabilities { native_tool_calling: false, vision: true },
    );
    let observer = RecordingObserver::default();
    let history = vec![ChatMessage::user("inspect [IMAGE:data:image/png;base64,iVBORw0KGgo=]")];

    let result = run_test_round(&provider, &observer, &history, false, &[], None, 0)
        .await
        .expect("vision provider should accept image markers");

    assert_eq!(result.response_text, "vision accepted");
    assert_eq!(provider.calls(), 1);
    assert_eq!(observer.events().len(), 2);
}

#[tokio::test]
async fn run_llm_round_text_fallback_assigns_ids_and_native_history() {
    let provider = TestProvider::with_response(ChatResponse {
        text: Some(
            "Before\n<tool_call>{\"name\":\"shell\",\"arguments\":{\"command\":\"pwd\"}}</tool_call>\nAfter"
                .to_string(),
        ),
        tool_calls: Vec::new(),
        usage: None,
        reasoning_content: Some("reasoning-testcov-0088".to_string()),
    });
    let observer = RecordingObserver::default();
    let history = vec![ChatMessage::user("use a tool")];
    let tool_specs =
        vec![ToolSpec::new("shell", "run command", serde_json::json!({"type": "object"}))];

    let result = run_test_round(&provider, &observer, &history, true, &tool_specs, None, 4)
        .await
        .expect("fallback tool call should succeed");

    assert_eq!(result.tool_calls.len(), 1);
    assert_eq!(result.tool_calls[0].name, "shell");
    assert_eq!(result.tool_calls[0].arguments["command"], "pwd");
    assert_eq!(result.tool_calls[0].tool_call_id.as_deref(), Some("fallback_5_1"));
    assert!(result.display_text.contains("Before"));
    assert!(result.display_text.contains("After"));
    assert!(result.assistant_history_content.contains("fallback_5_1"));
    assert!(result.assistant_history_content.contains("reasoning-testcov-0088"));
    assert_eq!(provider.last_tools_len(), Some(1));
}

#[tokio::test]
async fn run_llm_round_detects_malformed_tool_call_parse_issue() {
    let provider = TestProvider::with_response(ChatResponse {
        text: Some("<tool_call>{\"name\":\"shell\"".to_string()),
        tool_calls: Vec::new(),
        usage: None,
        reasoning_content: None,
    });
    let observer = RecordingObserver::default();
    let history = vec![ChatMessage::user("use a tool")];

    let result = run_test_round(&provider, &observer, &history, false, &[], None, 0)
        .await
        .expect("malformed response should still produce a round result");

    assert!(result.tool_calls.is_empty());
    assert!(result.parse_issue_detected);
    assert_eq!(result.display_text, "<tool_call>{\"name\":\"shell\"");
}

#[tokio::test]
async fn run_llm_round_native_tool_calls_skip_text_fallback() {
    let native_call = ToolCall {
        id: "native-call-testcov-0088".to_string(),
        name: "shell".to_string(),
        arguments: "{\"command\":\"date\"}".to_string(),
    };
    let provider = TestProvider::with_response(ChatResponse {
        text: Some(
            "native wins <tool_call>{\"name\":\"ignored\",\"arguments\":{}}</tool_call>"
                .to_string(),
        ),
        tool_calls: vec![native_call],
        usage: None,
        reasoning_content: Some("native-reasoning-testcov-0088".to_string()),
    });
    let observer = RecordingObserver::default();
    let history = vec![ChatMessage::user("call native tool")];

    let result = run_test_round(&provider, &observer, &history, true, &[], None, 1)
        .await
        .expect("native tool call should succeed");

    assert_eq!(result.tool_calls.len(), 1);
    assert_eq!(result.tool_calls[0].name, "shell");
    assert_eq!(result.tool_calls[0].tool_call_id.as_deref(), Some("native-call-testcov-0088"));
    assert_eq!(result.native_tool_calls.len(), 1);
    assert_eq!(
        result.display_text,
        "native wins <tool_call>{\"name\":\"ignored\",\"arguments\":{}}</tool_call>"
    );
    assert!(result.assistant_history_content.contains("native-call-testcov-0088"));
    assert!(result.assistant_history_content.contains("native-reasoning-testcov-0088"));
}

#[tokio::test]
async fn run_llm_round_rejects_image_markers_for_non_vision_provider() {
    let provider = TestProvider::with_response(ChatResponse {
        text: Some("should not be called".to_string()),
        tool_calls: Vec::new(),
        usage: None,
        reasoning_content: None,
    });
    let observer = RecordingObserver::default();
    let history = vec![ChatMessage::user("inspect [IMAGE:data:image/png;base64,iVBORw0KGgo=]")];

    let error = match run_test_round(&provider, &observer, &history, false, &[], None, 0).await {
        Ok(_) => panic!("non-vision provider must reject images before chat"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("provider_capability_error"));
    assert!(error.to_string().contains("capability=vision"));
    assert_eq!(provider.calls(), 0);
    assert!(observer.events().is_empty());
}

#[tokio::test]
async fn run_llm_round_uses_ocr_before_non_vision_provider() {
    let provider = TestProvider::with_response(ChatResponse {
        text: Some("ocr accepted".to_string()),
        tool_calls: Vec::new(),
        usage: None,
        reasoning_content: None,
    });
    let extractor = StubImageTextExtractor::default();
    let observer = RecordingObserver::default();
    let history = vec![ChatMessage::user("inspect [IMAGE:data:image/png;base64,iVBORw0KGgo=]")];
    let mut config = crate::app::agent::config::MultimodalConfig::default();
    config.ocr.enabled = true;
    config.ocr.model = Some("vision/reader".to_string());

    let result = run_llm_round_with_extractor(
        &provider,
        &history,
        &observer,
        "testcov-provider",
        "testcov-model",
        0.2,
        "testcov-channel",
        &config,
        None,
        None,
        &[],
        false,
        "turn-testcov-0088",
        0,
        &extractor,
    )
    .await
    .expect("OCR should prepare text before the non-vision provider call");

    assert_eq!(result.response_text, "ocr accepted");
    assert_eq!(extractor.calls(), 1);
    assert_eq!(provider.calls(), 1);
    let messages = provider.last_messages();
    assert!(messages.iter().all(|message| !message.content.contains("[IMAGE:")));
    assert!(messages[0].content.contains("visible OCR text"));
}

#[tokio::test]
async fn run_llm_round_fires_llm_input_hook() {
    let provider = TestProvider::with_response(ChatResponse {
        text: Some("hooked response".to_string()),
        tool_calls: Vec::new(),
        usage: None,
        reasoning_content: None,
    });
    let observer = RecordingObserver::default();
    let history = vec![ChatMessage::user("hello hook")];
    let hook_calls = Arc::new(AtomicUsize::new(0));
    let hook_models = Arc::new(Mutex::new(Vec::new()));
    let mut hooks = HookRunner::new();
    hooks.register(Box::new(RecordingLlmInputHook {
        calls: Arc::clone(&hook_calls),
        models: Arc::clone(&hook_models),
    }));

    let result = run_llm_round(
        &provider,
        &history,
        &observer,
        "testcov-provider",
        "testcov-model",
        0.2,
        "testcov-channel",
        &crate::app::agent::config::MultimodalConfig::default(),
        None,
        Some(&hooks),
        &[],
        false,
        "turn-testcov-0088",
        0,
    )
    .await
    .expect("round with hook should succeed");

    assert_eq!(result.response_text, "hooked response");
    assert_eq!(hook_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        hook_models.lock().expect("models lock should not be poisoned").as_slice(),
        ["testcov-model"]
    );
}

#[tokio::test]
async fn run_llm_round_cancellation_wins_over_slow_provider() {
    let provider = TestProvider::with_delay(
        Duration::from_secs(10),
        ChatResponse {
            text: Some("late response".to_string()),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        },
    );
    let observer = RecordingObserver::default();
    let history = vec![ChatMessage::user("cancel me")];
    let token = CancellationToken::new();
    token.cancel();

    let error =
        match run_test_round(&provider, &observer, &history, false, &[], Some(&token), 0).await {
            Ok(_) => panic!("cancelled round should fail"),
            Err(error) => error,
        };

    assert!(error.is::<ToolLoopCancelled>());
    assert_eq!(provider.calls(), 0);
    let events = observer.events();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], ObserverEvent::LlmRequest { .. }));
}

#[tokio::test]
async fn run_llm_round_provider_error_records_failed_response() {
    let provider = TestProvider::with_error("upstream testcov-0088 failed");
    let observer = RecordingObserver::default();
    let history = vec![ChatMessage::user("fail please")];

    let error = match run_test_round(&provider, &observer, &history, false, &[], None, 0).await {
        Ok(_) => panic!("provider error should propagate"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("upstream testcov-0088 failed"));
    let events = observer.events();
    assert_eq!(events.len(), 2);
    assert!(matches!(
        &events[1],
        ObserverEvent::LlmResponse {
            success: false,
            error_message: Some(message),
            input_tokens: None,
            output_tokens: None,
            cached_tokens: None,
            reasoning_tokens: None,
            ..
        } if message.contains("upstream testcov-0088 failed")
    ));
}

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
        &crate::app::agent::config::MultimodalConfig::default(),
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
