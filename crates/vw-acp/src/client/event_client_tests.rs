use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use agent_client_protocol as acp;
use agent_client_protocol::Client as _;
use parking_lot::Mutex;
use tokio::sync::mpsc;

use crate::filesystem::{FileSystemHandlers, FileSystemHandlersOptions};
use crate::terminal::{TerminalManager, TerminalManagerOptions};
use crate::types::{
    AcpAgentConfig, AuthPolicy, NonInteractivePermissionPolicy, PermissionMode, PermissionStats,
    SessionUpdateCallback,
};

use super::{AcpEventClient, InternalEvent};

fn event_client(
    event_tx: mpsc::UnboundedSender<InternalEvent>,
    expected_session_id: Option<String>,
) -> AcpEventClient {
    AcpEventClient {
        expected_session_id: Arc::new(Mutex::new(expected_session_id)),
        event_tx,
        filesystem: FileSystemHandlers::new(FileSystemHandlersOptions {
            cwd: std::env::temp_dir(),
            ..FileSystemHandlersOptions::default()
        }),
        terminal_manager: TerminalManager::new(TerminalManagerOptions {
            cwd: PathBuf::from("."),
            ..TerminalManagerOptions::default()
        }),
        permission_mode: PermissionMode::ApproveAll,
        non_interactive_permissions: None,
        on_session_update: None,
        permission_stats: Arc::new(Mutex::new(PermissionStats::default())),
        cancelling_session_ids: Arc::new(Mutex::new(HashSet::new())),
    }
}

fn event_client_with_cwd(
    event_tx: mpsc::UnboundedSender<InternalEvent>,
    cwd: PathBuf,
) -> AcpEventClient {
    AcpEventClient {
        expected_session_id: Arc::new(Mutex::new(None)),
        event_tx,
        filesystem: FileSystemHandlers::new(FileSystemHandlersOptions {
            cwd: cwd.clone(),
            permission_mode: PermissionMode::ApproveAll,
            ..FileSystemHandlersOptions::default()
        }),
        terminal_manager: TerminalManager::new(TerminalManagerOptions {
            cwd,
            permission_mode: PermissionMode::ApproveAll,
            kill_grace_ms: Some(50),
            ..TerminalManagerOptions::default()
        }),
        permission_mode: PermissionMode::ApproveAll,
        non_interactive_permissions: None,
        on_session_update: None,
        permission_stats: Arc::new(Mutex::new(PermissionStats::default())),
        cancelling_session_ids: Arc::new(Mutex::new(HashSet::new())),
    }
}

fn permission_request(session_id: &str) -> acp::RequestPermissionRequest {
    acp::RequestPermissionRequest::new(
        session_id.to_string(),
        acp::ToolCallUpdate::new(
            "tool-1",
            acp::ToolCallUpdateFields::new()
                .kind(acp::ToolKind::Edit)
                .title("write file".to_string()),
        ),
        vec![
            acp::PermissionOption::new("allow", "Allow", acp::PermissionOptionKind::AllowOnce),
            acp::PermissionOption::new("reject", "Reject", acp::PermissionOptionKind::RejectOnce),
        ],
    )
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).expect("system clock after epoch").as_nanos();
    let path = std::env::temp_dir().join(format!("vw-acp-event-client-{name}-{nanos}"));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

#[tokio::test]
async fn session_notification_forwards_text_delta() {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let client = event_client(event_tx, Some("session-1".to_string()));

    client
        .session_notification(acp::SessionNotification::new(
            "session-1",
            acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(acp::ContentBlock::Text(
                acp::TextContent::new("hello"),
            ))),
        ))
        .await
        .expect("session notification");

    match event_rx.recv().await.expect("event") {
        InternalEvent::Delta(delta) => assert_eq!(delta, "hello"),
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test]
async fn session_notification_forwards_resource_link_uri() {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let client = event_client(event_tx, Some("session-1".to_string()));

    client
        .session_notification(acp::SessionNotification::new(
            "session-1",
            acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(
                acp::ContentBlock::ResourceLink(acp::ResourceLink::new(
                    "doc",
                    "file:///tmp/doc.md",
                )),
            )),
        ))
        .await
        .expect("session notification");

    match event_rx.recv().await.expect("event") {
        InternalEvent::Delta(delta) => assert_eq!(delta, "file:///tmp/doc.md"),
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test]
async fn session_notification_ignores_non_text_message_chunks() {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let client = event_client(event_tx, Some("session-1".to_string()));

    client
        .session_notification(acp::SessionNotification::new(
            "session-1",
            acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(
                acp::ContentBlock::Image(acp::ImageContent::new("data", "image/png")),
            )),
        ))
        .await
        .expect("image message notification");

    assert!(event_rx.try_recv().is_err());
}

#[tokio::test]
async fn session_notification_ignores_empty_or_non_text_chunks() {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let client = event_client(event_tx, Some("session-1".to_string()));

    client
        .session_notification(acp::SessionNotification::new(
            "session-1",
            acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(acp::ContentBlock::Text(
                acp::TextContent::new(""),
            ))),
        ))
        .await
        .expect("empty message notification");
    client
        .session_notification(acp::SessionNotification::new(
            "session-1",
            acp::SessionUpdate::AgentThoughtChunk(acp::ContentChunk::new(
                acp::ContentBlock::ResourceLink(acp::ResourceLink::new(
                    "doc",
                    "file:///tmp/doc.md",
                )),
            )),
        ))
        .await
        .expect("non-text thought notification");

    assert!(event_rx.try_recv().is_err());
}

#[tokio::test]
async fn session_notification_without_expected_session_does_not_report_session_change() {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let client = event_client(event_tx, None);

    client
        .session_notification(acp::SessionNotification::new(
            "actual",
            acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(acp::ContentBlock::Text(
                acp::TextContent::new("hello"),
            ))),
        ))
        .await
        .expect("session notification");

    match event_rx.recv().await.expect("event") {
        InternalEvent::Delta(delta) => assert_eq!(delta, "hello"),
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(event_rx.try_recv().is_err());
    assert!(client.expected_session_id.lock().is_none());
}

#[tokio::test]
async fn session_notification_invokes_update_callback() {
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_for_callback = seen.clone();
    let mut client = event_client(event_tx, None);
    client.on_session_update = Some(Arc::new(move |notification| {
        seen_for_callback.lock().push(notification.session_id.to_string());
    }));

    client
        .session_notification(acp::SessionNotification::new(
            "session-1",
            acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(acp::ContentBlock::Text(
                acp::TextContent::new("hello"),
            ))),
        ))
        .await
        .expect("session notification");

    assert_eq!(&*seen.lock(), &["session-1".to_string()]);
}

#[tokio::test]
async fn session_notification_accepts_session_change_and_forwards_delta() {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let client = event_client(event_tx, Some("expected".to_string()));

    client
        .session_notification(acp::SessionNotification::new(
            "actual",
            acp::SessionUpdate::AgentThoughtChunk(acp::ContentChunk::new(acp::ContentBlock::Text(
                acp::TextContent::new("hidden"),
            ))),
        ))
        .await
        .expect("session notification");

    match event_rx.recv().await.expect("event") {
        InternalEvent::SessionChanged { expected, actual } => {
            assert_eq!(expected, "expected");
            assert_eq!(actual, "actual");
        }
        other => panic!("unexpected event: {other:?}"),
    }
    match event_rx.recv().await.expect("event") {
        InternalEvent::Delta(delta) => assert_eq!(delta, "hidden"),
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(event_rx.try_recv().is_err());
    assert_eq!(client.expected_session_id.lock().as_deref(), Some("actual"));
}

#[tokio::test]
async fn request_permission_tracks_approved_decision() {
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let client = event_client(event_tx, None);

    let response = client
        .request_permission(permission_request("session-1"))
        .await
        .expect("permission response");

    assert!(matches!(response.outcome, acp::RequestPermissionOutcome::Selected(_)));
    assert_eq!(
        *client.permission_stats.lock(),
        PermissionStats { requested: 1, approved: 1, denied: 0, cancelled: 0 }
    );
}

#[tokio::test]
async fn request_permission_returns_cancelled_for_cancelling_session() {
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let client = event_client(event_tx, None);
    client.cancelling_session_ids.lock().insert("session-1".to_string());

    let response = client
        .request_permission(permission_request("session-1"))
        .await
        .expect("permission response");

    assert!(matches!(response.outcome, acp::RequestPermissionOutcome::Cancelled));
    assert_eq!(
        *client.permission_stats.lock(),
        PermissionStats { requested: 0, approved: 0, denied: 0, cancelled: 1 }
    );
}

#[tokio::test]
async fn request_permission_without_options_tracks_resolved_cancellation() {
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let client = event_client(event_tx, None);
    let request = acp::RequestPermissionRequest::new(
        "session-1",
        acp::ToolCallUpdate::new(
            "tool-1",
            acp::ToolCallUpdateFields::new()
                .kind(acp::ToolKind::Edit)
                .title("write file".to_string()),
        ),
        Vec::new(),
    );

    let response = client.request_permission(request).await.expect("permission response");

    assert!(matches!(response.outcome, acp::RequestPermissionOutcome::Cancelled));
    assert_eq!(
        *client.permission_stats.lock(),
        PermissionStats { requested: 1, approved: 0, denied: 0, cancelled: 1 }
    );
}

#[tokio::test]
async fn request_permission_tracks_denied_decision() {
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let mut client = event_client(event_tx, None);
    client.permission_mode = PermissionMode::DenyAll;

    client.request_permission(permission_request("session-1")).await.expect("permission response");

    assert_eq!(
        *client.permission_stats.lock(),
        PermissionStats { requested: 1, approved: 0, denied: 1, cancelled: 0 }
    );
}

#[tokio::test]
async fn request_permission_tracks_prompt_unavailable_as_internal_error() {
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let mut client = event_client(event_tx, None);
    client.permission_mode = PermissionMode::ApproveReads;
    client.non_interactive_permissions = Some(NonInteractivePermissionPolicy::Fail);

    let err = client
        .request_permission(permission_request("session-1"))
        .await
        .expect_err("permission prompt should be unavailable");

    assert!(err.to_string().contains("Internal error"));
    assert_eq!(
        *client.permission_stats.lock(),
        PermissionStats { requested: 1, approved: 0, denied: 0, cancelled: 0 }
    );
}

#[tokio::test]
async fn file_methods_delegate_to_filesystem_and_map_permission_errors() {
    let root = unique_temp_dir("files");
    let path = root.join("notes.txt");
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let client = event_client_with_cwd(event_tx, root.clone());

    client
        .write_text_file(acp::WriteTextFileRequest::new("session-1", &path, "one\ntwo\n"))
        .await
        .expect("write file");
    let response = client
        .read_text_file(acp::ReadTextFileRequest::new("session-1", &path).line(2_u32))
        .await
        .expect("read file");

    assert_eq!(response.content, "two\n");

    let outside = root.join("..").join("outside.txt");
    let err = client
        .read_text_file(acp::ReadTextFileRequest::new("session-1", outside))
        .await
        .expect_err("outside path should fail");

    assert!(err.to_string().contains("Internal error"));
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn delegated_methods_map_errors_from_each_backend() {
    let root = unique_temp_dir("errors");
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let client = event_client_with_cwd(event_tx, root.clone());
    let outside = root.join("..").join("outside.txt");

    let write_err = client
        .write_text_file(acp::WriteTextFileRequest::new("session-1", outside, "blocked"))
        .await
        .expect_err("outside write should fail");
    let create_err = client
        .create_terminal(
            acp::CreateTerminalRequest::new("session-1", "sh").cwd(PathBuf::from("relative")),
        )
        .await
        .expect_err("relative cwd should fail");
    let output_err = client
        .terminal_output(acp::TerminalOutputRequest::new("session-1", "missing"))
        .await
        .expect_err("missing output terminal should fail");
    let kill_err = client
        .kill_terminal(acp::KillTerminalRequest::new("session-1", "missing"))
        .await
        .expect_err("missing kill terminal should fail");
    client
        .release_terminal(acp::ReleaseTerminalRequest::new("session-1", "missing"))
        .await
        .expect("missing release terminal should be idempotent");
    let wait_err = client
        .wait_for_terminal_exit(acp::WaitForTerminalExitRequest::new("session-1", "missing"))
        .await
        .expect_err("missing wait terminal should fail");

    for err in [write_err, create_err, output_err, kill_err, wait_err] {
        assert!(err.to_string().contains("Internal error"));
    }
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn terminal_methods_delegate_to_terminal_manager() {
    let root = unique_temp_dir("terminal");
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let client = event_client_with_cwd(event_tx, root.clone());
    let create_response = client
        .create_terminal(
            acp::CreateTerminalRequest::new("session-1", "sh")
                .args(vec!["-c".to_string(), "printf hello".to_string()]),
        )
        .await
        .expect("create terminal");
    let terminal_id = create_response.terminal_id.clone();

    let exit_response = client
        .wait_for_terminal_exit(acp::WaitForTerminalExitRequest::new(
            "session-1",
            terminal_id.clone(),
        ))
        .await
        .expect("wait for exit");
    let output_response = client
        .terminal_output(acp::TerminalOutputRequest::new("session-1", terminal_id.clone()))
        .await
        .expect("terminal output");
    client
        .release_terminal(acp::ReleaseTerminalRequest::new("session-1", terminal_id.clone()))
        .await
        .expect("release terminal");
    let err = client
        .kill_terminal(acp::KillTerminalRequest::new("session-1", terminal_id))
        .await
        .expect_err("released terminal cannot be killed");

    assert_eq!(exit_response.exit_status.exit_code, Some(0));
    assert_eq!(output_response.output, "hello");
    assert!(err.to_string().contains("Internal error"));
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn build_event_client_carries_runtime_configuration() {
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let expected_session_id = Arc::new(Mutex::new(Some("session-1".to_string())));
    let client = super::AcpClient::new(
        "test",
        AcpAgentConfig { command: "noop".to_string(), args: Vec::new(), env: Default::default() },
    )
    .with_permission_mode(PermissionMode::ApproveReads)
    .with_non_interactive_permissions(Some(NonInteractivePermissionPolicy::Deny))
    .with_auth_policy(AuthPolicy::Skip);

    let event_client =
        client.build_event_client(PathBuf::from(".").as_path(), expected_session_id, event_tx);

    assert_eq!(event_client.permission_mode, PermissionMode::ApproveReads);
    assert_eq!(
        event_client.non_interactive_permissions,
        Some(NonInteractivePermissionPolicy::Deny)
    );
    assert!(Arc::ptr_eq(&event_client.permission_stats, &client.permission_stats));
    assert!(Arc::ptr_eq(&event_client.cancelling_session_ids, &client.cancelling_session_ids));
}

#[tokio::test]
async fn build_event_client_carries_session_update_callback() {
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let expected_session_id = Arc::new(Mutex::new(Some("session-1".to_string())));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_for_callback = seen.clone();
    let callback: SessionUpdateCallback = Arc::new(move |notification| {
        seen_for_callback.lock().push(notification.session_id.to_string());
    });
    let client = super::AcpClient::new(
        "test",
        AcpAgentConfig { command: "noop".to_string(), args: Vec::new(), env: Default::default() },
    )
    .with_auth_policy(AuthPolicy::Skip)
    .with_session_update_callback(Some(callback));

    let event_client =
        client.build_event_client(PathBuf::from(".").as_path(), expected_session_id, event_tx);

    event_client
        .session_notification(acp::SessionNotification::new(
            "session-1",
            acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(acp::ContentBlock::Text(
                acp::TextContent::new("hello"),
            ))),
        ))
        .await
        .expect("session notification");

    assert_eq!(&*seen.lock(), &["session-1".to_string()]);
}

#[tokio::test]
async fn ext_method_is_explicitly_unsupported() {
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let client = event_client(event_tx, None);
    let params = serde_json::value::RawValue::from_string("{}".to_string()).expect("raw value");

    let err = client
        .ext_method(acp::ExtRequest::new("custom/test", params.into()))
        .await
        .expect_err("ext method unsupported");

    assert!(err.to_string().to_ascii_lowercase().contains("method not found"));
}

#[tokio::test]
async fn ext_notification_is_explicitly_unsupported() {
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let client = event_client(event_tx, None);
    let params = serde_json::value::RawValue::from_string("{}".to_string()).expect("raw value");

    let err = client
        .ext_notification(acp::ExtNotification::new("custom/test", params.into()))
        .await
        .expect_err("ext notification unsupported");

    assert!(err.to_string().to_ascii_lowercase().contains("method not found"));
}
