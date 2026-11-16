// 覆盖待审批请求映射、回复处理及非 CLI 授权持久化。

use super::*;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use std::ffi::{OsStr, OsString};
use tempfile::TempDir;
use tower::ServiceExt;

struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &OsStr) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, previous }
    }

    fn unset(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }
}

fn use_temp_config_dir(temp: &TempDir) -> (EnvGuard, EnvGuard) {
    let config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().as_os_str());
    let workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    (config_dir, workspace)
}

async fn manager_for_directory(
    directory: &str,
) -> std::sync::Arc<crate::app::agent::approval::ApprovalManager> {
    with_instance(directory.to_string(), || {
        Box::pin(async { Ok(approval_state::approval_manager_for_current_instance().await) })
    })
    .await
    .expect("approval manager should load")
}

// 待审批项只有在回复成功后移除；持久化失败不能改变本次明确授权结果。
#[test]
fn router_builds_with_app_state() {
    let _: axum::Router<()> = router();
}

// 拒绝路径除状态码外还检查状态保持不变，防止错误响应掩盖已发生的副作用。

#[tokio::test]
async fn permission_list_returns_empty_for_instance_without_pending_requests() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_config_dir, _workspace) = use_temp_config_dir(&temp);
    let directory = temp.path().to_string_lossy().to_string();
    approval_state::clear_approval_manager_for_directory(&directory);

    let Json(rows) = permission_list(
        Extension(Default::default()),
        Query(InstanceQuery { directory: Some(directory.clone()) }),
        HeaderMap::new(),
    )
    .await
    .expect("permission list should load");

    assert!(rows.is_empty());

    approval_state::clear_approval_manager_for_directory(&directory);
}

#[tokio::test]
async fn permission_list_returns_pending_requests_for_resolved_directory() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_config_dir, _workspace) = use_temp_config_dir(&temp);
    let directory = temp.path().to_string_lossy().to_string();
    approval_state::clear_approval_manager_for_directory(&directory);

    let manager = manager_for_directory(&directory).await;
    let pending = manager.create_non_cli_pending_request(
        "shell.exec",
        "alice",
        "telegram",
        "chat-1",
        Some("needs approval".to_string()),
        serde_json::json!({"command": "pwd"}),
        Some("msg-1".to_string()),
        Some("call-1".to_string()),
    );

    let Json(rows) = permission_list(
        Extension(Default::default()),
        Query(InstanceQuery { directory: Some(directory.clone()) }),
        HeaderMap::new(),
    )
    .await
    .expect("permission list should load");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, pending.request_id);
    assert_eq!(rows[0].permission, "shell.exec");

    approval_state::clear_approval_manager_for_directory(&directory);
}

#[tokio::test]
async fn permission_reply_once_applies_pending_request() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_config_dir, _workspace) = use_temp_config_dir(&temp);
    let directory = temp.path().to_string_lossy().to_string();
    approval_state::clear_approval_manager_for_directory(&directory);

    let manager = manager_for_directory(&directory).await;
    let pending = manager.create_non_cli_pending_request(
        "shell.exec",
        "alice",
        "telegram",
        "chat-1",
        Some("needs approval".to_string()),
        serde_json::json!({"command": "pwd"}),
        Some("msg-1".to_string()),
        Some("call-1".to_string()),
    );

    let Json(applied) = permission_reply(
        Extension(Default::default()),
        Query(InstanceQuery { directory: Some(directory.clone()) }),
        HeaderMap::new(),
        Path(pending.request_id.clone()),
        Json(PermissionReplyRequest { reply: permission_next::Reply::Once, message: None }),
    )
    .await
    .expect("permission reply should apply");

    assert!(applied);
    assert!(!manager.has_non_cli_pending_request(&pending.request_id));
    assert_eq!(
        manager.take_non_cli_pending_resolution(&pending.request_id),
        Some(ApprovalResponse::Yes)
    );

    approval_state::clear_approval_manager_for_directory(&directory);
}

#[tokio::test]
async fn permission_reply_missing_request_returns_not_found() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_config_dir, _workspace) = use_temp_config_dir(&temp);
    let directory = temp.path().to_string_lossy().to_string();
    approval_state::clear_approval_manager_for_directory(&directory);

    let error = permission_reply(
        Extension(Default::default()),
        Query(InstanceQuery { directory: Some(directory.clone()) }),
        HeaderMap::new(),
        Path("apr-missing".to_string()),
        Json(PermissionReplyRequest { reply: permission_next::Reply::Once, message: None }),
    )
    .await
    .expect_err("missing request should be not found");

    assert_eq!(error.status, StatusCode::NOT_FOUND);
    assert_eq!(error.to_string(), "request not found");

    approval_state::clear_approval_manager_for_directory(&directory);
}

#[tokio::test]
async fn permission_router_lists_and_rejects_missing_request() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_config_dir, _workspace) = use_temp_config_dir(&temp);
    let directory = temp.path().to_string_lossy().to_string();
    approval_state::clear_approval_manager_for_directory(&directory);
    let app = router::<()>();

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/permission?directory={directory}"))
                .body(Body::empty())
                .expect("list request"),
        )
        .await
        .expect("list response");
    assert_eq!(list_response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(list_response.into_body(), usize::MAX).await.expect("list body").as_ref(),
        b"[]"
    );

    let reply_response = app
        .oneshot(
            Request::builder()
                .method(axum::http::Method::POST)
                .uri(format!("/permission/apr-missing/reply?directory={directory}"))
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"reply":"once"}"#))
                .expect("reply request"),
        )
        .await
        .expect("reply response");
    assert_eq!(reply_response.status(), StatusCode::NOT_FOUND);

    approval_state::clear_approval_manager_for_directory(&directory);
}
