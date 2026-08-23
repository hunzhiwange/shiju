//! 测试工具执行的审批、安全上下文、并发约束与 MCP 路由。

use super::*;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::tools::{self as agent_tools, ToolResult};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

const MCP_TOOLSET_TOOL_ID: &str = "mcp__fixture_server__echo_tool";

struct Testcov0090Tool {
    name: &'static str,
    calls: Arc<AtomicUsize>,
    success: bool,
    read_only: bool,
}

struct Testcov0090CronTool {
    calls: Arc<AtomicUsize>,
    received_args: Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
}

#[async_trait]
impl Tool for Testcov0090Tool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "echoes test input"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let value = args["value"].as_str().unwrap_or("missing");
        Ok(ToolResult {
            success: self.success,
            output: format!("echo:{value}"),
            error: (!self.success).then(|| format!("failed:{value}")),
        })
    }

    fn is_read_only(&self) -> bool {
        self.read_only
    }

    fn is_concurrency_safe(&self) -> bool {
        self.read_only
    }
}

#[async_trait]
impl Tool for Testcov0090CronTool {
    fn name(&self) -> &str {
        "cron_add"
    }

    fn description(&self) -> &str {
        "records cron arguments"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object" })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.received_args.lock().unwrap().push(args);
        Ok(ToolResult { success: true, output: "cron-ok".to_string(), error: None })
    }
}

fn parsed_call(
    name: &str,
    arguments: serde_json::Value,
    tool_call_id: Option<&str>,
) -> ParsedToolCall {
    ParsedToolCall {
        name: name.to_string(),
        arguments,
        tool_call_id: tool_call_id.map(str::to_string),
    }
}

fn test_context() -> Arc<ToolUseContext> {
    Arc::new(ToolUseContext::default())
}

fn test_tool(calls: &Arc<AtomicUsize>) -> Box<dyn Tool> {
    named_test_tool("testcov_0090_echo", calls, true, false)
}

fn named_test_tool(
    name: &'static str,
    calls: &Arc<AtomicUsize>,
    success: bool,
    read_only: bool,
) -> Box<dyn Tool> {
    Box::new(Testcov0090Tool { name, calls: Arc::clone(calls), success, read_only })
}

fn assert_tool_result_message(message: &ChatMessage, expected: &str) {
    assert_eq!(message.role, "user");
    assert!(message.content.contains(expected), "{}", message.content);
}

fn write_mcp_toolset_server(tmp: &TempDir) -> PathBuf {
    let script = tmp.path().join("fixture_mcp_server.py");
    std::fs::write(
        &script,
        r#"
import json
import sys

TOOL = {
    "name": "Echo Tool",
    "description": "Echo a message from the MCP fixture.",
    "inputSchema": {
        "type": "object",
        "additionalProperties": False,
        "properties": {
            "message": {"type": "string"}
        },
        "required": ["message"]
    },
    "annotations": {
        "readOnlyHint": True,
        "destructiveHint": False,
        "idempotentHint": True
    }
}

def send(payload):
    sys.stdout.write(json.dumps(payload, separators=(",", ":")) + "\n")
    sys.stdout.flush()

for line in sys.stdin:
    if not line.strip():
        continue
    message = json.loads(line)
    method = message.get("method")
    request_id = message.get("id")
    if method == "initialize":
        params = message.get("params") or {}
        send({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "protocolVersion": params.get("protocolVersion", "2025-06-18"),
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": "fixture-mcp", "version": "1.0.0"},
                "instructions": "Fixture instructions for dynamic MCP tool tests."
            }
        })
    elif method == "tools/list":
        send({"jsonrpc": "2.0", "id": request_id, "result": {"tools": [TOOL]}})
    elif method == "tools/call":
        params = message.get("params") or {}
        arguments = params.get("arguments") or {}
        text = "echo:" + str(arguments.get("message", ""))
        send({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "content": [{"type": "text", "text": text}],
                "structuredContent": {"echoed": arguments.get("message", "")},
                "isError": False
            }
        })
    elif request_id is not None:
        send({"jsonrpc": "2.0", "id": request_id, "result": {}})
"#,
    )
    .expect("fixture MCP server script should be written");
    script
}

fn write_mcp_toolset_workspace(tmp: &TempDir) -> PathBuf {
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace should be created");
    let script = write_mcp_toolset_server(tmp);
    let config = serde_json::json!({
        "mcpServers": [{
            "name": "Fixture Server",
            "command": std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string()),
            "args": [script.to_string_lossy()]
        }]
    });
    std::fs::write(
        workspace.join(".vwacprc.json"),
        serde_json::to_vec_pretty(&config).expect("MCP config should serialize"),
    )
    .expect("MCP config should be written");
    workspace
}

#[test]
fn immediate_failure_preserves_tool_identity_and_marks_failure() {
    let (tool_name, call_id, outcome) =
        immediate_failure("shell".to_string(), Some("call-1".to_string()), "blocked".to_string());

    assert_eq!(tool_name, "shell");
    assert_eq!(call_id.as_deref(), Some("call-1"));
    assert_eq!(outcome.tool_name, "shell");
    assert_eq!(outcome.error_reason.as_deref(), Some("blocked"));
    assert!(!outcome.success);
}

#[test]
fn history_tool_call_id_uses_native_fallback_only_when_needed() {
    assert_eq!(
        history_tool_call_id(Some("call-testcov-0090"), true, 2, 3).as_deref(),
        Some("call-testcov-0090")
    );
    assert_eq!(history_tool_call_id(None, true, 2, 3).as_deref(), Some("fallback_2_3"));
    assert!(history_tool_call_id(None, false, 2, 3).is_none());
}

#[tokio::test]
async fn structured_progress_skips_when_result_dto_is_absent() {
    let (_, _, outcome) =
        immediate_failure("testcov_0090_echo".to_string(), None, "blocked".to_string());
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    send_structured_tool_result_progress(&tx, &outcome, 0).await;

    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn excluded_tool_writes_blocked_result_without_execution() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![test_tool(&calls)];
    let observer = NoopObserver;
    let context = test_context();
    let mut history = vec![ChatMessage::user("run blocked")];
    let tool_calls =
        vec![parsed_call("testcov_0090_echo", serde_json::json!({ "value": "blocked" }), None)];
    let mut seen = HashSet::new();

    execute_tool_calls_and_update_history(
        &mut history,
        &tool_calls,
        "assistant requested blocked tool".to_string(),
        &[],
        &tools,
        &observer,
        "test-channel",
        None,
        "test-provider",
        "test-model",
        "turn-testcov-0090",
        0,
        &context,
        None,
        &["testcov_0090_echo".to_string()],
        false,
        None,
        &mut seen,
        false,
    )
    .await
    .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(history.len(), 3);
    assert_eq!(history[1].role, "assistant");
    assert_tool_result_message(
        &history[2],
        "Tool 'testcov_0090_echo' is not available in this channel.",
    );
}

#[tokio::test]
async fn mcp_toolset_excluded_tool_is_blocked_by_channel_policy() {
    let tmp = TempDir::new().unwrap();
    let workspace = write_mcp_toolset_workspace(&tmp);
    let context = agent_tools::ToolRuntimeContext::new(
        "mcp-toolset-test",
        Some(workspace.to_string_lossy().to_string()),
    );
    let specs = agent_tools::tool_specs_for_context(&context);
    assert!(
        specs.iter().any(|spec| spec.id == MCP_TOOLSET_TOOL_ID),
        "configured MCP tool should be discoverable before channel policy excludes it"
    );

    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![named_test_tool(MCP_TOOLSET_TOOL_ID, &calls, true, true)];
    let observer = NoopObserver;
    let context = test_context();
    let mut history = vec![ChatMessage::user("run blocked MCP tool")];
    let tool_calls = vec![parsed_call(
        MCP_TOOLSET_TOOL_ID,
        serde_json::json!({ "message": "blocked" }),
        Some("call-mcp-blocked"),
    )];
    let mut seen = HashSet::new();

    execute_tool_calls_and_update_history(
        &mut history,
        &tool_calls,
        "assistant requested blocked MCP tool".to_string(),
        &[],
        &tools,
        &observer,
        "test-channel",
        None,
        "test-provider",
        "test-model",
        "turn-testcov-0090-mcp",
        0,
        &context,
        None,
        &[MCP_TOOLSET_TOOL_ID.to_string()],
        false,
        None,
        &mut seen,
        true,
    )
    .await
    .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(history.len(), 3);
    assert_eq!(history[1].role, "assistant");
    assert_eq!(history[2].role, "tool");
    assert!(
        history[2]
            .content
            .contains("Tool 'mcp__fixture_server__echo_tool' is not available in this channel."),
        "{}",
        history[2].content,
    );
}

#[tokio::test]
async fn duplicate_text_tool_call_is_reported_without_second_execution() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![test_tool(&calls)];
    let observer = NoopObserver;
    let context = test_context();
    let mut history = vec![ChatMessage::user("run twice")];
    let tool_calls = vec![
        parsed_call("testcov_0090_echo", serde_json::json!({ "value": "same" }), None),
        parsed_call("testcov_0090_echo", serde_json::json!({ "value": "same" }), None),
    ];
    let mut seen = HashSet::new();

    execute_tool_calls_and_update_history(
        &mut history,
        &tool_calls,
        "assistant requested duplicates".to_string(),
        &[],
        &tools,
        &observer,
        "test-channel",
        None,
        "test-provider",
        "test-model",
        "turn-testcov-0090",
        1,
        &context,
        None,
        &[],
        false,
        None,
        &mut seen,
        false,
    )
    .await
    .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(history.len(), 3);
    assert_tool_result_message(&history[2], "echo:same");
    assert_tool_result_message(
        &history[2],
        "Skipped duplicate tool call 'testcov_0090_echo' with identical arguments in this turn.",
    );
}

#[tokio::test]
async fn native_tool_ids_keep_same_arguments_distinct_and_write_tool_messages() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![test_tool(&calls)];
    let observer = NoopObserver;
    let context = test_context();
    let mut history = vec![ChatMessage::user("native calls")];
    let tool_calls = vec![
        parsed_call("testcov_0090_echo", serde_json::json!({ "value": "native" }), Some("call-a")),
        parsed_call("testcov_0090_echo", serde_json::json!({ "value": "native" }), Some("call-b")),
    ];
    let native_tool_calls = vec![
        ToolCall {
            id: "call-a".to_string(),
            name: "testcov_0090_echo".to_string(),
            arguments: r#"{"value":"native"}"#.to_string(),
        },
        ToolCall {
            id: "call-b".to_string(),
            name: "testcov_0090_echo".to_string(),
            arguments: r#"{"value":"native"}"#.to_string(),
        },
    ];
    let mut seen = HashSet::new();

    execute_tool_calls_and_update_history(
        &mut history,
        &tool_calls,
        "assistant requested native calls".to_string(),
        &native_tool_calls,
        &tools,
        &observer,
        "test-channel",
        None,
        "test-provider",
        "test-model",
        "turn-testcov-0090",
        2,
        &context,
        None,
        &[],
        false,
        None,
        &mut seen,
        true,
    )
    .await
    .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(history.len(), 4);
    assert_eq!(history[2].role, "tool");
    assert_eq!(history[3].role, "tool");
    assert!(history[2].content.contains("\"tool_call_id\":\"call-a\""));
    assert!(history[3].content.contains("\"tool_call_id\":\"call-b\""));
}

#[tokio::test]
async fn successful_tool_execution_emits_progress_and_structured_result() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![test_tool(&calls)];
    let observer = NoopObserver;
    let context = test_context();
    let mut history = vec![ChatMessage::user("run once")];
    let tool_calls = vec![parsed_call(
        "testcov_0090_echo",
        serde_json::json!({ "value": "progress" }),
        Some("call-progress"),
    )];
    let mut seen = HashSet::new();
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);

    execute_tool_calls_and_update_history(
        &mut history,
        &tool_calls,
        "assistant requested one tool".to_string(),
        &[],
        &tools,
        &observer,
        "test-channel",
        None,
        "test-provider",
        "test-model",
        "turn-testcov-0090",
        3,
        &context,
        Some(&tx),
        &[],
        false,
        None,
        &mut seen,
        false,
    )
    .await
    .unwrap();
    drop(tx);

    let mut progress_messages = Vec::new();
    while let Some(message) = rx.recv().await {
        progress_messages.push(message);
    }

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(history[1].content, "assistant requested one tool");
    assert_tool_result_message(&history[2], "echo:progress");
    assert!(progress_messages.iter().any(|message| message.contains("⏳")));
    assert!(progress_messages.iter().any(|message| message.contains("✅")));
    assert!(progress_messages.iter().any(|message| {
        message.contains(DRAFT_WS_EVENT_SENTINEL) && message.contains("\"event\":\"tool_result\"")
    }));
}

#[tokio::test]
async fn read_only_tools_execute_through_parallel_batch_and_keep_history_order() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![
        named_test_tool("testcov_0090_read_a", &calls, true, true),
        named_test_tool("testcov_0090_read_b", &calls, true, true),
    ];
    let observer = NoopObserver;
    let context = test_context();
    let mut history = vec![ChatMessage::user("run parallel")];
    let tool_calls = vec![
        parsed_call("testcov_0090_read_a", serde_json::json!({ "value": "A" }), None),
        parsed_call("testcov_0090_read_b", serde_json::json!({ "value": "B" }), None),
    ];
    let mut seen = HashSet::new();

    execute_tool_calls_and_update_history(
        &mut history,
        &tool_calls,
        "assistant requested parallel tools".to_string(),
        &[],
        &tools,
        &observer,
        "test-channel",
        None,
        "test-provider",
        "test-model",
        "turn-testcov-0090",
        4,
        &context,
        None,
        &[],
        false,
        None,
        &mut seen,
        false,
    )
    .await
    .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(history.len(), 3);
    assert_tool_result_message(&history[2], "echo:A");
    assert_tool_result_message(&history[2], "echo:B");
    assert!(
        history[2].content.find("echo:A").unwrap() < history[2].content.find("echo:B").unwrap()
    );
}

#[tokio::test]
async fn failed_tool_execution_emits_failure_progress_and_error_history() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![named_test_tool("testcov_0090_fail", &calls, false, false)];
    let observer = NoopObserver;
    let context = test_context();
    let mut history = vec![ChatMessage::user("run failing tool")];
    let tool_calls = vec![parsed_call(
        "testcov_0090_fail",
        serde_json::json!({ "value": "bad" }),
        Some("call-fail"),
    )];
    let mut seen = HashSet::new();
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);

    execute_tool_calls_and_update_history(
        &mut history,
        &tool_calls,
        "assistant requested failing tool".to_string(),
        &[],
        &tools,
        &observer,
        "test-channel",
        None,
        "test-provider",
        "test-model",
        "turn-testcov-0090",
        5,
        &context,
        Some(&tx),
        &[],
        false,
        None,
        &mut seen,
        false,
    )
    .await
    .unwrap();
    drop(tx);

    let mut progress_messages = Vec::new();
    while let Some(message) = rx.recv().await {
        progress_messages.push(message);
    }

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_tool_result_message(&history[2], "Error: failed:bad");
    assert!(progress_messages.iter().any(|message| message.contains("❌")));
    assert!(progress_messages.iter().any(|message| {
        message.contains(DRAFT_WS_EVENT_SENTINEL) && message.contains("\"success\":false")
    }));
}

#[tokio::test]
async fn cron_add_delivery_is_injected_before_tool_execution() {
    let calls = Arc::new(AtomicUsize::new(0));
    let received_args = Arc::new(std::sync::Mutex::new(Vec::new()));
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(Testcov0090CronTool {
        calls: Arc::clone(&calls),
        received_args: Arc::clone(&received_args),
    })];
    let observer = NoopObserver;
    let context = test_context();
    let mut history = vec![ChatMessage::user("schedule followup")];
    let tool_calls = vec![parsed_call(
        "cron_add",
        serde_json::json!({
            "prompt": "check status",
            "cron": "0 * * * *"
        }),
        Some("call-cron"),
    )];
    let mut seen = HashSet::new();

    execute_tool_calls_and_update_history(
        &mut history,
        &tool_calls,
        "assistant requested cron".to_string(),
        &[],
        &tools,
        &observer,
        "telegram",
        Some("chat-123"),
        "test-provider",
        "test-model",
        "turn-testcov-0090",
        6,
        &context,
        None,
        &[],
        false,
        None,
        &mut seen,
        false,
    )
    .await
    .unwrap();

    let args = received_args.lock().unwrap();
    let delivery = &args[0]["delivery"];
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(delivery["mode"], "announce");
    assert_eq!(delivery["channel"], "telegram");
    assert_eq!(delivery["to"], "chat-123");
    assert_tool_result_message(&history[2], "cron-ok");
}

#[tokio::test]
async fn native_fallback_tool_ids_do_not_make_duplicate_calls_distinct() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![test_tool(&calls)];
    let observer = NoopObserver;
    let context = test_context();
    let mut history = vec![ChatMessage::user("native fallback duplicates")];
    let tool_calls = vec![
        parsed_call(
            "testcov_0090_echo",
            serde_json::json!({ "value": "fallback" }),
            Some("fallback_6_0"),
        ),
        parsed_call(
            "testcov_0090_echo",
            serde_json::json!({ "value": "fallback" }),
            Some("fallback_6_1"),
        ),
    ];
    let mut seen = HashSet::new();

    execute_tool_calls_and_update_history(
        &mut history,
        &tool_calls,
        "assistant requested fallback duplicates".to_string(),
        &[],
        &tools,
        &observer,
        "test-channel",
        None,
        "test-provider",
        "test-model",
        "turn-testcov-0090",
        6,
        &context,
        None,
        &[],
        false,
        None,
        &mut seen,
        true,
    )
    .await
    .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(history.len(), 4);
    assert_eq!(history[2].role, "tool");
    assert_eq!(history[3].role, "tool");
    assert!(history[2].content.contains("\"tool_call_id\":\"fallback_6_0\""));
    assert!(history[2].content.contains("echo:fallback"));
    assert!(history[3].content.contains("\"tool_call_id\":\"fallback_6_1\""));
    assert!(
        history[3]
            .content
            .contains("Skipped duplicate tool call 'testcov_0090_echo' with identical arguments")
    );
}
