//! 测试批量工具执行的结果规范化、权限错误、取消与调度分组。
//! 可配置工具夹具区分权限失败、执行错误、失败结果和并发调度属性。

use super::*;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::observability::traits::ObserverMetric;
use async_trait::async_trait;
use std::any::Any;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

struct RecordingObserver {
    events: Mutex<Vec<ObserverEvent>>,
}

impl RecordingObserver {
    fn new() -> Self {
        Self { events: Mutex::new(Vec::new()) }
    }

    fn events(&self) -> Vec<ObserverEvent> {
        self.events.lock().expect("observer events mutex poisoned").clone()
    }
}

impl Observer for RecordingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        self.events.lock().expect("observer events mutex poisoned").push(event.clone());
    }

    fn record_metric(&self, _metric: &ObserverMetric) {}

    fn name(&self) -> &str {
        "recording"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct Testcov0093Tool {
    id: &'static str,
    calls: Arc<AtomicUsize>,
    output: &'static str,
    success: bool,
    error: Option<&'static str>,
    execute_error: Option<&'static str>,
    permission_error: Option<&'static str>,
    read_only: bool,
    concurrency_safe: bool,
    delay_ms: u64,
}

impl Testcov0093Tool {
    fn new(id: &'static str, calls: Arc<AtomicUsize>) -> Self {
        Self {
            id,
            calls,
            output: "ok",
            success: true,
            error: None,
            execute_error: None,
            permission_error: None,
            read_only: true,
            concurrency_safe: true,
            delay_ms: 0,
        }
    }

    fn with_output(mut self, output: &'static str) -> Self {
        self.output = output;
        self
    }

    fn with_tool_failure(mut self, output: &'static str, error: &'static str) -> Self {
        self.success = false;
        self.output = output;
        self.error = Some(error);
        self
    }

    fn with_failed_output(mut self, output: &'static str) -> Self {
        self.success = false;
        self.output = output;
        self.error = None;
        self
    }

    fn with_execute_error(mut self, error: &'static str) -> Self {
        self.execute_error = Some(error);
        self
    }

    fn with_permission_error(mut self, error: &'static str) -> Self {
        self.permission_error = Some(error);
        self
    }

    fn with_scheduling(mut self, read_only: bool, concurrency_safe: bool) -> Self {
        self.read_only = read_only;
        self.concurrency_safe = concurrency_safe;
        self
    }

    fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }
}

#[async_trait]
impl Tool for Testcov0093Tool {
    fn name(&self) -> &str {
        self.id
    }

    fn description(&self) -> &str {
        "test coverage tool"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            }
        })
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        if self.delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
        }
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(error) = self.execute_error {
            anyhow::bail!(error);
        }
        Ok(ToolResult {
            success: self.success,
            output: self.output.to_string(),
            error: self.error.map(ToString::to_string),
        })
    }

    async fn check_permissions(&self, _input: &serde_json::Value) -> anyhow::Result<()> {
        if let Some(error) = self.permission_error {
            anyhow::bail!(error);
        }
        Ok(())
    }

    fn is_read_only(&self) -> bool {
        self.read_only
    }

    fn is_concurrency_safe(&self) -> bool {
        self.concurrency_safe
    }
}

fn registry(tools: Vec<Testcov0093Tool>) -> Vec<Box<dyn Tool>> {
    tools.into_iter().map(|tool| Box::new(tool) as Box<dyn Tool>).collect()
}

fn pending(name: &str, value: &str, tool_call_id: Option<&str>) -> PendingToolCall {
    PendingToolCall {
        name: name.to_string(),
        arguments: serde_json::json!({ "value": value }),
        tool_call_id: tool_call_id.map(str::to_string),
    }
}

fn parsed(name: &str) -> ParsedToolCall {
    ParsedToolCall {
        name: name.to_string(),
        arguments: serde_json::json!({ "value": name }),
        tool_call_id: None,
    }
}

fn context() -> Arc<ToolUseContext> {
    Arc::new(ToolUseContext::default())
}

#[tokio::test]
async fn sequential_success_scrubs_output_and_enriches_result_dto() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = registry(vec![
        Testcov0093Tool::new("testcov_0093_success", calls.clone())
            .with_output(r#"done token="abcdef1234567890""#),
    ]);
    let observer = RecordingObserver::new();

    let outcomes = execute_tools_sequential(
        &[pending("testcov_0093_success", "ok", Some("call-0093"))],
        &tools,
        &observer,
        context(),
        None,
    )
    .await
    .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].success);
    assert!(outcomes[0].output.contains("abcd*[REDACTED]"));
    assert!(!outcomes[0].output.contains("abcdef1234567890"));

    let dto = outcomes[0].result_dto.as_ref().expect("success should produce dto");
    let telemetry = dto.telemetry.as_ref().expect("telemetry should be present");
    assert_eq!(dto.tool_use_id.as_deref(), Some("call-0093"));
    assert_eq!(dto.tool_id.as_ref().map(|id| id.as_ref()), Some("testcov_0093_success"));
    assert_eq!(dto.success, Some(true));
    assert_eq!(telemetry["success"], true);
    assert!(telemetry["attributes"]["duration_ms"].is_number());
    assert!(telemetry["attributes"]["duration_secs"].is_number());

    let events = observer.events();
    assert!(matches!(
        &events[0],
        ObserverEvent::ToolCallStart { tool } if tool == "testcov_0093_success"
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        ObserverEvent::ToolCall { tool, success: true, .. }
            if tool == "testcov_0093_success"
    )));
}

#[tokio::test]
async fn tool_result_failure_is_returned_as_failed_outcome_with_error_prefix() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = registry(vec![
        Testcov0093Tool::new("testcov_0093_tool_failure", calls.clone())
            .with_tool_failure("ignored output", r#"api_key="secret123456789""#),
    ]);

    let outcomes = execute_tools_sequential(
        &[pending("testcov_0093_tool_failure", "bad", None)],
        &tools,
        &NoopObserver,
        context(),
        None,
    )
    .await
    .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(!outcomes[0].success);
    assert!(outcomes[0].output.starts_with("Error: "));
    assert!(outcomes[0].output.contains("secr*[REDACTED]"));
    assert_eq!(outcomes[0].error_reason.as_deref(), Some(r#"api_key="secr*[REDACTED]""#));
    assert_eq!(outcomes[0].result_dto.as_ref().unwrap().success, Some(false));
}

#[tokio::test]
async fn failed_tool_result_without_error_uses_model_output_as_reason() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = registry(vec![
        Testcov0093Tool::new("testcov_0093_failed_output", calls.clone())
            .with_failed_output(r#"plain token="abcdef1234567890""#),
    ]);

    let outcomes = execute_tools_sequential(
        &[pending("testcov_0093_failed_output", "bad", Some("call-failed-output"))],
        &tools,
        &NoopObserver,
        context(),
        None,
    )
    .await
    .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(!outcomes[0].success);
    assert_eq!(outcomes[0].output, r#"Error: plain token="abcd*[REDACTED]""#);
    assert_eq!(outcomes[0].error_reason.as_deref(), Some(r#"plain token="abcd*[REDACTED]""#));
    let dto = outcomes[0].result_dto.as_ref().expect("failed output should produce dto");
    assert_eq!(dto.tool_use_id.as_deref(), Some("call-failed-output"));
    assert_eq!(dto.success, Some(false));
}

#[tokio::test]
async fn execution_error_and_permission_denial_become_failed_outcomes() {
    let failed_calls = Arc::new(AtomicUsize::new(0));
    let denied_calls = Arc::new(AtomicUsize::new(0));
    let tools = registry(vec![
        Testcov0093Tool::new("testcov_0093_execute_error", failed_calls.clone())
            .with_execute_error(r#"failed password="pass123456789""#),
        Testcov0093Tool::new("testcov_0093_denied", denied_calls.clone())
            .with_permission_error(r#"blocked secret="hide123456789""#),
    ]);

    let failed = execute_tools_sequential(
        &[pending("testcov_0093_execute_error", "bad", Some("call-failed"))],
        &tools,
        &NoopObserver,
        context(),
        None,
    )
    .await
    .unwrap();
    let denied = execute_tools_sequential(
        &[pending("testcov_0093_denied", "bad", Some("call-denied"))],
        &tools,
        &NoopObserver,
        context(),
        None,
    )
    .await
    .unwrap();

    assert_eq!(failed_calls.load(Ordering::SeqCst), 1);
    assert_eq!(denied_calls.load(Ordering::SeqCst), 0);
    assert!(!failed[0].success);
    assert_eq!(failed[0].output, r#"failed password="pass*[REDACTED]""#);
    assert!(!denied[0].success);
    assert_eq!(denied[0].output, r#"blocked secret="hide*[REDACTED]""#);
    assert_eq!(denied[0].result_dto.as_ref().unwrap().tool_use_id.as_deref(), Some("call-denied"));
}

#[tokio::test]
async fn unknown_tool_becomes_failed_outcome_with_dto() {
    let tools = registry(Vec::new());

    let outcomes = execute_tools_sequential(
        &[pending("testcov_0093_missing", "bad", Some("call-missing"))],
        &tools,
        &NoopObserver,
        context(),
        None,
    )
    .await
    .unwrap();

    assert_eq!(outcomes.len(), 1);
    assert!(!outcomes[0].success);
    assert_eq!(outcomes[0].tool_name, "testcov_0093_missing");
    assert_eq!(outcomes[0].output, "Unknown tool: testcov_0093_missing");
    assert_eq!(outcomes[0].error_reason.as_deref(), Some("Unknown tool: testcov_0093_missing"));
    let dto = outcomes[0].result_dto.as_ref().expect("unknown tool should produce dto");
    assert_eq!(dto.tool_use_id.as_deref(), Some("call-missing"));
    assert_eq!(dto.tool_id.as_ref().map(|id| id.as_ref()), Some("testcov_0093_missing"));
    assert_eq!(dto.success, Some(false));
}

#[tokio::test]
async fn parallel_execution_collects_all_outcomes_without_short_circuiting_failures() {
    let ok_calls = Arc::new(AtomicUsize::new(0));
    let failed_calls = Arc::new(AtomicUsize::new(0));
    let tools = registry(vec![
        Testcov0093Tool::new("testcov_0093_parallel_ok", ok_calls.clone()).with_output("ok"),
        Testcov0093Tool::new("testcov_0093_parallel_fail", failed_calls.clone())
            .with_execute_error("failed during parallel execution"),
    ]);

    let outcomes = execute_tools_parallel(
        &[
            pending("testcov_0093_parallel_ok", "ok", None),
            pending("testcov_0093_parallel_fail", "bad", None),
        ],
        &tools,
        &NoopObserver,
        context(),
        None,
    )
    .await
    .unwrap();

    assert_eq!(ok_calls.load(Ordering::SeqCst), 1);
    assert_eq!(failed_calls.load(Ordering::SeqCst), 1);
    assert_eq!(outcomes.len(), 2);
    assert!(outcomes[0].success);
    assert!(!outcomes[1].success);
    assert_eq!(outcomes[1].output, "failed during parallel execution");
}

#[tokio::test]
async fn empty_tool_calls_return_empty_outcomes() {
    let tools = registry(Vec::new());

    let sequential =
        execute_tools_sequential(&[], &tools, &NoopObserver, context(), None).await.unwrap();
    let parallel =
        execute_tools_parallel(&[], &tools, &NoopObserver, context(), None).await.unwrap();

    assert!(sequential.is_empty());
    assert!(parallel.is_empty());
}

#[tokio::test]
async fn cancellation_token_stops_execution_before_tool_runs() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools =
        registry(vec![Testcov0093Tool::new("testcov_0093_slow", calls.clone()).with_delay(50)]);
    let token = CancellationToken::new();
    token.cancel();

    let error = match execute_tools_sequential(
        &[pending("testcov_0093_slow", "wait", None)],
        &tools,
        &NoopObserver,
        context(),
        Some(&token),
    )
    .await
    {
        Ok(_) => panic!("cancelled token should stop execution"),
        Err(error) => error,
    };

    assert!(error.is::<ToolLoopCancelled>());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn legacy_result_dto_preserves_permission_request_and_saturates_duration_ms() {
    let permission_request = PermissionRequestDto {
        reason: "needs approval".to_string(),
        warning: Some("requires review".to_string()),
        updated_input: Some(serde_json::json!({ "value": "blocked" })),
    };

    let dto = legacy_result_dto(
        "testcov_0093_legacy",
        Some("call-legacy"),
        false,
        Some("blocked".to_string()),
        Some(permission_request.clone()),
        Duration::from_secs(u64::MAX),
    );

    let telemetry = dto.telemetry.as_ref().expect("telemetry should be present");
    assert_eq!(dto.tool_use_id.as_deref(), Some("call-legacy"));
    assert_eq!(dto.tool_id.as_ref().map(|id| id.as_ref()), Some("testcov_0093_legacy"));
    assert_eq!(dto.success, Some(false));
    assert_eq!(dto.permission_request.as_ref(), Some(&permission_request));
    assert_eq!(telemetry["attributes"]["duration_ms"], serde_json::json!(u64::MAX));
    assert_eq!(telemetry["attributes"]["duration_secs"], serde_json::json!(u64::MAX));
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
