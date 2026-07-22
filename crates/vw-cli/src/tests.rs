use std::fs;

use vw_shared::task::Task;

use super::{
    first_non_empty_line, parse_temperature, print_task_json, print_tasks_json, resolve_project_dir,
};

#[test]
fn parse_temperature_accepts_inclusive_bounds_and_rejects_invalid_values() {
    assert_eq!(parse_temperature("0").unwrap(), 0.0);
    assert_eq!(parse_temperature("2.0").unwrap(), 2.0);
    assert_eq!(parse_temperature("2.01").unwrap_err(), "temperature must be between 0.0 and 2.0");
    assert!(parse_temperature("not-a-number").is_err());
}

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
fn parse_temperature_rejects_negative_and_out_of_range_values() {
    let err_neg = parse_temperature("-1").unwrap_err();
    assert!(err_neg.contains("between 0.0 and 2.0"), "unexpected error: {err_neg}");

    let err_slightly_above = parse_temperature("2.0001").unwrap_err();
    assert!(
        err_slightly_above.contains("between 0.0 and 2.0"),
        "unexpected error: {err_slightly_above}"
    );

    let err_slightly_below = parse_temperature("-0.0001").unwrap_err();
    assert!(
        err_slightly_below.contains("between 0.0 and 2.0"),
        "unexpected error: {err_slightly_below}"
    );
}

#[test]
fn parse_temperature_succeeds_within_range() {
    assert_eq!(parse_temperature("0.5").unwrap(), 0.5);
    assert_eq!(parse_temperature("1.0").unwrap(), 1.0);
    assert_eq!(parse_temperature("1.5").unwrap(), 1.5);
}

#[test]
fn parse_temperature_propagates_parse_error_formatted() {
    let err = parse_temperature("").unwrap_err();
    assert!(err.contains("cannot parse"), "unexpected error: {err}");

    let err = parse_temperature("abc").unwrap_err();
    assert!(err.contains("invalid float"), "unexpected error: {err}");
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
fn parse_temperature_zero_and_two_exact_boundaries() {
    assert_eq!(parse_temperature("0.0").unwrap(), 0.0);
    assert_eq!(parse_temperature("2.0").unwrap(), 2.0);
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
