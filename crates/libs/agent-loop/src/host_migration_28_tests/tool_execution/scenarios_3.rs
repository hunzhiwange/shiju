use super::*;

pub async fn failed_tool_execution_emits_failure_progress_and_error_history<C: ContextHost, N>(services: &ToolExecutionServices<C, ParsedToolCall, PendingToolCall, ToolExecutionOutcome, N>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![named_test_tool("testcov_0090_fail", &calls, false, false)];
    let observer = NoopObserver;
    let context = test_context::<C>();
    let mut history = vec![ChatMessage::user("run failing tool")];
    let tool_calls = vec![parsed_call(
        "testcov_0090_fail",
        serde_json::json!({ "value": "bad" }),
        Some("call-fail"),
    )];
    let mut seen = HashSet::new();
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);

    tool_execution::execute_tool_calls_and_update_history(services,
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

pub async fn cron_add_delivery_is_injected_before_tool_execution<C: ContextHost, N>(services: &ToolExecutionServices<C, ParsedToolCall, PendingToolCall, ToolExecutionOutcome, N>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let received_args = Arc::new(std::sync::Mutex::new(Vec::new()));
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(Testcov0090CronTool {
        calls: Arc::clone(&calls),
        received_args: Arc::clone(&received_args),
    })];
    let observer = NoopObserver;
    let context = test_context::<C>();
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

    tool_execution::execute_tool_calls_and_update_history(services,
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

pub async fn native_fallback_tool_ids_do_not_make_duplicate_calls_distinct<C: ContextHost, N>(services: &ToolExecutionServices<C, ParsedToolCall, PendingToolCall, ToolExecutionOutcome, N>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = vec![test_tool(&calls)];
    let observer = NoopObserver;
    let context = test_context::<C>();
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

    tool_execution::execute_tool_calls_and_update_history(services,
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
