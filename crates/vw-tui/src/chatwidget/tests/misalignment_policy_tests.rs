//! 验证模型对齐策略违规会停止当前线程并生成唯一、清晰的诊断界面。
//! 测试同时驱动流错误和轮次终止，确保两条事件路径不会重复插入警告。
//! 宽窄视口快照固定提示的换行与强调样式。
//! 远程线程场景保留同一安全语义，不因运行位置而降低严重级别。

use super::*;
use pretty_assertions::assert_eq;

const ERROR_MESSAGE: &str = "Responses API returned misalignment_policy_violation";

// 违规诊断同时覆盖本地与远程线程，但始终采用拒绝继续输入的安全默认值。

#[tokio::test]
async fn misalignment_policy_failure_stops_the_thread_and_renders_once() {
    // 先注入策略违规流错误，再完成同一轮次以验证诊断只出现一次。
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let thread_id = ThreadId::new();
    chat.thread_id = Some(thread_id);
    handle_turn_started(&mut chat, "turn-1");
    chat.handle_server_notification(
        ServerNotification::ModelSafetyBufferingUpdated(ModelSafetyBufferingUpdatedNotification {
            thread_id: thread_id.to_string(),
            turn_id: "turn-1".to_string(),
            model: "current-model".to_string(),
            use_cases: Vec::new(),
            reasons: Vec::new(),
            show_buffering_ui: true,
            faster_model: None,
        }),
        /*replay_kind*/ None,
    );
    assert!(render_bottom_popup(&chat, /*width*/ 80).contains("thinking a bit more"));
    chat.queue_user_message(UserMessage::from("queued follow-up"));
    chat.bottom_pane
        .set_composer_text("stale draft".to_string(), Vec::new(), Vec::new());
    drain_insert_history(&mut rx);

    handle_error(
        &mut chat,
        ERROR_MESSAGE,
        Some(CodexErrorInfo::MisalignmentPolicyViolation),
    );

    assert!(!chat.bottom_pane.is_task_running());
    assert!(!chat.bottom_pane.composer_input_enabled());
    assert!(chat.queued_user_message_texts().is_empty());
    assert_eq!(chat.bottom_pane.composer_text_with_pending(), "");

    let popup = render_bottom_popup(&chat, /*width*/ 80);
    assert_chatwidget_snapshot!("misalignment_policy_violation", popup);
    assert_chatwidget_snapshot!(
        "misalignment_policy_violation_narrow",
        // 窄视口只改变换行，不改变警告内容与严重性。
        render_bottom_popup(&chat, /*width*/ 38)
    );

    chat.submit_user_message(UserMessage::from("direct follow-up"));
    chat.queue_user_message(UserMessage::from("queued follow-up"));
    assert!(!chat.submit_op(Op::Compact));
    assert!(!chat.submit_op(Op::run_user_shell_command("echo unsafe".to_string())));
    assert!(!chat.maybe_send_next_queued_input());
    assert!(chat.queued_user_message_texts().is_empty());
    assert_no_submit_op(&mut op_rx);

    chat.handle_server_request(
        ServerRequest::ToolRequestUserInput {
            request_id: AppServerRequestId::Integer(1),
            params: ToolRequestUserInputParams {
                thread_id: thread_id.to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "input-1".to_string(),
                questions: Vec::new(),
                is_blocking: true,
                auto_resolution_ms: None,
            },
        },
        /*replay_kind*/ None,
    );

    chat.handle_server_notification(
        ServerNotification::TurnCompleted(TurnCompletedNotification {
            thread_id: thread_id.to_string(),
            turn: app_server_turn(
                "turn-1",
                AppServerTurnStatus::Failed,
                /*duration_ms*/ None,
                Some(AppServerTurnError {
                    message: ERROR_MESSAGE.to_string(),
                    codex_error_info: Some(CodexErrorInfo::MisalignmentPolicyViolation),
                    additional_details: None,
                }),
            ),
        }),
        /*replay_kind*/ None,
    );

    assert_eq!(render_bottom_popup(&chat, /*width*/ 80), popup);
    assert!(drain_insert_history(&mut rx).is_empty());

    chat.handle_key_event(KeyEvent::from(KeyCode::Esc));
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
    assert_matches!(rx.try_recv(), Ok(AppEvent::NewSession { name: None }));
    assert!(rx.try_recv().is_err());

    chat.show_misalignment_policy_precaution();
    assert_eq!(render_bottom_popup(&chat, /*width*/ 80), popup);
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
    assert_matches!(rx.try_recv(), Ok(AppEvent::NewSession { name: None }));

    chat.show_misalignment_policy_precaution();
    chat.handle_key_event(KeyEvent::from(KeyCode::Down));
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
    assert_matches!(rx.try_recv(), Ok(AppEvent::OpenResumePicker));
    assert!(chat.bottom_pane.has_active_view());

    chat.remote_connection = Some(crate::status::remote_connection::RemoteConnectionStatus {
        // 远程连接只改变恢复入口，不降低策略违规的阻断范围。
        address: "wss://remote.example.com".to_string(),
        version: "v1.0.0".to_string(),
    });
    chat.show_misalignment_policy_precaution();
    assert_chatwidget_snapshot!(
        "misalignment_policy_violation_remote",
        render_bottom_popup(&chat, /*width*/ 80)
    );
    chat.handle_key_event(KeyEvent::from(KeyCode::Down));
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
    assert_matches!(rx.try_recv(), Ok(AppEvent::OpenAgentsOverview));
    assert!(chat.bottom_pane.has_active_view());
}
