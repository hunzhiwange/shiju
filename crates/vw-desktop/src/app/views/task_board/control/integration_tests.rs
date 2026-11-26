//! 员工视图与任务看板更新的集成测试。
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
fn task_pool_office_receives_logical_slots_for_non_git_projects() {
    use crate::app::message::task_board::{TaskBoardMessage, WorktreePoolMaintenanceOutcome, update};

    let mut app = test_app();
    let project_path = "/tmp/non-git-project".to_string();
    app.navigation_tabs.project_path = Some(project_path.clone());
    app.workspace_state.project_is_git_repo = false;
    app.task_board.task_board_worktree_snapshot_loading = true;

    let loaded = snapshot(vec![slot("running-agent", WorktreeState::Busy)]);
    let _ = update(
        &mut app,
        TaskBoardMessage::WorktreeSnapshotLoaded {
            project_path: project_path.clone(),
            result: Ok(Some(loaded)),
        },
    );
    assert!(!app.task_board.task_board_worktree_snapshot_loading);
    let current = app.task_board.task_board_worktree_snapshot.as_ref().unwrap();
    assert_eq!(current.busy_count, 1);

    app.task_board.task_board_worktree_maintenance_in_flight = true;
    let maintained = snapshot(vec![slot("available-station", WorktreeState::Idle)]);
    let _ = update(
        &mut app,
        TaskBoardMessage::WorktreePoolMaintained {
            project_path,
            result: Ok(WorktreePoolMaintenanceOutcome {
                idle_count: maintained.idle_count,
                target_idle_count: 1,
                snapshot: Some(maintained),
            }),
        },
    );
    assert!(!app.task_board.task_board_worktree_maintenance_in_flight);
    let current = app.task_board.task_board_worktree_snapshot.as_ref().unwrap();
    assert_eq!(current.idle_count, 1);
}

#[test]
fn task_pool_office_recycles_available_slots_and_preserves_capacity() {
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
    app.task_board.task_board_worktree_pixel_office = true;
    let _ =
        update(&mut app, TaskBoardMessage::RecycleWorktreeSlotPressed { slot_id: "busy".into() });
    assert!(!app.task_board.task_board_worktree_slot_recycle_in_flight);
    let _ =
        update(&mut app, TaskBoardMessage::RecycleWorktreeSlotPressed { slot_id: "dead".into() });
    assert!(app.task_board.task_board_worktree_slot_recycle_in_flight);
    let _ = super::build_control_buttons(&app);
    let _ = update(
        &mut app,
        TaskBoardMessage::WorktreeSlotRecycled { slot_id: "dead".into(), result: Ok(()) },
    );
    assert!(!app.task_board.task_board_worktree_slot_recycle_in_flight);
    assert_eq!(app.task_board.task_board_worktree_desired_capacity, Some(17));

    let _ = update(&mut app, TaskBoardMessage::ToggleWorktreePixelOffice(false));
    assert!(!app.task_board.task_board_worktree_pixel_office);
    let _ = super::build_control_buttons(&app);
    assert_eq!(app.task_board.task_board_worktree_snapshot.as_ref().unwrap().slots.len(), 5);
}
