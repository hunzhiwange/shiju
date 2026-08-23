// 验证实例目录解析、项目切换回调与本地项目状态清理。

// 清理守卫负责移除测试项目，避免实例切换测试残留进程级本地状态。
use super::*;
use axum::http::{HeaderMap, HeaderValue};
use uuid::Uuid;

// 清理守卫即使断言 panic 也会移除测试项目，避免污染后续实例解析。
struct StoredProjectCleanup(String);

impl Drop for StoredProjectCleanup {
    fn drop(&mut self) {
        let _ = project::remove_local_test_project(&self.0);
    }
}

fn current_instance_directory() -> project::BoxFuture<Result<String, ApiError>> {
    Box::pin(async move { Ok(crate::app::agent::project::instance::directory()) })
}

fn callback_error() -> project::BoxFuture<Result<(), ApiError>> {
    Box::pin(async move { Err(ApiError::bad_request("callback failed")) })
}

// 非法项目文件用于确认解析失败保留上下文，而不是静默退回其它实例。
fn write_invalid_project_json(temp: &tempfile::TempDir, suffix: &str) -> std::path::PathBuf {
    let git_dir = temp.path().join(".git");
    std::fs::create_dir(&git_dir).expect("git dir");
    let project_id = format!("bad-instance-json-{suffix}-{}", std::process::id());
    std::fs::write(git_dir.join("vibewindow"), &project_id).expect("cached project id");

    let storage_path = crate::app::agent::global::paths()
        .data
        .join("storage")
        .join("project")
        .join(format!("{project_id}.json"));
    std::fs::create_dir_all(storage_path.parent().expect("project storage parent"))
        .expect("project storage dir");
    std::fs::write(&storage_path, "{").expect("invalid project json");
    storage_path
}

#[test]
fn instance_query_deserializes_explicit_and_missing_directory() {
    let explicit: InstanceQuery =
        serde_json::from_str(r#"{"directory":"/workspace"}"#).expect("explicit directory");
    let missing: InstanceQuery = serde_json::from_str("{}").expect("missing directory");

    assert_eq!(explicit.directory.as_deref(), Some("/workspace"));
    assert_eq!(missing.directory, None);
}

#[test]
fn resolve_directory_prefers_query_over_header() {
    let query = InstanceQuery { directory: Some("/query".to_string()) };
    let mut headers = HeaderMap::new();
    headers.insert("x-vibewindow-directory", HeaderValue::from_static("/header"));

    assert_eq!(resolve_directory(&query, &headers), "/query");
}

#[test]
fn resolve_directory_preserves_non_empty_query_value() {
    let query = InstanceQuery { directory: Some(" /query with spaces ".to_string()) };

    assert_eq!(resolve_directory(&query, &HeaderMap::new()), " /query with spaces ");
}

#[test]
fn resolve_directory_uses_header_when_query_is_blank() {
    let query = InstanceQuery { directory: Some("   ".to_string()) };
    let mut headers = HeaderMap::new();
    headers.insert("x-vibewindow-directory", HeaderValue::from_static("/header"));

    assert_eq!(resolve_directory(&query, &headers), "/header");
}

#[test]
fn resolve_directory_ignores_blank_header() {
    let query = InstanceQuery { directory: None };
    let mut headers = HeaderMap::new();
    headers.insert("x-vibewindow-directory", HeaderValue::from_static("   "));

    assert_eq!(
        resolve_directory(&query, &headers),
        std::env::current_dir().expect("current dir").to_string_lossy().to_string()
    );
}

#[test]
fn resolve_directory_ignores_invalid_header_value() {
    let query = InstanceQuery { directory: None };
    let mut headers = HeaderMap::new();
    let invalid = HeaderValue::from_bytes(&[0xff]).expect("invalid utf8 header bytes");
    headers.insert("x-vibewindow-directory", invalid);

    assert_eq!(
        resolve_directory(&query, &headers),
        std::env::current_dir().expect("current dir").to_string_lossy().to_string()
    );
}

#[test]
fn resolve_directory_uses_current_dir_without_query_or_header() {
    let query = InstanceQuery { directory: None };
    let headers = HeaderMap::new();

    assert_eq!(
        resolve_directory(&query, &headers),
        std::env::current_dir().expect("current dir").to_string_lossy().to_string()
    );
}

#[test]
fn display_directory_falls_back_to_dot_when_current_dir_fails() {
    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing cwd");

    assert_eq!(display_directory(Err(err)), ".");
}

#[test]
fn display_directory_returns_supplied_path() {
    let directory = std::path::PathBuf::from("/workspace/project");

    assert_eq!(display_directory(Ok(directory)), "/workspace/project");
}

#[tokio::test]
async fn with_instance_runs_callback_inside_requested_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let directory = temp.path().to_string_lossy().to_string();
    let observed = with_instance(directory.clone(), current_instance_directory)
        .await
        .expect("with_instance should run callback");

    assert_eq!(observed, directory);
}

#[tokio::test]
async fn with_instance_returns_callback_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let err = with_instance(temp.path().to_string_lossy().to_string(), callback_error)
        .await
        .expect_err("callback error should be returned");

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn with_instance_ignores_legacy_invalid_project_json() {
    let temp = tempfile::tempdir().expect("tempdir");
    let storage_path = write_invalid_project_json(&temp, "directory");

    let observed =
        with_instance(temp.path().to_string_lossy().to_string(), current_instance_directory)
            .await
            .expect("legacy project storage should not affect database-backed instance binding");

    let _ = std::fs::remove_file(storage_path);
    assert_eq!(observed, temp.path().to_string_lossy());
}

#[tokio::test]
async fn with_instance_returns_callback_error_when_legacy_project_json_is_invalid() {
    let temp = tempfile::tempdir().expect("tempdir");
    let storage_path = write_invalid_project_json(&temp, "callback");

    let err = with_instance(temp.path().to_string_lossy().to_string(), callback_error)
        .await
        .expect_err("callback error should be returned");

    let _ = std::fs::remove_file(storage_path);
    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn instance_init_noops_without_project_context() {
    (instance_init())().await;
}

#[tokio::test]
async fn load_local_project_rejects_blank_project_id() {
    let err = load_local_project(" \t").await.expect_err("blank project id should be rejected");

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(err.message, "project id must not be empty");
}

#[tokio::test]
async fn load_local_project_returns_not_found_for_missing_project() {
    let project_id = format!("gateway-instance-missing-{}", Uuid::new_v4());

    let err =
        load_local_project(&project_id).await.expect_err("missing project should return not found");

    assert_eq!(err.status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(err.message, "project not found");
}

#[tokio::test]
async fn load_local_project_returns_stored_project() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_id = format!("gateway-instance-load-{}", Uuid::new_v4());
    let worktree = temp.path().to_string_lossy().to_string();
    let info = project::Info {
        id: project_id.clone(),
        worktree: worktree.clone(),
        vcs: None,
        name: Some("Gateway instance".to_string()),
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 10, updated: 20, initialized: None },
        sandboxes: Vec::new(),
    };
    project::store_local_test_project(&info).expect("project should be stored");
    let _cleanup = StoredProjectCleanup(project_id.clone());

    let loaded = load_local_project(&project_id).await.expect("stored project should load");

    assert_eq!(loaded.id, project_id);
    assert_eq!(loaded.worktree, worktree);
    assert_eq!(loaded.name.as_deref(), Some("Gateway instance"));
}

#[test]
fn normalize_rel_path_handles_empty_root_and_dot() {
    let root = std::path::PathBuf::from("/tmp/root");

    assert_eq!(normalize_rel_path(&root, ""), Some(String::new()));
    assert_eq!(normalize_rel_path(&root, "/"), Some(String::new()));
    assert_eq!(normalize_rel_path(&root, "."), Some(String::new()));
}

#[test]
fn normalize_rel_path_strips_root_from_absolute_path() {
    let root = std::path::PathBuf::from("/tmp/root");

    assert_eq!(
        normalize_rel_path(&root, "/tmp/root/nested/file.txt"),
        Some("nested/file.txt".to_string())
    );
}

#[test]
fn normalize_rel_path_maps_absolute_root_to_empty_path() {
    let root = std::path::PathBuf::from("/tmp/root");

    assert_eq!(normalize_rel_path(&root, "/tmp/root"), Some(String::new()));
}

#[test]
fn normalize_rel_path_keeps_relative_path_without_leading_slashes() {
    let root = std::path::PathBuf::from("/tmp/root");

    assert_eq!(normalize_rel_path(&root, "nested/file.txt"), Some("nested/file.txt".to_string()));
}

#[test]
fn normalize_rel_path_rejects_absolute_path_outside_root() {
    let root = std::path::PathBuf::from("/tmp/root");

    assert_eq!(normalize_rel_path(&root, "/etc/passwd"), None);
}

#[test]
fn normalize_rel_path_rejects_sibling_with_shared_text_prefix() {
    let root = std::path::PathBuf::from("/tmp/root");

    assert_eq!(normalize_rel_path(&root, "/tmp/root-sibling/file.txt"), None);
}

#[cfg(unix)]
#[test]
fn normalize_rel_path_normalizes_backslashes_after_stripping_root() {
    let root = std::path::PathBuf::from("/tmp/root");

    assert_eq!(
        normalize_rel_path(&root, "/tmp/root/nested\\file.txt"),
        Some("nested/file.txt".to_string())
    );
}
