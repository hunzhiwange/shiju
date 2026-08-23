//! 验证队列 IPC 客户端的请求标识、所有者通信与错误响应处理。

use super::*;

use std::collections::HashMap;
use std::error::Error as StdError;
#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;
#[cfg(unix)]
use std::sync::atomic::{AtomicUsize, Ordering};

use agent_client_protocol::{SetSessionConfigOptionResponse, StopReason};
use serde_json::json;
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(unix)]
use tokio::net::UnixListener;

use crate::queue_messages::QueueRequest;
#[cfg(unix)]
use crate::queue_paths::{queue_base_dir, queue_lock_file_path, queue_socket_base_dir};
use crate::types::{
    AcpJsonRpcMessage, OutputErrorCode, OutputErrorOrigin, OutputErrorParams, PermissionMode,
    PermissionStats, SESSION_RECORD_SCHEMA, SessionEventLog, SessionRecord, SessionSendResult,
    SessionTokenUsage,
};

#[cfg(unix)]
static NEXT_SOCKET_ID: AtomicUsize = AtomicUsize::new(1);

#[cfg(unix)]
struct HomeEnvGuard {
    previous: Option<std::ffi::OsString>,
}

#[cfg(unix)]
impl HomeEnvGuard {
    fn set(home_dir: &Path) -> Self {
        let previous = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", home_dir) };
        Self { previous }
    }
}

#[cfg(unix)]
impl Drop for HomeEnvGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }
}

#[cfg(unix)]
struct TempHome {
    path: PathBuf,
}

#[cfg(unix)]
impl TempHome {
    async fn new(name: &str) -> Self {
        let id = NEXT_SOCKET_ID.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir()
            .join(format!("vw-acp-queue-ipc-home-{name}-{}-{id}", std::process::id()));
        let _ = tokio::fs::remove_dir_all(&path).await;
        tokio::fs::create_dir_all(&path).await.expect("create temp home");
        Self { path }
    }
}

#[cfg(unix)]
impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
        if let Some(socket_dir) = queue_socket_base_dir(&self.path) {
            let _ = std::fs::remove_dir_all(socket_dir);
        }
    }
}

#[cfg(unix)]
struct TestSocket {
    dir: PathBuf,
    path: PathBuf,
}

#[cfg(unix)]
impl TestSocket {
    fn new(_name: &str) -> Self {
        let id = NEXT_SOCKET_ID.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("vqi-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("socket temp dir");
        Self { path: dir.join("owner.sock"), dir }
    }
}

#[cfg(unix)]
impl Drop for TestSocket {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[derive(Default)]
struct TestFormatter {
    contexts: Vec<String>,
    messages: Vec<AcpJsonRpcMessage>,
    errors: Vec<OutputErrorParams>,
    flushes: usize,
}

impl OutputFormatter for TestFormatter {
    fn set_context(&mut self, context: OutputFormatterContext) {
        self.contexts.push(context.session_id);
    }

    fn on_acp_message(&mut self, message: AcpJsonRpcMessage) {
        self.messages.push(message);
    }

    fn on_error(&mut self, params: OutputErrorParams) {
        self.errors.push(params);
    }

    fn flush(&mut self) {
        self.flushes += 1;
    }
}

fn owner_record(socket_path: PathBuf) -> QueueOwnerRecord {
    QueueOwnerRecord {
        pid: 123,
        session_id: "session-1".to_string(),
        socket_path,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        heartbeat_at: "2026-01-01T00:00:00Z".to_string(),
        owner_generation: 7,
        queue_depth: 0,
    }
}

#[cfg(unix)]
fn default_owner_record(session_id: &str, socket_path: PathBuf) -> QueueOwnerRecord {
    let timestamp = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("format timestamp");
    QueueOwnerRecord {
        pid: 123,
        session_id: session_id.to_string(),
        socket_path,
        created_at: timestamp.clone(),
        heartbeat_at: timestamp,
        owner_generation: 7,
        queue_depth: 0,
    }
}

#[cfg(unix)]
async fn write_default_owner_record(home_dir: &Path, record: &QueueOwnerRecord) {
    tokio::fs::create_dir_all(queue_base_dir(home_dir)).await.expect("create queue dir");
    if let Some(socket_dir) = queue_socket_base_dir(home_dir) {
        tokio::fs::create_dir_all(socket_dir).await.expect("create socket dir");
    }

    let payload = serde_json::to_vec_pretty(record).expect("serialize owner record");
    tokio::fs::write(queue_lock_file_path(&record.session_id, home_dir), payload)
        .await
        .expect("write owner record");
}

fn test_record() -> SessionRecord {
    SessionRecord {
        schema: SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: "record-1".to_string(),
        acp_session_id: "session-1".to_string(),
        agent_session_id: None,
        agent_command: "agent".to_string(),
        agent_config: None,
        cwd: "/tmp/project".to_string(),
        name: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_used_at: "2026-01-01T00:00:00Z".to_string(),
        last_seq: 0,
        last_request_id: None,
        event_log: SessionEventLog {
            active_path: "/tmp/project/events.jsonl".to_string(),
            segment_count: 1,
            max_segment_bytes: 1024,
            max_segments: 4,
            last_write_at: None,
            last_write_error: None,
        },
        closed: Some(false),
        closed_at: None,
        pid: None,
        agent_started_at: None,
        last_prompt_at: None,
        last_agent_exit_code: None,
        last_agent_exit_signal: None,
        last_agent_exit_at: None,
        last_agent_disconnect_reason: None,
        protocol_version: None,
        agent_capabilities: None,
        title: None,
        messages: Vec::new(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: HashMap::new(),
        vwacp: None,
    }
}

fn test_session_send_result() -> SessionSendResult {
    SessionSendResult {
        stop_reason: StopReason::EndTurn,
        permission_stats: PermissionStats::default(),
        session_id: "session-1".to_string(),
        record: test_record(),
        resumed: false,
        load_error: None,
    }
}

fn accepted_message(request_id: &str) -> QueueOwnerMessage {
    QueueOwnerMessage::Accepted { request_id: request_id.to_string(), owner_generation: Some(7) }
}

fn queue_error_message(
    request_id: &str,
    output_already_emitted: Option<bool>,
) -> QueueOwnerMessage {
    QueueOwnerMessage::Error {
        request_id: request_id.to_string(),
        owner_generation: Some(7),
        code: OutputErrorCode::Runtime,
        detail_code: Some("QUEUE_RUNTIME_PROMPT_FAILED".to_string()),
        origin: OutputErrorOrigin::Queue,
        message: "prompt failed".to_string(),
        retryable: Some(false),
        acp: None,
        output_already_emitted,
    }
}

#[cfg(unix)]
fn start_fake_owner<F>(socket_path: &Path, respond: F) -> tokio::task::JoinHandle<QueueRequest>
where
    F: FnOnce(&QueueRequest) -> Vec<QueueOwnerMessage> + Send + 'static,
{
    let listener = UnixListener::bind(socket_path).expect("bind fake owner");
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept queue request");
        let (read, mut write) = stream.into_split();
        let mut reader = BufReader::new(read);
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("read request line");
        let request = serde_json::from_str::<QueueRequest>(line.trim()).expect("queue request");

        for message in respond(&request) {
            let payload = serde_json::to_string(&message).expect("serialize owner message");
            write.write_all(payload.as_bytes()).await.expect("write owner message");
            write.write_all(b"\n").await.expect("write owner newline");
        }
        write.shutdown().await.expect("shutdown owner writer");
        request
    })
}

#[cfg(unix)]
fn start_fake_owner_lines<F>(
    socket_path: &Path,
    respond: F,
) -> tokio::task::JoinHandle<QueueRequest>
where
    F: FnOnce(&QueueRequest) -> Vec<String> + Send + 'static,
{
    let listener = UnixListener::bind(socket_path).expect("bind fake owner");
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept queue request");
        let (read, mut write) = stream.into_split();
        let mut reader = BufReader::new(read);
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("read request line");
        let request = serde_json::from_str::<QueueRequest>(line.trim()).expect("queue request");

        for message in respond(&request) {
            write.write_all(message.as_bytes()).await.expect("write owner line");
            write.write_all(b"\n").await.expect("write owner newline");
        }
        write.shutdown().await.expect("shutdown owner writer");
        request
    })
}

fn submit_options<'a>(
    formatter: &'a mut dyn OutputFormatter,
    wait_for_completion: bool,
) -> SubmitToQueueOwnerOptions<'a> {
    SubmitToQueueOwnerOptions {
        session_id: "session-1",
        message: "hello".to_string(),
        prompt: None,
        permission_mode: PermissionMode::ApproveReads,
        resume_policy: None,
        non_interactive_permissions: None,
        output_formatter: formatter,
        error_emission_policy: None,
        timeout_ms: Some(250),
        suppress_sdk_console_errors: Some(true),
        wait_for_completion,
        verbose: false,
    }
}

#[test]
fn next_queue_request_id_returns_unique_queue_prefixed_ids() {
    let first = next_queue_request_id();
    let second = next_queue_request_id();

    assert!(first.starts_with("queue-"));
    assert!(second.starts_with("queue-"));
    assert_ne!(first, second);
}

#[test]
fn queue_request_id_reads_all_request_variants() {
    let requests = [
        QueueRequest::SubmitPrompt {
            request_id: "submit".to_string(),
            owner_generation: Some(7),
            message: "hello".to_string(),
            prompt: crate::text_prompt("hello".to_string()),
            permission_mode: PermissionMode::ApproveReads,
            resume_policy: None,
            non_interactive_permissions: None,
            timeout_ms: None,
            suppress_sdk_console_errors: None,
            wait_for_completion: true,
        },
        QueueRequest::CancelPrompt { request_id: "cancel".to_string(), owner_generation: Some(7) },
        QueueRequest::SetMode {
            request_id: "mode".to_string(),
            owner_generation: Some(7),
            mode_id: "plan".to_string(),
            timeout_ms: Some(10),
        },
        QueueRequest::SetModel {
            request_id: "model".to_string(),
            owner_generation: Some(7),
            model_id: "model-a".to_string(),
            timeout_ms: Some(11),
        },
        QueueRequest::SetConfigOption {
            request_id: "config".to_string(),
            owner_generation: Some(7),
            config_id: "temperature".to_string(),
            value: "0.2".to_string(),
            timeout_ms: Some(12),
        },
    ];

    assert_eq!(
        requests.iter().map(queue_request_id).collect::<Vec<_>>(),
        ["submit", "cancel", "mode", "model", "config"]
    );
}

#[test]
fn queue_owner_message_request_id_reads_all_message_variants() {
    let messages = [
        accepted_message("accepted"),
        QueueOwnerMessage::Event {
            request_id: "event".to_string(),
            owner_generation: Some(7),
            message: agent_client_protocol::JsonRpcMessage::wrap(json!({
                "method": "session/update"
            })),
        },
        QueueOwnerMessage::Result {
            request_id: "result".to_string(),
            owner_generation: Some(7),
            result: Box::new(test_session_send_result()),
        },
        QueueOwnerMessage::CancelResult {
            request_id: "cancel".to_string(),
            owner_generation: Some(7),
            cancelled: true,
        },
        QueueOwnerMessage::SetModeResult {
            request_id: "mode".to_string(),
            owner_generation: Some(7),
            mode_id: "plan".to_string(),
        },
        QueueOwnerMessage::SetModelResult {
            request_id: "model".to_string(),
            owner_generation: Some(7),
            model_id: "model-a".to_string(),
        },
        QueueOwnerMessage::SetConfigOptionResult {
            request_id: "config".to_string(),
            owner_generation: Some(7),
            response: SetSessionConfigOptionResponse::new(Vec::new()),
        },
        queue_error_message("error", None),
    ];

    assert_eq!(
        messages.iter().map(queue_owner_message_request_id).collect::<Vec<_>>(),
        ["accepted", "event", "result", "cancel", "mode", "model", "config", "error"]
    );
}

#[test]
fn queue_connection_error_helpers_set_queue_runtime_shape() {
    let error = make_queue_connection_error("failed", "DETAIL", true);

    assert_eq!(error.message(), "failed");
    assert_eq!(error.output_code(), Some(OutputErrorCode::Runtime));
    assert_eq!(error.detail_code(), Some("DETAIL"));
    assert_eq!(error.origin(), Some(OutputErrorOrigin::Queue));
    assert_eq!(error.retryable(), Some(true));
}

#[test]
fn queue_connection_error_helper_preserves_source() {
    let error = make_queue_connection_error_with_source(
        "failed",
        "DETAIL",
        false,
        std::io::Error::other("source failure"),
    );

    assert_eq!(error.retryable(), Some(false));
    assert_eq!(
        StdError::source(&error).map(ToString::to_string).as_deref(),
        Some("source failure")
    );
}

#[test]
fn stale_owner_protocol_error_only_matches_recoverable_detail_codes() {
    let malformed = make_queue_connection_error("bad", "QUEUE_PROTOCOL_MALFORMED_MESSAGE", true);
    let unexpected = make_queue_connection_error("bad", "QUEUE_PROTOCOL_UNEXPECTED_RESPONSE", true);
    let invalid_json = make_queue_connection_error("bad", "QUEUE_PROTOCOL_INVALID_JSON", true);
    let plain = QueueConnectionError::new("bad", AcpxErrorOptions::default());

    assert!(is_stale_owner_protocol_error(&malformed));
    assert!(is_stale_owner_protocol_error(&unexpected));
    assert!(!is_stale_owner_protocol_error(&invalid_json));
    assert!(!is_stale_owner_protocol_error(&plain));
}

#[test]
fn assert_owner_generation_accepts_matching_and_missing_message_generation() {
    let owner = owner_record(PathBuf::from("owner.sock"));

    assert!(
        assert_owner_generation(
            &owner,
            QueueOwnerMessage::CancelResult {
                request_id: "req-1".to_string(),
                owner_generation: Some(7),
                cancelled: true,
            },
        )
        .is_ok()
    );
    assert!(
        assert_owner_generation(
            &owner,
            QueueOwnerMessage::CancelResult {
                request_id: "req-1".to_string(),
                owner_generation: None,
                cancelled: true,
            },
        )
        .is_ok()
    );
}

#[test]
fn assert_owner_generation_rejects_mismatched_generation() {
    let owner = owner_record(PathBuf::from("owner.sock"));
    let error = assert_owner_generation(
        &owner,
        QueueOwnerMessage::CancelResult {
            request_id: "req-1".to_string(),
            owner_generation: Some(8),
            cancelled: true,
        },
    )
    .expect_err("mismatched generation should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_OWNER_GENERATION_MISMATCH"));
    assert_eq!(error.retryable(), Some(true));
}

#[test]
fn parse_queue_owner_response_line_accepts_valid_matching_message() {
    let owner = owner_record(PathBuf::from("owner.sock"));
    let payload = serde_json::to_string(&QueueOwnerMessage::SetModelResult {
        request_id: "req-1".to_string(),
        owner_generation: Some(7),
        model_id: "model-a".to_string(),
    })
    .expect("serialize message");

    let message =
        parse_queue_owner_response_line(&owner, "req-1", &payload).expect("valid response");

    assert!(matches!(
        message,
        QueueOwnerMessage::SetModelResult { request_id, owner_generation, model_id }
            if request_id == "req-1"
                && owner_generation == Some(7)
                && model_id == "model-a"
    ));
}

#[test]
fn parse_queue_owner_response_line_rejects_invalid_json() {
    let owner = owner_record(PathBuf::from("owner.sock"));
    let error = parse_queue_owner_response_line(&owner, "req-1", "{").expect_err("invalid json");

    assert_eq!(error.detail_code(), Some("QUEUE_PROTOCOL_INVALID_JSON"));
    assert_eq!(error.retryable(), Some(true));
    assert!(StdError::source(&error).is_some());
}

#[test]
fn parse_queue_owner_response_line_rejects_malformed_message() {
    let owner = owner_record(PathBuf::from("owner.sock"));
    let error = parse_queue_owner_response_line(&owner, "req-1", r#"{"type":"accepted"}"#)
        .expect_err("malformed message");

    assert_eq!(error.detail_code(), Some("QUEUE_PROTOCOL_MALFORMED_MESSAGE"));
    assert_eq!(error.retryable(), Some(true));
}

#[test]
fn parse_queue_owner_response_line_rejects_mismatched_request_id() {
    let owner = owner_record(PathBuf::from("owner.sock"));
    let payload = serde_json::to_string(&accepted_message("other")).expect("serialize message");
    let error = parse_queue_owner_response_line(&owner, "req-1", &payload)
        .expect_err("mismatched request id");

    assert_eq!(error.detail_code(), Some("QUEUE_PROTOCOL_MALFORMED_MESSAGE"));
    assert_eq!(error.retryable(), Some(true));
}

#[test]
fn parse_queue_owner_response_line_rejects_mismatched_owner_generation() {
    let owner = owner_record(PathBuf::from("owner.sock"));
    let payload = serde_json::to_string(&QueueOwnerMessage::SetModeResult {
        request_id: "req-1".to_string(),
        owner_generation: Some(8),
        mode_id: "plan".to_string(),
    })
    .expect("serialize message");

    let error = parse_queue_owner_response_line(&owner, "req-1", &payload)
        .expect_err("mismatched owner generation");

    assert_eq!(error.detail_code(), Some("QUEUE_OWNER_GENERATION_MISMATCH"));
    assert_eq!(error.retryable(), Some(true));
}

#[tokio::test]
async fn stale_owner_recovery_ignores_nonrecoverable_protocol_error() {
    let owner = owner_record(PathBuf::from("owner.sock"));
    let error = make_queue_connection_error("bad json", "QUEUE_PROTOCOL_INVALID_JSON", true);

    assert!(
        !maybe_recover_stale_owner_after_protocol_mismatch("session-1", &owner, &error, false,)
            .await
    );
}

#[cfg(unix)]
#[tokio::test]
async fn run_queue_owner_request_writes_request_and_returns_acknowledged_response() {
    let socket = TestSocket::new("run-success");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            accepted_message(request_id),
            QueueOwnerMessage::CancelResult {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                cancelled: true,
            },
        ]
    });
    let request =
        QueueRequest::CancelPrompt { request_id: "req-run".to_string(), owner_generation: Some(7) };
    let mut accepted_calls = 0usize;

    let result = run_queue_owner_request(
        &owner,
        &request,
        || {
            accepted_calls += 1;
            Ok(None)
        },
        |message, acknowledged| match message {
            QueueOwnerMessage::CancelResult { cancelled, .. } => {
                assert!(acknowledged);
                Ok(Some(cancelled))
            }
            _ => panic!("unexpected message"),
        },
        |_| panic!("close handler should not run"),
    )
    .await
    .expect("queue request should succeed");

    assert_eq!(result, Some(true));
    assert_eq!(accepted_calls, 1);
    assert_eq!(handle.await.expect("fake owner task"), request);
}

#[cfg(unix)]
#[tokio::test]
async fn run_queue_owner_request_reports_ack_missing_for_response_before_accept() {
    let socket = TestSocket::new("ack-missing");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        vec![QueueOwnerMessage::CancelResult {
            request_id: queue_request_id(request).to_string(),
            owner_generation: Some(7),
            cancelled: true,
        }]
    });
    let request = QueueRequest::CancelPrompt {
        request_id: "req-missing-ack".to_string(),
        owner_generation: Some(7),
    };

    let error = run_queue_owner_request(
        &owner,
        &request,
        || Ok::<Option<bool>, QueueConnectionError>(None),
        |message, acknowledged| {
            if !acknowledged {
                return Err(make_queue_connection_error(
                    "Queue owner did not acknowledge request",
                    "QUEUE_ACK_MISSING",
                    true,
                ));
            }
            Ok(Some(matches!(message, QueueOwnerMessage::CancelResult { .. })))
        },
        |_| panic!("close handler should not run"),
    )
    .await
    .expect_err("missing ack should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_ACK_MISSING"));
    handle.await.expect("fake owner task");
}

#[cfg(unix)]
#[tokio::test]
async fn run_queue_owner_request_reports_disconnect_before_ack() {
    let socket = TestSocket::new("close-before-ack");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |_| Vec::new());
    let request = QueueRequest::CancelPrompt {
        request_id: "req-close-before-ack".to_string(),
        owner_generation: Some(7),
    };

    let error = run_queue_owner_request(
        &owner,
        &request,
        || Ok::<Option<bool>, QueueConnectionError>(None),
        |_, _| panic!("message handler should not run"),
        |acknowledged| {
            assert!(!acknowledged);
            Err(make_queue_connection_error(
                "Queue owner disconnected before acknowledging request",
                "QUEUE_DISCONNECTED_BEFORE_ACK",
                true,
            ))
        },
    )
    .await
    .expect_err("close before ack should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_DISCONNECTED_BEFORE_ACK"));
    handle.await.expect("fake owner task");
}

#[cfg(unix)]
#[tokio::test]
async fn run_queue_owner_request_reports_disconnect_after_ack() {
    let socket = TestSocket::new("close-after-ack");
    let owner = owner_record(socket.path.clone());
    let handle =
        start_fake_owner(&socket.path, |request| vec![accepted_message(queue_request_id(request))]);
    let request = QueueRequest::CancelPrompt {
        request_id: "req-close-after-ack".to_string(),
        owner_generation: Some(7),
    };

    let error = run_queue_owner_request(
        &owner,
        &request,
        || Ok::<Option<bool>, QueueConnectionError>(None),
        |_, _| panic!("message handler should not run"),
        |acknowledged| {
            assert!(acknowledged);
            Err(make_queue_connection_error(
                "Queue owner disconnected before responding",
                "QUEUE_DISCONNECTED_BEFORE_COMPLETION",
                true,
            ))
        },
    )
    .await
    .expect_err("close after ack should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_DISCONNECTED_BEFORE_COMPLETION"));
    handle.await.expect("fake owner task");
}

#[cfg(unix)]
#[tokio::test]
async fn run_queue_owner_request_skips_blank_lines_before_messages() {
    let socket = TestSocket::new("blank-lines");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner_lines(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            "   ".to_string(),
            serde_json::to_string(&accepted_message(request_id)).expect("serialize accepted"),
            serde_json::to_string(&QueueOwnerMessage::CancelResult {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                cancelled: true,
            })
            .expect("serialize cancel result"),
        ]
    });
    let request = QueueRequest::CancelPrompt {
        request_id: "req-blank-lines".to_string(),
        owner_generation: Some(7),
    };

    let result = run_queue_owner_request(
        &owner,
        &request,
        || Ok::<Option<bool>, QueueConnectionError>(None),
        |message, acknowledged| {
            assert!(acknowledged);
            match message {
                QueueOwnerMessage::CancelResult { cancelled, .. } => Ok(Some(cancelled)),
                _ => panic!("unexpected message"),
            }
        },
        |_| panic!("close handler should not run"),
    )
    .await
    .expect("blank lines should be ignored");

    assert_eq!(result, Some(true));
    handle.await.expect("fake owner task");
}

#[cfg(unix)]
#[tokio::test]
async fn run_queue_owner_request_rejects_oversized_message() {
    let socket = TestSocket::new("oversized-message");
    let owner = owner_record(socket.path.clone());
    let handle =
        start_fake_owner_lines(&socket.path, |_| vec!["x".repeat(MAX_MESSAGE_BUFFER_SIZE + 1)]);
    let request = QueueRequest::CancelPrompt {
        request_id: "req-oversized".to_string(),
        owner_generation: Some(7),
    };

    let error = run_queue_owner_request(
        &owner,
        &request,
        || Ok::<Option<bool>, QueueConnectionError>(None),
        |_, _| panic!("message handler should not run"),
        |_| panic!("close handler should not run"),
    )
    .await
    .expect_err("oversized message should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_PROTOCOL_MALFORMED_MESSAGE"));
    assert_eq!(error.retryable(), Some(true));
    handle.await.expect("fake owner task");
}

#[cfg(unix)]
#[tokio::test]
async fn submit_to_queue_owner_enqueue_returns_after_accept() {
    let socket = TestSocket::new("submit-enqueue");
    let owner = owner_record(socket.path.clone());
    let handle =
        start_fake_owner(&socket.path, |request| vec![accepted_message(queue_request_id(request))]);
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, false);

    let result = submit_to_queue_owner(&owner, &mut options)
        .await
        .expect("submit should succeed")
        .expect("accepted submit should return result");

    match result {
        SessionSendOutcome::SessionEnqueueResult(result) => {
            assert!(result.queued);
            assert_eq!(result.session_id, "session-1");
            assert!(result.request_id.starts_with("queue-"));
        }
        _ => panic!("unexpected submit result"),
    }
    let request = handle.await.expect("fake owner task");
    assert!(matches!(
        request,
        QueueRequest::SubmitPrompt {
            owner_generation: Some(7),
            permission_mode: PermissionMode::ApproveReads,
            timeout_ms: Some(250),
            suppress_sdk_console_errors: Some(true),
            wait_for_completion: false,
            ..
        }
    ));
    assert_eq!(formatter.contexts, ["session-1"]);
}

#[cfg(unix)]
#[tokio::test]
async fn submit_to_queue_owner_forwards_events_and_flushes_result() {
    let socket = TestSocket::new("submit-result");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            accepted_message(request_id),
            QueueOwnerMessage::Event {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                message: agent_client_protocol::JsonRpcMessage::wrap(json!({
                    "method": "session/update"
                })),
            },
            QueueOwnerMessage::Result {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                result: Box::new(test_session_send_result()),
            },
        ]
    });
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, true);

    let result = submit_to_queue_owner(&owner, &mut options)
        .await
        .expect("submit should succeed")
        .expect("completed submit should return result");

    match result {
        SessionSendOutcome::SessionSendResult(result) => {
            assert_eq!(result.session_id, "session-1");
        }
        _ => panic!("unexpected submit result"),
    }
    handle.await.expect("fake owner task");
    assert_eq!(formatter.messages.len(), 1);
    assert_eq!(formatter.flushes, 1);
}

#[cfg(unix)]
#[tokio::test]
async fn submit_to_queue_owner_emits_queue_error_when_needed() {
    let socket = TestSocket::new("submit-error");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![accepted_message(request_id), queue_error_message(request_id, Some(false))]
    });
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, true);

    let error =
        submit_to_queue_owner(&owner, &mut options).await.expect_err("queue error should fail");

    assert_eq!(error.message(), "prompt failed");
    assert_eq!(error.detail_code(), Some("QUEUE_RUNTIME_PROMPT_FAILED"));
    assert_eq!(error.origin(), Some(OutputErrorOrigin::Queue));
    assert_eq!(error.retryable(), Some(false));
    assert!(error.output_already_emitted());
    handle.await.expect("fake owner task");
    assert_eq!(formatter.errors.len(), 1);
    assert_eq!(formatter.flushes, 1);
}

#[cfg(unix)]
#[tokio::test]
async fn submit_to_queue_owner_skips_formatter_error_when_owner_already_emitted() {
    let socket = TestSocket::new("submit-error-skip");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![accepted_message(request_id), queue_error_message(request_id, Some(true))]
    });
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, true);

    let error =
        submit_to_queue_owner(&owner, &mut options).await.expect_err("queue error should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_RUNTIME_PROMPT_FAILED"));
    handle.await.expect("fake owner task");
    assert!(formatter.errors.is_empty());
    assert_eq!(formatter.flushes, 0);
}

#[cfg(unix)]
#[tokio::test]
async fn submit_to_queue_owner_forwards_explicit_prompt_and_policies() {
    let socket = TestSocket::new("submit-options");
    let owner = owner_record(socket.path.clone());
    let handle =
        start_fake_owner(&socket.path, |request| vec![accepted_message(queue_request_id(request))]);
    let explicit_prompt = crate::text_prompt("explicit prompt".to_string());
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, false);
    options.prompt = Some(explicit_prompt.clone());
    options.resume_policy = Some(SessionResumePolicy::SameSessionOnly);
    options.non_interactive_permissions = Some(NonInteractivePermissionPolicy::Fail);

    submit_to_queue_owner(&owner, &mut options)
        .await
        .expect("submit should succeed")
        .expect("accepted submit should return result");

    let request = handle.await.expect("fake owner task");
    match request {
        QueueRequest::SubmitPrompt {
            message,
            prompt,
            resume_policy,
            non_interactive_permissions,
            ..
        } => {
            assert_eq!(message, "hello");
            assert_eq!(
                serde_json::to_value(prompt).expect("serialize forwarded prompt"),
                serde_json::to_value(explicit_prompt).expect("serialize expected prompt")
            );
            assert_eq!(resume_policy, Some(SessionResumePolicy::SameSessionOnly));
            assert_eq!(non_interactive_permissions, Some(NonInteractivePermissionPolicy::Fail));
        }
        _ => panic!("unexpected queue request"),
    }
}

#[cfg(unix)]
#[tokio::test]
async fn submit_to_queue_owner_rejects_event_before_ack() {
    let socket = TestSocket::new("submit-event-before-ack");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        vec![QueueOwnerMessage::Event {
            request_id: queue_request_id(request).to_string(),
            owner_generation: Some(7),
            message: agent_client_protocol::JsonRpcMessage::wrap(json!({
                "method": "session/update"
            })),
        }]
    });
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, true);

    let error = submit_to_queue_owner(&owner, &mut options)
        .await
        .expect_err("event before ack should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_ACK_MISSING"));
    handle.await.expect("fake owner task");
    assert!(formatter.messages.is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn submit_to_queue_owner_rejects_result_before_ack() {
    let socket = TestSocket::new("submit-result-before-ack");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        vec![QueueOwnerMessage::Result {
            request_id: queue_request_id(request).to_string(),
            owner_generation: Some(7),
            result: Box::new(test_session_send_result()),
        }]
    });
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, true);

    let error = submit_to_queue_owner(&owner, &mut options)
        .await
        .expect_err("result before ack should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_ACK_MISSING"));
    handle.await.expect("fake owner task");
    assert_eq!(formatter.flushes, 0);
}

#[cfg(unix)]
#[tokio::test]
async fn submit_to_queue_owner_rejects_unexpected_response_after_ack() {
    let socket = TestSocket::new("submit-unexpected-response");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            accepted_message(request_id),
            QueueOwnerMessage::CancelResult {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                cancelled: true,
            },
        ]
    });
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, true);

    let error = submit_to_queue_owner(&owner, &mut options)
        .await
        .expect_err("unexpected response should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_PROTOCOL_UNEXPECTED_RESPONSE"));
    handle.await.expect("fake owner task");
}

#[cfg(unix)]
#[tokio::test]
async fn submit_to_queue_owner_reports_disconnect_before_ack() {
    let socket = TestSocket::new("submit-close-before-ack");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |_| Vec::new());
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, true);

    let error = submit_to_queue_owner(&owner, &mut options)
        .await
        .expect_err("disconnect before ack should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_DISCONNECTED_BEFORE_ACK"));
    handle.await.expect("fake owner task");
}

#[cfg(unix)]
#[tokio::test]
async fn submit_to_queue_owner_reports_disconnect_after_ack() {
    let socket = TestSocket::new("submit-close-after-ack");
    let owner = owner_record(socket.path.clone());
    let handle =
        start_fake_owner(&socket.path, |request| vec![accepted_message(queue_request_id(request))]);
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, true);

    let error = submit_to_queue_owner(&owner, &mut options)
        .await
        .expect_err("disconnect after ack should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_DISCONNECTED_BEFORE_COMPLETION"));
    handle.await.expect("fake owner task");
}

#[cfg(unix)]
#[tokio::test]
async fn submit_to_queue_owner_policy_can_require_local_error_emission() {
    let socket = TestSocket::new("submit-error-policy");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![accepted_message(request_id), queue_error_message(request_id, Some(true))]
    });
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, true);
    options.error_emission_policy =
        Some(OutputErrorEmissionPolicy { queue_error_already_emitted: false });

    let error =
        submit_to_queue_owner(&owner, &mut options).await.expect_err("queue error should fail");

    assert!(!error.output_already_emitted());
    handle.await.expect("fake owner task");
    assert_eq!(formatter.errors.len(), 1);
    assert_eq!(formatter.flushes, 1);
}

#[cfg(unix)]
#[tokio::test]
async fn submit_control_to_queue_owner_returns_expected_response() {
    let socket = TestSocket::new("control-success");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            accepted_message(request_id),
            QueueOwnerMessage::SetModeResult {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                mode_id: "plan".to_string(),
            },
        ]
    });
    let request = QueueRequest::SetMode {
        request_id: "req-control".to_string(),
        owner_generation: Some(7),
        mode_id: "plan".to_string(),
        timeout_ms: Some(50),
    };

    let result = submit_control_to_queue_owner(&owner, &request, |message| match message {
        QueueOwnerMessage::SetModeResult { mode_id, .. } => Some(mode_id),
        _ => None,
    })
    .await
    .expect("control request should succeed");

    assert_eq!(result.as_deref(), Some("plan"));
    handle.await.expect("fake owner task");
}

#[cfg(unix)]
#[tokio::test]
async fn submit_control_to_queue_owner_propagates_owner_error() {
    let socket = TestSocket::new("control-error");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![accepted_message(request_id), queue_error_message(request_id, None)]
    });
    let request = QueueRequest::CancelPrompt {
        request_id: "req-control-error".to_string(),
        owner_generation: Some(7),
    };

    let error = submit_control_to_queue_owner::<bool>(&owner, &request, |_| None)
        .await
        .expect_err("owner error should fail");

    assert_eq!(error.message(), "prompt failed");
    assert_eq!(error.detail_code(), Some("QUEUE_RUNTIME_PROMPT_FAILED"));
    assert_eq!(error.output_code(), Some(OutputErrorCode::Runtime));
    handle.await.expect("fake owner task");
}

#[cfg(unix)]
#[tokio::test]
async fn submit_control_to_queue_owner_rejects_response_before_ack() {
    let socket = TestSocket::new("control-before-ack");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        vec![QueueOwnerMessage::SetModeResult {
            request_id: queue_request_id(request).to_string(),
            owner_generation: Some(7),
            mode_id: "plan".to_string(),
        }]
    });
    let request = QueueRequest::SetMode {
        request_id: "req-control-before-ack".to_string(),
        owner_generation: Some(7),
        mode_id: "plan".to_string(),
        timeout_ms: None,
    };

    let error = submit_control_to_queue_owner(&owner, &request, |message| match message {
        QueueOwnerMessage::SetModeResult { mode_id, .. } => Some(mode_id),
        _ => None,
    })
    .await
    .expect_err("response before ack should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_ACK_MISSING"));
    handle.await.expect("fake owner task");
}

#[cfg(unix)]
#[tokio::test]
async fn submit_control_to_queue_owner_rejects_unexpected_response() {
    let socket = TestSocket::new("control-unexpected");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            accepted_message(request_id),
            QueueOwnerMessage::SetModeResult {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                mode_id: "plan".to_string(),
            },
        ]
    });
    let request = QueueRequest::CancelPrompt {
        request_id: "req-control-unexpected".to_string(),
        owner_generation: Some(7),
    };

    let error = submit_control_to_queue_owner(&owner, &request, |message| match message {
        QueueOwnerMessage::CancelResult { cancelled, .. } => Some(cancelled),
        _ => None,
    })
    .await
    .expect_err("unexpected response should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_PROTOCOL_UNEXPECTED_RESPONSE"));
    handle.await.expect("fake owner task");
}

#[cfg(unix)]
#[tokio::test]
async fn submit_control_to_queue_owner_reports_disconnect_before_ack() {
    let socket = TestSocket::new("control-close-before-ack");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |_| Vec::new());
    let request = QueueRequest::CancelPrompt {
        request_id: "req-control-close-before-ack".to_string(),
        owner_generation: Some(7),
    };

    let error = submit_control_to_queue_owner::<bool>(&owner, &request, |_| None)
        .await
        .expect_err("disconnect before ack should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_DISCONNECTED_BEFORE_ACK"));
    handle.await.expect("fake owner task");
}

#[cfg(unix)]
#[tokio::test]
async fn submit_control_to_queue_owner_reports_disconnect_after_ack() {
    let socket = TestSocket::new("control-close-after-ack");
    let owner = owner_record(socket.path.clone());
    let handle =
        start_fake_owner(&socket.path, |request| vec![accepted_message(queue_request_id(request))]);
    let request = QueueRequest::CancelPrompt {
        request_id: "req-control-close-after-ack".to_string(),
        owner_generation: Some(7),
    };

    let error = submit_control_to_queue_owner::<bool>(&owner, &request, |_| None)
        .await
        .expect_err("disconnect after ack should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_DISCONNECTED_BEFORE_COMPLETION"));
    handle.await.expect("fake owner task");
}

#[cfg(unix)]
#[tokio::test]
async fn submit_cancel_to_queue_owner_returns_cancel_result() {
    let socket = TestSocket::new("cancel");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            accepted_message(request_id),
            QueueOwnerMessage::CancelResult {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                cancelled: false,
            },
        ]
    });

    let result = submit_cancel_to_queue_owner(&owner)
        .await
        .expect("cancel should succeed")
        .expect("cancel result");

    assert!(!result);
    assert!(matches!(
        handle.await.expect("fake owner task"),
        QueueRequest::CancelPrompt { owner_generation: Some(7), .. }
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn submit_set_mode_to_queue_owner_sends_mode_and_timeout() {
    let socket = TestSocket::new("set-mode");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            accepted_message(request_id),
            QueueOwnerMessage::SetModeResult {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                mode_id: "plan".to_string(),
            },
        ]
    });

    let result = submit_set_mode_to_queue_owner(&owner, "plan", Some(51))
        .await
        .expect("set mode should succeed")
        .expect("set mode result");

    assert!(result);
    assert!(matches!(
        handle.await.expect("fake owner task"),
        QueueRequest::SetMode {
            owner_generation: Some(7),
            mode_id,
            timeout_ms: Some(51),
            ..
        } if mode_id == "plan"
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn submit_set_model_to_queue_owner_sends_model_and_timeout() {
    let socket = TestSocket::new("set-model");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            accepted_message(request_id),
            QueueOwnerMessage::SetModelResult {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                model_id: "model-a".to_string(),
            },
        ]
    });

    let result = submit_set_model_to_queue_owner(&owner, "model-a", Some(52))
        .await
        .expect("set model should succeed")
        .expect("set model result");

    assert!(result);
    assert!(matches!(
        handle.await.expect("fake owner task"),
        QueueRequest::SetModel {
            owner_generation: Some(7),
            model_id,
            timeout_ms: Some(52),
            ..
        } if model_id == "model-a"
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn submit_set_config_option_to_queue_owner_sends_config_value_and_timeout() {
    let socket = TestSocket::new("set-config");
    let owner = owner_record(socket.path.clone());
    let handle = start_fake_owner(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            accepted_message(request_id),
            QueueOwnerMessage::SetConfigOptionResult {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                response: SetSessionConfigOptionResponse::new(Vec::new()),
            },
        ]
    });

    let result = submit_set_config_option_to_queue_owner(&owner, "temperature", "0.2", Some(53))
        .await
        .expect("set config should succeed")
        .expect("set config result");

    assert_eq!(result, SetSessionConfigOptionResponse::new(Vec::new()));
    assert!(matches!(
        handle.await.expect("fake owner task"),
        QueueRequest::SetConfigOption {
            owner_generation: Some(7),
            config_id,
            value,
            timeout_ms: Some(53),
            ..
        } if config_id == "temperature" && value == "0.2"
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn try_running_owner_entrypoints_return_none_without_owner_record() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let temp_home = TempHome::new("missing-owner").await;
    let _home_guard = HomeEnvGuard::set(&temp_home.path);
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, false);

    assert!(try_submit_to_running_owner(&mut options).await.expect("submit lookup").is_none());
    assert!(
        try_cancel_on_running_owner("missing-session", false)
            .await
            .expect("cancel lookup")
            .is_none()
    );
    assert!(
        try_set_mode_on_running_owner("missing-session", "plan", Some(10), false)
            .await
            .expect("set mode lookup")
            .is_none()
    );
    assert!(
        try_set_model_on_running_owner("missing-session", "model-a", Some(11), false)
            .await
            .expect("set model lookup")
            .is_none()
    );
    assert!(
        try_set_config_option_on_running_owner(
            "missing-session",
            "temperature",
            "0.2",
            Some(12),
            false,
        )
        .await
        .expect("set config lookup")
        .is_none()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn try_running_owner_entrypoints_forward_successful_requests() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let temp_home = TempHome::new("success").await;
    let _home_guard = HomeEnvGuard::set(&temp_home.path);

    let submit_socket = TestSocket::new("ok-sub");
    write_default_owner_record(
        &temp_home.path,
        &default_owner_record("session-1", submit_socket.path.clone()),
    )
    .await;
    let submit_handle = start_fake_owner(&submit_socket.path, |request| {
        vec![accepted_message(queue_request_id(request))]
    });
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, false);
    let submit_result = try_submit_to_running_owner(&mut options)
        .await
        .expect("submit should succeed")
        .expect("submit result");
    assert!(matches!(submit_result, SessionSendOutcome::SessionEnqueueResult(_)));
    assert!(matches!(
        submit_handle.await.expect("submit owner task"),
        QueueRequest::SubmitPrompt { wait_for_completion: false, owner_generation: Some(7), .. }
    ));

    let cancel_socket = TestSocket::new("ok-can");
    write_default_owner_record(
        &temp_home.path,
        &default_owner_record("session-cancel", cancel_socket.path.clone()),
    )
    .await;
    let cancel_handle = start_fake_owner(&cancel_socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            accepted_message(request_id),
            QueueOwnerMessage::CancelResult {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                cancelled: true,
            },
        ]
    });
    assert_eq!(
        try_cancel_on_running_owner("session-cancel", false).await.expect("cancel success"),
        Some(true)
    );
    assert!(matches!(
        cancel_handle.await.expect("cancel owner task"),
        QueueRequest::CancelPrompt { owner_generation: Some(7), .. }
    ));

    let mode_socket = TestSocket::new("ok-mode");
    write_default_owner_record(
        &temp_home.path,
        &default_owner_record("session-mode", mode_socket.path.clone()),
    )
    .await;
    let mode_handle = start_fake_owner(&mode_socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            accepted_message(request_id),
            QueueOwnerMessage::SetModeResult {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                mode_id: "plan".to_string(),
            },
        ]
    });
    assert_eq!(
        try_set_mode_on_running_owner("session-mode", "plan", Some(21), false)
            .await
            .expect("set mode success"),
        Some(true)
    );
    assert!(matches!(
        mode_handle.await.expect("mode owner task"),
        QueueRequest::SetMode { mode_id, timeout_ms: Some(21), .. } if mode_id == "plan"
    ));

    let model_socket = TestSocket::new("ok-model");
    write_default_owner_record(
        &temp_home.path,
        &default_owner_record("session-model", model_socket.path.clone()),
    )
    .await;
    let model_handle = start_fake_owner(&model_socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            accepted_message(request_id),
            QueueOwnerMessage::SetModelResult {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                model_id: "model-a".to_string(),
            },
        ]
    });
    assert_eq!(
        try_set_model_on_running_owner("session-model", "model-a", Some(22), false)
            .await
            .expect("set model success"),
        Some(true)
    );
    assert!(matches!(
        model_handle.await.expect("model owner task"),
        QueueRequest::SetModel { model_id, timeout_ms: Some(22), .. } if model_id == "model-a"
    ));

    let config_socket = TestSocket::new("ok-conf");
    write_default_owner_record(
        &temp_home.path,
        &default_owner_record("session-config", config_socket.path.clone()),
    )
    .await;
    let config_handle = start_fake_owner(&config_socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            accepted_message(request_id),
            QueueOwnerMessage::SetConfigOptionResult {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                response: SetSessionConfigOptionResponse::new(Vec::new()),
            },
        ]
    });
    assert_eq!(
        try_set_config_option_on_running_owner(
            "session-config",
            "temperature",
            "0.2",
            Some(23),
            false,
        )
        .await
        .expect("set config success"),
        Some(SetSessionConfigOptionResponse::new(Vec::new()))
    );
    assert!(matches!(
        config_handle.await.expect("config owner task"),
        QueueRequest::SetConfigOption { config_id, value, timeout_ms: Some(23), .. }
            if config_id == "temperature" && value == "0.2"
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn try_submit_to_running_owner_recovers_stale_owner_protocol_mismatch() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let temp_home = TempHome::new("stale-owner").await;
    let _home_guard = HomeEnvGuard::set(&temp_home.path);
    let socket = TestSocket::new("stale");
    write_default_owner_record(
        &temp_home.path,
        &default_owner_record("session-1", socket.path.clone()),
    )
    .await;
    let handle = start_fake_owner(&socket.path, |request| {
        let request_id = queue_request_id(request);
        vec![
            accepted_message(request_id),
            QueueOwnerMessage::SetModeResult {
                request_id: request_id.to_string(),
                owner_generation: Some(7),
                mode_id: "plan".to_string(),
            },
        ]
    });
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, true);

    let result =
        try_submit_to_running_owner(&mut options).await.expect("stale owner should recover");

    assert!(result.is_none());
    handle.await.expect("fake owner task");
    assert!(
        !tokio::fs::try_exists(queue_lock_file_path("session-1", &temp_home.path))
            .await
            .expect("check lock path")
    );
}

#[cfg(unix)]
#[tokio::test]
async fn try_submit_to_running_owner_propagates_nonrecoverable_protocol_error() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let temp_home = TempHome::new("invalid-json").await;
    let _home_guard = HomeEnvGuard::set(&temp_home.path);
    let socket = TestSocket::new("bad-json");
    write_default_owner_record(
        &temp_home.path,
        &default_owner_record("session-1", socket.path.clone()),
    )
    .await;
    let handle = start_fake_owner_lines(&socket.path, |request| {
        vec![
            serde_json::to_string(&accepted_message(queue_request_id(request)))
                .expect("serialize accepted"),
            "{".to_string(),
        ]
    });
    let mut formatter = TestFormatter::default();
    let mut options = submit_options(&mut formatter, true);

    let error =
        try_submit_to_running_owner(&mut options).await.expect_err("invalid json should fail");

    assert_eq!(error.detail_code(), Some("QUEUE_PROTOCOL_INVALID_JSON"));
    handle.await.expect("fake owner task");
    assert!(
        tokio::fs::try_exists(queue_lock_file_path("session-1", &temp_home.path))
            .await
            .expect("check lock path")
    );
}
