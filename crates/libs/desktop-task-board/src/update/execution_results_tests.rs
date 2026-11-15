#[test]
fn late_success_requires_the_specific_running_timeout_failure() {
    use desktop_task_model::{Task, TaskStatus};

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
