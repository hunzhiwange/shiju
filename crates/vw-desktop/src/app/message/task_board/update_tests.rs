use super::*;

fn snapshot(
    task_id: &str,
    status_key: &str,
) -> vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerTaskBoardTaskDto {
    use vw_gateway_client::vw_api_types::agent_scheduler::{
        AgentSchedulerTaskBoardTaskDto, AgentSchedulerTaskBoardTaskReadModelDto,
    };

    AgentSchedulerTaskBoardTaskDto {
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
        updated_at_ms: vw_gateway_client::vw_api_types::common::TimestampMs(0),
    }
}

fn response(
    task_id: &str,
    status_key: &str,
) -> vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerTaskBoardStatusResponse {
    use vw_gateway_client::vw_api_types::agent_scheduler::{
        AgentSchedulerTaskBoardSnapshotReadModelDto, AgentSchedulerTaskBoardStatusResponse,
    };

    AgentSchedulerTaskBoardStatusResponse {
        project_id: "project-test".into(),
        read_model: AgentSchedulerTaskBoardSnapshotReadModelDto::default(),
        tasks: vec![snapshot(task_id, status_key)],
    }
}

#[test]
fn agent_scheduler_snapshot_preserves_task_pending_persistence() {
    let (mut app, _task) = crate::app::App::new();
    let mut task = Task::new(1);
    task.id = "T-guarded".to_string();
    task.status = TaskStatus::Completed;
    app.task_board_tasks = vec![task];
    app.task_board_tasks_pending_persistence.insert("T-guarded".to_string());

    apply_agent_scheduler_status(&mut app, response("T-guarded", "running"));

    assert_eq!(app.task_board_tasks[0].status, TaskStatus::Completed);
}

#[test]
fn persistence_ack_releases_snapshot_guard() {
    for result in [Ok(()), Err("gateway unavailable".to_string())] {
        let (mut app, _task) = crate::app::App::new();
        app.project_path = Some("/tmp/vibewindow-persistence-ack-test".to_string());
        let mut task = Task::new(1);
        task.id = "T-guarded".to_string();
        task.status = TaskStatus::Completed;
        app.task_board_tasks = vec![task];
        app.task_board_tasks_pending_persistence.insert("T-guarded".to_string());

        let _task = update(
            &mut app,
            TaskBoardMessage::TaskResultPersistenceCompleted {
                project_path: "/tmp/vibewindow-persistence-ack-test".to_string(),
                task_id: "T-guarded".to_string(),
                result,
            },
        );
        apply_agent_scheduler_status(&mut app, response("T-guarded", "running"));

        assert!(!app.task_board_tasks_pending_persistence.contains("T-guarded"));
        assert_eq!(app.task_board_tasks[0].status, TaskStatus::Running);
    }
}
