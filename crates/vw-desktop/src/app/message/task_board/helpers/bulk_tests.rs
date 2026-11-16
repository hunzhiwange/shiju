// 宿主保留网关空操作、草稿投影、选择状态与合并队列任务接线；元数据转换在 desktop-task-board 独立测试。
// 验证任务看板批量选择对状态、归档与删除项的过滤。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use crate::app::task::{Task, TaskStatus};

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

#[test]
fn manual_merge_without_eligible_task_does_not_notify() {
    let (mut app, _) = crate::app::App::new();
    app.project_path = Some("/tmp/project".into());
    app.task_board_tasks = vec![task("T-pending", TaskStatus::Pending)];
    app.task_board_manual_merge_queue = vec!["T-missing".into(), "T-pending".into()];
    app.notifications.clear();
    let _ = super::start_manual_merge_queue_task(&mut app);
    assert!(app.notifications.is_empty());
}
