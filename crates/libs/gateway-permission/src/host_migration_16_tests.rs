//! Approval mapping, resolution and persistence use the real approval/config services.
use super::*;
use agent_config::runtime_schema as schema;
use agent_config::runtime::AutonomyConfig;
use axum::http::StatusCode;
use std::ffi::{OsStr, OsString};
#[cfg(unix)]
use std::path::{Path, PathBuf};
use tempfile::TempDir;
fn global_async_env_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    &LOCK
}
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

#[cfg(unix)]
struct DirModeGuard {
    path: PathBuf,
    original_mode: u32,
}

#[cfg(unix)]
impl DirModeGuard {
    fn set(path: &Path, mode: u32) -> Self {
        use std::os::unix::fs::PermissionsExt;

        let metadata = std::fs::metadata(path).expect("directory metadata should load");
        let original_mode = metadata.permissions().mode() & 0o777;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
            .expect("directory permissions should be updated");
        Self { path: path.to_path_buf(), original_mode }
    }
}

#[cfg(unix)]
impl Drop for DirModeGuard {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;

        let _ = std::fs::set_permissions(
            &self.path,
            std::fs::Permissions::from_mode(self.original_mode),
        );
    }
}

fn use_temp_config_dir(temp: &TempDir) -> (EnvGuard, EnvGuard) {
    let config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().as_os_str());
    let workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    (config_dir, workspace)
}

async fn write_autonomy_config(temp: &TempDir, auto_approve: &[&str], always_ask: &[&str]) {
    let mut current = config::Config::default();
    current.config_path = schema::config_load::config_json_path(temp.path());
    current.workspace_dir = temp.path().join("workspace");
    current.autonomy.auto_approve = auto_approve.iter().map(|entry| (*entry).to_string()).collect();
    current.autonomy.always_ask = always_ask.iter().map(|entry| (*entry).to_string()).collect();
    save_config(&current).await.expect("config should save");
}

fn pending_request(
    reason: Option<&str>,
    arguments: serde_json::Value,
    message_id: Option<&str>,
    call_id: Option<&str>,
) -> PendingNonCliApprovalRequest {
    PendingNonCliApprovalRequest {
        request_id: "apr-1".to_string(),
        tool_name: "shell.exec".to_string(),
        arguments,
        message_id: message_id.map(str::to_string),
        call_id: call_id.map(str::to_string),
        requested_by: "alice".to_string(),
        requested_channel: "telegram".to_string(),
        requested_reply_target: "chat-1".to_string(),
        reason: reason.map(str::to_string),
        created_at: "2026-01-02T03:04:05Z".to_string(),
        expires_at: "2026-01-02T03:34:05Z".to_string(),
    }
}

fn manager_with_pending_request() -> (agent_approval::ApprovalManager, String) {
    let manager =
        agent_approval::ApprovalManager::from_config(&AutonomyConfig::default());
    let request = manager.create_non_cli_pending_request(
        "shell.exec",
        "alice",
        "telegram",
        "chat-1",
        Some("needs approval".to_string()),
        serde_json::json!({"command": "pwd"}),
        Some("msg-1".to_string()),
        Some("call-1".to_string()),
    );

    (manager, request.request_id)
}

#[test]
fn acp_permission_request_omits_expiration() {
    let mut pending = pending_request(None, serde_json::json!({}), None, None);
    pending.requested_channel = approval_state::ACP_APPROVAL_CHANNEL.to_string();
    let request = permission_request_from_pending(pending);
    assert!(!request.metadata.contains_key("expires_at"));
    assert!(request.always.is_empty());
}

#[test]
fn permission_reply_request_deserializes_reply_field() {
    let body: PermissionReplyRequest = serde_json::from_value(serde_json::json!({"reply": "once"}))
        .expect("valid permission reply");

    assert_eq!(body.reply, permission_next::Reply::Once);
    assert!(body.message.is_none());
}

#[test]
fn permission_reply_request_deserializes_all_reply_variants_and_message() {
    let always: PermissionReplyRequest = serde_json::from_value(serde_json::json!({
        "reply": "always",
        "message": "remember this"
    }))
    .expect("valid always reply");
    let reject: PermissionReplyRequest =
        serde_json::from_value(serde_json::json!({"reply": "reject"})).expect("valid reject reply");

    assert_eq!(always.reply, permission_next::Reply::Always);
    assert_eq!(always.message.as_deref(), Some("remember this"));
    assert_eq!(reject.reply, permission_next::Reply::Reject);
    assert!(reject.message.is_none());
}

#[test]
fn permission_reply_request_rejects_missing_or_unknown_reply() {
    assert!(serde_json::from_value::<PermissionReplyRequest>(serde_json::json!({})).is_err());
    assert!(
        serde_json::from_value::<PermissionReplyRequest>(serde_json::json!({"reply": "later"}))
            .is_err()
    );
    assert!(
        serde_json::from_value::<PermissionReplyRequest>(serde_json::json!({
            "reply": "once",
            "message": 7
        }))
        .is_err()
    );
}

#[test]
fn permission_request_from_pending_maps_metadata_and_tool_info() {
    let request = permission_request_from_pending(pending_request(
        Some(" needs approval "),
        serde_json::json!({"command": "pwd"}),
        Some("msg-1"),
        Some("call-1"),
    ));

    assert_eq!(request.id, "apr-1");
    assert_eq!(request.session_id, "chat-1");
    assert_eq!(request.permission, "shell.exec");
    assert!(request.patterns.is_empty());
    assert_eq!(request.always, vec!["shell.exec".to_string()]);
    assert_eq!(request.metadata["reason"], " needs approval ");
    assert_eq!(request.metadata["arguments"], serde_json::json!({"command": "pwd"}));
    assert_eq!(request.metadata["requested_by"], "alice");
    assert_eq!(request.metadata["requested_channel"], "telegram");
    assert_eq!(request.metadata["requested_reply_target"], "chat-1");
    assert_eq!(request.metadata["created_at"], "2026-01-02T03:04:05Z");
    assert_eq!(request.metadata["expires_at"], "2026-01-02T03:34:05Z");

    let tool = request.tool.expect("tool info");
    assert_eq!(tool.message_id, "msg-1");
    assert_eq!(tool.call_id, "call-1");
}

#[test]
fn permission_request_from_pending_skips_blank_optional_fields() {
    let request = permission_request_from_pending(pending_request(
        Some("   "),
        serde_json::Value::Null,
        Some("msg-1"),
        Some("  "),
    ));

    assert!(!request.metadata.contains_key("reason"));
    assert!(!request.metadata.contains_key("arguments"));
    assert!(request.tool.is_none());
}

#[test]
fn permission_request_from_pending_requires_both_non_blank_tool_identifiers() {
    let cases = [
        (None, Some("call-1")),
        (Some("msg-1"), None),
        (Some("   "), Some("call-1")),
        (Some("msg-1"), Some("\t")),
    ];

    for (message_id, call_id) in cases {
        let request = permission_request_from_pending(pending_request(
            None,
            serde_json::json!([]),
            message_id,
            call_id,
        ));

        assert!(!request.metadata.contains_key("reason"));
        assert_eq!(request.metadata["arguments"], serde_json::json!([]));
        assert!(request.tool.is_none());
    }
}

#[test]
fn permission_request_from_pending_preserves_non_object_arguments() {
    let request = permission_request_from_pending(pending_request(
        Some("batch approval"),
        serde_json::json!(["pwd", false, 3]),
        Some("msg-1"),
        Some("call-1"),
    ));

    assert_eq!(request.metadata["arguments"], serde_json::json!(["pwd", false, 3]));
}

#[test]
fn map_pending_approval_error_preserves_public_status_and_message() {
    let cases = [
        (PendingApprovalError::NotFound, StatusCode::NOT_FOUND, "request not found"),
        (PendingApprovalError::Expired, StatusCode::BAD_REQUEST, "request expired"),
        (
            PendingApprovalError::RequesterMismatch,
            StatusCode::BAD_REQUEST,
            "request actor mismatch",
        ),
    ];

    for (input, status, message) in cases {
        let error = map_pending_approval_error(input);
        assert_eq!(error.status, status);
        assert_eq!(error.to_string(), message);
    }
}

#[tokio::test]
async fn reply_pending_request_returns_false_for_missing_request() {
    let manager =
        agent_approval::ApprovalManager::from_config(&AutonomyConfig::default());

    let applied =
        reply_pending_request(&manager, "apr-missing", permission_next::Reply::Once, None)
            .await
            .expect("missing request is not an api error");

    assert!(!applied);
}

#[tokio::test]
async fn reply_pending_request_once_confirms_and_records_yes_resolution() {
    let (manager, request_id) = manager_with_pending_request();

    let applied = reply_pending_request(
        &manager,
        &request_id,
        permission_next::Reply::Once,
        Some("approved".to_string()),
    )
    .await
    .expect("reply should apply");

    assert!(applied);
    assert!(!manager.has_non_cli_pending_request(&request_id));
    assert_eq!(manager.take_non_cli_pending_resolution(&request_id), Some(ApprovalResponse::Yes));
    assert!(!manager.is_non_cli_session_granted("shell.exec"));
}

#[tokio::test]
async fn reply_pending_request_always_confirms_grants_and_records_yes_resolution() {
    let _env_lock = global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_config_dir, _workspace) = use_temp_config_dir(&temp);
    let (manager, request_id) = manager_with_pending_request();

    let applied =
        reply_pending_request(&manager, &request_id, permission_next::Reply::Always, None)
            .await
            .expect("reply should apply");

    assert!(applied);
    assert!(!manager.has_non_cli_pending_request(&request_id));
    assert_eq!(manager.take_non_cli_pending_resolution(&request_id), Some(ApprovalResponse::Yes));
    assert!(manager.is_non_cli_session_granted("shell.exec"));
    assert!(manager.auto_approve_tools().contains("shell.exec"));
    assert!(!manager.always_ask_tools().contains("shell.exec"));

    let persisted = config::get().await;
    assert!(persisted.autonomy.auto_approve.iter().any(|entry| entry == "shell.exec"));
}

#[cfg(unix)]
#[tokio::test]
async fn reply_pending_request_always_applies_even_when_persist_fails() {
    let _env_lock = global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_config_dir, _workspace) = use_temp_config_dir(&temp);
    write_autonomy_config(&temp, &[], &[]).await;
    std::fs::create_dir_all(temp.path().join("workspace")).expect("workspace should be created");
    let _mode = DirModeGuard::set(temp.path(), 0o500);
    let (manager, request_id) = manager_with_pending_request();

    let applied =
        reply_pending_request(&manager, &request_id, permission_next::Reply::Always, None)
            .await
            .expect("reply should still apply");

    assert!(applied);
    assert!(!manager.has_non_cli_pending_request(&request_id));
    assert_eq!(manager.take_non_cli_pending_resolution(&request_id), Some(ApprovalResponse::Yes));
    assert!(manager.is_non_cli_session_granted("shell.exec"));
    assert!(manager.auto_approve_tools().contains("shell.exec"));
}

#[tokio::test]
async fn reply_pending_request_reject_removes_request_and_records_no_resolution() {
    let (manager, request_id) = manager_with_pending_request();

    let applied =
        reply_pending_request(&manager, &request_id, permission_next::Reply::Reject, None)
            .await
            .expect("reply should apply");

    assert!(applied);
    assert!(!manager.has_non_cli_pending_request(&request_id));
    assert_eq!(manager.take_non_cli_pending_resolution(&request_id), Some(ApprovalResponse::No));
}

#[tokio::test]
async fn persist_non_cli_approval_adds_auto_approve_and_removes_always_ask() {
    let _env_lock = global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_config_dir, _workspace) = use_temp_config_dir(&temp);
    write_autonomy_config(&temp, &["file.read"], &["shell.exec", "file.write"]).await;

    persist_non_cli_approval("shell.exec").await.expect("approval should persist");

    let persisted = config::get().await;
    assert!(persisted.autonomy.auto_approve.iter().any(|entry| entry == "file.read"));
    assert!(persisted.autonomy.auto_approve.iter().any(|entry| entry == "shell.exec"));
    assert!(persisted.autonomy.always_ask.iter().any(|entry| entry == "file.write"));
    assert!(!persisted.autonomy.always_ask.iter().any(|entry| entry == "shell.exec"));
}

#[tokio::test]
async fn persist_non_cli_approval_adds_tool_without_changing_unrelated_always_ask_entries() {
    let _env_lock = global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_config_dir, _workspace) = use_temp_config_dir(&temp);
    write_autonomy_config(&temp, &["file.read"], &["file.write"]).await;

    persist_non_cli_approval("shell.exec").await.expect("approval should persist");

    let persisted = config::get().await;
    assert_eq!(
        persisted.autonomy.auto_approve,
        vec!["file.read".to_string(), "shell.exec".to_string()]
    );
    assert_eq!(persisted.autonomy.always_ask, vec!["file.write".to_string()]);
}

#[tokio::test]
async fn persist_non_cli_approval_removes_always_ask_when_tool_is_already_approved() {
    let _env_lock = global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_config_dir, _workspace) = use_temp_config_dir(&temp);
    write_autonomy_config(&temp, &["shell.exec"], &["shell.exec", "file.write"]).await;

    persist_non_cli_approval("shell.exec").await.expect("approval should persist");

    let persisted = config::get().await;
    assert_eq!(persisted.autonomy.auto_approve, vec!["shell.exec".to_string()]);
    assert_eq!(persisted.autonomy.always_ask, vec!["file.write".to_string()]);
}

#[tokio::test]
async fn persist_non_cli_approval_noops_when_tool_is_already_persisted() {
    let _env_lock = global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_config_dir, _workspace) = use_temp_config_dir(&temp);
    write_autonomy_config(&temp, &["shell.exec"], &["file.write"]).await;

    persist_non_cli_approval("shell.exec").await.expect("approval should persist");

    let persisted = config::get().await;
    assert_eq!(
        persisted
            .autonomy
            .auto_approve
            .iter()
            .filter(|entry| entry.as_str() == "shell.exec")
            .count(),
        1
    );
    assert_eq!(persisted.autonomy.always_ask, vec!["file.write".to_string()]);
}
