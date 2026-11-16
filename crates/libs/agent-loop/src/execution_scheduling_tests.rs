//! Pure batch grouping belongs to the execution library; no host context is used.
use super::*;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use tool_services::scheduler as tools;

struct Testcov0093Tool {
    id: &'static str,
    calls: Arc<AtomicUsize>,
    read_only: bool,
    concurrency_safe: bool,
}

impl Testcov0093Tool {
    fn new(id: &'static str, calls: Arc<AtomicUsize>) -> Self {
        Self { id, calls, read_only: true, concurrency_safe: true }
    }

    fn with_scheduling(mut self, read_only: bool, concurrency_safe: bool) -> Self {
        self.read_only = read_only;
        self.concurrency_safe = concurrency_safe;
        self
    }
}

#[async_trait]
impl Tool for Testcov0093Tool {
    fn name(&self) -> &str { self.id }
    fn description(&self) -> &str { "test coverage tool" }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": { "value": { "type": "string" } } })
    }
    async fn execute(&self, _args: Value) -> Result<ToolResult> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ToolResult { success: true, output: "ok".to_string(), error: None })
    }
    fn is_read_only(&self) -> bool { self.read_only }
    fn is_concurrency_safe(&self) -> bool { self.concurrency_safe }
}

fn registry(tools: Vec<Testcov0093Tool>) -> Vec<Box<dyn Tool>> {
    tools.into_iter().map(|tool| Box::new(tool) as Box<dyn Tool>).collect()
}

fn parsed(name: &str) -> ParsedToolCall {
    ParsedToolCall {
        name: name.to_string(),
        arguments: serde_json::json!({ "value": name }),
        tool_call_id: None,
    }
}

#[test]
fn schedule_batches_and_parallel_decision_follow_tool_metadata() {
    let parallel_a = Arc::new(AtomicUsize::new(0));
    let serial = Arc::new(AtomicUsize::new(0));
    let parallel_b = Arc::new(AtomicUsize::new(0));
    let tools = registry(vec![
        Testcov0093Tool::new("testcov_0093_parallel_a", parallel_a).with_scheduling(true, true),
        Testcov0093Tool::new("testcov_0093_serial", serial).with_scheduling(false, false),
        Testcov0093Tool::new("testcov_0093_parallel_b", parallel_b).with_scheduling(true, true),
    ]);

    let batches = schedule_tool_batches(
        &[
            parsed("testcov_0093_parallel_a"),
            parsed("testcov_0093_serial"),
            parsed("testcov_0093_parallel_b"),
            parsed("missing_tool"),
        ],
        &tools,
    );

    assert_eq!(batches.len(), 4);
    assert!(matches!(batches[0].mode, tools::ScheduledToolBatchMode::Parallel));
    assert!(matches!(batches[1].mode, tools::ScheduledToolBatchMode::Sequential));
    assert!(matches!(batches[2].mode, tools::ScheduledToolBatchMode::Parallel));
    assert!(matches!(batches[3].mode, tools::ScheduledToolBatchMode::Sequential));

    assert!(should_execute_tools_in_parallel(&[parsed("testcov_0093_parallel_a")], &tools));
    assert!(!should_execute_tools_in_parallel(&[parsed("testcov_0093_serial")], &tools));
    assert!(!should_execute_tools_in_parallel(&[], &tools));
}
