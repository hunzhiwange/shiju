use super::*;
use crate::errors::AcpxErrorOptions;
use crate::queue_messages::{QueueOwnerMessage, QueueRequest};
use crate::types::{
    NonInteractivePermissionPolicy, OutputErrorCode, OutputErrorOrigin, PermissionMode,
    PermissionStats, SESSION_RECORD_SCHEMA, SessionEventLog, SessionRecord, SessionResumePolicy,
    SessionSendResult, SessionTokenUsage,
};
use agent_client_protocol::StopReason;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::net::unix::OwnedReadHalf;

static NEXT_SOCKET_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug)]
struct TestSocketDir {
    dir: PathBuf,
    socket_path: PathBuf,
}

impl TestSocketDir {
    fn new(name: &str) -> Self {
        let id = NEXT_SOCKET_ID.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("vwq-{name}-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("socket temp dir");

        Self { socket_path: dir.join("owner.sock"), dir }
    }
}

impl Drop for TestSocketDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[derive(Default)]
struct TestControlHandlers {
    fail: bool,
    calls: std::sync::Mutex<Vec<String>>,
}

impl TestControlHandlers {
    fn failing() -> Self {
        Self { fail: true, calls: std::sync::Mutex::new(Vec::new()) }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("calls lock").clone()
    }

    fn maybe_fail(&self) -> Result<(), QueueConnectionError> {
        if !self.fail {
            return Ok(());
        }

        Err(QueueConnectionError::new(
            "control denied",
            AcpxErrorOptions {
                output_code: Some(OutputErrorCode::PermissionDenied),
                detail_code: Some("CONTROL_DENIED".to_string()),
                origin: Some(OutputErrorOrigin::Runtime),
                retryable: Some(false),
                ..Default::default()
            },
        ))
    }
}

#[async_trait::async_trait]
impl QueueOwnerControlHandlers for TestControlHandlers {
    async fn cancel_prompt(&self) -> Result<bool, QueueConnectionError> {
        self.calls.lock().expect("calls lock").push("cancel".to_string());
        self.maybe_fail()?;
        Ok(true)
    }

    async fn set_session_mode(
        &self,
        mode_id: String,
        timeout_ms: Option<u64>,
    ) -> Result<(), QueueConnectionError> {
        self.calls.lock().expect("calls lock").push(format!("mode:{mode_id}:{timeout_ms:?}"));
        self.maybe_fail()
    }

    async fn set_session_model(
        &self,
        model_id: String,
        timeout_ms: Option<u64>,
    ) -> Result<(), QueueConnectionError> {
        self.calls.lock().expect("calls lock").push(format!("model:{model_id}:{timeout_ms:?}"));
        self.maybe_fail()
    }

    async fn set_session_config_option(
        &self,
        config_id: String,
        value: String,
        timeout_ms: Option<u64>,
    ) -> Result<SetSessionConfigOptionResponse, QueueConnectionError> {
        self.calls
            .lock()
            .expect("calls lock")
            .push(format!("config:{config_id}:{value}:{timeout_ms:?}"));
        self.maybe_fail()?;
        Ok(SetSessionConfigOptionResponse::new(Vec::new()))
    }
}

fn request_line(request: &QueueRequest) -> String {
    serde_json::to_string(request).expect("serialize queue request")
}

async fn start_owner(
    socket_path: &Path,
    owner_generation: Option<u64>,
    max_queue_depth: Option<usize>,
    on_queue_depth_changed: Option<QueueDepthChangedCallback>,
    control_handlers: Arc<dyn QueueOwnerControlHandlers>,
) -> SessionQueueOwner {
    SessionQueueOwner::start(
        QueueOwnerSocketLease { socket_path: socket_path.to_path_buf(), owner_generation },
        control_handlers,
        SessionQueueOwnerOptions { max_queue_depth, on_queue_depth_changed },
    )
    .await
    .expect("start queue owner")
}

async fn connect_and_send(socket_path: &Path, line: &str) -> BufReader<OwnedReadHalf> {
    let stream = UnixStream::connect(socket_path).await.expect("connect queue owner");
    let (read, mut write) = stream.into_split();
    write.write_all(line.as_bytes()).await.expect("write request");
    write.write_all(b"\n").await.expect("write newline");
    write.shutdown().await.expect("shutdown client write");
    BufReader::new(read)
}

async fn read_message(reader: &mut BufReader<OwnedReadHalf>) -> Option<QueueOwnerMessage> {
    let mut line = String::new();
    let bytes_read = reader.read_line(&mut line).await.expect("read response");
    if bytes_read == 0 {
        return None;
    }

    crate::queue_messages::parse_queue_owner_message(
        &serde_json::from_str(line.trim()).expect("response json"),
    )
}

async fn read_messages_until_close(socket_path: &Path, line: &str) -> Vec<QueueOwnerMessage> {
    let mut reader = connect_and_send(socket_path, line).await;
    let mut messages = Vec::new();
    while let Some(message) = read_message(&mut reader).await {
        messages.push(message);
    }
    messages
}

async fn wait_for_queue_depth(owner: &SessionQueueOwner, expected: usize) {
    for _ in 0..50 {
        if owner.queue_depth() == expected {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }

    assert_eq!(owner.queue_depth(), expected);
}

fn submit_request(
    request_id: &str,
    owner_generation: Option<u64>,
    wait_for_completion: bool,
) -> QueueRequest {
    QueueRequest::SubmitPrompt {
        request_id: request_id.to_string(),
        owner_generation,
        message: "hello queue".to_string(),
        prompt: Vec::new(),
        permission_mode: PermissionMode::ApproveReads,
        resume_policy: Some(SessionResumePolicy::SameSessionOnly),
        non_interactive_permissions: Some(NonInteractivePermissionPolicy::Fail),
        timeout_ms: Some(250),
        suppress_sdk_console_errors: Some(true),
        wait_for_completion,
    }
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

#[test]
fn queue_request_helpers_extract_id_and_generation() {
    let request = QueueRequest::SetMode {
        request_id: "req-1".to_string(),
        owner_generation: Some(9),
        mode_id: "plan".to_string(),
        timeout_ms: Some(100),
    };

    assert_eq!(queue_request_id(&request), "req-1");
    assert_eq!(queue_request_owner_generation(&request), Some(9));
}

#[test]
fn with_owner_generation_overwrites_message_generation() {
    let message = with_owner_generation(
        QueueOwnerMessage::CancelResult {
            request_id: "req-1".to_string(),
            owner_generation: Some(1),
            cancelled: true,
        },
        Some(2),
    );

    match message {
        QueueOwnerMessage::CancelResult { owner_generation, cancelled, .. } => {
            assert_eq!(owner_generation, Some(2));
            assert!(cancelled);
        }
        _ => panic!("unexpected message variant"),
    }
}

#[test]
fn with_owner_generation_updates_event_result_and_error_variants() {
    let event = with_owner_generation(
        QueueOwnerMessage::Event {
            request_id: "event".to_string(),
            owner_generation: None,
            message: agent_client_protocol::JsonRpcMessage::wrap(json!({
                "method": "session/update"
            })),
        },
        Some(8),
    );
    assert!(matches!(
        event,
        QueueOwnerMessage::Event { request_id, owner_generation, .. }
            if request_id == "event" && owner_generation == Some(8)
    ));

    let result = with_owner_generation(
        QueueOwnerMessage::Result {
            request_id: "result".to_string(),
            owner_generation: None,
            result: Box::new(test_session_send_result()),
        },
        Some(9),
    );
    assert!(matches!(
        result,
        QueueOwnerMessage::Result { request_id, owner_generation, result }
            if request_id == "result"
                && owner_generation == Some(9)
                && result.session_id == "session-1"
    ));

    let error = with_owner_generation(
        QueueOwnerMessage::Error {
            request_id: "error".to_string(),
            owner_generation: None,
            code: OutputErrorCode::Runtime,
            detail_code: Some("DETAIL".to_string()),
            origin: OutputErrorOrigin::Queue,
            message: "failed".to_string(),
            retryable: Some(false),
            acp: None,
            output_already_emitted: Some(true),
        },
        Some(10),
    );
    assert!(matches!(
        error,
        QueueOwnerMessage::Error { request_id, owner_generation, output_already_emitted, .. }
            if request_id == "error"
                && owner_generation == Some(10)
                && output_already_emitted == Some(true)
    ));
}

#[test]
fn with_owner_generation_updates_accepted_and_set_result_variants() {
    let accepted = with_owner_generation(
        QueueOwnerMessage::Accepted { request_id: "accepted".to_string(), owner_generation: None },
        Some(11),
    );
    assert!(matches!(
        accepted,
        QueueOwnerMessage::Accepted { request_id, owner_generation }
            if request_id == "accepted" && owner_generation == Some(11)
    ));

    let mode = with_owner_generation(
        QueueOwnerMessage::SetModeResult {
            request_id: "mode".to_string(),
            owner_generation: None,
            mode_id: "plan".to_string(),
        },
        Some(12),
    );
    assert!(matches!(
        mode,
        QueueOwnerMessage::SetModeResult { request_id, owner_generation, mode_id }
            if request_id == "mode" && owner_generation == Some(12) && mode_id == "plan"
    ));

    let model = with_owner_generation(
        QueueOwnerMessage::SetModelResult {
            request_id: "model".to_string(),
            owner_generation: None,
            model_id: "fast".to_string(),
        },
        Some(13),
    );
    assert!(matches!(
        model,
        QueueOwnerMessage::SetModelResult { request_id, owner_generation, model_id }
            if request_id == "model" && owner_generation == Some(13) && model_id == "fast"
    ));

    let config = with_owner_generation(
        QueueOwnerMessage::SetConfigOptionResult {
            request_id: "config".to_string(),
            owner_generation: None,
            response: SetSessionConfigOptionResponse::new(Vec::new()),
        },
        Some(14),
    );
    assert!(matches!(
        config,
        QueueOwnerMessage::SetConfigOptionResult { request_id, owner_generation, .. }
            if request_id == "config" && owner_generation == Some(14)
    ));
}

#[test]
fn make_queue_owner_error_sets_queue_runtime_shape() {
    let message = make_queue_owner_error(
        "req-1".to_string(),
        "closed",
        "QUEUE_OWNER_CLOSED",
        Some(true),
        Some(4),
    );

    match message {
        QueueOwnerMessage::Error {
            code, origin, detail_code, retryable, owner_generation, ..
        } => {
            assert_eq!(code, OutputErrorCode::Runtime);
            assert_eq!(origin, OutputErrorOrigin::Queue);
            assert_eq!(detail_code.as_deref(), Some("QUEUE_OWNER_CLOSED"));
            assert_eq!(retryable, Some(true));
            assert_eq!(owner_generation, Some(4));
        }
        _ => panic!("unexpected message variant"),
    }
}

#[test]
fn make_queue_owner_error_from_connection_error_uses_default_shape() {
    let error = QueueConnectionError::new("plain failure", AcpxErrorOptions::default());
    let message = make_queue_owner_error_from_connection_error(
        "req-1".to_string(),
        &error,
        "QUEUE_CONTROL_REQUEST_FAILED",
        Some(4),
    );

    match message {
        QueueOwnerMessage::Error {
            request_id,
            code,
            detail_code,
            origin,
            message,
            retryable,
            owner_generation,
            ..
        } => {
            assert_eq!(request_id, "req-1");
            assert_eq!(code, OutputErrorCode::Runtime);
            assert_eq!(detail_code.as_deref(), Some("QUEUE_CONTROL_REQUEST_FAILED"));
            assert_eq!(origin, OutputErrorOrigin::Queue);
            assert_eq!(message, "plain failure");
            assert_eq!(retryable, None);
            assert_eq!(owner_generation, Some(4));
        }
        _ => panic!("unexpected message variant"),
    }
}

#[test]
fn make_queue_owner_error_from_connection_error_preserves_rich_error_fields() {
    let error = QueueConnectionError::new(
        "rich failure",
        AcpxErrorOptions {
            output_code: Some(OutputErrorCode::PermissionDenied),
            detail_code: Some("RICH_DETAIL".to_string()),
            origin: Some(OutputErrorOrigin::Acp),
            retryable: Some(true),
            acp: Some(crate::types::OutputErrorAcpPayload {
                code: -32000,
                message: "acp failure".to_string(),
                data: Some(json!({"reason": "blocked"})),
            }),
            ..Default::default()
        },
    );

    let message = make_queue_owner_error_from_connection_error(
        "req-rich".to_string(),
        &error,
        "QUEUE_CONTROL_REQUEST_FAILED",
        Some(5),
    );

    match message {
        QueueOwnerMessage::Error {
            request_id,
            code,
            detail_code,
            origin,
            message,
            retryable,
            acp,
            output_already_emitted,
            owner_generation,
        } => {
            assert_eq!(request_id, "req-rich");
            assert_eq!(code, OutputErrorCode::PermissionDenied);
            assert_eq!(detail_code.as_deref(), Some("RICH_DETAIL"));
            assert_eq!(origin, OutputErrorOrigin::Acp);
            assert_eq!(message, "rich failure");
            assert_eq!(retryable, Some(true));
            assert_eq!(owner_generation, Some(5));
            assert_eq!(output_already_emitted, None);
            let acp = acp.expect("acp payload preserved");
            assert_eq!(acp.code, -32000);
            assert_eq!(acp.message, "acp failure");
        }
        _ => panic!("unexpected message variant"),
    }
}

#[tokio::test]
async fn start_normalizes_zero_max_depth_and_next_task_times_out_when_empty() {
    let socket = TestSocketDir::new("start");
    let owner = start_owner(
        &socket.socket_path,
        Some(7),
        Some(0),
        None,
        Arc::new(TestControlHandlers::default()),
    )
    .await;

    assert_eq!(owner.max_queue_depth(), 1);
    assert_eq!(owner.queue_depth(), 0);
    assert!(owner.next_task(Some(1)).await.is_none());

    owner.close().await.expect("close owner");
}

#[tokio::test]
async fn start_uses_default_max_depth_and_exposes_control_handlers() {
    let socket = TestSocketDir::new("start-defaults");
    let handlers: Arc<dyn QueueOwnerControlHandlers> = Arc::new(TestControlHandlers::default());
    let owner = start_owner(&socket.socket_path, None, None, None, handlers.clone()).await;

    assert_eq!(owner.max_queue_depth(), 16);
    assert_eq!(owner.queue_depth(), 0);
    assert!(Arc::ptr_eq(owner.control_handlers(), &handlers));

    owner.close().await.expect("close owner");
}

#[tokio::test]
async fn close_returns_ok_when_state_is_already_closed() {
    let (shutdown_tx, _shutdown_rx) = watch::channel(false);
    let owner = SessionQueueOwner {
        control_handlers: Arc::new(TestControlHandlers::default()),
        owner_generation: Some(1),
        max_queue_depth: 1,
        on_queue_depth_changed: None,
        state: Arc::new(Mutex::new(QueueOwnerState { pending: VecDeque::new(), closed: true })),
        notify: Arc::new(Notify::new()),
        shutdown_tx,
        accept_task: tokio::spawn(async {}),
    };

    owner.close().await.expect("close closed owner");
}

#[tokio::test]
async fn start_returns_bind_error_when_socket_parent_is_missing() {
    let id = NEXT_SOCKET_ID.fetch_add(1, Ordering::SeqCst);
    let socket_path = std::env::temp_dir()
        .join(format!("vwq-missing-{}-{id}", std::process::id()))
        .join("missing")
        .join("owner.sock");

    let result = SessionQueueOwner::start(
        QueueOwnerSocketLease { socket_path, owner_generation: Some(7) },
        Arc::new(TestControlHandlers::default()),
        SessionQueueOwnerOptions::default(),
    )
    .await;

    assert_eq!(result.err().expect("bind should fail").kind(), std::io::ErrorKind::NotFound);
}

#[tokio::test]
async fn next_task_waits_without_timeout_until_request_arrives() {
    let socket = TestSocketDir::new("next-wait");
    let owner = start_owner(
        &socket.socket_path,
        Some(2),
        Some(2),
        None,
        Arc::new(TestControlHandlers::default()),
    )
    .await;
    let socket_path = socket.socket_path.clone();
    let line = request_line(&submit_request("next-wait", Some(2), false));
    let sender = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        read_messages_until_close(&socket_path, &line).await
    });

    let task = tokio::time::timeout(std::time::Duration::from_millis(200), owner.next_task(None))
        .await
        .expect("next task wait should finish")
        .expect("queued task");

    assert_eq!(task.request_id, "next-wait");
    assert!(!task.wait_for_completion);
    task.close().await;
    assert!(matches!(
        sender.await.expect("sender task").as_slice(),
        [QueueOwnerMessage::Accepted { request_id, owner_generation }]
            if request_id == "next-wait" && *owner_generation == Some(2)
    ));
    owner.close().await.expect("close owner");
}

#[tokio::test]
async fn next_task_with_timeout_returns_task_that_arrives_before_deadline() {
    let socket = TestSocketDir::new("next-timeout-arrival");
    let owner = start_owner(
        &socket.socket_path,
        Some(21),
        Some(2),
        None,
        Arc::new(TestControlHandlers::default()),
    )
    .await;
    let socket_path = socket.socket_path.clone();
    let line = request_line(&submit_request("next-timeout-arrival", Some(21), false));
    let sender = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        read_messages_until_close(&socket_path, &line).await
    });

    let task = owner.next_task(Some(500)).await.expect("queued task within timeout");
    assert_eq!(task.request_id, "next-timeout-arrival");
    task.close().await;
    assert!(matches!(
        sender.await.expect("sender task").as_slice(),
        [QueueOwnerMessage::Accepted { request_id, owner_generation }]
            if request_id == "next-timeout-arrival" && *owner_generation == Some(21)
    ));
    owner.close().await.expect("close owner");
}

#[tokio::test]
async fn next_task_returns_none_when_owner_closes_while_waiting() {
    let socket = TestSocketDir::new("next-closed-wait");
    let owner = Arc::new(
        start_owner(
            &socket.socket_path,
            Some(24),
            Some(2),
            None,
            Arc::new(TestControlHandlers::default()),
        )
        .await,
    );

    let waiter = {
        let owner = owner.clone();
        tokio::spawn(async move { owner.next_task(Some(1_000)).await })
    };

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    {
        let mut state = owner.state.lock();
        state.closed = true;
    }
    owner.notify.notify_waiters();

    let result = tokio::time::timeout(std::time::Duration::from_millis(500), waiter)
        .await
        .expect("waiter resolves after close")
        .expect("waiter task");
    assert!(result.is_none());

    Arc::try_unwrap(owner)
        .map_err(|_| "owner still shared")
        .expect("sole owner reference")
        .close()
        .await
        .expect("close owner");
}

#[tokio::test]
async fn closed_state_connection_returns_closed_error() {
    let (client, server) = UnixStream::pair().expect("unix stream pair");
    let (read, _write) = client.into_split();
    let state = Arc::new(Mutex::new(QueueOwnerState { pending: VecDeque::new(), closed: true }));
    let notify = Arc::new(Notify::new());
    let task = tokio::spawn(handle_connection(
        server,
        state,
        notify,
        Arc::new(TestControlHandlers::default()),
        Some(31),
        1,
        None,
    ));
    let mut reader = BufReader::new(read);

    assert!(matches!(
        read_message(&mut reader).await,
        Some(QueueOwnerMessage::Error {
            request_id,
            detail_code,
            retryable,
            owner_generation,
            ..
        }) if request_id == "unknown"
            && detail_code.as_deref() == Some("QUEUE_OWNER_CLOSED")
            && retryable == Some(true)
            && owner_generation == Some(31)
    ));
    assert!(read_message(&mut reader).await.is_none());
    task.await.expect("connection task should finish");
}

#[tokio::test]
async fn connection_without_request_closes_cleanly() {
    let (client, server) = UnixStream::pair().expect("unix stream pair");
    drop(client);
    let state = Arc::new(Mutex::new(QueueOwnerState { pending: VecDeque::new(), closed: false }));

    tokio::time::timeout(
        std::time::Duration::from_millis(100),
        handle_connection(
            server,
            state,
            Arc::new(Notify::new()),
            Arc::new(TestControlHandlers::default()),
            Some(32),
            1,
            None,
        ),
    )
    .await
    .expect("connection should close on eof");
}

#[tokio::test]
async fn queue_task_responder_close_and_send_are_idempotent_after_close() {
    let (_client, server) = UnixStream::pair().expect("unix stream pair");
    let (_read, write) = server.into_split();
    let responder = QueueTaskResponder {
        writer: Arc::new(AsyncMutex::new(Some(write))),
        owner_generation: Some(33),
    };

    responder.close().await;
    assert!(responder.writer.lock().await.is_none());
    responder.close().await;
    responder
        .send(QueueOwnerMessage::Accepted {
            request_id: "ignored".to_string(),
            owner_generation: None,
        })
        .await;
    assert!(responder.writer.lock().await.is_none());
}

#[tokio::test]
async fn queue_task_responder_drops_writer_after_peer_disconnect() {
    let (client, server) = UnixStream::pair().expect("unix stream pair");
    drop(client);
    let (_read, write) = server.into_split();
    let responder = QueueTaskResponder {
        writer: Arc::new(AsyncMutex::new(Some(write))),
        owner_generation: Some(34),
    };

    responder
        .send(QueueOwnerMessage::Accepted {
            request_id: "broken-pipe".to_string(),
            owner_generation: None,
        })
        .await;
    assert!(responder.writer.lock().await.is_none());
}

#[tokio::test]
async fn submit_prompt_accepts_and_enqueues_task_with_defaults() {
    let socket = TestSocketDir::new("submit");
    let depths = Arc::new(std::sync::Mutex::new(Vec::new()));
    let owner = start_owner(
        &socket.socket_path,
        Some(7),
        Some(2),
        Some(Arc::new({
            let depths = depths.clone();
            move |depth| depths.lock().expect("depth lock").push(depth)
        })),
        Arc::new(TestControlHandlers::default()),
    )
    .await;

    let line = request_line(&submit_request("submit-1", Some(7), true));
    let mut reader = connect_and_send(&socket.socket_path, &line).await;
    match read_message(&mut reader).await.expect("accepted") {
        QueueOwnerMessage::Accepted { request_id, owner_generation } => {
            assert_eq!(request_id, "submit-1");
            assert_eq!(owner_generation, Some(7));
        }
        _ => panic!("unexpected response"),
    }

    let task = owner.next_task(Some(100)).await.expect("queued task");
    assert_eq!(task.request_id, "submit-1");
    assert_eq!(task.message, "hello queue");
    assert_eq!(task.prompt, text_prompt("hello queue"));
    assert_eq!(task.permission_mode, PermissionMode::ApproveReads);
    assert_eq!(task.resume_policy, Some(SessionResumePolicy::SameSessionOnly));
    assert_eq!(task.non_interactive_permissions, Some(NonInteractivePermissionPolicy::Fail));
    assert_eq!(task.timeout_ms, Some(250));
    assert_eq!(task.suppress_sdk_console_errors, Some(true));
    assert!(task.wait_for_completion);
    assert_eq!(owner.queue_depth(), 0);
    assert_eq!(*depths.lock().expect("depth lock"), vec![1, 0]);

    task.close().await;
    assert!(read_message(&mut reader).await.is_none());
    owner.close().await.expect("close owner");
}

#[tokio::test]
async fn submit_prompt_preserves_non_empty_prompt() {
    let socket = TestSocketDir::new("structured-submit");
    let owner = start_owner(
        &socket.socket_path,
        Some(8),
        Some(2),
        None,
        Arc::new(TestControlHandlers::default()),
    )
    .await;
    let mut request = submit_request("structured-submit", Some(8), false);
    let structured_prompt = text_prompt("structured prompt");
    if let QueueRequest::SubmitPrompt { prompt, message, .. } = &mut request {
        *message = "fallback message".to_string();
        *prompt = structured_prompt.clone();
    }

    let messages = read_messages_until_close(&socket.socket_path, &request_line(&request)).await;
    assert!(matches!(
        messages.as_slice(),
        [QueueOwnerMessage::Accepted { request_id, owner_generation }]
            if request_id == "structured-submit" && *owner_generation == Some(8)
    ));

    let task = owner.next_task(Some(100)).await.expect("queued task");
    assert_eq!(task.message, "fallback message");
    assert_eq!(task.prompt, structured_prompt);
    task.close().await;
    owner.close().await.expect("close owner");
}

#[tokio::test]
async fn submit_prompt_skips_blank_request_lines_before_processing() {
    let socket = TestSocketDir::new("blank-lines");
    let owner = start_owner(
        &socket.socket_path,
        Some(3),
        Some(2),
        None,
        Arc::new(TestControlHandlers::default()),
    )
    .await;

    let line = format!("\n\n{}", request_line(&submit_request("blank-submit", Some(3), false)));
    let messages = read_messages_until_close(&socket.socket_path, &line).await;

    assert!(matches!(
        messages.as_slice(),
        [QueueOwnerMessage::Accepted { request_id, owner_generation }]
            if request_id == "blank-submit" && *owner_generation == Some(3)
    ));

    let task = owner.next_task(Some(100)).await.expect("queued task");
    assert_eq!(task.request_id, "blank-submit");
    task.close().await;
    owner.close().await.expect("close owner");
}

#[tokio::test]
async fn enqueue_task_rejects_closed_state_for_waiting_and_detached_tasks() {
    let state = Arc::new(Mutex::new(QueueOwnerState { pending: VecDeque::new(), closed: true }));
    let notify = Arc::new(Notify::new());

    let (client, server) = UnixStream::pair().expect("unix stream pair");
    let (read, _client_write) = client.into_split();
    let (_server_read, write) = server.into_split();
    let mut reader = BufReader::new(read);
    let waiting_task = QueueTask {
        request_id: "closed-waiting".to_string(),
        message: "waiting".to_string(),
        prompt: text_prompt("waiting"),
        permission_mode: PermissionMode::ApproveReads,
        resume_policy: None,
        non_interactive_permissions: None,
        timeout_ms: None,
        suppress_sdk_console_errors: None,
        wait_for_completion: true,
        enqueued_at: Instant::now(),
        responder: QueueTaskResponder {
            writer: Arc::new(AsyncMutex::new(Some(write))),
            owner_generation: Some(35),
        },
    };

    enqueue_task(waiting_task, state.clone(), notify.clone(), Some(35), 1, None).await;
    assert!(matches!(
        read_message(&mut reader).await,
        Some(QueueOwnerMessage::Error { request_id, detail_code, owner_generation, .. })
            if request_id == "closed-waiting"
                && detail_code.as_deref() == Some("QUEUE_OWNER_SHUTTING_DOWN")
                && owner_generation == Some(35)
    ));
    assert!(read_message(&mut reader).await.is_none());

    let (client, server) = UnixStream::pair().expect("unix stream pair");
    let (read, _client_write) = client.into_split();
    let (_server_read, write) = server.into_split();
    let mut reader = BufReader::new(read);
    let detached_task = QueueTask {
        request_id: "closed-detached".to_string(),
        message: "detached".to_string(),
        prompt: text_prompt("detached"),
        permission_mode: PermissionMode::ApproveReads,
        resume_policy: None,
        non_interactive_permissions: None,
        timeout_ms: None,
        suppress_sdk_console_errors: None,
        wait_for_completion: false,
        enqueued_at: Instant::now(),
        responder: QueueTaskResponder {
            writer: Arc::new(AsyncMutex::new(Some(write))),
            owner_generation: Some(35),
        },
    };

    enqueue_task(detached_task, state, notify, Some(35), 1, None).await;
    assert!(read_message(&mut reader).await.is_none());
}

#[tokio::test]
async fn queue_task_send_forwards_messages_with_owner_generation() {
    let socket = TestSocketDir::new("task-send");
    let owner = start_owner(
        &socket.socket_path,
        Some(4),
        Some(2),
        None,
        Arc::new(TestControlHandlers::default()),
    )
    .await;

    let mut reader = connect_and_send(
        &socket.socket_path,
        &request_line(&submit_request("task-send", Some(4), true)),
    )
    .await;
    assert!(matches!(
        read_message(&mut reader).await,
        Some(QueueOwnerMessage::Accepted { request_id, owner_generation })
            if request_id == "task-send" && owner_generation == Some(4)
    ));

    let task = owner.next_task(Some(100)).await.expect("queued task");
    task.send(QueueOwnerMessage::Error {
        request_id: "task-send".to_string(),
        owner_generation: None,
        code: OutputErrorCode::Runtime,
        detail_code: Some("TASK_FAILED".to_string()),
        origin: OutputErrorOrigin::Queue,
        message: "task failed".to_string(),
        retryable: Some(false),
        acp: None,
        output_already_emitted: None,
    })
    .await;

    assert!(matches!(
        read_message(&mut reader).await,
        Some(QueueOwnerMessage::Error {
            request_id,
            detail_code,
            retryable,
            owner_generation,
            ..
        }) if request_id == "task-send"
            && detail_code.as_deref() == Some("TASK_FAILED")
            && retryable == Some(false)
            && owner_generation == Some(4)
    ));

    task.close().await;
    assert!(read_message(&mut reader).await.is_none());
    owner.close().await.expect("close owner");
}

#[tokio::test]
async fn submit_prompt_without_completion_closes_after_acceptance_but_keeps_task() {
    let socket = TestSocketDir::new("submit-no-wait");
    let owner = start_owner(
        &socket.socket_path,
        Some(3),
        Some(2),
        None,
        Arc::new(TestControlHandlers::default()),
    )
    .await;

    let messages = read_messages_until_close(
        &socket.socket_path,
        &request_line(&submit_request("submit-no-wait", Some(3), false)),
    )
    .await;

    assert!(matches!(
        messages.as_slice(),
        [QueueOwnerMessage::Accepted { request_id, owner_generation }]
            if request_id == "submit-no-wait" && *owner_generation == Some(3)
    ));

    let task = owner.next_task(Some(100)).await.expect("queued task");
    assert!(!task.wait_for_completion);
    task.close().await;
    owner.close().await.expect("close owner");
}

#[tokio::test]
async fn submit_prompt_over_capacity_returns_retryable_overloaded_error() {
    let socket = TestSocketDir::new("overloaded");
    let owner = start_owner(
        &socket.socket_path,
        Some(5),
        Some(1),
        None,
        Arc::new(TestControlHandlers::default()),
    )
    .await;

    let mut first_reader = connect_and_send(
        &socket.socket_path,
        &request_line(&submit_request("first", Some(5), true)),
    )
    .await;
    assert!(matches!(
        read_message(&mut first_reader).await,
        Some(QueueOwnerMessage::Accepted { request_id, .. }) if request_id == "first"
    ));
    wait_for_queue_depth(&owner, 1).await;

    let messages = read_messages_until_close(
        &socket.socket_path,
        &request_line(&submit_request("second", Some(5), true)),
    )
    .await;

    assert!(matches!(
        messages.as_slice(),
        [
            QueueOwnerMessage::Accepted { request_id: accepted_id, .. },
            QueueOwnerMessage::Error { request_id, detail_code, retryable, owner_generation, .. },
        ] if accepted_id == "second"
            && request_id == "second"
            && detail_code.as_deref() == Some("QUEUE_OWNER_OVERLOADED")
            && *retryable == Some(true)
            && *owner_generation == Some(5)
    ));

    owner.close().await.expect("close owner");
}

#[tokio::test]
async fn submit_prompt_over_capacity_without_completion_closes_after_acceptance() {
    let socket = TestSocketDir::new("overloaded-no-wait");
    let owner = start_owner(
        &socket.socket_path,
        Some(6),
        Some(1),
        None,
        Arc::new(TestControlHandlers::default()),
    )
    .await;

    let mut first_reader = connect_and_send(
        &socket.socket_path,
        &request_line(&submit_request("first-no-wait-overload", Some(6), true)),
    )
    .await;
    assert!(matches!(
        read_message(&mut first_reader).await,
        Some(QueueOwnerMessage::Accepted { request_id, .. })
            if request_id == "first-no-wait-overload"
    ));
    wait_for_queue_depth(&owner, 1).await;

    let messages = read_messages_until_close(
        &socket.socket_path,
        &request_line(&submit_request("second-no-wait-overload", Some(6), false)),
    )
    .await;

    assert!(matches!(
        messages.as_slice(),
        [QueueOwnerMessage::Accepted { request_id, owner_generation }]
            if request_id == "second-no-wait-overload" && *owner_generation == Some(6)
    ));

    owner.close().await.expect("close owner");
}

#[tokio::test]
async fn close_drains_pending_waiting_tasks_with_shutdown_error() {
    let socket = TestSocketDir::new("close-drain");
    let depths = Arc::new(std::sync::Mutex::new(Vec::new()));
    let owner = start_owner(
        &socket.socket_path,
        Some(11),
        Some(2),
        Some(Arc::new({
            let depths = depths.clone();
            move |depth| depths.lock().expect("depth lock").push(depth)
        })),
        Arc::new(TestControlHandlers::default()),
    )
    .await;

    let mut reader = connect_and_send(
        &socket.socket_path,
        &request_line(&submit_request("pending", Some(11), true)),
    )
    .await;
    assert!(matches!(
        read_message(&mut reader).await,
        Some(QueueOwnerMessage::Accepted { request_id, .. }) if request_id == "pending"
    ));
    wait_for_queue_depth(&owner, 1).await;

    owner.close().await.expect("close owner");

    assert!(matches!(
        read_message(&mut reader).await,
        Some(QueueOwnerMessage::Error {
            request_id,
            detail_code,
            retryable,
            owner_generation,
            ..
        }) if request_id == "pending"
            && detail_code.as_deref() == Some("QUEUE_OWNER_SHUTTING_DOWN")
            && retryable == Some(true)
            && owner_generation == Some(11)
    ));
    assert!(read_message(&mut reader).await.is_none());
    assert_eq!(*depths.lock().expect("depth lock"), vec![1, 0]);
}

#[tokio::test]
async fn control_requests_return_accepted_then_results() {
    let socket = TestSocketDir::new("control");
    let handlers = Arc::new(TestControlHandlers::default());
    let owner = start_owner(&socket.socket_path, Some(13), Some(2), None, handlers.clone()).await;

    let cancel_messages = read_messages_until_close(
        &socket.socket_path,
        &request_line(&QueueRequest::CancelPrompt {
            request_id: "cancel".to_string(),
            owner_generation: Some(13),
        }),
    )
    .await;
    assert!(matches!(
        cancel_messages.as_slice(),
        [
            QueueOwnerMessage::Accepted { request_id: accepted_id, owner_generation: accepted_generation },
            QueueOwnerMessage::CancelResult { request_id, owner_generation, cancelled },
        ] if accepted_id == "cancel"
            && *accepted_generation == Some(13)
            && request_id == "cancel"
            && *owner_generation == Some(13)
            && *cancelled
    ));

    let mode_messages = read_messages_until_close(
        &socket.socket_path,
        &request_line(&QueueRequest::SetMode {
            request_id: "mode".to_string(),
            owner_generation: Some(13),
            mode_id: "plan".to_string(),
            timeout_ms: Some(25),
        }),
    )
    .await;
    assert!(matches!(
        mode_messages.as_slice(),
        [
            QueueOwnerMessage::Accepted { .. },
            QueueOwnerMessage::SetModeResult { request_id, owner_generation, mode_id },
        ] if request_id == "mode" && *owner_generation == Some(13) && mode_id == "plan"
    ));

    let model_messages = read_messages_until_close(
        &socket.socket_path,
        &request_line(&QueueRequest::SetModel {
            request_id: "model".to_string(),
            owner_generation: Some(13),
            model_id: "fast".to_string(),
            timeout_ms: None,
        }),
    )
    .await;
    assert!(matches!(
        model_messages.as_slice(),
        [
            QueueOwnerMessage::Accepted { .. },
            QueueOwnerMessage::SetModelResult { request_id, owner_generation, model_id },
        ] if request_id == "model" && *owner_generation == Some(13) && model_id == "fast"
    ));

    let config_messages = read_messages_until_close(
        &socket.socket_path,
        &request_line(&QueueRequest::SetConfigOption {
            request_id: "config".to_string(),
            owner_generation: Some(13),
            config_id: "effort".to_string(),
            value: "high".to_string(),
            timeout_ms: Some(50),
        }),
    )
    .await;
    assert!(matches!(
        config_messages.as_slice(),
        [
            QueueOwnerMessage::Accepted { .. },
            QueueOwnerMessage::SetConfigOptionResult { request_id, owner_generation, .. },
        ] if request_id == "config" && *owner_generation == Some(13)
    ));

    assert_eq!(
        handlers.calls(),
        vec!["cancel", "mode:plan:Some(25)", "model:fast:None", "config:effort:high:Some(50)",]
    );

    owner.close().await.expect("close owner");
}

#[tokio::test]
async fn control_request_errors_preserve_connection_error_shape() {
    let socket = TestSocketDir::new("control-error");
    let owner = start_owner(
        &socket.socket_path,
        Some(17),
        Some(2),
        None,
        Arc::new(TestControlHandlers::failing()),
    )
    .await;

    let messages = read_messages_until_close(
        &socket.socket_path,
        &request_line(&QueueRequest::SetModel {
            request_id: "model-error".to_string(),
            owner_generation: Some(17),
            model_id: "fast".to_string(),
            timeout_ms: None,
        }),
    )
    .await;

    assert!(matches!(
        messages.as_slice(),
        [
            QueueOwnerMessage::Accepted { request_id: accepted_id, owner_generation: accepted_generation },
            QueueOwnerMessage::Error {
                request_id,
                code,
                detail_code,
                origin,
                retryable,
                owner_generation,
                ..
            },
        ] if accepted_id == "model-error"
            && *accepted_generation == Some(17)
            && request_id == "model-error"
            && *code == OutputErrorCode::PermissionDenied
            && detail_code.as_deref() == Some("CONTROL_DENIED")
            && *origin == OutputErrorOrigin::Runtime
            && *retryable == Some(false)
            && *owner_generation == Some(17)
    ));

    owner.close().await.expect("close owner");
}

#[tokio::test]
async fn bad_request_payloads_return_queue_errors_without_acceptance() {
    let socket = TestSocketDir::new("bad-request");
    let owner = start_owner(
        &socket.socket_path,
        Some(19),
        Some(2),
        None,
        Arc::new(TestControlHandlers::default()),
    )
    .await;

    let invalid_json = read_messages_until_close(&socket.socket_path, "{not-json").await;
    assert!(matches!(
        invalid_json.as_slice(),
        [QueueOwnerMessage::Error {
            request_id,
            detail_code,
            retryable,
            owner_generation,
            ..
        }] if request_id == "unknown"
            && detail_code.as_deref() == Some("QUEUE_REQUEST_PAYLOAD_INVALID_JSON")
            && *retryable == Some(false)
            && *owner_generation == Some(19)
    ));

    let invalid_request = read_messages_until_close(&socket.socket_path, "{}").await;
    assert!(matches!(
        invalid_request.as_slice(),
        [QueueOwnerMessage::Error {
            request_id,
            detail_code,
            retryable,
            owner_generation,
            ..
        }] if request_id == "unknown"
            && detail_code.as_deref() == Some("QUEUE_REQUEST_INVALID")
            && *retryable == Some(false)
            && *owner_generation == Some(19)
    ));

    owner.close().await.expect("close owner");
}

#[tokio::test]
async fn generation_mismatch_rejects_request_before_control_handler_runs() {
    let socket = TestSocketDir::new("generation");
    let handlers = Arc::new(TestControlHandlers::default());
    let owner = start_owner(&socket.socket_path, Some(23), Some(2), None, handlers.clone()).await;

    let messages = read_messages_until_close(
        &socket.socket_path,
        &request_line(&QueueRequest::CancelPrompt {
            request_id: "stale".to_string(),
            owner_generation: Some(22),
        }),
    )
    .await;

    assert!(matches!(
        messages.as_slice(),
        [QueueOwnerMessage::Error {
            request_id,
            detail_code,
            retryable,
            owner_generation,
            ..
        }] if request_id == "stale"
            && detail_code.as_deref() == Some("QUEUE_OWNER_GENERATION_MISMATCH")
            && *retryable == Some(false)
            && *owner_generation == Some(23)
    ));
    assert!(handlers.calls().is_empty());

    owner.close().await.expect("close owner");
}
