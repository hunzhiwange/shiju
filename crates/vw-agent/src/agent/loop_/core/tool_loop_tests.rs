//! 测试工具调用循环的终止、重试、迭代耗尽与工具过滤策略。
//! 队列式供应商精确控制每轮响应，验证重试次数和迭代耗尽的停止位置。

use super::*;
use crate::providers::traits::ProviderCapabilities;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio_util::sync::CancellationToken;

struct StaticProvider {
    responses: Mutex<VecDeque<crate::app::agent::providers::ChatResponse>>,
    calls: Arc<AtomicUsize>,
    requested_tools: Arc<Mutex<Vec<Vec<String>>>>,
}

impl StaticProvider {
    fn from_text(response: &'static str, calls: Arc<AtomicUsize>) -> Self {
        Self::from_responses(
            vec![crate::app::agent::providers::ChatResponse {
                text: Some(response.to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            }],
            calls,
        )
    }

    fn from_responses(
        responses: Vec<crate::app::agent::providers::ChatResponse>,
        calls: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            calls,
            requested_tools: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn requested_tools(&self) -> Vec<Vec<String>> {
        self.requested_tools.lock().expect("requested tools mutex poisoned").clone()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for StaticProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities { native_tool_calling: true, vision: false }
    }

    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        anyhow::bail!("tool loop tests should use structured chat")
    }

    async fn chat(
        &self,
        request: crate::app::agent::providers::ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<crate::app::agent::providers::ChatResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let tool_ids =
            request.tools.unwrap_or_default().iter().map(|spec| spec.id.clone()).collect();
        self.requested_tools.lock().expect("requested tools mutex poisoned").push(tool_ids);
        self.responses
            .lock()
            .expect("responses mutex poisoned")
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("static provider response script exhausted"))
    }
}

struct EchoTool {
    name: &'static str,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "Echoes the provided value"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            }
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> anyhow::Result<crate::app::agent::tools::ToolResult> {
        Ok(crate::app::agent::tools::ToolResult {
            success: true,
            output: args
                .get("value")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            error: None,
        })
    }
}

#[test]
fn completion_detection_is_reexported_for_tool_loop() {
    assert!(looks_like_unverified_action_completion_without_tool_call(
        "I have created the file successfully"
    ));
}

#[tokio::test]
async fn run_tool_call_loop_returns_final_text_and_updates_history_when_max_iterations_is_zero() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = StaticProvider::from_text("final answer", Arc::clone(&calls));
    let mut history = vec![ChatMessage::user("hello")];
    let observer = crate::app::agent::observability::NoopObserver;
    let tools_registry: Vec<Box<dyn Tool>> = Vec::new();

    let result = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "static",
        "model",
        0.0,
        true,
        None,
        "cli",
        &crate::app::agent::config::MultimodalConfig::default(),
        0,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect("zero max iterations should fall back to the default limit");

    assert_eq!(result, "final answer");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(history.last().map(|msg| msg.role.as_str()), Some("assistant"));
    assert_eq!(history.last().map(|msg| msg.content.as_str()), Some("final answer"));
}

#[tokio::test]
async fn run_tool_call_loop_exits_before_provider_call_when_cancelled() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = StaticProvider::from_text("should not be used", Arc::clone(&calls));
    let mut history = vec![ChatMessage::user("hello")];
    let observer = crate::app::agent::observability::NoopObserver;
    let tools_registry: Vec<Box<dyn Tool>> = Vec::new();
    let token = CancellationToken::new();
    token.cancel();

    let err = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "static",
        "model",
        0.0,
        false,
        None,
        "cli",
        &crate::app::agent::config::MultimodalConfig::default(),
        1,
        Some(token),
        None,
        None,
        None,
        &[],
    )
    .await
    .expect_err("cancelled token should abort before requesting the provider");

    assert!(err.is::<ToolLoopCancelled>());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(history.len(), 1);
}

#[tokio::test]
async fn run_tool_call_loop_reports_exhausted_after_last_tool_iteration() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = StaticProvider::from_responses(
        vec![crate::app::agent::providers::ChatResponse {
            text: Some("calling tool".to_string()),
            tool_calls: vec![crate::app::agent::providers::ToolCall {
                id: "call-1".to_string(),
                name: "echo_tool".to_string(),
                arguments: r#"{"value":"once"}"#.to_string(),
            }],
            usage: None,
            reasoning_content: None,
        }],
        Arc::clone(&calls),
    );
    let mut history = vec![ChatMessage::user("hello")];
    let observer = crate::app::agent::observability::NoopObserver;
    let tools_registry: Vec<Box<dyn Tool>> = vec![Box::new(EchoTool { name: "echo_tool" })];

    let err = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "static",
        "model",
        0.0,
        true,
        None,
        "cli",
        &crate::app::agent::config::MultimodalConfig::default(),
        1,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect_err("loop should fail when the only allowed iteration is spent on a tool call");

    let err_text = err.to_string();
    assert!(err_text.contains("Agent exceeded maximum tool iterations"), "{err_text}");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(
        history.iter().any(|message| message.role == "tool" && message.content.contains("once")),
        "tool result should be written before the exhaustion error"
    );
}

#[tokio::test]
async fn run_tool_call_loop_retries_empty_final_response_until_text_arrives() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = StaticProvider::from_responses(
        vec![
            crate::app::agent::providers::ChatResponse {
                text: Some(" \n\t".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
            crate::app::agent::providers::ChatResponse {
                text: Some("verified result".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
        ],
        Arc::clone(&calls),
    );
    let mut history = vec![ChatMessage::user("finish with a result")];
    let observer = crate::app::agent::observability::NoopObserver;
    let tools_registry: Vec<Box<dyn Tool>> = Vec::new();

    let result = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "static",
        "model",
        0.0,
        true,
        None,
        "cli",
        &crate::app::agent::config::MultimodalConfig::default(),
        2,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect("empty response should be retried");

    assert_eq!(result, "verified result");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(history.iter().any(|message| {
        message.role == "user" && message.content == EMPTY_FINAL_RESPONSE_RETRY_PROMPT
    }));
}

#[tokio::test]
async fn run_tool_call_loop_reports_exhausted_when_all_final_responses_are_empty() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = StaticProvider::from_responses(
        vec![
            crate::app::agent::providers::ChatResponse {
                text: None,
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
            crate::app::agent::providers::ChatResponse {
                text: Some("\n".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
        ],
        Arc::clone(&calls),
    );
    let mut history = vec![ChatMessage::user("finish with a result")];
    let observer = crate::app::agent::observability::NoopObserver;
    let tools_registry: Vec<Box<dyn Tool>> = Vec::new();

    let error = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "static",
        "model",
        0.0,
        true,
        None,
        "cli",
        &crate::app::agent::config::MultimodalConfig::default(),
        2,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect_err("empty responses must not be accepted as a result");

    assert!(error.to_string().contains("non-empty final response"));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn run_tool_call_loop_omits_excluded_tools_from_provider_request() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = StaticProvider::from_text("final answer", Arc::clone(&calls));
    let mut history = vec![ChatMessage::user("hello")];
    let observer = crate::app::agent::observability::NoopObserver;
    let tools_registry: Vec<Box<dyn Tool>> = vec![
        Box::new(EchoTool { name: "available_tool" }),
        Box::new(EchoTool { name: "blocked_tool" }),
    ];

    let result = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "static",
        "model",
        0.0,
        true,
        None,
        "cli",
        &crate::app::agent::config::MultimodalConfig::default(),
        1,
        None,
        None,
        None,
        None,
        &["blocked_tool".to_string()],
    )
    .await
    .expect("excluded tool filtering should still allow a final response");

    assert_eq!(result, "final answer");
    assert_eq!(provider.requested_tools(), vec![vec!["available_tool".to_string()]]);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn run_tool_call_loop_retries_once_when_completion_claim_lacks_tool_call() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = StaticProvider::from_responses(
        vec![
            crate::app::agent::providers::ChatResponse {
                text: Some(
                    "Done. I have created the file in the current working directory.".to_string(),
                ),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
            crate::app::agent::providers::ChatResponse {
                text: Some("final answer after retry".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
        ],
        Arc::clone(&calls),
    );
    let mut history = vec![ChatMessage::user("create the file")];
    let observer = crate::app::agent::observability::NoopObserver;
    let tools_registry: Vec<Box<dyn Tool>> = vec![Box::new(EchoTool { name: "echo_tool" })];
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);

    let result = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "static",
        "model",
        0.0,
        true,
        None,
        "cli",
        &crate::app::agent::config::MultimodalConfig::default(),
        2,
        None,
        Some(tx),
        None,
        None,
        &[],
    )
    .await
    .expect("completion claim without a tool call should get one retry");

    assert_eq!(result, "final answer after retry");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(
        history.iter().any(|message| {
            message.role == "user" && message.content.contains("no valid tool call was emitted")
        }),
        "retry prompt should be added to history before the second provider call"
    );
    assert!(
        rx.try_recv().is_ok_and(|message| message.contains("思考中")),
        "thinking progress should be sent before the retry"
    );
    assert!(
        rx.try_recv().is_ok_and(|message| message.contains("Retrying")),
        "retry progress should be sent when the first response lacks a tool call"
    );
}

#[tokio::test]
async fn run_tool_call_loop_retries_once_when_tool_payload_cannot_be_parsed() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = StaticProvider::from_responses(
        vec![
            crate::app::agent::providers::ChatResponse {
                text: Some("<tool_call>{\"name\":\"echo_tool\"".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
            crate::app::agent::providers::ChatResponse {
                text: Some("final answer after parse retry".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
        ],
        Arc::clone(&calls),
    );
    let mut history = vec![ChatMessage::user("use the echo tool")];
    let observer = crate::app::agent::observability::NoopObserver;
    let tools_registry: Vec<Box<dyn Tool>> = vec![Box::new(EchoTool { name: "echo_tool" })];

    let result = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "static",
        "model",
        0.0,
        true,
        None,
        "cli",
        &crate::app::agent::config::MultimodalConfig::default(),
        2,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect("an unparseable tool payload should get one retry");

    assert_eq!(result, "final answer after parse retry");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(history.iter().any(|message| {
        message.role == "user" && message.content.contains("no valid tool call was emitted")
    }));
}

#[tokio::test]
async fn run_tool_call_loop_errors_when_completion_claim_repeats_without_tool_call() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = StaticProvider::from_responses(
        vec![
            crate::app::agent::providers::ChatResponse {
                text: Some("Done. I have created the file in the workspace.".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
            crate::app::agent::providers::ChatResponse {
                text: Some(
                    "Finished successfully. I wrote the file to the workspace path.".to_string(),
                ),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
        ],
        Arc::clone(&calls),
    );
    let mut history = vec![ChatMessage::user("create the file")];
    let observer = crate::app::agent::observability::NoopObserver;
    let tools_registry: Vec<Box<dyn Tool>> = vec![Box::new(EchoTool { name: "echo_tool" })];

    let err = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "static",
        "model",
        0.0,
        true,
        None,
        "cli",
        &crate::app::agent::config::MultimodalConfig::default(),
        2,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect_err("repeated completion claims without tool calls should fail");

    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(err.to_string().contains("deferred action without emitting a tool call"));
}
