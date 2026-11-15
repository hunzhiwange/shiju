#[test]
fn start_execution_rejects_invalid_source_status() {
    use desktop_task_model::{Task, TaskStatus};

    // 来源为 Pool（须先提升到 Pending）时 StartExecution 必须被拒绝，不进入 Running。
    let mut pool_task = Task::new(1);
    assert_eq!(pool_task.status, TaskStatus::Pool);
    assert!(pool_task.start_execution("启动".into()).is_err());
    assert_eq!(pool_task.status, TaskStatus::Pool);
    assert!(pool_task.execution_started_at_ms.is_none());

    // 合法来源 Pending 仍能正常进入 Running。
    let mut pending_task = Task::new(1);
    pending_task.set_status(TaskStatus::Pending);
    assert!(pending_task.start_execution("启动".into()).is_ok());
    assert_eq!(pending_task.status, TaskStatus::Running);
}

#[test]
fn manual_retry_continues_existing_session_only_for_failed_tasks() {
    use desktop_task_model::{Task, TaskStatus};

    let mut failed_task = Task::new(1);
    failed_task.status = TaskStatus::Failed;
    assert!(super::should_continue_existing_session_for_manual_retry(&failed_task, true));

    let mut pending_task = Task::new(1);
    pending_task.status = TaskStatus::Pending;
    assert!(!super::should_continue_existing_session_for_manual_retry(&pending_task, true));
    assert!(!super::should_continue_existing_session_for_manual_retry(&failed_task, false));
}
