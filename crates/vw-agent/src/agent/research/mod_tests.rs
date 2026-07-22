use super::*;
use crate::app::agent::observability::{NoopObserver, Observer};
use crate::app::agent::providers::traits::{ProviderCapabilities, TokenUsage};
use crate::app::agent::providers::{ChatMessage, ChatRequest, ChatResponse, Provider, ToolCall};
use crate::app::agent::tools::{Tool, ToolResult};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
struct CapturedRequest {
    messages: Vec<ChatMessage>,
    tools_len: Option<usize>,
    model: String,
    temperature: f64,
}

struct ScriptedProvider {
    native_tools: bool,
    responses: Mutex<VecDeque<ChatResponse>>,
    requests: Mutex<Vec<CapturedRequest>>,
}

impl ScriptedProvider {
    fn new(native_tools: bool, responses: Vec<ChatResponse>) -> Self {
        Self {
            native_tools,
            responses: Mutex::new(VecDeque::from(responses)),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn requests(&self) -> Vec<CapturedRequest> {
        self.requests.lock().unwrap().clone()
    }
}

struct FailingProvider;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for ScriptedProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities { native_tool_calling: self.native_tools, vision: false }
    }

    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("fallback".into())
    }

    async fn chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        self.requests.lock().unwrap().push(CapturedRequest {
            messages: request.messages.to_vec(),
            tools_len: request.tools.map(|tools| tools.len()),
            model: model.to_string(),
            temperature,
        });
        Ok(self.responses.lock().unwrap().pop_front().unwrap_or_else(|| ChatResponse {
            text: Some("[RESEARCH COMPLETE]\n- fallback".into()),
            tool_calls: Vec::new(),
            usage: Some(TokenUsage::default()),
            reasoning_content: None,
        }))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for FailingProvider {
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
        Ok("unused".into())
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        anyhow::bail!("provider chat failed")
    }
}

struct EchoTool {
    name: &'static str,
    fail: bool,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "Echo a text field"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {"text": {"type": "string"}}
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        if self.fail {
            anyhow::bail!("tool failed");
        }
        Ok(ToolResult {
            success: true,
            output: args
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("<missing>")
                .to_string(),
            error: None,
        })
    }
}

fn response(text: Option<&str>, tool_calls: Vec<ToolCall>) -> ChatResponse {
    ChatResponse {
        text: text.map(str::to_string),
        tool_calls,
        usage: None,
        reasoning_content: None,
    }
}

fn call(name: &str, arguments: &str) -> ToolCall {
    ToolCall {
        id: format!("call-{name}"),
        name: name.to_string(),
        arguments: arguments.to_string(),
    }
}

fn config(max_iterations: usize) -> ResearchPhaseConfig {
    ResearchPhaseConfig {
        enabled: true,
        trigger: ResearchTrigger::Always,
        max_iterations,
        show_progress: false,
        ..ResearchPhaseConfig::default()
    }
}

fn observer() -> Arc<dyn Observer> {
    Arc::new(NoopObserver)
}

#[test]
fn trigger_edges_cover_keyword_length_and_question_boundaries() {
    let mut cfg = ResearchPhaseConfig {
        enabled: true,
        trigger: ResearchTrigger::Keywords,
        keywords: vec!["Find".into()],
        ..ResearchPhaseConfig::default()
    };
    assert!(should_trigger(&cfg, "please find this"));
    assert!(!should_trigger(&cfg, "please locate this"));

    cfg.trigger = ResearchTrigger::Length;
    cfg.min_message_length = 5;
    assert!(should_trigger(&cfg, "12345"));
    assert!(!should_trigger(&cfg, "1234"));

    cfg.trigger = ResearchTrigger::Question;
    assert!(should_trigger(&cfg, "mid? sentence"));
    assert!(!should_trigger(&cfg, "fullwidth question？"));
}

#[test]
fn truncate_handles_short_exact_and_tiny_limits() {
    assert_eq!(truncate("hello", 10), "hello");
    assert_eq!(truncate("hello", 5), "hello");
    assert_eq!(truncate("hello world", 8), "hello...");
    assert_eq!(truncate("abcdef", 2), "...");
}

#[tokio::test]
async fn native_research_completes_immediately_from_marker() {
    let provider = ScriptedProvider::new(
        true,
        vec![response(Some("draft\n[RESEARCH COMPLETE]\n- fact"), Vec::new())],
    );
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(EchoTool { name: "echo", fail: false })];

    let result =
        run_research_phase(&config(3), &provider, &tools, "question", "model-a", 0.2, observer())
            .await
            .unwrap();

    assert_eq!(result.context, "[RESEARCH COMPLETE]\n- fact");
    assert_eq!(result.tool_call_count, 0);

    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].tools_len, Some(1));
    assert_eq!(requests[0].model, "model-a");
    assert_eq!(requests[0].temperature, 0.2);
    assert!(requests[0].messages[0].content.contains("RESEARCH MODE"));
}

#[tokio::test]
async fn native_research_executes_tool_then_uses_followup_summary() {
    let provider = ScriptedProvider::new(
        true,
        vec![
            response(None, vec![call("echo", r#"{"text":"from tool"}"#)]),
            response(Some("[RESEARCH COMPLETE]\n- tool said from tool"), Vec::new()),
        ],
    );
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(EchoTool { name: "echo", fail: false })];

    let result =
        run_research_phase(&config(3), &provider, &tools, "inspect", "model-b", 0.0, observer())
            .await
            .unwrap();

    assert_eq!(result.context, "[RESEARCH COMPLETE]\n- tool said from tool");
    assert_eq!(result.tool_call_count, 1);
    assert_eq!(result.tool_summaries[0].tool_name, "echo");
    assert_eq!(result.tool_summaries[0].arguments_preview, r#"{"text":"from tool"}"#);
    assert_eq!(result.tool_summaries[0].result_preview, "from tool");
    assert!(result.tool_summaries[0].success);
    assert_eq!(provider.requests().len(), 2);
}

#[tokio::test]
async fn native_research_records_unknown_tool_and_accepts_plain_text_completion() {
    let provider = ScriptedProvider::new(
        true,
        vec![
            response(None, vec![call("missing", "{}")]),
            response(Some("plain completion"), Vec::new()),
        ],
    );
    let tools: Vec<Box<dyn Tool>> = Vec::new();

    let result =
        run_research_phase(&config(3), &provider, &tools, "inspect", "model-c", 0.0, observer())
            .await
            .unwrap();

    assert_eq!(result.context, "plain completion");
    assert_eq!(result.tool_call_count, 1);
    assert_eq!(result.tool_summaries[0].tool_name, "missing");
    assert!(!result.tool_summaries[0].success);
    assert!(result.tool_summaries[0].result_preview.contains("Unknown tool: missing"));
}

#[tokio::test]
async fn native_research_with_progress_records_followup_messages() {
    let provider = ScriptedProvider::new(
        true,
        vec![
            response(None, vec![call("echo", r#"{"text":"progress"}"#)]),
            response(Some("done without marker"), Vec::new()),
        ],
    );
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(EchoTool { name: "echo", fail: false })];
    let mut cfg = config(3);
    cfg.show_progress = true;

    let result =
        run_research_phase(&cfg, &provider, &tools, "inspect", "model-progress", 0.3, observer())
            .await
            .unwrap();

    assert_eq!(result.context, "done without marker");
    assert_eq!(result.tool_call_count, 1);

    let requests = provider.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].messages.len(), 4);
    assert_eq!(requests[1].messages[2].role, "assistant");
    assert!(requests[1].messages[2].content.contains("Called tool `echo`"));
    assert_eq!(requests[1].messages[3].role, "user");
    assert!(requests[1].messages[3].content.contains("Tool result:\nprogress"));
}

#[tokio::test]
async fn prompt_guided_research_injects_tool_instructions_and_parses_xml_call() {
    let provider = ScriptedProvider::new(
        false,
        vec![
            response(
                Some(r#"<tool_call>{"name":"echo","arguments":{"text":"xml result"}}</tool_call>"#),
                Vec::new(),
            ),
            response(Some("[RESEARCH COMPLETE]\n- xml result"), Vec::new()),
        ],
    );
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(EchoTool { name: "echo", fail: false })];
    let mut cfg = config(3);
    cfg.system_prompt_prefix = "Custom research prefix".into();

    let result = run_research_phase(&cfg, &provider, &tools, "inspect", "model-d", 0.0, observer())
        .await
        .unwrap();

    assert_eq!(result.context, "[RESEARCH COMPLETE]\n- xml result");
    assert_eq!(result.tool_call_count, 1);
    assert_eq!(result.tool_summaries[0].result_preview, "xml result");

    let requests = provider.requests();
    assert_eq!(requests[0].tools_len, None);
    assert!(requests[0].messages[0].content.contains("Custom research prefix"));
    assert!(requests[0].messages[0].content.contains("## Tool Use Protocol"));
    assert!(requests[0].messages[0].content.contains("**echo**"));
}

#[tokio::test]
async fn response_without_text_or_tool_calls_leaves_empty_context() {
    let provider = ScriptedProvider::new(true, vec![response(None, Vec::new())]);
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(EchoTool { name: "echo", fail: false })];

    let result = run_research_phase(
        &config(2),
        &provider,
        &tools,
        "inspect",
        "model-empty",
        0.0,
        observer(),
    )
    .await
    .unwrap();

    assert_eq!(result.context, "");
    assert_eq!(result.tool_call_count, 0);
    assert_eq!(provider.requests().len(), 1);
}

#[tokio::test]
async fn tool_execution_errors_are_captured_as_failed_summaries() {
    let provider = ScriptedProvider::new(
        true,
        vec![
            response(None, vec![call("echo", r#"{"text":"boom"}"#)]),
            response(Some("[RESEARCH COMPLETE]\n- saw failure"), Vec::new()),
        ],
    );
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(EchoTool { name: "echo", fail: true })];

    let result =
        run_research_phase(&config(2), &provider, &tools, "inspect", "model-e", 0.0, observer())
            .await
            .unwrap();

    assert_eq!(result.tool_call_count, 1);
    assert!(!result.tool_summaries[0].success);
    assert!(result.tool_summaries[0].result_preview.contains("Error: tool failed"));
}

#[tokio::test]
async fn provider_chat_errors_are_returned_to_caller() {
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(EchoTool { name: "echo", fail: false })];
    let err = run_research_phase(
        &config(1),
        &FailingProvider,
        &tools,
        "inspect",
        "model-error",
        0.0,
        observer(),
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("provider chat failed"));
}

#[tokio::test]
async fn invalid_tool_arguments_fall_back_to_empty_object() {
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(EchoTool { name: "echo", fail: false })];
    let result = execute_tool_call(&tools, &call("echo", "not-json")).await;

    assert!(result.success);
    assert_eq!(result.output, "<missing>");
}

#[test]
fn truncate_zero_limit_returns_ellipsis() {
    assert_eq!(truncate("abcdef", 0), "...");
}

#[tokio::test]
async fn max_iterations_can_end_with_tool_summaries_but_no_context() {
    let provider =
        ScriptedProvider::new(true, vec![response(None, vec![call("echo", r#"{"text":"last"}"#)])]);
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(EchoTool { name: "echo", fail: false })];

    let result =
        run_research_phase(&config(1), &provider, &tools, "inspect", "model-f", 0.0, observer())
            .await
            .unwrap();

    assert_eq!(result.context, "");
    assert_eq!(result.tool_call_count, 1);
    assert_eq!(result.tool_summaries[0].result_preview, "last");
}
