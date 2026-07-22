use super::*;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn load_tasks_by_status_returns_every_builtin_bucket_empty_on_native() {
    let project = tempfile::tempdir().expect("temp project");

    let tasks_by_status = load_tasks_by_status(&project.path().to_string_lossy());

    assert_eq!(tasks_by_status.len(), TaskStatus::all().len());
    for status in TaskStatus::all() {
        let tasks = tasks_by_status.get(&status).expect("built-in status bucket");
        assert!(tasks.is_empty());
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_create_task_fails_closed_after_scheduler_cutover() {
    let project = tempfile::tempdir().expect("temp project");
    let path = project.path().to_string_lossy();
    let error = create_task(&path, Task::new(1)).expect_err("native create disabled");

    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert!(error.to_string().contains("global Agent Scheduler"));
    assert!(!super::super::paths::get_task_log_dir(&path).exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_rebuild_failed_task_fails_closed_without_removing_logs() {
    let project = tempfile::tempdir().expect("temp project");
    let path = project.path().to_string_lossy();
    let old_task = Task::new(1);
    let error = rebuild_failed_task_from_current_branch(&path, old_task)
        .expect_err("native rebuild disabled");

    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert!(error.to_string().contains("global Agent Scheduler"));
    assert!(!super::super::paths::get_task_log_dir(&path).exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_update_task_fails_closed_without_creating_task_storage() {
    let project = tempfile::tempdir().expect("temp project");
    let path = project.path().to_string_lossy();
    let error = update_task(&path, &Task::new(1)).expect_err("native update disabled");

    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert!(error.to_string().contains("global Agent Scheduler"));
    assert!(!super::super::paths::get_task_log_dir(&path).exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_reorder_tasks_fails_closed_without_creating_task_storage() {
    let project = tempfile::tempdir().expect("temp project");
    let path = project.path().to_string_lossy();
    let error =
        reorder_tasks_in_status(&path, TaskStatus::Pending, vec!["T20260717.0001".to_string()])
            .expect_err("native reorder disabled");

    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert!(error.to_string().contains("global Agent Scheduler"));
    assert!(!super::super::paths::get_task_log_dir(&path).exists());
}
