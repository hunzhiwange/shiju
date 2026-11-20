use super::*;

fn snapshot(
    task_id: &str,
    status_key: &str,
) -> vw_api_agent_scheduler::AgentSchedulerTaskBoardTaskDto {
    use vw_api_agent_scheduler::{
        AgentSchedulerTaskBoardTaskDto, AgentSchedulerTaskBoardTaskReadModelDto,
    };

    AgentSchedulerTaskBoardTaskDto {
        placement: None,
        project_id: "project-test".into(),
        task_id: task_id.to_string().into(),
        status_key: status_key.to_string(),
        status: None,
        priority: 1,
        order_no: 0,
        model: "auto".to_string(),
        agent: None,
        acp_agent: None,
        reasoning_effort: None,
        prompt: "test".to_string(),
        retry_count: 0,
        run_id: None,
        run_stage: None,
        workspace: None,
        last_error: None,
        pause_reason: None,
        merge_source_branch: None,
        merge_target_branch: None,
        selected_worktree_path: None,
        selected_worktree_slot_id: None,
        selected_worktree_branch: None,
        read_model: AgentSchedulerTaskBoardTaskReadModelDto::default(),
        updated_at_ms: vw_api_transport::TimestampMs(0),
    }
}

fn response(
    task_id: &str,
    status_key: &str,
) -> vw_api_agent_scheduler::AgentSchedulerTaskBoardStatusResponse {
    use vw_api_agent_scheduler::{
        AgentSchedulerTaskBoardSnapshotReadModelDto, AgentSchedulerTaskBoardStatusResponse,
    };

    AgentSchedulerTaskBoardStatusResponse {
        project_id: "project-test".into(),
        read_model: AgentSchedulerTaskBoardSnapshotReadModelDto::default(),
        tasks: vec![snapshot(task_id, status_key)],
    }
}

#[test]
fn persistence_ack_releases_snapshot_guard() {
    for result in [Ok(()), Err("gateway unavailable".to_string())] {
        let (mut app, _task) = super::fixture_pair();
        app.project_path = Some("/tmp/vibewindow-persistence-ack-test".to_string());
        let mut task = Task::new(1);
        task.id = "T-guarded".to_string();
        task.status = TaskStatus::Completed;
        app.task_board_tasks = vec![task];
        app.task_board_tasks_pending_persistence.insert("T-guarded".to_string());

        projection_state(&mut app).acknowledge("T-guarded", &result);
        projection_state(&mut app).status(response("T-guarded", "running"));

        assert!(!app.task_board_tasks_pending_persistence.contains("T-guarded"));
        assert_eq!(app.task_board_tasks[0].status, TaskStatus::Running);
    }
}

#[test]
fn work_plan_detail_entry_selects_basic_and_preserves_log_entry() {
    use crate::message::{TaskBoardDetailTab, TaskBoardMessage};
    use crate::update::local::{LocalEffect, update_local};

    let mut app = Fixture::default();
    app.project_path = Some("/work".into());
    app.task_board_tasks.push(Task {
        id: "task-1".into(),
        status: TaskStatus::Running,
        prompt: "task prompt".into(),
        ..Task::default()
    });

    let opened = update_local(
        local_state(&mut app),
        TaskBoardMessage::ViewTaskLogs { project_path: "/work".into(), task_id: "task-1".into() },
        "main",
    );
    assert!(matches!(opened, Ok(LocalEffect::OpenDetail(task)) if task.id == "task-1"));
    assert_eq!(app.task_board_detail_tab, TaskBoardDetailTab::Logs);
    assert_eq!(app.task_board_editing_task_id.as_deref(), Some("task-1"));
    assert_eq!(app.task_board_draft.prompt, "task prompt");

    let opened = update_local(
        local_state(&mut app),
        TaskBoardMessage::OpenTaskDetail { project_path: "/work".into(), task_id: "task-1".into() },
        "main",
    );
    assert!(matches!(opened, Ok(LocalEffect::OpenDetail(task)) if task.status == TaskStatus::Running));
    assert_eq!(app.task_board_detail_tab, TaskBoardDetailTab::Basic);

    app.task_board_draft.prompt = "unsaved edit".into();
    for tab in [TaskBoardDetailTab::Logs, TaskBoardDetailTab::Basic] {
        let _ = update_local(local_state(&mut app), TaskBoardMessage::SelectDetailTab(tab), "main");
        assert_eq!(app.task_board_detail_tab, tab);
        assert_eq!(app.task_board_draft.prompt, "unsaved edit");
    }
}
