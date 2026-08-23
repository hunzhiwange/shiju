//! 验证文件通配搜索的匹配结果、排序、路径边界与调用频率限制。

use super::super::*;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;

fn test_security(workspace: PathBuf) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: workspace,
        ..SecurityPolicy::default()
    })
}

fn test_security_with(
    workspace: PathBuf,
    autonomy: AutonomyLevel,
    max_actions_per_hour: u32,
) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy,
        workspace_dir: workspace,
        max_actions_per_hour,
        ..SecurityPolicy::default()
    })
}

#[test]
fn glob_search_name_and_schema() {
    let tool = GlobSearchTool::new(test_security(std::env::temp_dir()));
    assert_eq!(tool.name(), "glob_search");
    assert!(tool.description().contains("glob pattern"));
    assert!(tool.description().contains("src/**/mod.rs"));

    let schema = tool.parameters_schema();
    assert!(schema["properties"]["pattern"].is_object());
    assert!(schema["properties"]["path"].is_object());
    assert!(schema["required"].as_array().unwrap().contains(&json!("pattern")));
}

#[tokio::test]
async fn glob_search_single_file() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("hello.txt"), "content").unwrap();

    let tool = GlobSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "hello.txt"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("hello.txt"));
}

#[tokio::test]
async fn glob_search_multiple_files() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("a.txt"), "").unwrap();
    std::fs::write(dir.path().join("b.txt"), "").unwrap();
    std::fs::write(dir.path().join("c.rs"), "").unwrap();

    let tool = GlobSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "*.txt"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("a.txt"));
    assert!(result.output.contains("b.txt"));
    assert!(!result.output.contains("c.rs"));
}

#[tokio::test]
async fn glob_search_recursive() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("sub/deep")).unwrap();
    std::fs::write(dir.path().join("root.txt"), "").unwrap();
    std::fs::write(dir.path().join("sub/mid.txt"), "").unwrap();
    std::fs::write(dir.path().join("sub/deep/leaf.txt"), "").unwrap();

    let tool = GlobSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "**/*.txt"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("root.txt"));
    assert!(result.output.contains("mid.txt"));
    assert!(result.output.contains("leaf.txt"));
}

#[tokio::test]
async fn glob_search_uses_relative_path_argument() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("sub/nested")).unwrap();
    std::fs::write(dir.path().join("root.md"), "").unwrap();
    std::fs::write(dir.path().join("sub/nested/inside.md"), "").unwrap();

    let tool = GlobSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "**/*.md", "path": "sub"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("sub/nested/inside.md"));
    assert!(!result.output.contains("root.md"));
}

#[tokio::test]
async fn glob_search_no_matches() {
    let dir = TempDir::new().unwrap();

    let tool = GlobSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "*.nonexistent"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("No files matching pattern"));
}

#[tokio::test]
async fn glob_search_missing_param() {
    let tool = GlobSearchTool::new(test_security(std::env::temp_dir()));
    let result = tool.execute(json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn glob_search_rejects_absolute_path() {
    let tool = GlobSearchTool::new(test_security(std::env::temp_dir()));
    let result = tool.execute(json!({"pattern": "/etc/**/*"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("Absolute paths"));
}

#[tokio::test]
async fn glob_search_rejects_path_traversal() {
    let tool = GlobSearchTool::new(test_security(std::env::temp_dir()));
    let result = tool.execute(json!({"pattern": "../../../etc/passwd"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("Path traversal"));
}

#[tokio::test]
async fn glob_search_rejects_disallowed_relative_search_path() {
    let dir = TempDir::new().unwrap();
    let tool = GlobSearchTool::new(test_security(dir.path().to_path_buf()));

    let result = tool.execute(json!({"pattern": "*.txt", "path": "../outside"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_deref().unwrap().contains("Path not allowed by security policy"));
}

#[tokio::test]
async fn glob_search_rejects_dotdot_only() {
    let tool = GlobSearchTool::new(test_security(std::env::temp_dir()));
    let result = tool.execute(json!({"pattern": ".."})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("Path traversal"));
}

#[cfg(unix)]
#[tokio::test]
async fn glob_search_filters_symlink_escape() {
    use std::os::unix::fs::symlink;

    let root = TempDir::new().unwrap();
    let workspace = root.path().join("workspace");
    let outside = root.path().join("outside");

    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), "leaked").unwrap();

    // Symlink inside workspace pointing outside
    symlink(outside.join("secret.txt"), workspace.join("escape.txt")).unwrap();
    // Also add a legitimate file
    std::fs::write(workspace.join("legit.txt"), "ok").unwrap();

    let tool = GlobSearchTool::new(test_security(workspace.clone()));
    let result = tool.execute(json!({"pattern": "*.txt"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("legit.txt"));
    assert!(!result.output.contains("escape.txt"));
    assert!(!result.output.contains("secret.txt"));
}

#[tokio::test]
async fn glob_search_readonly_mode() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("file.txt"), "").unwrap();

    let tool = GlobSearchTool::new(test_security_with(
        dir.path().to_path_buf(),
        AutonomyLevel::ReadOnly,
        20,
    ));
    let result = tool.execute(json!({"pattern": "*.txt"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("file.txt"));
}

#[tokio::test]
async fn glob_search_rate_limited() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("file.txt"), "").unwrap();

    let tool = GlobSearchTool::new(test_security_with(
        dir.path().to_path_buf(),
        AutonomyLevel::Supervised,
        0,
    ));
    let result = tool.execute(json!({"pattern": "*.txt"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("Rate limit"));
}

#[tokio::test]
async fn glob_search_results_sorted() {
    use std::thread::sleep;
    use std::time::Duration;

    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("c.txt"), "").unwrap();
    sleep(Duration::from_millis(20));
    std::fs::write(dir.path().join("a.txt"), "").unwrap();
    sleep(Duration::from_millis(20));
    std::fs::write(dir.path().join("b.txt"), "").unwrap();

    let tool = GlobSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "*.txt"})).await.unwrap();

    assert!(result.success);
    let lines: Vec<&str> = result.output.lines().collect();
    // First 3 lines should be sorted by mtime descending, then path ascending.
    assert!(lines.len() >= 3);
    assert_eq!(lines[0], "b.txt");
    assert_eq!(lines[1], "a.txt");
    assert_eq!(lines[2], "c.txt");
}

#[tokio::test]
async fn glob_search_excludes_directories() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("subdir")).unwrap();
    std::fs::write(dir.path().join("file.txt"), "").unwrap();

    let tool = GlobSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "*"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("file.txt"));
    assert!(!result.output.contains("subdir"));
}

#[tokio::test]
async fn glob_search_invalid_pattern() {
    let dir = TempDir::new().unwrap();

    let tool = GlobSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "[invalid"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("Invalid glob pattern"));
}

#[tokio::test]
async fn glob_search_rejects_blank_pattern_before_filesystem_access() {
    let tool = GlobSearchTool::new(test_security(PathBuf::from("/path/that/does/not/exist")));

    let result = tool.execute(json!({"pattern": " \t\n "})).await.unwrap();

    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("Missing pattern"));
}

#[tokio::test]
async fn glob_search_reports_missing_and_non_directory_search_paths() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("file.txt"), "content").unwrap();
    let tool = GlobSearchTool::new(test_security(dir.path().to_path_buf()));

    let missing = tool.execute(json!({"pattern": "*.txt", "path": "missing"})).await.unwrap();
    assert!(!missing.success);
    assert!(missing.error.as_deref().unwrap().contains("does not exist"));

    let file = tool.execute(json!({"pattern": "*.txt", "path": "file.txt"})).await.unwrap();
    assert!(!file.success);
    assert!(file.error.as_deref().unwrap().contains("not a directory"));
}
