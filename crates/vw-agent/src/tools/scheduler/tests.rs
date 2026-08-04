//! 工具调用调度器测试。
//!
//! 这些用例验证只读且并发安全的工具会被并行分批，而未知或写入型工具保持串行，
//! 避免调度器扩大未声明的并发能力。

use super::*;
use crate::tools::ToolResult;
use async_trait::async_trait;

struct FakeTool {
    name: &'static str,
    read_only: bool,
    concurrency_safe: bool,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for FakeTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "scheduler test tool"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
        })
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: self.name.to_string(), error: None })
    }

    fn is_read_only(&self) -> bool {
        self.read_only
    }

    fn is_concurrency_safe(&self) -> bool {
        self.concurrency_safe
    }
}

#[test]
fn schedule_tool_calls_batches_parallel_and_serial_segments() {
    let registry: Vec<Box<dyn Tool>> = vec![
        Box::new(FakeTool { name: "read_a", read_only: true, concurrency_safe: true }),
        Box::new(FakeTool { name: "write_a", read_only: false, concurrency_safe: false }),
        Box::new(FakeTool { name: "read_b", read_only: true, concurrency_safe: true }),
    ];

    let calls = vec![
        PendingToolCall {
            name: "read_a".to_string(),
            arguments: serde_json::json!({}),
            tool_call_id: None,
        },
        PendingToolCall {
            name: "write_a".to_string(),
            arguments: serde_json::json!({}),
            tool_call_id: None,
        },
        PendingToolCall {
            name: "read_b".to_string(),
            arguments: serde_json::json!({}),
            tool_call_id: None,
        },
    ];

    let batches = schedule_tool_calls(&calls, &registry);
    assert_eq!(batches.len(), 3);
    assert_eq!(batches[0].mode, ScheduledToolBatchMode::Parallel);
    assert_eq!(batches[1].mode, ScheduledToolBatchMode::Sequential);
    assert_eq!(batches[2].mode, ScheduledToolBatchMode::Parallel);
}

#[test]
fn schedule_tool_calls_treats_unknown_tools_as_serial() {
    let registry: Vec<Box<dyn Tool>> =
        vec![Box::new(FakeTool { name: "read_a", read_only: true, concurrency_safe: true })];

    let calls = vec![PendingToolCall {
        name: "unknown_tool".to_string(),
        arguments: serde_json::json!({}),
        tool_call_id: None,
    }];

    let batches = schedule_tool_calls(&calls, &registry);
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].mode, ScheduledToolBatchMode::Sequential);
}

#[test]
fn schedule_tool_calls_allows_isolated_agent_tools_to_run_in_parallel() {
    let registry: Vec<Box<dyn Tool>> =
        vec![Box::new(FakeTool { name: "AgentTool", read_only: false, concurrency_safe: true })];
    let calls = (0..3)
        .map(|index| PendingToolCall {
            name: "AgentTool".to_string(),
            arguments: serde_json::json!({ "prompt": format!("task-{index}") }),
            tool_call_id: None,
        })
        .collect::<Vec<_>>();

    let batches = schedule_tool_calls(&calls, &registry);

    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].mode, ScheduledToolBatchMode::Parallel);
    assert_eq!(batches[0].calls.len(), 3);
}
