//! 验证协议事件转发、会话变更、权限请求与回调触发行为。
//!
//! 协议连接、回调和通道 fixture 用于隔离验证客户端边界，不替代生产实现。

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
        claude_permissions: false,
        kimi_permissions: false,
        codebuddy_permissions: false,
        transient_permissions: false,
        reject_untyped_workflow_permissions: false,
        expected_session_id: Arc::new(Mutex::new(expected_session_id)),
        event_tx,
        desktop_workspace: None,
        active_prompt: Arc::new(Mutex::new(None)),
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
        claude_permissions: false,
        kimi_permissions: false,
        codebuddy_permissions: false,
        transient_permissions: false,
        reject_untyped_workflow_permissions: false,
        expected_session_id: Arc::new(Mutex::new(None)),
        event_tx,
        desktop_workspace: None,
        active_prompt: Arc::new(Mutex::new(None)),
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

// 下列行为用例覆盖有效响应、无效状态以及协议或通道失败路径。
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
async fn build_event_client_propagates_full_access_to_filesystem() {
    let root = unique_temp_dir("full-access-root");
    let outside_root = unique_temp_dir("full-access-outside");
    let outside_file = outside_root.join("outside.txt");
    fs::write(&outside_file, "outside content").expect("write outside fixture");
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let client = super::AcpClient::new(
        "test",
        AcpAgentConfig { command: "noop".to_string(), args: Vec::new(), env: Default::default() },
    )
    .with_full_access(true);
    let event_client = client.build_event_client(&root, Arc::new(Mutex::new(None)), event_tx);

    let response = event_client
        .read_text_file(acp::ReadTextFileRequest::new("session-1", &outside_file))
        .await;

    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(outside_root);

    assert_eq!(response.expect("full access propagated to filesystem").content, "outside content");
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

#[tokio::test]
async fn acp_skill_mount_qwen_scoped_sources_allow_reads_but_not_writes_or_unselected_files() {
    let workspace = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let foreign = tempfile::tempdir().unwrap();
    let attachment = source.path().join("value.txt");
    let outside = foreign.path().join("value.txt");
    fs::write(&attachment, "selected attachment").unwrap();
    fs::write(&outside, "unselected attachment").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, source.path().join("escape.txt")).unwrap();
    for mode in [PermissionMode::ApproveReads, PermissionMode::DenyAll] {
        let (tx, _rx) = mpsc::unbounded_channel();
        let client = super::AcpClient::new("qwen", AcpAgentConfig::default())
            .with_permission_mode(mode)
            .with_skill_mounts(
                Some(crate::SessionMountSelection {
                    version: 1,
                    skills: vec![crate::SessionSkillMount {
                        name: "selected".into(),
                        path: source.path().to_string_lossy().into(),
                    }],
                }),
                None,
            );
        let events = client.build_event_client(workspace.path(), Arc::new(Mutex::new(None)), tx);
        let read =
            events.read_text_file(acp::ReadTextFileRequest::new("fixture", &attachment)).await;
        if mode == PermissionMode::DenyAll {
            assert!(read.is_err());
        } else {
            assert_eq!(read.unwrap().content, "selected attachment");
        }
        assert!(
            events
                .read_text_file(acp::ReadTextFileRequest::new("fixture", &outside))
                .await
                .is_err()
        );
        #[cfg(unix)]
        assert!(
            events
                .read_text_file(acp::ReadTextFileRequest::new(
                    "fixture",
                    source.path().join("escape.txt")
                ))
                .await
                .is_err()
        );
        assert!(
            events
                .write_text_file(acp::WriteTextFileRequest::new(
                    "fixture",
                    &attachment,
                    "overwrite"
                ))
                .await
                .is_err()
        );
    }
    assert_eq!(fs::read_to_string(attachment).unwrap(), "selected attachment");
}

#[tokio::test]
async fn acp_skill_mount_gemini_scoped_sources_allow_reads_but_not_writes_or_unselected_files() {
    let workspace = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let foreign = tempfile::tempdir().unwrap();
    let attachment = source.path().join("value.txt");
    let outside = foreign.path().join("value.txt");
    fs::write(&attachment, "selected attachment").unwrap();
    fs::write(&outside, "unselected attachment").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, source.path().join("escape.txt")).unwrap();
    for mode in [PermissionMode::ApproveReads, PermissionMode::DenyAll] {
        let (tx, _rx) = mpsc::unbounded_channel();
        let client = super::AcpClient::new("gemini", AcpAgentConfig::default())
            .with_permission_mode(mode)
            .with_skill_mounts(
                Some(crate::SessionMountSelection {
                    version: 1,
                    skills: vec![crate::SessionSkillMount {
                        name: "selected".into(),
                        path: source.path().to_string_lossy().into(),
                    }],
                }),
                None,
            );
        let events = client.build_event_client(workspace.path(), Arc::new(Mutex::new(None)), tx);
        let read =
            events.read_text_file(acp::ReadTextFileRequest::new("fixture", &attachment)).await;
        if mode == PermissionMode::DenyAll {
            assert!(read.is_err());
        } else {
            assert_eq!(read.unwrap().content, "selected attachment");
        }
        assert!(
            events
                .read_text_file(acp::ReadTextFileRequest::new("fixture", &outside))
                .await
                .is_err()
        );
        #[cfg(unix)]
        assert!(
            events
                .read_text_file(acp::ReadTextFileRequest::new(
                    "fixture",
                    source.path().join("escape.txt")
                ))
                .await
                .is_err()
        );
        assert!(
            events
                .write_text_file(acp::WriteTextFileRequest::new(
                    "fixture",
                    &attachment,
                    "overwrite"
                ))
                .await
                .is_err()
        );
    }
    assert_eq!(fs::read_to_string(attachment).unwrap(), "selected attachment");
}

#[tokio::test]
async fn acp_skill_mount_codebuddy_scoped_sources_allow_reads_but_not_writes_or_unselected_files() {
    let workspace = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let foreign = tempfile::tempdir().unwrap();
    let attachment = source.path().join("value.txt");
    let outside = foreign.path().join("value.txt");
    fs::write(&attachment, "selected attachment").unwrap();
    fs::write(&outside, "unselected attachment").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, source.path().join("escape.txt")).unwrap();
    for mode in [PermissionMode::ApproveReads, PermissionMode::DenyAll] {
        let (tx, _rx) = mpsc::unbounded_channel();
        let client = super::AcpClient::new("codebuddy", AcpAgentConfig::default())
            .with_permission_mode(mode)
            .with_skill_mounts(
                Some(crate::SessionMountSelection {
                    version: 1,
                    skills: vec![crate::SessionSkillMount {
                        name: "selected".into(),
                        path: source.path().to_string_lossy().into(),
                    }],
                }),
                None,
            );
        let events = client.build_event_client(workspace.path(), Arc::new(Mutex::new(None)), tx);
        let read =
            events.read_text_file(acp::ReadTextFileRequest::new("fixture", &attachment)).await;
        if mode == PermissionMode::DenyAll {
            assert!(read.is_err());
        } else {
            assert_eq!(read.unwrap().content, "selected attachment");
        }
        assert!(
            events
                .read_text_file(acp::ReadTextFileRequest::new("fixture", &outside))
                .await
                .is_err()
        );
        #[cfg(unix)]
        assert!(
            events
                .read_text_file(acp::ReadTextFileRequest::new(
                    "fixture",
                    source.path().join("escape.txt")
                ))
                .await
                .is_err()
        );
        assert!(
            events
                .write_text_file(acp::WriteTextFileRequest::new(
                    "fixture",
                    &attachment,
                    "overwrite"
                ))
                .await
                .is_err()
        );
    }
    assert_eq!(fs::read_to_string(attachment).unwrap(), "selected attachment");
}

#[tokio::test]
async fn acp_skill_mount_kiro_scoped_sources_allow_reads_but_not_writes_or_unselected_files() {
    let workspace = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let foreign = tempfile::tempdir().unwrap();
    let attachment = source.path().join("value.txt");
    let outside = foreign.path().join("value.txt");
    fs::write(&attachment, "selected attachment").unwrap();
    fs::write(&outside, "unselected attachment").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, source.path().join("escape.txt")).unwrap();
    for mode in [PermissionMode::ApproveReads, PermissionMode::DenyAll] {
        let (tx, _rx) = mpsc::unbounded_channel();
        let client = super::AcpClient::new("kiro", AcpAgentConfig::default())
            .with_permission_mode(mode)
            .with_skill_mounts(
                Some(crate::SessionMountSelection {
                    version: 1,
                    skills: vec![crate::SessionSkillMount {
                        name: "selected".into(),
                        path: source.path().to_string_lossy().into(),
                    }],
                }),
                None,
            );
        let events = client.build_event_client(workspace.path(), Arc::new(Mutex::new(None)), tx);
        let read =
            events.read_text_file(acp::ReadTextFileRequest::new("fixture", &attachment)).await;
        if mode == PermissionMode::DenyAll {
            assert!(read.is_err());
        } else {
            assert_eq!(read.unwrap().content, "selected attachment");
        }
        assert!(
            events
                .read_text_file(acp::ReadTextFileRequest::new("fixture", &outside))
                .await
                .is_err()
        );
        #[cfg(unix)]
        assert!(
            events
                .read_text_file(acp::ReadTextFileRequest::new(
                    "fixture",
                    source.path().join("escape.txt")
                ))
                .await
                .is_err()
        );
        assert!(
            events
                .write_text_file(acp::WriteTextFileRequest::new(
                    "fixture",
                    &attachment,
                    "overwrite"
                ))
                .await
                .is_err()
        );
    }
    assert_eq!(fs::read_to_string(attachment).unwrap(), "selected attachment");
}

#[cfg(unix)]
async fn with_workspace_rpc(
    client: &super::AcpClient,
    root: &std::path::Path,
    run: impl AsyncFnOnce(tokio::io::DuplexStream),
) {
    use tokio_util::compat::TokioAsyncReadCompatExt;

    tokio::task::LocalSet::new()
        .run_until(async {
            let (client_io, agent_io) = tokio::io::duplex(8192);
            let (reader, writer) = tokio::io::split(client_io);
            let (event_tx, _) = mpsc::unbounded_channel();
            let (cancel_tx, _) = tokio::sync::watch::channel(false);
            let (_completed_tx, completed_rx) = tokio::sync::watch::channel(false);
            client.register_active_prompt("workspace".into(), cancel_tx, completed_rx);
            let events = client.build_event_client(
                root,
                Arc::new(Mutex::new(Some("workspace".into()))),
                event_tx,
            );
            let (_conn, io_task) = acp::ClientSideConnection::new(
                events,
                tokio_util::compat::TokioAsyncWriteCompatExt::compat_write(writer),
                reader.compat(),
                |fut| {
                    tokio::task::spawn_local(fut);
                },
            );
            tokio::task::spawn_local(io_task);
            run(agent_io).await;
        })
        .await;
}

#[cfg(unix)]
async fn workspace_rpc(
    io: &mut tokio::io::BufReader<tokio::io::DuplexStream>,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    let request =
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    io.write_all(format!("{request}\n").as_bytes()).await.unwrap();
    let mut response = String::new();
    tokio::time::timeout(std::time::Duration::from_secs(2), io.read_line(&mut response))
        .await
        .unwrap()
        .unwrap();
    serde_json::from_str(&response).unwrap()
}

#[cfg(unix)]
#[tokio::test]
async fn acp_desktop_approval_policy_allows_workspace_edits() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let file = root.path().join("new/note.txt");
    let escaped = outside.path().join("escaped");
    let existing = outside.path().join("existing");
    fs::write(&existing, "original").unwrap();
    fs::hard_link(&existing, root.path().join("hardlink")).unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("link")).unwrap();
    let client = super::AcpClient::new("claude", AcpAgentConfig::default())
        .with_desktop_workspace(Some(root.path().canonicalize().unwrap()))
        .with_permission_mode(PermissionMode::ApproveReads)
        .with_non_interactive_permissions(Some(NonInteractivePermissionPolicy::Fail));

    with_workspace_rpc(&client, root.path(), async |io| {
        let mut io = tokio::io::BufReader::new(io);
        for content in ["first", "second"] {
            let response = workspace_rpc(
                &mut io,
                "fs/write_text_file",
                serde_json::json!({
                    "sessionId": "workspace", "path": file, "content": content,
                }),
            )
            .await;
            assert!(response.get("result").is_some(), "{response}");
            assert_eq!(fs::read_to_string(&file).unwrap(), content);
        }
        for (session, path) in [
            ("foreign", root.path().join("foreign.txt")),
            ("workspace", escaped.clone()),
            ("workspace", root.path().join("link/escaped")),
            ("workspace", root.path().join(".git/config")),
            ("workspace", root.path().join(".mcp.json")),
            ("workspace", root.path().join("hardlink")),
        ] {
            let response = workspace_rpc(
                &mut io,
                "fs/write_text_file",
                serde_json::json!({
                    "sessionId": session, "path": path, "content": "denied",
                }),
            )
            .await;
            assert!(response.get("error").is_some(), "{path:?}: {response}");
        }
        assert!(!root.path().join("foreign.txt").exists());
        assert!(!root.path().join(".git").exists());
        assert!(!escaped.exists());
        assert_eq!(fs::read_to_string(&existing).unwrap(), "original");
        assert_eq!(client.permission_stats.lock().requested, 0);
    })
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn acp_desktop_approval_policy_keeps_generic_approvals_and_terminal_restricted() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("note.txt");
    let client = super::AcpClient::new("claude", AcpAgentConfig::default())
        .with_desktop_workspace(Some(root.path().canonicalize().unwrap()))
        .with_permission_mode(PermissionMode::ApproveReads)
        .with_non_interactive_permissions(Some(NonInteractivePermissionPolicy::Fail));

    with_workspace_rpc(&client, root.path(), async |io| {
        let mut io = tokio::io::BufReader::new(io);
        for kind in [
            None,
            Some(acp::ToolKind::Edit),
            Some(acp::ToolKind::Delete),
            Some(acp::ToolKind::Execute),
        ] {
            let mut request = permission_request("workspace");
            request.tool_call.fields.kind = kind;
            request.tool_call.fields.title = Some("Read a note".into());
            request.tool_call.fields.raw_input = Some(serde_json::json!({"file_path": target}));
            let response = workspace_rpc(
                &mut io,
                "session/request_permission",
                serde_json::to_value(request).unwrap(),
            )
            .await;
            assert!(response.get("error").is_some(), "{response}");
        }
        let terminal_file = root.path().join("terminal.txt");
        let response = workspace_rpc(
            &mut io,
            "terminal/create",
            serde_json::json!({
                "sessionId": "workspace", "command": "touch", "args": [terminal_file],
            }),
        )
        .await;
        assert!(response.get("error").is_some(), "{response}");
        assert!(!terminal_file.exists());
        assert!(!target.exists());
    })
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn acp_desktop_approval_policy_checks_replaced_parent_at_write() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let parent = root.path().join("subdir");
    let file = parent.join("note.txt");
    fs::create_dir(&parent).unwrap();
    let attacker_parent = parent.clone();
    let attacker_outside = outside.path().to_path_buf();
    let client = super::AcpClient::new("claude", AcpAgentConfig::default())
        .with_desktop_workspace(Some(root.path().canonicalize().unwrap()))
        .with_permission_mode(PermissionMode::ApproveReads)
        .with_client_operation_callback(Some(Arc::new(move |operation| {
            if operation.status == crate::types::ClientOperationStatus::Running {
                fs::rename(&attacker_parent, attacker_parent.with_extension("old")).unwrap();
                std::os::unix::fs::symlink(&attacker_outside, &attacker_parent).unwrap();
            }
        })));

    with_workspace_rpc(&client, root.path(), async |io| {
        let response = workspace_rpc(
            &mut tokio::io::BufReader::new(io),
            "fs/write_text_file",
            serde_json::json!({
                "sessionId": "workspace", "path": file, "content": "denied",
            }),
        )
        .await;
        assert!(response.get("error").is_some(), "{response}");
        assert!(!outside.path().join("note.txt").exists());
        assert!(!parent.with_extension("old").join("note.txt").exists());
    })
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn acp_desktop_approval_policy_only_current_full_grant_allows_external_writes() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let file = outside.path().join("note.txt");
    for (mode, full, expected) in [
        (PermissionMode::ApproveReads, false, false),
        (PermissionMode::ApproveAll, true, true),
        (PermissionMode::ApproveReads, false, false),
        (PermissionMode::DenyAll, true, false),
    ] {
        let client = super::AcpClient::new("claude", AcpAgentConfig::default())
            .with_desktop_workspace(Some(root.path().canonicalize().unwrap()))
            .with_permission_mode(mode)
            .with_full_access(full);
        with_workspace_rpc(&client, root.path(), async |io| {
            let mut io = tokio::io::BufReader::new(io);
            let response = workspace_rpc(
                &mut io,
                "fs/write_text_file",
                serde_json::json!({
                    "sessionId": "foreign", "path": file, "content": "foreign",
                }),
            )
            .await;
            assert!(response.get("error").is_some(), "{response}");
            let response = workspace_rpc(
                &mut io,
                "fs/write_text_file",
                serde_json::json!({
                    "sessionId": "workspace", "path": file, "content": "current",
                }),
            )
            .await;
            assert_eq!(response.get("result").is_some(), expected, "{response}");
            assert_eq!(file.exists(), expected);
            if expected {
                let foreign_read = workspace_rpc(
                    &mut io,
                    "fs/read_text_file",
                    serde_json::json!({"sessionId": "foreign", "path": file}),
                )
                .await;
                assert!(foreign_read.get("error").is_some(), "{foreign_read}");
                client.clear_active_prompt("workspace");
                let idle_read = workspace_rpc(
                    &mut io,
                    "fs/read_text_file",
                    serde_json::json!({"sessionId": "workspace", "path": file}),
                )
                .await;
                assert!(idle_read.get("error").is_some(), "{idle_read}");
                assert_eq!(fs::read_to_string(&file).unwrap(), "current");
                fs::remove_file(&file).unwrap();
            }
        })
        .await;
    }
}
