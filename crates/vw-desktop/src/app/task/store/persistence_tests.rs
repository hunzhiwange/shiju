#![cfg(not(target_arch = "wasm32"))]

//! 原生任务持久化切换测试，确认旧索引与任务文件不再被读取或改写。
//! 所有遗留写操作都应失败关闭，并完整保留磁盘上的既有文件。

use super::*;

fn legacy_task_dir(project: &std::path::Path) -> std::path::PathBuf {
    project.join(vw_config_types::paths::HOME_CONFIG_DIR_NAME).join("tasks")
}

fn assert_native_cutover(error: io::Error) {
    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert_eq!(
        error.to_string(),
        "native desktop task persistence moved to the global Agent Scheduler"
    );
}

#[test]
fn native_rebuild_index_fails_closed_without_creating_task_storage() {
    let project = tempfile::tempdir().expect("temp project");
    let path = project.path().to_string_lossy();

    let error = rebuild_index_from_task_files(&path).expect_err("native rebuild disabled");

    assert_native_cutover(error);
    assert!(!legacy_task_dir(project.path()).exists());
}

#[test]
fn native_load_index_ignores_legacy_index_and_returns_fresh_state() {
    let project = tempfile::tempdir().expect("temp project");
    let task_dir = legacy_task_dir(project.path());
    std::fs::create_dir_all(&task_dir).expect("legacy task directory");
    let mut legacy_index = TaskIndex::new();
    legacy_index.tasks.insert("T20260717.0001".to_string(), "running".to_string());
    legacy_index.last_task_date = Some("20260717".to_string());
    legacy_index.last_task_seq = 1;
    std::fs::write(
        task_dir.join("_index.json"),
        serde_json::to_vec(&legacy_index).expect("serialize legacy index"),
    )
    .expect("write legacy index");
    let path = project.path().to_string_lossy();

    let index = load_index(&path);

    assert!(index.tasks.is_empty());
    assert!(index.order_by_status.values().all(Vec::is_empty));
    assert_eq!(index.last_task_date, None);
    assert_eq!(index.last_task_seq, 0);
}

#[test]
fn native_save_index_fails_closed_without_writing_legacy_index() {
    let project = tempfile::tempdir().expect("temp project");
    let path = project.path().to_string_lossy();
    let index = TaskIndex::new();

    let error = save_index(&path, &index).expect_err("native index save disabled");

    assert_native_cutover(error);
    assert!(!legacy_task_dir(project.path()).join("_index.json").exists());
}

#[test]
fn native_load_task_ignores_existing_legacy_task_file() {
    let project = tempfile::tempdir().expect("temp project");
    let task_dir = legacy_task_dir(project.path());
    std::fs::create_dir_all(&task_dir).expect("legacy task directory");
    let mut task = Task::new(1);
    task.id = "T20260717.0001".to_string();
    std::fs::write(
        task_dir.join(format!("{}.json", task.id)),
        serde_json::to_vec(&task).expect("serialize legacy task"),
    )
    .expect("write legacy task");
    let path = project.path().to_string_lossy();

    assert!(load_task(&path, &task.id).is_none());
}

#[test]
fn native_save_task_fails_closed_without_creating_task_storage() {
    let project = tempfile::tempdir().expect("temp project");
    let path = project.path().to_string_lossy();
    let task = Task::new(1);
    let error = save_task(&path, &task).expect_err("native save disabled");

    assert_native_cutover(error);
    assert!(!legacy_task_dir(project.path()).exists());
}

#[test]
fn native_delete_task_fails_closed_and_preserves_legacy_file() {
    let project = tempfile::tempdir().expect("temp project");
    let task_dir = legacy_task_dir(project.path());
    std::fs::create_dir_all(&task_dir).expect("legacy task directory");
    let task_id = "T20260717.0001";
    let task_file = task_dir.join(format!("{task_id}.json"));
    std::fs::write(&task_file, b"legacy task").expect("write legacy task");
    let path = project.path().to_string_lossy();

    let error = delete_task_file(&path, task_id).expect_err("native delete disabled");

    assert_native_cutover(error);
    assert_eq!(std::fs::read(&task_file).expect("preserved task"), b"legacy task");
}

#[test]
fn native_load_all_tasks_ignores_legacy_index_and_task_files() {
    let project = tempfile::tempdir().expect("temp project");
    let task_dir = legacy_task_dir(project.path());
    std::fs::create_dir_all(&task_dir).expect("legacy task directory");
    let mut task = Task::new(1);
    task.id = "T20260717.0001".to_string();
    std::fs::write(
        task_dir.join(format!("{}.json", task.id)),
        serde_json::to_vec(&task).expect("serialize legacy task"),
    )
    .expect("write legacy task");
    let mut index = TaskIndex::new();
    index.tasks.insert(task.id.clone(), task.status.to_string_key().to_string());
    std::fs::write(
        task_dir.join("_index.json"),
        serde_json::to_vec(&index).expect("serialize legacy index"),
    )
    .expect("write legacy index");
    let path = project.path().to_string_lossy();

    assert!(load_all_tasks(&path).is_empty());
}
