//! 工具调用调度与并行执行测试。
//!
//! 本模块覆盖只读工具的并行分组、非只读工具的串行屏障，以及代理
//! 循环在 native 模式下保持工具结果顺序的行为。

use super::*;
use tool_services::scheduler::{PendingToolCall, ScheduledToolBatchMode, schedule_tool_calls};

#[test]
fn scheduler_groups_read_only_concurrency_safe_calls_into_parallel_batch() {
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let tools_registry: Vec<Box<dyn Tool>> = vec![
        Box::new(DelayTool::new("delay_a", 50, Arc::clone(&active), Arc::clone(&max_active))),
        Box::new(DelayTool::new("delay_b", 50, Arc::clone(&active), Arc::clone(&max_active))),
    ];
    let calls = vec![
        PendingToolCall {
            name: "delay_a".to_string(),
            arguments: serde_json::json!({"value": "A"}),
            tool_call_id: None,
        },
        PendingToolCall {
            name: "delay_b".to_string(),
            arguments: serde_json::json!({"value": "B"}),
            tool_call_id: None,
        },
    ];

    let batches = schedule_tool_calls(&calls, &tools_registry);
    // 两个只读且并发安全的工具可以放入同一批次，以缩短总执行时间。
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].mode, ScheduledToolBatchMode::Parallel);
    assert_eq!(batches[0].calls.len(), 2);
}

#[test]
fn scheduler_keeps_non_read_only_calls_serial() {
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let tools_registry: Vec<Box<dyn Tool>> =
        vec![Box::new(DelayTool::new("shell", 50, Arc::clone(&active), Arc::clone(&max_active)))];
    let calls = vec![PendingToolCall {
        name: "shell".to_string(),
        arguments: serde_json::json!({"command": "pwd"}),
        tool_call_id: None,
    }];

    let batches = schedule_tool_calls(&calls, &tools_registry);
    // shell 类工具可能修改外部状态，调度器必须默认串行执行。
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].mode, ScheduledToolBatchMode::Sequential);
}

#[test]
fn scheduler_splits_parallel_groups_around_serial_barriers() {
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let tools_registry: Vec<Box<dyn Tool>> = vec![
        Box::new(DelayTool::new("delay_a", 50, Arc::clone(&active), Arc::clone(&max_active))),
        Box::new(DelayTool::new("shell", 50, Arc::clone(&active), Arc::clone(&max_active))),
        Box::new(DelayTool::new("delay_b", 50, Arc::clone(&active), Arc::clone(&max_active))),
    ];
    let calls = vec![
        PendingToolCall {
            name: "delay_a".to_string(),
            arguments: serde_json::json!({"value": "A"}),
            tool_call_id: None,
        },
        PendingToolCall {
            name: "shell".to_string(),
            arguments: serde_json::json!({"command": "pwd"}),
            tool_call_id: None,
        },
        PendingToolCall {
            name: "delay_b".to_string(),
            arguments: serde_json::json!({"value": "B"}),
            tool_call_id: None,
        },
    ];

    let batches = schedule_tool_calls(&calls, &tools_registry);
    // 串行工具会作为屏障切开前后的并行组，避免状态变更互相穿插。
    assert_eq!(batches.len(), 3);
    assert_eq!(batches[0].mode, ScheduledToolBatchMode::Parallel);
    assert_eq!(batches[1].mode, ScheduledToolBatchMode::Sequential);
    assert_eq!(batches[2].mode, ScheduledToolBatchMode::Parallel);
}

#[tokio::test]
async fn run_tool_call_loop_executes_multiple_tools_with_ordered_results() {
    let provider = ScriptedProvider::from_text_responses(vec![
        r#"<tool_call>
{"name":"delay_a","arguments":{"value":"A"}}
</tool_call>
<tool_call>
{"name":"delay_b","arguments":{"value":"B"}}
</tool_call>"#,
        "done",
    ])
    .with_native_tool_support();

    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let tools_registry: Vec<Box<dyn Tool>> = vec![
        Box::new(DelayTool::new("delay_a", 200, Arc::clone(&active), Arc::clone(&max_active))),
        Box::new(DelayTool::new("delay_b", 200, Arc::clone(&active), Arc::clone(&max_active))),
    ];

    let approval_cfg = agent_config::runtime_schema::AutonomyConfig {
        level: agent_security::AutonomyLevel::Full,
        ..agent_config::runtime_schema::AutonomyConfig::default()
    };
    let approval_mgr = Arc::new(ApprovalManager::from_config(&approval_cfg));

    let mut history = vec![ChatMessage::system("test-system"), ChatMessage::user("run tool calls")];
    let observer = NoopObserver;

    // 即使工具可并行运行，写回对话历史时也需要保持模型输出中的调用顺序，
    // 这样后续模型能稳定地把每条结果对应回原工具调用。
    let result = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "mock-provider",
        "mock-model",
        0.0,
        true,
        Some(approval_mgr.clone()),
        "telegram",
        &agent_config::runtime_schema::MultimodalConfig::default(),
        4,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect("parallel execution should complete");

    assert_eq!(result, "done");
    assert!(max_active.load(Ordering::SeqCst) >= 1, "tools should execute successfully");

    let tool_results: Vec<_> = history.iter().filter(|msg| msg.role == "tool").collect();
    assert_eq!(tool_results.len(), 2, "native mode should emit one tool message per result");
    assert!(tool_results[0].content.contains("ok:A"));
    assert!(tool_results[1].content.contains("ok:B"));
    assert!(
        history
            .iter()
            .all(|msg| !(msg.role == "user" && msg.content.starts_with("[Tool results]"))),
        "native mode should not emit prompt-style tool results"
    );
}
