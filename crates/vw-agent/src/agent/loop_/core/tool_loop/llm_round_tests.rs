//! 真实插件模型事件、取消与敏感信息边界验收。
use super::*;
use model_types::{ChatResponse, ProviderCapabilities, TokenUsage};
use agent_observability::traits::ObserverMetric;
use async_trait::async_trait;
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

#[tokio::test]
async fn codex_plugin_compat_t41_real_provider_failure_and_success() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("events");
    let context = crate::plugins::t41_tests::fixture(root.path(), state.path(), &marker);
    crate::tools::scope_tool_use_context(context.clone(), async {
        let observer = RecordingObserver::default();
        let history = vec![ChatMessage::user("private input must not enter hooks")];
        let failed = TestProvider::with_error("secret-provider-error");
        assert!(run_test_round(&failed, &observer, &history, false, &[], None, 0).await.is_err());
        let success = TestProvider::with_response(ChatResponse {
            text: Some("done".into()),
            tool_calls: Vec::new(),
            reasoning_content: None,
            usage: Some(TokenUsage {
                input_tokens: Some(3),
                output_tokens: Some(2),
                cached_tokens: Some(1),
                reasoning_tokens: Some(1),
            }),
        });
        assert!(run_test_round(&success, &observer, &history, false, &[], None, 1).await.is_ok());
        let delayed = TestProvider::with_delay(
            Duration::from_secs(60),
            ChatResponse {
                text: None,
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
        );
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        assert!(
            run_test_round(&delayed, &observer, &history, false, &[], Some(&cancellation), 2)
                .await
                .is_err()
        );
    })
    .await;
    let text = std::fs::read_to_string(marker).unwrap();
    let events: Vec<serde_json::Value> =
        text.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    assert_eq!(events.len(), 6);
    assert_eq!(events[1]["status"], "failed");
    assert_eq!(events[3]["status"], "completed");
    assert_eq!(events[3]["token_usage"]["total_tokens"], 5);
    assert_eq!(events[5]["status"], "cancelled");
    assert!(!text.contains("secret-provider-error"));
    assert!(!text.contains("private input"));
    assert!(context.take_hook_contexts().is_empty());
}
