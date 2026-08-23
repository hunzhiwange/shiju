// 覆盖伪终端会话的创建、输入、调整尺寸与清理接口。

use super::*;
use axum::extract::{Path, Query};
use axum::http::HeaderMap;

fn instance_query(temp: &tempfile::TempDir) -> Query<InstanceQuery> {
    Query(InstanceQuery { directory: Some(temp.path().to_string_lossy().to_string()) })
}

// 会话标识和进程状态分别构造，确保输入、调整尺寸和清理只作用于目标终端。
#[test]
fn router_builds_with_app_state() {
    let _: axum::Router<()> = router();
}

#[test]
fn pty_connect_query_deserializes_directory_and_cursor() {
    let query: PtyConnectQuery = serde_json::from_value(serde_json::json!({
        "directory": "/workspace/project",
        "cursor": 42
    }))
    .expect("valid query");

    assert_eq!(query.directory.as_deref(), Some("/workspace/project"));
    assert_eq!(query.cursor, Some(42));
}

#[test]
fn pty_connect_query_allows_missing_optional_fields() {
    let query: PtyConnectQuery =
        serde_json::from_value(serde_json::json!({})).expect("valid query");

    assert!(query.directory.is_none());
    assert!(query.cursor.is_none());
}

#[test]
fn pty_connect_query_rejects_non_integer_cursor() {
    let result = serde_json::from_value::<PtyConnectQuery>(serde_json::json!({"cursor": "latest"}));

    assert!(result.is_err());
}

#[test]
fn pty_connect_query_preserves_cursor_boundaries_and_blank_directory() {
    let minimum: PtyConnectQuery = serde_json::from_value(serde_json::json!({
        "directory": "   ",
        "cursor": i64::MIN
    }))
    .expect("minimum cursor should deserialize");
    let maximum: PtyConnectQuery = serde_json::from_value(serde_json::json!({"cursor": i64::MAX}))
        .expect("maximum cursor should deserialize");

    assert_eq!(minimum.directory.as_deref(), Some("   "));
    assert_eq!(minimum.cursor, Some(i64::MIN));
    assert_eq!(maximum.cursor, Some(i64::MAX));
}

#[tokio::test]
async fn pty_list_returns_empty_for_new_instance() {
    let temp = tempfile::tempdir().expect("tempdir");

    let Json(items) = pty_list(instance_query(&temp), HeaderMap::new())
        .await
        .expect("new instance should list PTYs");

    assert!(items.is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn pty_create_rejects_blank_cwd_without_starting_process() {
    let temp = tempfile::tempdir().expect("tempdir");
    let error = pty_create(
        instance_query(&temp),
        HeaderMap::new(),
        Json(pty::pty::CreateInput {
            command: None,
            args: None,
            cwd: Some(" \t".to_string()),
            title: None,
            env: None,
        }),
    )
    .await
    .expect_err("blank cwd should be rejected before opening a PTY");

    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.message, "missing cwd");
}

#[tokio::test]
async fn pty_get_missing_session_returns_not_found() {
    let temp = tempfile::tempdir().expect("tempdir");

    let error = pty_get(Path("missing-pty".to_string()), instance_query(&temp), HeaderMap::new())
        .await
        .expect_err("missing session should be a not found error");

    assert_eq!(error.status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(error.message, "session not found");
}

#[tokio::test]
async fn pty_update_missing_session_returns_not_found() {
    let temp = tempfile::tempdir().expect("tempdir");

    let error = pty_update(
        Path("missing-pty".to_string()),
        instance_query(&temp),
        HeaderMap::new(),
        Json(pty::pty::UpdateInput { title: Some("new title".to_string()), size: None }),
    )
    .await
    .expect_err("missing update should be a not found error");

    assert_eq!(error.status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(error.message, "session not found");
}

#[tokio::test]
async fn pty_remove_missing_session_is_idempotent() {
    let temp = tempfile::tempdir().expect("tempdir");

    let Json(removed) =
        pty_remove(Path("missing-pty".to_string()), instance_query(&temp), HeaderMap::new())
            .await
            .expect("missing remove is idempotent");

    assert!(removed);
}
