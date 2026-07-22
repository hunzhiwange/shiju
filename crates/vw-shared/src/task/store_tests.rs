use super::*;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_task_pool_persistence_is_disabled_without_creating_sqlite() {
    let project = tempfile::tempdir().expect("temp project");
    let project_path = project.path().to_string_lossy();
    let task = Task::new(1);

    let error = save_task(&project_path, &task).expect_err("native save must be disabled");
    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert!(!get_task_root_dir(&project_path).join("_index.sqlite3").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_task_pool_reads_return_empty_compatibility_views() {
    let project = tempfile::tempdir().expect("temp project");
    let project_path = project.path().to_string_lossy();

    assert!(load_index(&project_path).tasks.is_empty());
    assert!(load_task(&project_path, "T20260717.0001").is_none());
    assert!(load_all_tasks(&project_path).is_empty());
    assert!(!task_file_exists(&project_path, "T20260717.0001"));

    let grouped = load_tasks_by_status(&project_path);
    for status in TaskStatus::all() {
        assert!(grouped.get(&status).is_some_and(Vec::is_empty));
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_task_pool_mutations_fail_closed_as_unsupported() {
    let project = tempfile::tempdir().expect("temp project");
    let project_path = project.path().to_string_lossy();
    let task = Task::new(1);
    let index = TaskIndex::new();

    let errors = [
        save_index(&project_path, &index).expect_err("native index save must be disabled"),
        save_task(&project_path, &task).expect_err("native task save must be disabled"),
        delete_task_file(&project_path, "T20260717.0001")
            .expect_err("native task delete must be disabled"),
        create_task(&project_path, task).expect_err("native task create must be disabled"),
        update_task_status(&project_path, "T20260717.0001", TaskStatus::Completed)
            .expect_err("native status update must be disabled"),
        rebuild_index_from_task_files(&project_path)
            .expect_err("native index rebuild must be disabled"),
    ];

    assert!(errors.iter().all(|error| error.kind() == io::ErrorKind::Unsupported));
    assert!(!get_task_root_dir(&project_path).exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_task_artifact_operations_fail_closed_as_unsupported() {
    let project = tempfile::tempdir().expect("temp project");
    let project_path = project.path().to_string_lossy();
    let payload = serde_json::json!({"result": "ok"});

    let errors = [
        save_task_execution_result(&project_path, "task", &payload)
            .expect_err("native execution result save must be disabled"),
        load_task_execution_result(&project_path, "task")
            .expect_err("native execution result load must be disabled"),
        save_task_code_review_result(&project_path, "task", &payload)
            .expect_err("native review result save must be disabled"),
        load_task_code_review_result(&project_path, "task")
            .expect_err("native review result load must be disabled"),
    ];

    assert!(errors.iter().all(|error| error.kind() == io::ErrorKind::Unsupported));
    assert!(!get_task_root_dir(&project_path).exists());
}

#[test]
fn board_settings_remain_json_presentation_state() {
    let project = tempfile::tempdir().expect("temp project");
    let project_path = project.path().to_string_lossy();
    let settings = super::super::models::TaskBoardSettings::default();

    save_task_board_settings(&project_path, &settings).expect("save settings");
    assert_eq!(
        serde_json::to_value(load_task_board_settings(&project_path)).expect("serialize loaded"),
        serde_json::to_value(settings).expect("serialize expected")
    );
    assert!(get_task_root_dir(&project_path).join("board_settings.json").exists());
}

#[test]
fn invalid_board_settings_fall_back_to_defaults() {
    let project = tempfile::tempdir().expect("temp project");
    let project_path = project.path().to_string_lossy();
    ensure_task_dir(&project_path).expect("create task directory");
    std::fs::write(get_task_root_dir(&project_path).join("board_settings.json"), b"not json")
        .expect("write invalid settings");

    let loaded = load_task_board_settings(&project_path);
    let expected = super::super::models::TaskBoardSettings::default();

    assert_eq!(
        serde_json::to_value(loaded).expect("serialize loaded"),
        serde_json::to_value(expected).expect("serialize expected")
    );
}

#[test]
fn task_paths_stay_under_project_metadata_directory() {
    let project = tempfile::tempdir().expect("temp project");
    let project_path = project.path().to_string_lossy();
    let expected = project.path().join(vw_config_types::paths::HOME_CONFIG_DIR_NAME).join("tasks");

    assert_eq!(get_task_root_dir(&project_path), expected);
    assert_eq!(get_task_logs_dir(&project_path), expected.join("logs"));
    assert_eq!(sanitize_project_path(&project_path), project.path());
}
