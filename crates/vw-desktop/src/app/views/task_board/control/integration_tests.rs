// Task 20 ownership: Retain host worktree command lifecycle: update dispatches confirmations, async completion logs and control composition across executor and board.
use crate::app::{App, task::{WorktreeSlotSnapshot, WorktreePoolSnapshot, WorktreeState}};
fn test_app() -> App { App::new().0 }
fn slot(id: &str, state: WorktreeState) -> WorktreeSlotSnapshot {
    WorktreeSlotSnapshot {
        agent_id: None,
        run_id: None,
        id: id.to_string(),
        path: format!("/tmp/{id}"),
        branch: format!("task/{id}"),
        state,
        leased_task_id: matches!(state, WorktreeState::Busy).then(|| format!("task-{id}")),
        taint_reason: matches!(state, WorktreeState::Tainted).then(|| "dirty tree".to_string()),
    }
}

fn snapshot(slots: Vec<WorktreeSlotSnapshot>) -> WorktreePoolSnapshot {
    WorktreePoolSnapshot {
        repo_root: "/repo".to_string(),
        pool_root: "/repo/.worktrees".to_string(),
        base_branch: "main".to_string(),
        idle_count: slots.iter().filter(|slot| slot.state == WorktreeState::Idle).count(),
        busy_count: slots.iter().filter(|slot| slot.state == WorktreeState::Busy).count(),
        tainted_count: slots.iter().filter(|slot| slot.state == WorktreeState::Tainted).count(),
        recycling_count: slots.iter().filter(|slot| slot.state == WorktreeState::Recycling).count(),
        dead_count: slots.iter().filter(|slot| slot.state == WorktreeState::Dead).count(),
        merge_target_locks: Vec::new(),
        slots,
    }
}

#[test]
fn task_pool_office_capacity_and_all_employee_interactions_are_preserved() {
    let mut app = test_app();
    let project = tempfile::tempdir().unwrap();
    app.navigation_tabs.project_path = Some(project.path().to_string_lossy().into_owned());
    app.task_board.task_board_worktree_snapshot = Some(snapshot(vec![
        slot("idle", WorktreeState::Idle),
        slot("busy", WorktreeState::Busy),
        slot("tainted", WorktreeState::Tainted),
        slot("recycling", WorktreeState::Recycling),
        slot("dead", WorktreeState::Dead),
    ]));
    use crate::app::message::{TaskBoardMessage, task_board::update};
    app.task_board.task_board_worktree_desired_capacity = Some(17);
    for (message, kind) in [
        (TaskBoardMessage::CleanAllWorktreesPressed, "cleanup"),
        (TaskBoardMessage::DeleteAllWorktreesPressed, "delete"),
        (TaskBoardMessage::CommitMergeAllWorktreesPressed, "merge"),
    ] {
        let _ = update(&mut app, message);
        assert_eq!(app.task_board.task_board_worktree_manual_confirm_kind, Some(kind));
        let _ = update(&mut app, TaskBoardMessage::CancelWorktreeManualConfirm);
        assert_eq!(app.task_board.task_board_worktree_manual_confirm_kind, None);
        assert_eq!(app.task_board.task_board_worktree_desired_capacity, Some(17));
        let press = || match kind {
            "cleanup" => TaskBoardMessage::CleanAllWorktreesPressed,
            "delete" => TaskBoardMessage::DeleteAllWorktreesPressed,
            _ => TaskBoardMessage::CommitMergeAllWorktreesPressed,
        };
        let _ = update(&mut app, press());
        let _ = update(&mut app, press());
        assert_eq!(app.task_board.task_board_worktree_manual_action_kind, Some(kind));
        assert!(!app.task_board.task_board_worktree_action_logs.is_empty());
        let error = Err("显式任务未配置".to_string());
        let completed = match kind {
            "cleanup" => TaskBoardMessage::CleanAllWorktreesCompleted(error),
            "delete" => TaskBoardMessage::DeleteAllWorktreesCompleted(error),
            _ => TaskBoardMessage::CommitMergeAllWorktreesCompleted(error),
        };
        let _ = update(&mut app, completed);
        assert!(app.task_board.task_board_worktree_manual_action_kind.is_none());
        assert!(
            app.task_board.task_board_worktree_action_logs.iter().any(|line| line.contains("显式任务未配置"))
        );
        assert_eq!(app.task_board.task_board_worktree_desired_capacity, Some(17));
    }
    let _ =
        update(&mut app, TaskBoardMessage::RecycleWorktreeSlotPressed { slot_id: "busy".into() });
    assert!(app.task_board.task_board_worktree_manual_action_kind.is_none());
    let _ =
        update(&mut app, TaskBoardMessage::RecycleWorktreeSlotPressed { slot_id: "dead".into() });
    assert_eq!(app.task_board.task_board_worktree_manual_action_kind, Some("slot_recycle"));
    // 丢弃未轮询的异步命令，测试不连接生产网关。
    app.task_board.task_board_worktree_manual_action_kind = None;
    let expanded = app.task_board.task_board_worktree_panel_expanded;
    let _ = update(&mut app, TaskBoardMessage::ToggleWorktreePanelExpanded);
    assert_eq!(app.task_board.task_board_worktree_panel_expanded, !expanded);
    let _ = update(&mut app, TaskBoardMessage::ToggleWorktreePixelOffice(false));
    assert!(!app.task_board.task_board_worktree_pixel_office);
    for expanded in [false, true] {
        for pixel in [false, true] {
            app.task_board.task_board_worktree_panel_expanded = expanded;
            app.task_board.task_board_worktree_pixel_office = pixel;
            let _ = super::build_control_buttons(&app);
            assert_eq!(app.task_board.task_board_worktree_snapshot.as_ref().unwrap().slots.len(), 5);
        }
    }
}
