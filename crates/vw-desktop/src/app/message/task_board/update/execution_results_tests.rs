#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("execution_results_tests"));
}

#[test]
fn late_success_requires_the_specific_running_timeout_failure() {
    use crate::app::task::{Task, TaskStatus};

    let mut task = Task::new(1);
    task.status = TaskStatus::Failed;
    task.last_error = Some("任务仍未完成，自动标记失败".to_string());
    assert!(super::is_late_success_after_running_timeout(&task, &Ok("done".to_string())));

    assert!(!super::is_late_success_after_running_timeout(&task, &Err("still failed".to_string())));
    task.last_error = Some("普通执行失败".to_string());
    assert!(!super::is_late_success_after_running_timeout(&task, &Ok("done".to_string())));
    task.status = TaskStatus::Running;
    task.last_error = Some("任务仍未完成，自动标记失败".to_string());
    assert!(!super::is_late_success_after_running_timeout(&task, &Ok("done".to_string())));
}

#[test]
fn execution_success_uses_workflow_transition_to_review() {
    use crate::app::task::{Task, TaskStatus};

    // 关闭自动审核：执行成功后只推进到 CodeComplete，后续 review/merge
    // 由 Agent Scheduler command/tick 接管，桌面不再直接推进审核阶段。
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-exec-success-test".to_string());

    let mut task = Task::new(1);
    task.id = "T-exec-success".to_string();
    task.status = TaskStatus::Running;
    task.execution_started_at_ms = Some(crate::app::time::now_ms());
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-exec-success".to_string()];

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskExecutionCompleted {
            project_path: "/tmp/vibewindow-exec-success-test".to_string(),
            task_id: "T-exec-success".to_string(),
            result: Ok("done".to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-exec-success")
        .expect("task should remain in task board");
    assert!(task.logs.iter().any(|entry| {
        entry.status_from == Some(TaskStatus::Running)
            && entry.status_to == Some(TaskStatus::CodeComplete)
    }));
    assert_eq!(task.status, TaskStatus::CodeComplete);
    assert!(task.logs.iter().any(|entry| entry.message.contains("等待代码完成调度")));
    assert!(!task.logs.iter().any(|entry| entry.message.contains("状态转移被状态机拒绝")));
}

#[test]
fn merge_success_uses_workflow_transition_to_completed() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-merge-success-test".to_string());

    let mut task = Task::new(1);
    task.id = "T-merge-success".to_string();
    task.status = TaskStatus::PrSubmitted;
    task.merge_source_branch = Some("vw/task/T-merge-success".to_string());
    task.merge_target_branch = Some("main".to_string());
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-merge-success".to_string()];

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskMergeCompleted {
            project_path: "/tmp/vibewindow-merge-success-test".to_string(),
            task_id: "T-merge-success".to_string(),
            result: Ok("merged".to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-merge-success")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::Completed);
    assert!(task.logs.iter().any(|entry| {
        entry.status_from == Some(TaskStatus::PrSubmitted)
            && entry.status_to == Some(TaskStatus::Completed)
    }));
    assert!(!task.logs.iter().any(|entry| entry.message.contains("状态转移被状态机拒绝")));
}

#[test]
fn merge_failure_preserves_worktree_context() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-merge-failure-test".to_string());
    app.task_board_settings.recycle_worktree_on_task_finish = true;

    let mut task = Task::new(1);
    task.id = "T-merge-failure".to_string();
    task.status = TaskStatus::PrSubmitted;
    task.merge_source_branch = Some("vw/task/T-merge-failure".to_string());
    task.merge_target_branch = Some("main".to_string());
    task.selected_worktree_path = Some("/tmp/vibewindow-merge-worktree".to_string());
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-merge-failure".to_string()];

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskMergeCompleted {
            project_path: "/tmp/vibewindow-merge-failure-test".to_string(),
            task_id: "T-merge-failure".to_string(),
            result: Err("fatal: Unable to create '.git/index.lock': File exists".to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-merge-failure")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::Paused);
    assert_eq!(task.selected_worktree_path.as_deref(), Some("/tmp/vibewindow-merge-worktree"));
    assert!(task.logs.iter().any(|entry| {
        entry.message.contains("合并失败详情")
            && entry.message.contains("source=vw/task/T-merge-failure")
            && entry.message.contains("target=main")
            && entry.message.contains("worktree=/tmp/vibewindow-merge-worktree")
            && entry.message.contains("index.lock")
    }));
    assert!(task.logs.iter().any(|entry| {
        entry.message.contains("合并失败")
            && entry.message.contains("等待网关运行时清理 worktree 槽位")
    }));
}

#[test]
fn code_review_success_without_merge_metadata_pauses_and_recycles_worktree() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-review-missing-metadata-test".to_string());

    let mut task = Task::new(1);
    task.id = "T-review-missing-metadata".to_string();
    task.status = TaskStatus::CodeReview;
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-review-missing-metadata".to_string()];

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskCodeReviewCompleted {
            project_path: "/tmp/vibewindow-review-missing-metadata-test".to_string(),
            task_id: "T-review-missing-metadata".to_string(),
            result: Ok(r#"{"status":"ok"}"#.to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-review-missing-metadata")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::Paused);
    assert!(task.pause_reason.as_deref().unwrap_or_default().contains("缺少有效 source branch"));
    assert!(task.logs.iter().any(|entry| entry.message.contains("合并前校验")));
    assert!(task.logs.iter().any(|entry| {
        entry.message.contains("缺少有效 source branch")
            && entry.message.contains("等待网关运行时清理 worktree 槽位")
    }));
}

#[test]
fn code_review_timeout_error_uses_timeout_pause_reason() {
    use crate::app::task::{Task, TaskStatus};

    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/vibewindow-review-timeout-test".to_string());

    let mut task = Task::new(1);
    task.id = "T-review-timeout".to_string();
    task.status = TaskStatus::CodeReview;
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-review-timeout".to_string()];

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskCodeReviewCompleted {
            project_path: "/tmp/vibewindow-review-timeout-test".to_string(),
            task_id: "T-review-timeout".to_string(),
            result: Err("LLM RESPONSE TIMED OUT".to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-review-timeout")
        .expect("task should remain in task board");
    assert_eq!(task.status, TaskStatus::Paused);
    assert!(task.pause_reason.as_deref().unwrap_or_default().contains("代码审核模型响应超时"));
    assert!(task.logs.iter().any(|entry| {
        entry.message.contains("代码审核模型响应超时")
            && entry.message.contains("等待网关运行时清理 worktree 槽位")
    }));
}
