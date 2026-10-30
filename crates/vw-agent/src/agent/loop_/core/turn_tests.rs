//! 测试代理轮次对审批上下文、回复目标和工具循环参数的传递。
//! 任务局部变量在有无回复目标和审批上下文时分别验证，防止跨轮泄漏。

use super::*;
use crate::providers::traits::ProviderCapabilities;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

struct RecordingProvider {
    responses: Mutex<VecDeque<crate::app::agent::providers::ChatResponse>>,
    calls: Arc<AtomicUsize>,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

#[derive(Debug, Clone, PartialEq)]
struct RecordedRequest {
    model: String,
    temperature: f64,
    message_roles: Vec<String>,
    tools: Vec<String>,
}

impl RecordingProvider {
    fn from_responses(
        responses: Vec<crate::app::agent::providers::ChatResponse>,
        calls: Arc<AtomicUsize>,
    ) -> Self {
        Self { responses: Mutex::new(responses.into()), calls, requests: Arc::default() }
    }

    fn from_text(response: &str, calls: Arc<AtomicUsize>) -> Self {
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

    fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().expect("requests mutex poisoned").clone()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for RecordingProvider {
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
        anyhow::bail!("turn tests use structured chat")
    }

    async fn chat(
        &self,
        request: crate::app::agent::providers::ChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<crate::app::agent::providers::ChatResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let tools = request.tools.unwrap_or_default().iter().map(|tool| tool.id.clone()).collect();
        let message_roles = request.messages.iter().map(|message| message.role.clone()).collect();
        self.requests.lock().expect("requests mutex poisoned").push(RecordedRequest {
            model: model.to_string(),
            temperature,
            message_roles,
            tools,
        });
        self.responses
            .lock()
            .expect("responses mutex poisoned")
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("recording provider response script exhausted"))
    }
}

struct ContextCaptureTool;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ContextCaptureTool {
    fn name(&self) -> &str {
        "testcov_0091_context_capture"
    }

    fn description(&self) -> &str {
        "Captures task-local turn context for tests"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }

    async fn execute(
        &self,
        _args: serde_json::Value,
    ) -> anyhow::Result<crate::app::agent::tools::ToolResult> {
        let reply_target = TOOL_LOOP_REPLY_TARGET
            .try_with(Clone::clone)
            .ok()
            .flatten()
            .unwrap_or_else(|| "unset".to_string());
        let approval_context = TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT
            .try_with(Clone::clone)
            .ok()
            .flatten()
            .map(|ctx| format!("{}:{}", ctx.sender, ctx.reply_target))
            .unwrap_or_else(|| "unset".to_string());

        Ok(crate::app::agent::tools::ToolResult {
            success: true,
            output: format!("reply={reply_target};approval={approval_context}"),
            error: None,
        })
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }
}

#[test]
fn reply_target_task_local_defaults_to_unset_outside_scope() {
    assert!(TOOL_LOOP_REPLY_TARGET.try_with(|target| target.clone()).is_err());
}

#[tokio::test]
async fn agent_turn_delegates_to_tool_loop_with_channel_defaults() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = RecordingProvider::from_text("plain response", Arc::clone(&calls));
    let mut history = vec![ChatMessage::user("hello")];
    let tools_registry: Vec<Box<dyn Tool>> = Vec::new();
    let observer = crate::app::agent::observability::NoopObserver;

    let result = agent_turn(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "recording",
        "turn-model",
        0.25,
        true,
        &crate::app::agent::config::MultimodalConfig::default(),
        1,
        None,
        None,
    )
    .await
    .expect("plain agent turn should return provider text");

    assert_eq!(result, "plain response");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        provider.requests(),
        vec![RecordedRequest {
            model: "turn-model".to_string(),
            temperature: 0.25,
            message_roles: vec!["user".to_string()],
            tools: Vec::new(),
        }]
    );
    assert_eq!(history.last().map(|message| message.role.as_str()), Some("assistant"));
    assert_eq!(history.last().map(|message| message.content.as_str()), Some("plain response"));
}

#[tokio::test]
async fn run_tool_call_loop_with_non_cli_approval_context_scopes_approval_and_reply_target() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = RecordingProvider::from_responses(
        vec![
            crate::app::agent::providers::ChatResponse {
                text: Some("capture approval context".to_string()),
                tool_calls: vec![crate::app::agent::providers::ToolCall {
                    id: "call-1".to_string(),
                    name: "testcov_0091_context_capture".to_string(),
                    arguments: "{}".to_string(),
                }],
                usage: None,
                reasoning_content: None,
            },
            crate::app::agent::providers::ChatResponse {
                text: Some("done".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
        ],
        Arc::clone(&calls),
    );
    let mut history = vec![ChatMessage::user("hello")];
    let tools_registry: Vec<Box<dyn Tool>> = vec![Box::new(ContextCaptureTool)];
    let observer = crate::app::agent::observability::NoopObserver;
    let (prompt_tx, _prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let approval_context = NonCliApprovalContext {
        sender: "alice".to_string(),
        reply_target: "approval-room".to_string(),
        prompt_tx,
    };

    let result = run_tool_call_loop_with_non_cli_approval_context(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "recording",
        "turn-model",
        0.0,
        true,
        None,
        "telegram",
        Some(approval_context),
        &crate::app::agent::config::MultimodalConfig::default(),
        3,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect("non-cli approval scoped tool loop should finish");

    assert_eq!(result, "done");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(
        history.iter().any(|message| {
            message.role == "tool"
                && message.content.contains("reply=approval-room;approval=alice:approval-room")
        }),
        "tool result should include scoped approval and derived reply target"
    );
    assert!(TOOL_LOOP_REPLY_TARGET.try_with(|target| target.clone()).is_err());
    assert!(TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT.try_with(|ctx| ctx.clone()).is_err());
}

#[tokio::test]
async fn run_tool_call_loop_with_non_cli_approval_context_scopes_none_when_context_is_absent() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = RecordingProvider::from_responses(
        vec![
            crate::app::agent::providers::ChatResponse {
                text: Some("capture absent approval context".to_string()),
                tool_calls: vec![crate::app::agent::providers::ToolCall {
                    id: "call-1".to_string(),
                    name: "testcov_0091_context_capture".to_string(),
                    arguments: "{}".to_string(),
                }],
                usage: None,
                reasoning_content: None,
            },
            crate::app::agent::providers::ChatResponse {
                text: Some("done".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
        ],
        Arc::clone(&calls),
    );
    let mut history = vec![ChatMessage::user("hello")];
    let tools_registry: Vec<Box<dyn Tool>> = vec![Box::new(ContextCaptureTool)];
    let observer = crate::app::agent::observability::NoopObserver;

    let result = run_tool_call_loop_with_non_cli_approval_context(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "recording",
        "turn-model",
        0.0,
        true,
        None,
        "telegram",
        None,
        &crate::app::agent::config::MultimodalConfig::default(),
        3,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect("missing non-cli approval context should still scope empty context");

    assert_eq!(result, "done");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(
        history.iter().any(|message| {
            message.role == "tool" && message.content.contains("reply=unset;approval=unset")
        }),
        "tool result should show that no approval context was scoped"
    );
    assert!(TOOL_LOOP_REPLY_TARGET.try_with(|target| target.clone()).is_err());
    assert!(TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT.try_with(|ctx| ctx.clone()).is_err());
}
