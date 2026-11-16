use super::*;

pub async fn duplicate_text_tool_call_is_reported_without_second_execution<C: ContextHost, N>(services: &ToolExecutionServices<C, ParsedToolCall, PendingToolCall, ToolExecutionOutcome, N>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![test_tool(&calls)];
    let observer = NoopObserver;
    let context = test_context::<C>();
    let mut history = vec![ChatMessage::user("run twice")];
    let tool_calls = vec![
        parsed_call("testcov_0090_echo", serde_json::json!({ "value": "same" }), None),
        parsed_call("testcov_0090_echo", serde_json::json!({ "value": "same" }), None),
    ];
    let mut seen = HashSet::new();

    tool_execution::execute_tool_calls_and_update_history(services,
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

pub async fn native_tool_ids_keep_same_arguments_distinct_and_write_tool_messages<C: ContextHost, N>(services: &ToolExecutionServices<C, ParsedToolCall, PendingToolCall, ToolExecutionOutcome, N>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![test_tool(&calls)];
    let observer = NoopObserver;
    let context = test_context::<C>();
    let mut history = vec![ChatMessage::user("native calls")];
    let tool_calls = vec![
        parsed_call("testcov_0090_echo", serde_json::json!({ "value": "native" }), Some("call-a")),
        parsed_call("testcov_0090_echo", serde_json::json!({ "value": "native" }), Some("call-b")),
    ];
    let native_tool_calls = vec![
        ToolCall {
            kind: Default::default(),
            namespace: None,
            id: "call-a".to_string(),
            name: "testcov_0090_echo".to_string(),
            arguments: r#"{"value":"native"}"#.to_string(),
        },
        ToolCall {
            kind: Default::default(),
            namespace: None,
            id: "call-b".to_string(),
            name: "testcov_0090_echo".to_string(),
            arguments: r#"{"value":"native"}"#.to_string(),
        },
    ];
    let mut seen = HashSet::new();

    tool_execution::execute_tool_calls_and_update_history(services,
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

pub async fn successful_tool_execution_emits_progress_and_structured_result<C: ContextHost, N>(services: &ToolExecutionServices<C, ParsedToolCall, PendingToolCall, ToolExecutionOutcome, N>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![test_tool(&calls)];
    let observer = NoopObserver;
    let context = test_context::<C>();
    let mut history = vec![ChatMessage::user("run once")];
    let tool_calls = vec![parsed_call(
        "testcov_0090_echo",
        serde_json::json!({ "value": "progress" }),
        Some("call-progress"),
    )];
    let mut seen = HashSet::new();
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);

    tool_execution::execute_tool_calls_and_update_history(services,
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

pub async fn read_only_tools_execute_through_parallel_batch_and_keep_history_order<C: ContextHost, N>(services: &ToolExecutionServices<C, ParsedToolCall, PendingToolCall, ToolExecutionOutcome, N>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![
        named_test_tool("testcov_0090_read_a", &calls, true, true),
        named_test_tool("testcov_0090_read_b", &calls, true, true),
    ];
    let observer = NoopObserver;
    let context = test_context::<C>();
    let mut history = vec![ChatMessage::user("run parallel")];
    let tool_calls = vec![
        parsed_call("testcov_0090_read_a", serde_json::json!({ "value": "A" }), None),
        parsed_call("testcov_0090_read_b", serde_json::json!({ "value": "B" }), None),
    ];
    let mut seen = HashSet::new();

    tool_execution::execute_tool_calls_and_update_history(services,
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
