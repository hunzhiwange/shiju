//! 验证运行时状态、队列输出适配、协议消息复制与记录更新。
//!
//! 最小会话记录、控制器和异步通道 fixture 用于驱动运行时状态与竞态分支。

use super::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use agent_client_protocol::{
    ContentBlock, ContentChunk, SessionNotification, SessionUpdate, SetSessionConfigOptionResponse,
    TextContent,
};
use serde_json::{Value, json};
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(unix)]
use tokio::net::UnixStream;
#[cfg(unix)]
use tokio::net::unix::OwnedReadHalf;

use crate::SessionAcpxState;
use crate::errors::QueueConnectionError;
use crate::prompt_content::text_prompt;
#[cfg(unix)]
use crate::queue_ipc_server::{
    QueueOwnerControlHandlers, QueueOwnerSocketLease, SessionQueueOwner, SessionQueueOwnerOptions,
};
#[cfg(unix)]
use crate::queue_messages::{QueueOwnerMessage, QueueRequest};
use crate::session_event_log::session_event_active_path;
use crate::session_events::SessionEventWriterOptions;
use crate::types::{
    AcpMessageDirection, ClientOperationMethod, ClientOperationStatus,
    NonInteractivePermissionPolicy, OutputErrorCode, OutputErrorOrigin, OutputErrorParams,
    PermissionMode, SessionAgentContent, SessionMessage, SessionResumePolicy, SessionTokenUsage,
    SessionUserContent, SessionUserMessage,
};

struct EnvVarGuard {
    _lock: tokio::sync::MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl EnvVarGuard {
    async fn set_home(home: &Path) -> Self {
        let lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
        let saved = save_home_env();
        unsafe { std::env::set_var("HOME", home) };
        unsafe { std::env::set_var("USERPROFILE", home) };
        unsafe { std::env::remove_var("HOMEDRIVE") };
        unsafe { std::env::remove_var("HOMEPATH") };
        Self { _lock: lock, saved }
    }

    fn unset_home_blocking() -> Self {
        let lock = crate::queue_paths::TEST_HOME_ENV_LOCK.blocking_lock();
        let saved = save_home_env();
        unsafe { std::env::remove_var("HOME") };
        unsafe { std::env::remove_var("USERPROFILE") };
        unsafe { std::env::remove_var("HOMEDRIVE") };
        unsafe { std::env::remove_var("HOMEPATH") };
        Self { _lock: lock, saved }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        for (key, value) in self.saved.drain(..) {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

fn save_home_env() -> Vec<(&'static str, Option<std::ffi::OsString>)> {
    ["HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH"]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect()
}

fn temp_home(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("vw-acp-runtime-state-{name}-{unique}"))
}

fn acp_message(value: Value) -> AcpJsonRpcMessage {
    serde_json::from_value(value).expect("valid ACP JSON-RPC message")
}

fn acp_value(message: &AcpJsonRpcMessage) -> Value {
    serde_json::to_value(message).expect("ACP message serializes")
}

fn request_message(id: impl Into<Value>, method: &str) -> AcpJsonRpcMessage {
    acp_message(json!({
        "jsonrpc": "2.0",
        "id": id.into(),
        "method": method,
        "params": {}
    }))
}

fn test_record() -> SessionRecord {
    SessionRecord {
        schema: crate::SESSION_RECORD_SCHEMA.to_string(),
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
            active_path: "/tmp/project/active.ndjson".to_string(),
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
        title: Some("Stored title".to_string()),
        messages: vec![SessionMessage::User(SessionUserMessage {
            id: "user-1".to_string(),
            content: vec![SessionUserContent::Text("previous".to_string())],
        })],
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: HashMap::new(),
        vwacp: Some(SessionAcpxState {
            current_mode_id: Some("code".to_string()),
            desired_mode_id: None,
            current_model_id: None,
            available_models: None,
            available_commands: None,
            config_options: None,
            desired_config_options: None,
            session_options: Some(SessionStateOptions {
                model: Some("stored-model".to_string()),
                allowed_tools: Some(vec!["shell".to_string()]),
                max_turns: Some(3),
            }),
        }),
    }
}

#[cfg(unix)]
#[derive(Default)]
struct NoopControlHandlers;

#[cfg(unix)]
#[async_trait::async_trait]
impl QueueOwnerControlHandlers for NoopControlHandlers {
    async fn cancel_prompt(&self) -> Result<bool, QueueConnectionError> {
        Ok(false)
    }

    async fn set_session_mode(
        &self,
        _mode_id: String,
        _timeout_ms: Option<u64>,
    ) -> Result<(), QueueConnectionError> {
        Ok(())
    }

    async fn set_session_model(
        &self,
        _model_id: String,
        _timeout_ms: Option<u64>,
    ) -> Result<(), QueueConnectionError> {
        Ok(())
    }

    async fn set_session_config_option(
        &self,
        _config_id: String,
        _value: String,
        _timeout_ms: Option<u64>,
    ) -> Result<SetSessionConfigOptionResponse, QueueConnectionError> {
        Ok(SetSessionConfigOptionResponse::new(Vec::new()))
    }
}

#[cfg(unix)]
async fn connect_and_send(socket_path: &Path, request: QueueRequest) -> BufReader<OwnedReadHalf> {
    let stream = UnixStream::connect(socket_path).await.expect("connect queue owner");
    let (read, mut write) = stream.into_split();
    let line = serde_json::to_string(&request).expect("serialize queue request");
    write.write_all(line.as_bytes()).await.expect("write request");
    write.write_all(b"\n").await.expect("write newline");
    write.shutdown().await.expect("shutdown client write");
    BufReader::new(read)
}

#[cfg(unix)]
async fn read_queue_owner_message(
    reader: &mut BufReader<OwnedReadHalf>,
) -> Option<QueueOwnerMessage> {
    let mut line = String::new();
    let bytes_read = reader.read_line(&mut line).await.expect("read response");
    if bytes_read == 0 {
        return None;
    }

    crate::queue_messages::parse_queue_owner_message(
        &serde_json::from_str(line.trim()).expect("response json"),
    )
}

#[derive(Default)]
struct CapturingFormatter {
    messages: Vec<AcpJsonRpcMessage>,
    flush_count: usize,
}

impl OutputFormatter for CapturingFormatter {
    fn set_context(&mut self, _context: OutputFormatterContext) {}

    fn on_acp_message(&mut self, message: AcpJsonRpcMessage) {
        self.messages.push(message);
    }

    fn on_error(&mut self, _params: OutputErrorParams) {}

    fn flush(&mut self) {
        self.flush_count += 1;
    }
}

// 下列行为用例覆盖正常完成、超时中断、回退重试和资源清理。
#[test]
fn discard_output_formatter_accepts_empty_lifecycle() {
    let mut formatter = DiscardOutputFormatter;

    formatter.set_context(OutputFormatterContext { session_id: "session-1".to_string() });
    formatter.on_acp_message(request_message("ignored", "session/update"));
    formatter.on_error(OutputErrorParams {
        code: OutputErrorCode::Runtime,
        detail_code: None,
        origin: None,
        message: "ignored".to_string(),
        retryable: None,
        acp: None,
        timestamp: None,
    });
    formatter.flush();
}

#[tokio::test]
async fn queue_task_output_formatter_ignores_output_after_channel_is_closed() {
    let mut formatter = QueueTaskOutputFormatter {
        request_id: "request-closed".to_string(),
        event_tx: None,
        forward_task: None,
        emitted_messages: Arc::new(AtomicBool::new(false)),
    };

    formatter.set_context(OutputFormatterContext { session_id: "session-1".to_string() });
    formatter.flush();
    formatter.on_acp_message(request_message("ignored", "session/update"));
    formatter.on_error(OutputErrorParams {
        code: OutputErrorCode::Runtime,
        detail_code: None,
        origin: Some(OutputErrorOrigin::Runtime),
        message: "ignored".to_string(),
        retryable: None,
        acp: None,
        timestamp: None,
    });
    formatter.finish().await;

    assert!(formatter.has_messages());
}

#[test]
fn duplicate_acp_message_returns_independent_copy() {
    let message = request_message("req-1", "session/prompt");
    let duplicate = duplicate_acp_message(&message).expect("duplicate message");

    assert_eq!(acp_value(&duplicate), acp_value(&message));
}

#[test]
fn runtime_state_new_records_prompt_and_apply_preserves_vwacp() {
    let record = test_record();
    let state = RuntimeState::new(&record, &text_prompt("new prompt"));
    let mut next_record = record.clone();

    state.apply_to_record(&mut next_record);

    assert_eq!(next_record.title.as_deref(), Some("Stored title"));
    assert_eq!(next_record.vwacp, record.vwacp);
    assert_eq!(next_record.messages.len(), 2);
    assert!(matches!(
        next_record.messages.last(),
        Some(SessionMessage::User(user))
            if matches!(user.content.first(), Some(SessionUserContent::Text(text)) if text == "new prompt")
    ));
}

#[test]
fn runtime_state_take_methods_drain_buffers() {
    let state = Arc::new(RuntimeState::new(&test_record(), &text_prompt("prompt")));
    let (on_message, on_output, _, _) =
        build_runtime_state_callbacks(state.clone(), None, None, None);
    let message_value = json!({"jsonrpc":"2.0","id":1,"result":{"ok":true}});
    let output_value =
        json!({"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s1"}});

    on_message.expect("message callback")(
        AcpMessageDirection::Inbound,
        acp_message(message_value.clone()),
    );
    on_output.as_ref().expect("output callback")(
        AcpMessageDirection::Outbound,
        acp_message(output_value.clone()),
    );
    state.set_buffering_connect_output(false);
    on_output.expect("output callback")(
        AcpMessageDirection::Outbound,
        acp_message(output_value.clone()),
    );
    state
        .prompt_output_messages()
        .lock()
        .push(acp_message(json!({"jsonrpc":"2.0","id":3,"result":{"c":3}})));

    assert!(state.saw_acp_message());
    assert_eq!(
        state.take_pending_messages().iter().map(acp_value).collect::<Vec<_>>(),
        vec![message_value]
    );
    assert!(state.take_pending_messages().is_empty());
    assert_eq!(
        state.take_connect_output_messages().iter().map(acp_value).collect::<Vec<_>>(),
        vec![output_value.clone()]
    );
    assert!(state.take_connect_output_messages().is_empty());
    assert_eq!(
        state.take_prompt_output_messages().iter().map(acp_value).collect::<Vec<_>>(),
        vec![output_value, json!({"jsonrpc":"2.0","id":3,"result":{"c":3}})]
    );
    assert!(state.take_prompt_output_messages().is_empty());
}

#[test]
fn runtime_state_callbacks_forward_events_and_track_side_effects() {
    let state = Arc::new(RuntimeState::new(&test_record(), &text_prompt("prompt")));
    let seen_messages = Arc::new(Mutex::new(Vec::new()));
    let seen_updates = Arc::new(Mutex::new(Vec::new()));
    let seen_operations = Arc::new(Mutex::new(Vec::new()));
    let on_acp_message = {
        let seen_messages = seen_messages.clone();
        Arc::new(move |direction, message| seen_messages.lock().push((direction, message)))
            as AcpMessageCallback
    };
    let on_session_update = {
        let seen_updates = seen_updates.clone();
        Arc::new(move |notification| seen_updates.lock().push(notification))
            as SessionUpdateCallback
    };
    let on_client_operation = {
        let seen_operations = seen_operations.clone();
        Arc::new(move |operation| seen_operations.lock().push(operation)) as ClientOperationCallback
    };
    let (on_message, _, on_update, on_operation) = build_runtime_state_callbacks(
        state.clone(),
        Some(on_acp_message),
        Some(on_session_update),
        Some(on_client_operation),
    );
    let notification = SessionNotification::new(
        "session-1",
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "agent said hi",
        )))),
    );
    let operation = ClientOperation {
        method: ClientOperationMethod::TerminalWaitForExit,
        status: ClientOperationStatus::Completed,
        summary: "exit 0".to_string(),
        details: None,
        timestamp: "2026-01-02T00:00:00Z".to_string(),
    };

    on_message.expect("message callback")(
        AcpMessageDirection::Inbound,
        acp_message(json!({"jsonrpc":"2.0","id":7,"result":null})),
    );
    assert!(!state.prompt_turn_had_side_effects());
    state.set_buffering_connect_output(false);
    state.mark_prompt_turn_active(true);
    on_update.expect("session update callback")(notification);
    on_operation.expect("client operation callback")(operation);

    assert!(state.prompt_turn_had_side_effects());
    assert_eq!(seen_messages.lock().len(), 1);
    assert_eq!(seen_updates.lock().len(), 1);
    assert_eq!(seen_operations.lock().len(), 1);

    let mut record = test_record();
    state.apply_to_record(&mut record);
    assert!(record.messages.iter().any(|message| {
        matches!(
            message,
            SessionMessage::Agent(agent)
                if agent.content.iter().any(|content| {
                    matches!(content, SessionAgentContent::Text(text) if text == "agent said hi")
                })
        )
    }));
}

#[test]
fn runtime_state_callbacks_ignore_side_effects_outside_prompt_turn() {
    let state = Arc::new(RuntimeState::new(&test_record(), &text_prompt("prompt")));
    let (_, _, on_update, on_operation) =
        build_runtime_state_callbacks(state.clone(), None, None, None);
    let notification = SessionNotification::new(
        "session-1",
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "outside prompt",
        )))),
    );
    let operation = ClientOperation {
        method: ClientOperationMethod::TerminalWaitForExit,
        status: ClientOperationStatus::Completed,
        summary: "exit 0".to_string(),
        details: None,
        timestamp: "2026-01-02T00:00:00Z".to_string(),
    };

    state.mark_prompt_turn_active(false);
    on_update.expect("session update callback")(notification);
    on_operation.expect("client operation callback")(operation);

    assert!(!state.prompt_turn_had_side_effects());
}

#[test]
fn connection_history_replay_does_not_append_duplicate_messages() {
    let mut record = test_record();
    let state = Arc::new(RuntimeState::new(&record, &text_prompt("current request")));
    let (_, _, on_update, _) = build_runtime_state_callbacks(state.clone(), None, None, None);
    on_update.unwrap()(SessionNotification::new(
        "session-1",
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "replayed answer",
        )))),
    ));
    state.apply_to_record(&mut record);
    assert_eq!(record.messages.len(), 2);
    assert!(matches!(record.messages.last(), Some(SessionMessage::User(user))
        if matches!(user.content.first(), Some(SessionUserContent::Text(text)) if text == "current request")));
}

#[test]
fn replay_output_buffer_drains_messages_and_flushes_once() {
    let messages = Mutex::new(vec![
        acp_message(json!({"jsonrpc":"2.0","id":1,"result":{"a":1}})),
        acp_message(json!({"jsonrpc":"2.0","id":2,"result":{"b":2}})),
    ]);
    let mut formatter = CapturingFormatter::default();

    replay_output_buffer(&mut formatter, &messages);

    assert_eq!(formatter.messages.len(), 2);
    assert_eq!(formatter.flush_count, 1);
    assert!(messages.lock().is_empty());
}

#[test]
fn replay_output_messages_preserves_order_without_flush() {
    let messages =
        vec![request_message("req-1", "session/one"), request_message("req-2", "session/two")];
    let mut formatter = CapturingFormatter::default();
    let expected = messages.iter().map(acp_value).collect::<Vec<_>>();

    replay_output_messages(&mut formatter, messages);

    assert_eq!(formatter.messages.iter().map(acp_value).collect::<Vec<_>>(), expected);
    assert_eq!(formatter.flush_count, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn flush_pending_messages_writes_batch_and_skips_empty_state() {
    let home = temp_home("flush-success");
    let _env = EnvVarGuard::set_home(&home).await;
    let state = Arc::new(RuntimeState::new(&test_record(), &text_prompt("prompt")));
    let mut writer = SessionEventWriter::open(
        test_record(),
        SessionEventWriterOptions { max_segment_bytes: Some(1024), max_segments: Some(3) },
    )
    .await
    .expect("open event writer");

    flush_pending_messages(&mut writer, &state, false).await.expect("flush empty batch");
    assert!(!session_event_active_path("record-1", &home).exists());

    let (on_message, _, _, _) = build_runtime_state_callbacks(state.clone(), None, None, None);
    on_message.expect("message callback")(
        AcpMessageDirection::Inbound,
        request_message("req-1", "session/prompt"),
    );

    flush_pending_messages(&mut writer, &state, false).await.expect("flush pending messages");

    assert_eq!(writer.get_record().last_seq, 1);
    assert_eq!(writer.get_record().last_request_id.as_deref(), Some("req-1"));
    assert!(state.take_pending_messages().is_empty());
    assert!(session_event_active_path("record-1", &home).exists());

    writer.close(SessionEventAppendOptions { checkpoint: false }).await.expect("close writer");
    let _ = tokio::fs::remove_dir_all(&home).await;
}

#[tokio::test(flavor = "current_thread")]
async fn flush_pending_messages_maps_writer_errors() {
    let home = temp_home("flush-error");
    let _env = EnvVarGuard::set_home(&home).await;
    let state = Arc::new(RuntimeState::new(&test_record(), &text_prompt("prompt")));
    let (on_message, _, _, _) = build_runtime_state_callbacks(state.clone(), None, None, None);
    let mut writer = SessionEventWriter::open(
        test_record(),
        SessionEventWriterOptions { max_segment_bytes: Some(1024), max_segments: Some(3) },
    )
    .await
    .expect("open event writer");

    writer.close(SessionEventAppendOptions { checkpoint: false }).await.expect("close writer");
    on_message.expect("message callback")(
        AcpMessageDirection::Inbound,
        request_message("req-1", "session/prompt"),
    );

    let error = flush_pending_messages(&mut writer, &state, false)
        .await
        .expect_err("closed writer should fail");

    assert!(error.to_string().contains("closed"));
    assert!(state.take_pending_messages().is_empty());

    let _ = tokio::fs::remove_dir_all(&home).await;
}

#[test]
fn fallback_event_log_uses_defaults_without_home() {
    let _env = EnvVarGuard::unset_home_blocking();

    let event_log = fallback_event_log("session/1");

    assert_eq!(event_log.segment_count, DEFAULT_EVENT_MAX_SEGMENTS);
    assert_eq!(event_log.max_segment_bytes, DEFAULT_EVENT_SEGMENT_MAX_BYTES);
    assert_eq!(event_log.max_segments, DEFAULT_EVENT_MAX_SEGMENTS);
    assert!(event_log.active_path.ends_with("session%2F1.stream.ndjson"));
    assert_eq!(event_log.last_write_at, None);
    assert_eq!(event_log.last_write_error, None);
}

#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn queue_task_output_formatter_forwards_events_and_errors() {
    let dir = std::env::temp_dir().join(format!(
        "vwq-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos()
    ));
    tokio::fs::create_dir_all(&dir).await.expect("create socket dir");
    let socket_path = dir.join("owner.sock");
    let owner = SessionQueueOwner::start(
        QueueOwnerSocketLease { socket_path: socket_path.clone(), owner_generation: Some(9) },
        Arc::new(NoopControlHandlers),
        SessionQueueOwnerOptions { max_queue_depth: Some(2), on_queue_depth_changed: None },
    )
    .await
    .expect("start queue owner");
    let mut reader = connect_and_send(
        &socket_path,
        QueueRequest::SubmitPrompt {
            request_id: "queue-request".to_string(),
            owner_generation: Some(9),
            message: "hello queue".to_string(),
            prompt: Vec::new(),
            permission_mode: PermissionMode::ApproveReads,
            resume_policy: Some(SessionResumePolicy::SameSessionOnly),
            non_interactive_permissions: Some(NonInteractivePermissionPolicy::Fail),
            timeout_ms: Some(250),
            suppress_sdk_console_errors: Some(true),
            wait_for_completion: true,
        },
    )
    .await;
    assert!(matches!(
        read_queue_owner_message(&mut reader).await,
        Some(QueueOwnerMessage::Accepted { request_id, owner_generation })
            if request_id == "queue-request" && owner_generation == Some(9)
    ));
    let task = owner.next_task(Some(100)).await.expect("queued task");
    let mut formatter = QueueTaskOutputFormatter::new("formatter-request", task.clone());
    let event = request_message("event-1", "session/update");
    let event_value = acp_value(&event);

    formatter.set_context(OutputFormatterContext { session_id: "session-1".to_string() });
    assert!(!formatter.has_messages());
    formatter.on_acp_message(event);
    formatter.on_error(OutputErrorParams {
        code: OutputErrorCode::Runtime,
        detail_code: Some("DETAIL".to_string()),
        origin: None,
        message: "runtime failed".to_string(),
        retryable: Some(true),
        acp: None,
        timestamp: Some("2026-01-01T00:00:00Z".to_string()),
    });
    assert!(formatter.has_messages());
    formatter.finish().await;

    assert!(matches!(
        read_queue_owner_message(&mut reader).await,
        Some(QueueOwnerMessage::Event { request_id, owner_generation, message })
            if request_id == "formatter-request"
                && owner_generation == Some(9)
                && acp_value(&message) == event_value
    ));
    assert!(matches!(
        read_queue_owner_message(&mut reader).await,
        Some(QueueOwnerMessage::Error {
            request_id,
            owner_generation,
            detail_code,
            origin,
            retryable,
            message,
            ..
        }) if request_id == "formatter-request"
            && owner_generation == Some(9)
            && detail_code.as_deref() == Some("DETAIL")
            && origin == OutputErrorOrigin::Runtime
            && retryable == Some(true)
            && message == "runtime failed"
    ));

    task.close().await;
    owner.close().await.expect("close owner");
    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[test]
fn session_options_from_record_filters_blank_values() {
    let mut record = test_record();
    record.vwacp.as_mut().expect("vwacp").session_options = Some(SessionStateOptions {
        model: Some("  model-a  ".to_string()),
        allowed_tools: Some(vec![" shell ".to_string(), "  ".to_string(), "read".to_string()]),
        max_turns: Some(5),
    });

    let options = session_options_from_record(&record).expect("valid options");

    assert_eq!(options.model.as_deref(), Some("model-a"));
    assert_eq!(options.allowed_tools, Some(vec!["shell".to_string(), "read".to_string()]));
    assert_eq!(options.max_turns, Some(5));

    record.vwacp.as_mut().expect("vwacp").session_options = Some(SessionStateOptions {
        model: Some("   ".to_string()),
        allowed_tools: Some(vec![" ".to_string()]),
        max_turns: None,
    });

    assert_eq!(session_options_from_record(&record), None);
}

#[test]
fn session_options_from_record_returns_none_without_stored_options() {
    let mut record = test_record();
    record.vwacp = None;
    assert_eq!(session_options_from_record(&record), None);

    record.vwacp = Some(SessionAcpxState {
        current_mode_id: None,
        desired_mode_id: None,
        current_model_id: None,
        available_models: None,
        available_commands: None,
        config_options: None,
        desired_config_options: None,
        session_options: None,
    });

    assert_eq!(session_options_from_record(&record), None);
}

#[test]
fn persist_session_options_writes_values_and_clears_empty_input() {
    let mut record = test_record();
    record.vwacp = None;
    let options = AcpSessionOptions {
        model: Some(" model-b ".to_string()),
        allowed_tools: Some(vec![" shell ".to_string(), "".to_string()]),
        max_turns: Some(2),
    };

    persist_session_options(&mut record, Some(&options));

    let stored = record
        .vwacp
        .as_ref()
        .and_then(|state| state.session_options.as_ref())
        .expect("stored session options");
    assert_eq!(stored.model.as_deref(), Some("model-b"));
    assert_eq!(stored.allowed_tools, Some(vec!["shell".to_string()]));
    assert_eq!(stored.max_turns, Some(2));

    persist_session_options(&mut record, Some(&AcpSessionOptions::default()));

    assert_eq!(record.vwacp.as_ref().and_then(|state| state.session_options.as_ref()), None);
}

#[test]
fn persist_session_options_keeps_max_turns_and_clears_none() {
    let mut record = test_record();
    record.vwacp = None;
    let options = AcpSessionOptions { model: None, allowed_tools: None, max_turns: Some(4) };

    persist_session_options(&mut record, Some(&options));

    let stored = record
        .vwacp
        .as_ref()
        .and_then(|state| state.session_options.as_ref())
        .expect("stored max turns");
    assert_eq!(stored.model, None);
    assert_eq!(stored.allowed_tools, None);
    assert_eq!(stored.max_turns, Some(4));

    persist_session_options(&mut record, None);

    assert_eq!(record.vwacp.as_ref().and_then(|state| state.session_options.as_ref()), None);
}

#[test]
fn helper_filters_trimmed_strings_and_tool_lists() {
    assert_eq!(non_empty_trimmed(Some(" value ")).as_deref(), Some("value"));
    assert_eq!(non_empty_trimmed(Some("   ")), None);
    assert_eq!(non_empty_trimmed(None), None);
    assert_eq!(
        clone_non_empty_tools(&[" shell ".to_string(), "".to_string(), " read ".to_string()]),
        Some(vec!["shell".to_string(), "read".to_string()])
    );
    assert_eq!(clone_non_empty_tools(&[" ".to_string()]), None);
}
