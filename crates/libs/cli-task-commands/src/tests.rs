//! 本地任务命令辅助行为的回归测试。
//! 覆盖首个非空行、项目目录解析在成功与错误边界下的结果。
//! 任务 JSON 输出场景则使用典型数据确认单项和列表路径均可安全执行。

use std::fs;

use vw_shared::task::Task;

use super::{
    first_non_empty_line, print_task_json, print_tasks_json, resolve_project_dir,
};

#[test]
fn first_non_empty_line_trims_and_returns_empty_when_content_is_blank() {
    assert_eq!(first_non_empty_line("\n \t\nfirst\nsecond"), "first");
    assert_eq!(first_non_empty_line("  leading  \nsecond"), "leading");
    assert_eq!(first_non_empty_line("\n \t"), "");
}

#[test]
fn resolve_project_dir_rejects_empty_missing_and_file_paths() {
    assert!(
        resolve_project_dir("   ")
            .unwrap_err()
            .to_string()
            .contains("--project-dir cannot be empty")
    );
    assert!(resolve_project_dir("/tmp/vibewindow-missing-project-dir-for-test").is_err());

    let file_path =
        std::env::temp_dir().join(format!("vw-cli-resolve-project-file-{}", std::process::id()));
    fs::write(&file_path, "not a directory").unwrap();
    let error = resolve_project_dir(file_path.to_str().unwrap()).unwrap_err().to_string();
    fs::remove_file(&file_path).unwrap();

    assert!(error.contains("project directory is not a folder"));
}

#[test]
fn resolve_project_dir_returns_canonical_directory() {
    let dir_path =
        std::env::temp_dir().join(format!("vw-cli-resolve-project-dir-{}", std::process::id()));
    fs::create_dir_all(&dir_path).unwrap();

    let resolved = resolve_project_dir(dir_path.to_str().unwrap()).unwrap();
    let expected = fs::canonicalize(&dir_path).unwrap().to_string_lossy().to_string();
    fs::remove_dir(&dir_path).unwrap();

    assert_eq!(resolved, expected);
}

#[test]
fn first_non_empty_line_handles_edge_inputs() {
    assert_eq!(first_non_empty_line(""), "");
    assert_eq!(first_non_empty_line("   "), "");
    assert_eq!(first_non_empty_line("\t\n "), "");
    assert_eq!(first_non_empty_line(&"x".repeat(10_000)).len(), 10_000);
    assert_eq!(first_non_empty_line("single"), "single");
    assert_eq!(first_non_empty_line("中文\n第二行"), "中文");
    assert_eq!(first_non_empty_line("emoji 🦀\nsecond"), "emoji 🦀");
}

#[test]
fn resolve_project_dir_dot_returns_canonical_current_dir() {
    let resolved = resolve_project_dir(".").unwrap();
    let expected = std::fs::canonicalize(".").unwrap().to_string_lossy().to_string();
    assert_eq!(resolved, expected);
}

#[test]
fn resolve_project_dir_rejects_whitespace_only_and_empty_string() {
    let err_ws = resolve_project_dir("   ").unwrap_err().to_string();
    assert!(err_ws.contains("--project-dir cannot be empty"), "unexpected error: {err_ws}");

    let err_empty = resolve_project_dir("").unwrap_err().to_string();
    assert!(err_empty.contains("--project-dir cannot be empty"), "unexpected error: {err_empty}");
}

#[test]
fn print_task_json_and_tasks_no_panic_for_typical_inputs() {
    let task = Task::new(7);

    // Just verify it doesn't panic — output goes to stdout
    print_task_json(&task).unwrap();
    print_tasks_json(&[] as &[Task]).unwrap();
    print_tasks_json(&[Task::new(1)]).unwrap();
}

#[test]
fn resolve_project_dir_non_existent_path_returns_error() {
    let temp = std::env::temp_dir();
    let bad_path = temp.join(format!("vw-cli-nonexistent-{}", std::process::id()));
    let err = resolve_project_dir(bad_path.to_str().unwrap()).unwrap_err();
    assert!(
        err.to_string().contains("Failed to resolve project directory"),
        "unexpected error: {err}"
    );
}
