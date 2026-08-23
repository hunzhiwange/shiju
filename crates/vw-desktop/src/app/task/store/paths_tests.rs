//! 任务存储路径测试，覆盖项目配置目录派生、目录创建和非法根路径错误。
//! 路径迁移未受支持时应返回清晰的原生切换提示，不回退到旧存储。

use super::*;

#[test]
fn task_paths_are_derived_from_the_project_config_directory() {
    let project = std::path::Path::new("project-root");
    let task_directory = project.join(vw_config_types::paths::HOME_CONFIG_DIR_NAME).join("tasks");

    assert_eq!(get_task_dir("project-root"), task_directory);
    assert_eq!(
        get_task_file_path("project-root", "T20260717.0001"),
        task_directory.join("T20260717.0001.json")
    );
    assert_eq!(get_legacy_index_file_path("project-root"), task_directory.join("_index.json"));
    assert_eq!(get_task_log_dir("project-root"), task_directory.join("logs"));
}

#[test]
fn ensure_task_dir_creates_the_missing_directory_tree() {
    let project = tempfile::tempdir().expect("temp project");
    let project_path = project.path().to_string_lossy();

    ensure_task_dir(&project_path).expect("create task directory");

    assert!(get_task_dir(&project_path).is_dir());
}

#[test]
fn ensure_task_dir_accepts_an_existing_directory() {
    let project = tempfile::tempdir().expect("temp project");
    let project_path = project.path().to_string_lossy();
    ensure_task_dir(&project_path).expect("create task directory");

    ensure_task_dir(&project_path).expect("reuse task directory");

    assert!(get_task_dir(&project_path).is_dir());
}

#[test]
fn ensure_task_dir_returns_error_when_config_root_is_a_file() {
    let project = tempfile::tempdir().expect("temp project");
    let config_root = project.path().join(vw_config_types::paths::HOME_CONFIG_DIR_NAME);
    std::fs::write(config_root, b"not a directory").expect("create blocking file");
    let project_path = project.path().to_string_lossy();

    let result = ensure_task_dir(&project_path);

    assert!(result.is_err());
}

#[test]
fn native_cutover_error_reports_unsupported_scheduler_migration() {
    let error = native_cutover_error();

    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert_eq!(
        error.to_string(),
        "native desktop task persistence moved to the global Agent Scheduler"
    );
}
