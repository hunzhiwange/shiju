// 宿主保留网关空操作、草稿投影、选择状态与合并队列任务接线；元数据转换在 desktop-task-board 独立测试。
// 验证任务看板批量选择对状态、归档与删除项的过滤。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use crate::app::task::{Task, TaskSplitMode, TaskStatus};

fn task(id: &str, status: TaskStatus) -> Task {
    let mut task = Task::new(1);
    task.id = id.to_string();
    task.status = status;
    task
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。

#[tokio::test(flavor = "multi_thread")]
async fn empty_bulk_command_short_circuits_without_gateway_access() {
    let result =
        super::submit_agent_scheduler_bulk_command("", Vec::new(), "empty operation").await;

    assert_eq!(result, Ok(()));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[tokio::test]
async fn empty_gateway_bulk_wrappers_short_circuit_without_gateway_access() {
    assert_eq!(
        super::persist_updated_tasks_via_gateway("", &[], "批量更新", "workflow").await,
        Ok(())
    );
    assert_eq!(super::delete_tasks_via_gateway("", &[]).await, Ok(()));
    assert_eq!(super::send_archive_commands_via_gateway("", &[]).await, Ok(()));
    assert_eq!(
        super::move_tasks_to_status_via_gateway("", &[], TaskStatus::Running, "workflow").await,
        Ok(())
    );
}

#[test]
fn reset_bulk_operation_inputs_copies_draft_and_recent_choices() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_draft.priority = " 17 ".to_string();
    app.task_board_draft.agent = Some("specialist".to_string());
    app.task_board_draft.task_split_mode = TaskSplitMode::Disabled;
    app.task_board_last_model = "  provider/model  ".to_string();
    app.task_board_last_acp_agent = Some("codex".to_string());

    super::reset_bulk_operation_inputs(&mut app);

    assert_eq!(app.task_board_bulk_priority_input, "17");
    assert_eq!(app.task_board_bulk_model_input, "provider/model");
    assert_eq!(app.task_board_bulk_agent, "specialist");
    assert_eq!(app.task_board_bulk_acp_agent.as_deref(), Some("codex"));
    assert_eq!(app.task_board_bulk_reasoning_effort, app.task_board_draft.reasoning_effort);
    assert_eq!(app.task_board_bulk_task_split_mode, TaskSplitMode::Disabled);
}

#[test]
fn reset_bulk_operation_inputs_uses_priority_model_and_agent_defaults() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_draft.priority = "0".to_string();
    app.task_board_draft.agent = None;
    app.task_board_last_model = "   ".to_string();

    super::reset_bulk_operation_inputs(&mut app);

    assert_eq!(app.task_board_bulk_priority_input, "999");
    assert_eq!(app.task_board_bulk_model_input, crate::app::task::TASK_MODEL_AUTO);
    assert_eq!(app.task_board_bulk_agent, crate::app::task::TASK_AGENT_MAIN);
}

#[test]
fn deactivate_bulk_selection_mode_clears_only_active_status_selection() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_tasks =
        vec![task("T-pending", TaskStatus::Pending), task("T-running", TaskStatus::Running)];
    app.task_board_selected_tasks.extend(["T-pending".to_string(), "T-running".to_string()]);
    app.task_board_bulk_active_status = Some(TaskStatus::Pending);

    super::deactivate_bulk_selection_mode(&mut app);

    assert_eq!(app.task_board_bulk_active_status, None);
    assert!(!app.task_board_selected_tasks.contains("T-pending"));
    assert!(app.task_board_selected_tasks.contains("T-running"));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn deactivate_bulk_selection_mode_without_active_status_preserves_selection() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_selected_tasks.insert("T-selected".to_string());

    super::deactivate_bulk_selection_mode(&mut app);

    assert!(app.task_board_selected_tasks.contains("T-selected"));
}

#[test]
fn manual_merge_running_requires_running_pr_submitted_board_task() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_tasks = vec![
        task("T-running-status", TaskStatus::Running),
        task("T-merge", TaskStatus::PrSubmitted),
    ];
    app.task_board_executor.running_tasks =
        vec!["T-missing".to_string(), "T-running-status".to_string()];

    assert!(!super::manual_merge_running(&app));

    app.task_board_executor.running_tasks.push("T-merge".to_string());
    assert!(super::manual_merge_running(&app));
}

#[test]
fn pop_next_manual_merge_task_discards_invalid_entries_and_returns_first_eligible_task() {
    let (mut app, _task) = crate::app::App::new();
    let wrong_status = task("T-wrong-status", TaskStatus::Running);
    let mut deleted = task("T-deleted", TaskStatus::PrSubmitted);
    deleted.deleted = true;
    let mut archived = task("T-archived", TaskStatus::PrSubmitted);
    archived.archived = true;
    let already_running = task("T-running", TaskStatus::PrSubmitted);
    let eligible = task("T-eligible", TaskStatus::PrSubmitted);
    app.task_board_tasks = vec![wrong_status, deleted, archived, already_running, eligible];
    app.task_board_executor.running_tasks.push("T-running".to_string());
    app.task_board_manual_merge_queue = vec![
        "T-missing".to_string(),
        "T-wrong-status".to_string(),
        "T-deleted".to_string(),
        "T-archived".to_string(),
        "T-running".to_string(),
        "T-eligible".to_string(),
        "T-later".to_string(),
    ];

    let mut selected = super::pop_next_manual_merge_task(&mut app).expect("eligible merge task");

    assert_eq!(selected.id, "T-eligible");
    assert_eq!(app.task_board_manual_merge_queue, vec!["T-later".to_string()]);
    selected.prompt = "changed clone".to_string();
    assert_eq!(app.task_board_tasks[4].prompt, "");
}

#[test]
fn pop_next_manual_merge_task_drains_queue_without_eligible_task() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_tasks = vec![task("T-pending", TaskStatus::Pending)];
    app.task_board_manual_merge_queue = vec!["T-missing".to_string(), "T-pending".to_string()];

    assert!(super::pop_next_manual_merge_task(&mut app).is_none());
    assert!(app.task_board_manual_merge_queue.is_empty());
}

#[test]
fn start_manual_merge_queue_task_returns_none_when_pr_merge_is_already_running() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/project".into());
    app.task_board_tasks = vec![task("T-running", TaskStatus::PrSubmitted)];
    app.task_board_executor.running_tasks = vec!["T-running".to_string()];
    app.task_board_manual_merge_queue = vec!["T-running".to_string(), "T-later".to_string()];

    let _command = super::start_manual_merge_queue_task(&mut app);

    assert_eq!(
        app.task_board_manual_merge_queue,
        vec!["T-running".to_string(), "T-later".to_string()]
    );
    assert_eq!(app.task_board_tasks[0].logs.len(), 1);
}

#[test]
fn start_manual_merge_queue_task_with_project_and_no_eligible_task_drains_invalid_queue() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/project".into());
    app.task_board_tasks = vec![task("T-pending", TaskStatus::Pending)];
    app.task_board_manual_merge_queue = vec!["T-missing".to_string(), "T-pending".to_string()];
    app.notifications.clear();

    let _command = super::start_manual_merge_queue_task(&mut app);

    assert!(app.task_board_manual_merge_queue.is_empty());
    assert!(app.notifications.is_empty());
}

#[test]
fn start_manual_merge_queue_task_with_eligible_task_adds_log_and_pops_queue() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/project".into());
    app.task_board_tasks = vec![task("T-merge", TaskStatus::PrSubmitted)];
    app.task_board_manual_merge_queue = vec!["T-merge".to_string(), "T-later".to_string()];

    let _command = super::start_manual_merge_queue_task(&mut app);

    assert_eq!(app.task_board_manual_merge_queue, vec!["T-later".to_string()]);
    assert_eq!(app.task_board_tasks[0].logs.len(), 2);
    assert!(
        app.task_board_tasks[0].logs[1]
            .message
            .contains("手动批量合并已发送 Agent Scheduler 合并命令")
    );
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn start_manual_merge_queue_task_without_project_clears_queue_and_notifies() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = None;
    app.task_board_tasks = vec![task("T-merge", TaskStatus::PrSubmitted)];
    app.task_board_manual_merge_queue = vec!["T-merge".to_string()];
    app.notifications.clear();

    let _command = super::start_manual_merge_queue_task(&mut app);

    assert!(app.task_board_manual_merge_queue.is_empty());
    assert_eq!(app.notifications.len(), 1);
    assert_eq!(app.notifications[0].message, "缺少项目路径，无法执行批量合并");
}
