//! 验证工具循环在非 CLI 通道中的审批与通道禁用策略。
//!
//! 这些测试保护安全边界：受监管工具必须等待明确审批，被通道排除的工具必须
//! 阻止执行，并且拒绝原因需要反馈给模型历史。

use super::*;
use agent_loop::approval::{NonCliApprovalContext, NonCliApprovalPrompt};

#[tokio::test]
async fn run_tool_call_loop_denies_supervised_tools_on_non_cli_channels() {
    // Telegram 等非 CLI 通道没有本地交互式确认时，应直接拒绝 shell 这类受监管工具。
    let provider = ScriptedProvider::from_text_responses(vec![
        r#"<tool_call>
{"name":"shell","arguments":{"command":"echo hi"}}
</tool_call>"#,
        "done",
    ])
    .with_native_tool_support();

    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let tools_registry: Vec<Box<dyn Tool>> =
        vec![Box::new(DelayTool::new("shell", 50, Arc::clone(&active), Arc::clone(&max_active)))];

    let approval_mgr = Arc::new(ApprovalManager::from_config(
        &agent_config::runtime_schema::AutonomyConfig::default(),
    ));

    let mut history = vec![ChatMessage::system("test-system"), ChatMessage::user("run shell")];
    let observer = NoopObserver;

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
    .expect("tool loop should complete with denied tool execution");

    assert_eq!(result, "done");
    assert_eq!(
        max_active.load(Ordering::SeqCst),
        0,
        "shell tool must not execute when approval is unavailable on non-CLI channels"
    );
}

#[tokio::test]
async fn run_tool_call_loop_waits_for_non_cli_approval_resolution() {
    let provider = ScriptedProvider::from_text_responses(vec![
        r#"<tool_call>
{"name":"shell","arguments":{"command":"echo hi"}}
</tool_call>"#,
        "done",
    ])
    .with_native_tool_support();

    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let tools_registry: Vec<Box<dyn Tool>> =
        vec![Box::new(DelayTool::new("shell", 50, Arc::clone(&active), Arc::clone(&max_active)))];

    let approval_mgr = Arc::new(ApprovalManager::from_config(
        &agent_config::runtime_schema::AutonomyConfig::default(),
    ));
    let (prompt_tx, mut prompt_rx) = tokio::sync::mpsc::unbounded_channel::<NonCliApprovalPrompt>();
    let approval_mgr_for_task = Arc::clone(&approval_mgr);
    // 审批解析发生在独立任务中，模拟外部通道收到 prompt 后回传用户决定。
    let approval_task = tokio::spawn(async move {
        let prompt: NonCliApprovalPrompt =
            prompt_rx.recv().await.expect("approval prompt should arrive");
        approval_mgr_for_task
            .confirm_non_cli_pending_request(
                &prompt.request_id,
                "alice",
                "telegram",
                "chat-approval",
            )
            .expect("pending approval should confirm");
        approval_mgr_for_task
            .record_non_cli_pending_resolution(&prompt.request_id, ApprovalResponse::Yes);
    });

    let mut history = vec![ChatMessage::system("test-system"), ChatMessage::user("run shell")];
    let observer = NoopObserver;

    let result = run_tool_call_loop_with_non_cli_approval_context(
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
        Some(NonCliApprovalContext {
            sender: "alice".to_string(),
            reply_target: "chat-approval".to_string(),
            prompt_tx,
        }),
        &agent_config::runtime_schema::MultimodalConfig::default(),
        4,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect("tool loop should continue after non-cli approval");

    approval_task.await.expect("approval task should complete");
    assert_eq!(result, "done");
    assert_eq!(
        max_active.load(Ordering::SeqCst),
        1,
        "shell tool should execute after non-cli approval is resolved"
    );
}

#[tokio::test]
async fn run_tool_call_loop_consumes_one_time_non_cli_allow_all_token() {
    let provider = ScriptedProvider::from_text_responses(vec![
        r#"<tool_call>
{"name":"shell","arguments":{"command":"echo hi"}}
</tool_call>"#,
        "done",
    ])
    .with_native_tool_support();

    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let tools_registry: Vec<Box<dyn Tool>> =
        vec![Box::new(DelayTool::new("shell", 50, Arc::clone(&active), Arc::clone(&max_active)))];

    let approval_mgr = Arc::new(ApprovalManager::from_config(
        &agent_config::runtime_schema::AutonomyConfig::default(),
    ));
    approval_mgr.grant_non_cli_allow_all_once();
    // 一次性 token 只允许当前待审批工具通过，防止“允许全部”静默扩大到后续轮次。
    assert_eq!(approval_mgr.non_cli_allow_all_once_remaining(), 1);

    let mut history = vec![ChatMessage::system("test-system"), ChatMessage::user("run shell once")];
    let observer = NoopObserver;

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
    .expect("tool loop should consume one-time allow-all token");

    assert_eq!(result, "done");
    assert_eq!(
        max_active.load(Ordering::SeqCst),
        1,
        "shell tool should execute after consuming one-time allow-all token"
    );
    assert_eq!(approval_mgr.non_cli_allow_all_once_remaining(), 0);
}

#[tokio::test]
async fn run_tool_call_loop_blocks_tools_excluded_for_channel() {
    // 通道排除列表优先于工具执行，即使没有审批管理器也不能放行该工具。
    let provider = ScriptedProvider::from_text_responses(vec![
        r#"<tool_call>
{"name":"shell","arguments":{"command":"echo hi"}}
</tool_call>"#,
        "done",
    ])
    .with_native_tool_support();

    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let tools_registry: Vec<Box<dyn Tool>> =
        vec![Box::new(DelayTool::new("shell", 50, Arc::clone(&active), Arc::clone(&max_active)))];

    let mut history = vec![ChatMessage::system("test-system"), ChatMessage::user("run shell")];
    let observer = NoopObserver;
    let excluded_tools = vec!["shell".to_string()];

    let result = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "mock-provider",
        "mock-model",
        0.0,
        true,
        None,
        "telegram",
        &agent_config::runtime_schema::MultimodalConfig::default(),
        4,
        None,
        None,
        None,
        None,
        &excluded_tools,
    )
    .await
    .expect("tool loop should complete with blocked tool execution");

    assert_eq!(result, "done");
    assert_eq!(
        max_active.load(Ordering::SeqCst),
        0,
        "excluded tool must not execute even if the model requests it"
    );

    let tool_results_message = history
        .iter()
        .find(|msg| msg.role == "tool")
        .expect("tool results message should be present");
    assert!(
        tool_results_message.content.contains("not available in this channel"),
        "blocked reason should be visible to the model"
    );
    assert!(
        history
            .iter()
            .all(|msg| !(msg.role == "user" && msg.content.starts_with("[Tool results]"))),
        "native mode should not emit prompt-style tool results"
    );
}
