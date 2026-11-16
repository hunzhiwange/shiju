//! 测试工具执行的审批、安全上下文、并发约束与 MCP 路由。
//! 计数工具和临时 MCP 服务用于区分去重、策略阻断、执行失败与成功历史。

use crate::core::tool_execution::{self, ToolExecutionServices};
use crate::execution::ToolExecutionOutcome;
use crate::parsing::ParsedToolCall;
use tool_services::scheduler::PendingToolCall;
use tool_context::{host::ContextHost, context::ToolUseContext};
use model_types::{ChatMessage, ToolCall};
use tool_types::Tool;
use crate::progress::{DRAFT_PROGRESS_SENTINEL, DRAFT_WS_EVENT_SENTINEL};
use std::{sync::Arc, collections::HashSet};
use agent_observability::NoopObserver;
use tool_types::ToolResult;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

pub const MCP_TOOLSET_TOOL_ID: &str = "mcp__fixture_server__echo_tool_2fb49d0879816c90";

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

// 固定解析后调用的最小字段，避免执行测试重复依赖文本解析器。
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

fn test_context<C: ContextHost>() -> Arc<ToolUseContext<C>> {
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

pub fn write_mcp_toolset_workspace(tmp: &TempDir) -> PathBuf {
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

mod scenarios_1;
pub use scenarios_1::*;

mod scenarios_2;
pub use scenarios_2::*;

mod scenarios_3;
pub use scenarios_3::*;
