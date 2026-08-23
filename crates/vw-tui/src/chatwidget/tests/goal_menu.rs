//! 验证目标菜单在活跃、暂停、阻塞和额度限制状态下的展示与操作。
//! 固定目标夹具统一状态、令牌预算和摘要，便于比较状态差异。
//! 编辑流程检查可恢复状态保持不变，终态则重置为可继续的活跃状态。
//! 恢复提示还覆盖确认继续与保持暂停两条用户选择路径。

use super::*;

#[tokio::test]
async fn goal_menu_active_snapshot() {
    // 五种目标状态快照共同固定菜单摘要、预算和可用动作的差异。
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let thread_id = ThreadId::new();

    chat.show_goal_summary(test_goal(
        thread_id,
        AppThreadGoalStatus::Active,
        /*token_budget*/ Some(80_000),
    ));

    assert_chatwidget_snapshot!("goal_menu_active", rendered_goal_summary(&mut rx));
}

#[tokio::test]
async fn goal_menu_paused_snapshot() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let thread_id = ThreadId::new();

    chat.show_goal_summary(test_goal(
        thread_id,
        AppThreadGoalStatus::Paused,
        /*token_budget*/ None,
    ));

    assert_chatwidget_snapshot!("goal_menu_paused", rendered_goal_summary(&mut rx));
}

#[tokio::test]
async fn goal_menu_blocked_snapshot() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let thread_id = ThreadId::new();

    chat.show_goal_summary(test_goal(
        thread_id,
        AppThreadGoalStatus::Blocked,
        /*token_budget*/ None,
    ));

    assert_chatwidget_snapshot!("goal_menu_blocked", rendered_goal_summary(&mut rx));
}

#[tokio::test]
async fn goal_menu_usage_limited_snapshot() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let thread_id = ThreadId::new();

    chat.show_goal_summary(test_goal(
        thread_id,
        AppThreadGoalStatus::UsageLimited,
        /*token_budget*/ None,
    ));

    assert_chatwidget_snapshot!("goal_menu_usage_limited", rendered_goal_summary(&mut rx));
}

#[tokio::test]
async fn goal_menu_budget_limited_snapshot() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let thread_id = ThreadId::new();

    chat.show_goal_summary(test_goal(
        thread_id,
        AppThreadGoalStatus::BudgetLimited,
        /*token_budget*/ Some(80_000),
    ));

    assert_chatwidget_snapshot!("goal_menu_budget_limited", rendered_goal_summary(&mut rx));
}

#[tokio::test]
async fn resume_paused_goal_prompt_snapshot() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let thread_id = ThreadId::new();

    chat.show_resume_paused_goal_prompt(
        thread_id,
        "Keep improving the bare goal command until it feels calm and useful.".to_string(),
    );

    assert_chatwidget_snapshot!(
        "resume_paused_goal_prompt",
        render_bottom_popup(&chat, /*width*/ 100)
    );
}

#[tokio::test]
async fn goal_edit_prompt_snapshot() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let thread_id = ThreadId::new();

    chat.show_goal_edit_prompt(
        thread_id,
        test_goal(
            thread_id,
            AppThreadGoalStatus::Active,
            /*token_budget*/ Some(80_000),
        ),
    );

    assert_chatwidget_snapshot!(
        "goal_edit_prompt",
        render_bottom_popup(&chat, /*width*/ 100)
    );
}

#[tokio::test]
async fn goal_edit_prompt_submits_preserved_status_and_budget() {
    // 编辑可恢复目标时保留原状态和预算，只更新用户确认的目标内容。
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let thread_id = ThreadId::new();

    chat.show_goal_edit_prompt(
        thread_id,
        test_goal(
            thread_id,
            AppThreadGoalStatus::Paused,
            /*token_budget*/ Some(80_000),
        ),
    );
    chat.handle_paste(" with clearer wording".to_string());
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    match rx.try_recv() {
        Ok(AppEvent::SetThreadGoalDraft {
            thread_id: event_thread_id,
            draft,
            mode:
                crate::app_event::ThreadGoalSetMode::UpdateExisting {
                    status,
                    token_budget,
                },
        }) => {
            assert_eq!(event_thread_id, thread_id);
            assert_eq!(
                draft.objective,
                "Keep improving the bare goal command until it feels calm and useful. with clearer wording"
            );
            assert_eq!(status, AppThreadGoalStatus::Paused);
            assert_eq!(token_budget, Some(80_000));
        }
        other => panic!("expected SetThreadGoalDraft event, got {other:?}"),
    }
    assert!(chat.no_modal_or_popup_active());
}

#[tokio::test]
async fn goal_edit_prompt_preserves_resumable_stopped_statuses() {
    for stopped_status in [
        AppThreadGoalStatus::Blocked,
        AppThreadGoalStatus::UsageLimited,
    ] {
        let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
        let thread_id = ThreadId::new();

        chat.show_goal_edit_prompt(
            thread_id,
            test_goal(
                thread_id,
                stopped_status,
                /*token_budget*/ Some(80_000),
            ),
        );
        chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

        match rx.try_recv() {
            Ok(AppEvent::SetThreadGoalDraft {
                mode:
                    crate::app_event::ThreadGoalSetMode::UpdateExisting {
                        status,
                        token_budget,
                    },
                ..
            }) => {
                assert_eq!(status, stopped_status);
                assert_eq!(token_budget, Some(80_000));
            }
            other => panic!("expected SetThreadGoalDraft event, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn goal_edit_prompt_resets_terminal_status_to_active() {
    // 已结束目标重新编辑后必须回到活跃状态，才能继续执行。
    let cases = [
        AppThreadGoalStatus::BudgetLimited,
        AppThreadGoalStatus::Complete,
    ];

    for terminal_status in cases {
        let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
        let thread_id = ThreadId::new();

        chat.show_goal_edit_prompt(
            thread_id,
            test_goal(
                thread_id,
                terminal_status,
                /*token_budget*/ Some(80_000),
            ),
        );
        chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

        match rx.try_recv() {
            Ok(AppEvent::SetThreadGoalDraft {
                mode:
                    crate::app_event::ThreadGoalSetMode::UpdateExisting {
                        status,
                        token_budget,
                    },
                ..
            }) => {
                assert_eq!(status, AppThreadGoalStatus::Active);
                assert_eq!(token_budget, Some(80_000));
            }
            other => panic!("expected SetThreadGoalDraft event, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn resume_paused_goal_prompt_default_resumes_goal() {
    // 暂停目标提示覆盖默认恢复和显式保持暂停两条选择路径。
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let thread_id = ThreadId::new();

    chat.show_resume_paused_goal_prompt(thread_id, "Finish the paused goal.".to_string());
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    match rx.try_recv() {
        Ok(AppEvent::SetThreadGoalStatus {
            thread_id: event_thread_id,
            status,
        }) => {
            assert_eq!(event_thread_id, thread_id);
            assert_eq!(status, AppThreadGoalStatus::Active);
        }
        other => panic!("expected SetThreadGoalStatus event, got {other:?}"),
    }
    assert!(chat.no_modal_or_popup_active());
}

#[tokio::test]
async fn resume_paused_goal_prompt_can_leave_goal_paused() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let thread_id = ThreadId::new();

    chat.show_resume_paused_goal_prompt(thread_id, "Finish the paused goal.".to_string());
    chat.handle_key_event(KeyEvent::from(KeyCode::Down));
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    assert!(chat.no_modal_or_popup_active());
}

fn test_goal(
    thread_id: ThreadId,
    status: AppThreadGoalStatus,
    token_budget: Option<i64>,
) -> AppThreadGoal {
    AppThreadGoal {
        thread_id: thread_id.to_string(),
        objective: "Keep improving the bare goal command until it feels calm and useful."
            .to_string(),
        status,
        token_budget,
        tokens_used: 12_500,
        time_used_seconds: 90,
        created_at: 1_776_272_400,
        updated_at: 1_776_272_460,
    }
}

fn rendered_goal_summary(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<crate::app_event::AppEvent>,
) -> String {
    drain_insert_history(rx)
        .iter()
        .map(|lines| lines_to_single_string(lines))
        .collect::<Vec<_>>()
        .join("\n")
}
