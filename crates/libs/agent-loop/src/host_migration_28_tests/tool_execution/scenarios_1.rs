use super::*;

pub fn immediate_failure_preserves_tool_identity_and_marks_failure<C: ContextHost, N>(services: &ToolExecutionServices<C, ParsedToolCall, PendingToolCall, ToolExecutionOutcome, N>) {
    let (tool_name, call_id, outcome) =
        tool_execution::immediate_failure(services, "shell".to_string(), Some("call-1".to_string()), "blocked".to_string());

    assert_eq!(tool_name, "shell");
    assert_eq!(call_id.as_deref(), Some("call-1"));
    assert_eq!(outcome.tool_name, "shell");
    assert_eq!(outcome.error_reason.as_deref(), Some("blocked"));
    assert!(!outcome.success);
}

pub async fn structured_progress_skips_when_result_dto_is_absent<C: ContextHost, N>(services: &ToolExecutionServices<C, ParsedToolCall, PendingToolCall, ToolExecutionOutcome, N>) {
    let (_, _, outcome) =
        tool_execution::immediate_failure(services, "testcov_0090_echo".to_string(), None, "blocked".to_string());
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    tool_execution::send_structured_tool_result_progress(&tx, &(services.outcome_view)(&outcome), 0, services).await;

    assert!(rx.try_recv().is_err());
}

pub async fn excluded_tool_writes_blocked_result_without_execution<C: ContextHost, N>(services: &ToolExecutionServices<C, ParsedToolCall, PendingToolCall, ToolExecutionOutcome, N>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![test_tool(&calls)];
    let observer = NoopObserver;
    let context = test_context::<C>();
    let mut history = vec![ChatMessage::user("run blocked")];
    let tool_calls =
        vec![parsed_call("testcov_0090_echo", serde_json::json!({ "value": "blocked" }), None)];
    let mut seen = HashSet::new();

    tool_execution::execute_tool_calls_and_update_history(services,
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

pub async fn mcp_toolset_excluded_tool_is_blocked_by_channel_policy<C: ContextHost, N>(services: &ToolExecutionServices<C, ParsedToolCall, PendingToolCall, ToolExecutionOutcome, N>, specs: Vec<tool_types::ToolSpec>) {
    assert!(
        specs.iter().any(|spec| spec.id == MCP_TOOLSET_TOOL_ID),
        "configured MCP tool should be discoverable before channel policy excludes it"
    );

    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![named_test_tool(MCP_TOOLSET_TOOL_ID, &calls, true, true)];
    let observer = NoopObserver;
    let context = test_context::<C>();
    let mut history = vec![ChatMessage::user("run blocked MCP tool")];
    let tool_calls = vec![parsed_call(
        MCP_TOOLSET_TOOL_ID,
        serde_json::json!({ "message": "blocked" }),
        Some("call-mcp-blocked"),
    )];
    let mut seen = HashSet::new();

    tool_execution::execute_tool_calls_and_update_history(services,
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
            .contains("Tool 'mcp__fixture_server__echo_tool_2fb49d0879816c90' is not available in this channel."),
        "{}",
        history[2].content,
    );
}
