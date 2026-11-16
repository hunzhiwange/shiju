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

#[tokio::test]
async fn pty_list_returns_empty_for_new_instance() {
    let temp = tempfile::tempdir().expect("tempdir");

    let Json(items) = pty_list::<Host>(instance_query(&temp), HeaderMap::new())
        .await
        .expect("new instance should list PTYs");

    assert!(items.is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn pty_create_rejects_blank_cwd_without_starting_process() {
    let temp = tempfile::tempdir().expect("tempdir");
    let error = pty_create::<Host>(
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

    let error = pty_get::<Host>(Path("missing-pty".to_string()), instance_query(&temp), HeaderMap::new())
        .await
        .expect_err("missing session should be a not found error");

    assert_eq!(error.status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(error.message, "session not found");
}

#[tokio::test]
async fn pty_update_missing_session_returns_not_found() {
    let temp = tempfile::tempdir().expect("tempdir");

    let error = pty_update::<Host>(
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
        pty_remove::<Host>(Path("missing-pty".to_string()), instance_query(&temp), HeaderMap::new())
            .await
            .expect("missing remove is idempotent");

    assert!(removed);
}
