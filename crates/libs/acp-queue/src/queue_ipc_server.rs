//! 队列所有者服务端的连接处理与任务调度。
//!
//! 本模块实现后台队列所有者的服务端逻辑，负责监听 IPC 请求、维护待处理任务队列，
//! 并将外部请求转交给当前活动会话控制器执行。
//!
//! # 主要组件
//!
//! - QueueOwnerSocketLease：描述当前队列所有者占有的套接字租约
//! - QueueTask：表示一项待执行的会话任务及其响应通道
//! - QueueOwnerControlHandlers：抽象对活动会话的控制能力
//! - SessionQueueOwner：管理监听、排队、发送和关闭的核心服务对象

#[cfg(unix)]
use std::collections::VecDeque;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(unix)]
use std::time::Instant;

use agent_client_protocol::SetSessionConfigOptionResponse;
use async_trait::async_trait;
#[cfg(unix)]
use parking_lot::Mutex;
#[cfg(unix)]
use serde_json::Value;
#[cfg(unix)]
use tokio::sync::{Mutex as AsyncMutex, Notify, watch};
#[cfg(unix)]
use tokio::task::JoinHandle;

#[cfg(not(unix))]
use acp_types::NonInteractivePermissionPolicy;
use acp_types::errors::QueueConnectionError;
#[cfg(unix)]
use acp_metrics::perf_metrics::record_perf_duration;
use acp_types::prompt_content::PromptInput;
use crate::queue_messages::QueueOwnerMessage;
#[cfg(unix)]
use crate::queue_messages::{QueueRequest, parse_queue_request};
#[cfg(unix)]
use acp_types::types::{OutputErrorCode, OutputErrorOrigin, PermissionMode, SessionResumePolicy};
#[cfg(not(unix))]
use acp_types::types::{PermissionMode, SessionResumePolicy};
#[cfg(unix)]
use acp_types::{NonInteractivePermissionPolicy, text_prompt};

/// 当前队列所有者占有的套接字租约信息。
///
/// 它描述服务端实际监听的套接字路径，以及与租约记录对应的 owner 代次，
/// 用于帮助客户端识别返回消息是否来自当前有效的 owner。
#[derive(Debug, Clone)]
pub struct QueueOwnerSocketLease {
    pub socket_path: PathBuf,
    pub owner_generation: Option<u64>,
}

/// 一项排队中的会话任务。
///
/// 任务中不仅包含 prompt 内容，还包含权限模式、恢复策略、超时、
/// 输出抑制等运行控制参数，以及把结果回写给客户端的响应通道。
#[derive(Clone)]
pub struct QueueTask {
    pub request_id: String,
    pub message: String,
    pub prompt: PromptInput,
    pub permission_mode: PermissionMode,
    pub resume_policy: Option<SessionResumePolicy>,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub timeout_ms: Option<u64>,
    pub suppress_sdk_console_errors: Option<bool>,
    pub wait_for_completion: bool,
    #[cfg(unix)]
    enqueued_at: Instant,
    responder: QueueTaskResponder,
}

impl QueueTask {
    pub async fn send(&self, message: QueueOwnerMessage) {
        self.responder.send(message).await;
    }

    pub async fn close(&self) {
        self.responder.close().await;
    }
}

/// 活动会话控制器需要提供的最小控制接口。
///
/// 队列所有者本身不关心底层会话具体如何实现，只依赖这里约定的取消、
/// 切换模式、切换模型和更新配置能力。
#[async_trait]
pub trait QueueOwnerControlHandlers: Send + Sync {
    async fn cancel_prompt(&self) -> Result<bool, QueueConnectionError>;
    async fn set_session_mode(
        &self,
        mode_id: String,
        timeout_ms: Option<u64>,
    ) -> Result<(), QueueConnectionError>;
    async fn set_session_model(
        &self,
        model_id: String,
        timeout_ms: Option<u64>,
    ) -> Result<(), QueueConnectionError>;
    async fn set_session_config_option(
        &self,
        config_id: String,
        value: String,
        timeout_ms: Option<u64>,
    ) -> Result<SetSessionConfigOptionResponse, QueueConnectionError>;
}

/// 队列深度变化时触发的回调。
pub type QueueDepthChangedCallback = Arc<dyn Fn(usize) + Send + Sync>;

/// 创建 `SessionQueueOwner` 时可选的行为配置。
#[derive(Clone, Default)]
pub struct SessionQueueOwnerOptions {
    pub max_queue_depth: Option<usize>,
    pub on_queue_depth_changed: Option<QueueDepthChangedCallback>,
}

struct QueueTaskResponder {
    #[cfg(unix)]
    writer: Arc<AsyncMutex<Option<tokio::net::unix::OwnedWriteHalf>>>,
    #[cfg(unix)]
    owner_generation: Option<u64>,
}

impl Clone for QueueTaskResponder {
    fn clone(&self) -> Self {
        Self {
            #[cfg(unix)]
            writer: self.writer.clone(),
            #[cfg(unix)]
            owner_generation: self.owner_generation,
        }
    }
}

impl QueueTaskResponder {
    #[cfg(unix)]
    async fn send(&self, message: QueueOwnerMessage) {
        let mut writer_guard = self.writer.lock().await;
        let Some(writer) = writer_guard.as_mut() else {
            return;
        };
        let payload = with_owner_generation(message, self.owner_generation);
        let mut output = match serde_json::to_vec(&payload) {
            Ok(output) => output,
            Err(_) => return,
        };
        output.push(b'\n');
        if tokio::io::AsyncWriteExt::write_all(writer, &output).await.is_err() {
            *writer_guard = None;
        }
    }

    #[cfg(not(unix))]
    async fn send(&self, _message: QueueOwnerMessage) {}

    #[cfg(unix)]
    async fn close(&self) {
        let mut writer = self.writer.lock().await;
        let Some(mut writer) = writer.take() else {
            return;
        };
        let _ = tokio::io::AsyncWriteExt::shutdown(&mut writer).await;
    }

    #[cfg(not(unix))]
    async fn close(&self) {}
}

#[cfg(unix)]
struct QueueOwnerState {
    pending: VecDeque<QueueTask>,
    closed: bool,
}

/// 后台会话队列所有者服务对象。
///
/// 该对象持有监听循环、待处理队列、关闭信号和活动控制器引用，
/// 用于把外部 IPC 请求串行地接入当前会话执行环境。
pub struct SessionQueueOwner {
    #[cfg(unix)]
    control_handlers: Arc<dyn QueueOwnerControlHandlers>,
    #[cfg(unix)]
    owner_generation: Option<u64>,
    #[cfg(unix)]
    max_queue_depth: usize,
    #[cfg(unix)]
    on_queue_depth_changed: Option<QueueDepthChangedCallback>,
    #[cfg(unix)]
    state: Arc<Mutex<QueueOwnerState>>,
    #[cfg(unix)]
    notify: Arc<Notify>,
    #[cfg(unix)]
    shutdown_tx: watch::Sender<bool>,
    #[cfg(unix)]
    accept_task: JoinHandle<()>,
}

#[cfg(unix)]
impl SessionQueueOwner {
    pub async fn start(
        lease: QueueOwnerSocketLease,
        control_handlers: Arc<dyn QueueOwnerControlHandlers>,
        options: SessionQueueOwnerOptions,
    ) -> io::Result<Self> {
        let listener = tokio::net::UnixListener::bind(&lease.socket_path)?;
        let state =
            Arc::new(Mutex::new(QueueOwnerState { pending: VecDeque::new(), closed: false }));
        let notify = Arc::new(Notify::new());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let max_queue_depth = options.max_queue_depth.unwrap_or(16).max(1);
        let on_queue_depth_changed = options.on_queue_depth_changed.clone();
        let accept_task = tokio::spawn(run_accept_loop(
            listener,
            state.clone(),
            notify.clone(),
            shutdown_rx,
            control_handlers.clone(),
            lease.owner_generation,
            max_queue_depth,
            on_queue_depth_changed.clone(),
        ));

        Ok(Self {
            control_handlers,
            owner_generation: lease.owner_generation,
            max_queue_depth,
            on_queue_depth_changed,
            state,
            notify,
            shutdown_tx,
            accept_task,
        })
    }

    pub async fn close(self) -> io::Result<()> {
        {
            let mut state = self.state.lock();
            if state.closed {
                return Ok(());
            }
            state.closed = true;
        }

        self.notify.notify_waiters();
        let _ = self.shutdown_tx.send(true);

        let drained = {
            let mut state = self.state.lock();
            state.pending.drain(..).collect::<Vec<_>>()
        };
        self.emit_queue_depth(0);

        for task in drained {
            if task.wait_for_completion {
                task.send(make_queue_owner_error(
                    task.request_id.clone(),
                    "Queue owner shutting down before prompt execution",
                    "QUEUE_OWNER_SHUTTING_DOWN",
                    Some(true),
                    self.owner_generation,
                ))
                .await;
            }
            task.close().await;
        }

        let _ = self.accept_task.await;
        Ok(())
    }

    pub async fn next_task(&self, timeout_ms: Option<u64>) -> Option<QueueTask> {
        let started_at = Instant::now();

        loop {
            let notified = self.notify.notified();
            let maybe_task = {
                let mut state = self.state.lock();
                let task = state.pending.pop_front();
                if let Some(task) = &task {
                    let queue_depth = state.pending.len();
                    drop(state);
                    self.emit_queue_depth(queue_depth);
                    record_perf_duration(
                        "queue.owner.wait_ms",
                        task.enqueued_at.elapsed().as_secs_f64() * 1_000.0,
                    );
                    return task.clone().into();
                }
                if state.closed {
                    return None;
                }
                None
            };

            if maybe_task.is_some() {
                return maybe_task;
            }

            match timeout_ms {
                Some(timeout_ms) => {
                    let elapsed_ms = started_at.elapsed().as_millis() as u64;
                    if elapsed_ms >= timeout_ms {
                        return None;
                    }
                    let remaining = timeout_ms - elapsed_ms;
                    if tokio::time::timeout(std::time::Duration::from_millis(remaining), notified)
                        .await
                        .is_err()
                    {
                        return None;
                    }
                }
                None => notified.await,
            }
        }
    }

    pub fn queue_depth(&self) -> usize {
        self.state.lock().pending.len()
    }

    pub fn max_queue_depth(&self) -> usize {
        self.max_queue_depth
    }

    pub fn control_handlers(&self) -> &Arc<dyn QueueOwnerControlHandlers> {
        &self.control_handlers
    }

    fn emit_queue_depth(&self, queue_depth: usize) {
        if let Some(callback) = &self.on_queue_depth_changed {
            callback(queue_depth);
        }
    }
}

#[cfg(unix)]
async fn run_accept_loop(
    listener: tokio::net::UnixListener,
    state: Arc<Mutex<QueueOwnerState>>,
    notify: Arc<Notify>,
    mut shutdown_rx: watch::Receiver<bool>,
    control_handlers: Arc<dyn QueueOwnerControlHandlers>,
    owner_generation: Option<u64>,
    max_queue_depth: usize,
    on_queue_depth_changed: Option<QueueDepthChangedCallback>,
) {
    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => break,
            accepted = listener.accept() => {
                let Ok((socket, _)) = accepted else {
                    break;
                };
                tokio::spawn(handle_connection(
                    socket,
                    state.clone(),
                    notify.clone(),
                    control_handlers.clone(),
                    owner_generation,
                    max_queue_depth,
                    on_queue_depth_changed.clone(),
                ));
            }
        }
    }
}

#[cfg(unix)]
async fn handle_connection(
    socket: tokio::net::UnixStream,
    state: Arc<Mutex<QueueOwnerState>>,
    notify: Arc<Notify>,
    control_handlers: Arc<dyn QueueOwnerControlHandlers>,
    owner_generation: Option<u64>,
    max_queue_depth: usize,
    on_queue_depth_changed: Option<QueueDepthChangedCallback>,
) {
    let (reader, writer) = socket.into_split();
    let responder =
        QueueTaskResponder { writer: Arc::new(AsyncMutex::new(Some(writer))), owner_generation };

    if state.lock().closed {
        responder
            .send(make_queue_owner_error(
                "unknown".to_string(),
                "Queue owner is closed",
                "QUEUE_OWNER_CLOSED",
                Some(true),
                owner_generation,
            ))
            .await;
        responder.close().await;
        return;
    }

    let mut reader = tokio::io::BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let Ok(bytes_read) = tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut line).await
        else {
            responder.close().await;
            return;
        };
        if bytes_read == 0 {
            responder.close().await;
            return;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        process_request_line(
            trimmed,
            responder,
            state,
            notify,
            control_handlers,
            owner_generation,
            max_queue_depth,
            on_queue_depth_changed,
        )
        .await;
        return;
    }
}

#[cfg(unix)]
async fn process_request_line(
    line: &str,
    responder: QueueTaskResponder,
    state: Arc<Mutex<QueueOwnerState>>,
    notify: Arc<Notify>,
    control_handlers: Arc<dyn QueueOwnerControlHandlers>,
    owner_generation: Option<u64>,
    max_queue_depth: usize,
    on_queue_depth_changed: Option<QueueDepthChangedCallback>,
) {
    let parsed = match serde_json::from_str::<Value>(line) {
        Ok(parsed) => parsed,
        Err(_) => {
            responder
                .send(make_queue_owner_error(
                    "unknown".to_string(),
                    "Invalid queue request payload",
                    "QUEUE_REQUEST_PAYLOAD_INVALID_JSON",
                    Some(false),
                    owner_generation,
                ))
                .await;
            responder.close().await;
            return;
        }
    };

    let Some(request) = parse_queue_request(&parsed) else {
        responder
            .send(make_queue_owner_error(
                "unknown".to_string(),
                "Invalid queue request",
                "QUEUE_REQUEST_INVALID",
                Some(false),
                owner_generation,
            ))
            .await;
        responder.close().await;
        return;
    };

    if let Some(request_owner_generation) = queue_request_owner_generation(&request)
        && let Some(expected_owner_generation) = owner_generation
        && request_owner_generation != expected_owner_generation
    {
        responder
            .send(make_queue_owner_error(
                queue_request_id(&request).to_string(),
                "Queue request targeted a stale queue owner generation",
                "QUEUE_OWNER_GENERATION_MISMATCH",
                Some(false),
                owner_generation,
            ))
            .await;
        responder.close().await;
        return;
    }

    match request {
        QueueRequest::CancelPrompt { request_id, .. } => {
            handle_control_request(responder, request_id.clone(), owner_generation, async move {
                Ok(QueueOwnerMessage::CancelResult {
                    request_id,
                    owner_generation: None,
                    cancelled: control_handlers.cancel_prompt().await?,
                })
            })
            .await;
        }
        QueueRequest::SetMode { request_id, mode_id, timeout_ms, .. } => {
            handle_control_request(responder, request_id.clone(), owner_generation, async move {
                control_handlers.set_session_mode(mode_id.clone(), timeout_ms).await?;
                Ok(QueueOwnerMessage::SetModeResult { request_id, owner_generation: None, mode_id })
            })
            .await;
        }
        QueueRequest::SetModel { request_id, model_id, timeout_ms, .. } => {
            handle_control_request(responder, request_id.clone(), owner_generation, async move {
                control_handlers.set_session_model(model_id.clone(), timeout_ms).await?;
                Ok(QueueOwnerMessage::SetModelResult {
                    request_id,
                    owner_generation: None,
                    model_id,
                })
            })
            .await;
        }
        QueueRequest::SetConfigOption { request_id, config_id, value, timeout_ms, .. } => {
            handle_control_request(responder, request_id.clone(), owner_generation, async move {
                let response = control_handlers
                    .set_session_config_option(config_id, value, timeout_ms)
                    .await?;
                Ok(QueueOwnerMessage::SetConfigOptionResult {
                    request_id,
                    owner_generation: None,
                    response,
                })
            })
            .await;
        }
        QueueRequest::SubmitPrompt {
            request_id,
            message,
            prompt,
            permission_mode,
            resume_policy,
            non_interactive_permissions,
            timeout_ms,
            suppress_sdk_console_errors,
            wait_for_completion,
            ..
        } => {
            responder
                .send(QueueOwnerMessage::Accepted {
                    request_id: request_id.clone(),
                    owner_generation,
                })
                .await;

            if !wait_for_completion {
                responder.close().await;
            }

            let task = QueueTask {
                request_id,
                message: message.clone(),
                prompt: if prompt.is_empty() { text_prompt(message) } else { prompt },
                permission_mode,
                resume_policy,
                non_interactive_permissions,
                timeout_ms,
                suppress_sdk_console_errors,
                wait_for_completion,
                enqueued_at: Instant::now(),
                responder,
            };

            enqueue_task(
                task,
                state,
                notify,
                owner_generation,
                max_queue_depth,
                on_queue_depth_changed,
            )
            .await;
        }
    }
}

#[cfg(unix)]
async fn handle_control_request<Fut>(
    responder: QueueTaskResponder,
    request_id: String,
    owner_generation: Option<u64>,
    run: Fut,
) where
    Fut: std::future::Future<Output = Result<QueueOwnerMessage, QueueConnectionError>>,
{
    responder
        .send(QueueOwnerMessage::Accepted { request_id: request_id.clone(), owner_generation })
        .await;

    let message = match run.await {
        Ok(message) => with_owner_generation(message, owner_generation),
        Err(error) => make_queue_owner_error_from_connection_error(
            request_id,
            &error,
            "QUEUE_CONTROL_REQUEST_FAILED",
            owner_generation,
        ),
    };

    responder.send(message).await;
    responder.close().await;
}

#[cfg(unix)]
async fn enqueue_task(
    task: QueueTask,
    state: Arc<Mutex<QueueOwnerState>>,
    notify: Arc<Notify>,
    owner_generation: Option<u64>,
    max_queue_depth: usize,
    on_queue_depth_changed: Option<QueueDepthChangedCallback>,
) {
    if state.lock().closed {
        if task.wait_for_completion {
            task.send(make_queue_owner_error(
                task.request_id.clone(),
                "Queue owner is shutting down",
                "QUEUE_OWNER_SHUTTING_DOWN",
                Some(true),
                owner_generation,
            ))
            .await;
        }
        task.close().await;
        return;
    }

    let queue_depth = {
        let mut state = state.lock();
        if state.pending.len() >= max_queue_depth {
            None
        } else {
            state.pending.push_back(task.clone());
            Some(state.pending.len())
        }
    };

    let Some(queue_depth) = queue_depth else {
        if task.wait_for_completion {
            task.send(make_queue_owner_error(
                task.request_id.clone(),
                format!(
                    "Queue owner is overloaded ({}/{max_queue_depth} queued)",
                    state.lock().pending.len()
                ),
                "QUEUE_OWNER_OVERLOADED",
                Some(true),
                owner_generation,
            ))
            .await;
        }
        task.close().await;
        return;
    };

    if let Some(callback) = on_queue_depth_changed {
        callback(queue_depth);
    }
    notify.notify_one();
}

#[cfg(unix)]
fn queue_request_id(request: &QueueRequest) -> &str {
    match request {
        QueueRequest::SubmitPrompt { request_id, .. }
        | QueueRequest::CancelPrompt { request_id, .. }
        | QueueRequest::SetMode { request_id, .. }
        | QueueRequest::SetModel { request_id, .. }
        | QueueRequest::SetConfigOption { request_id, .. } => request_id,
    }
}

#[cfg(unix)]
fn queue_request_owner_generation(request: &QueueRequest) -> Option<u64> {
    match request {
        QueueRequest::SubmitPrompt { owner_generation, .. }
        | QueueRequest::CancelPrompt { owner_generation, .. }
        | QueueRequest::SetMode { owner_generation, .. }
        | QueueRequest::SetModel { owner_generation, .. }
        | QueueRequest::SetConfigOption { owner_generation, .. } => *owner_generation,
    }
}

#[cfg(unix)]
fn with_owner_generation(
    message: QueueOwnerMessage,
    owner_generation: Option<u64>,
) -> QueueOwnerMessage {
    match message {
        QueueOwnerMessage::Accepted { request_id, .. } => {
            QueueOwnerMessage::Accepted { request_id, owner_generation }
        }
        QueueOwnerMessage::Event { request_id, message, .. } => {
            QueueOwnerMessage::Event { request_id, owner_generation, message }
        }
        QueueOwnerMessage::Result { request_id, result, .. } => {
            QueueOwnerMessage::Result { request_id, owner_generation, result }
        }
        QueueOwnerMessage::CancelResult { request_id, cancelled, .. } => {
            QueueOwnerMessage::CancelResult { request_id, owner_generation, cancelled }
        }
        QueueOwnerMessage::SetModeResult { request_id, mode_id, .. } => {
            QueueOwnerMessage::SetModeResult { request_id, owner_generation, mode_id }
        }
        QueueOwnerMessage::SetModelResult { request_id, model_id, .. } => {
            QueueOwnerMessage::SetModelResult { request_id, owner_generation, model_id }
        }
        QueueOwnerMessage::SetConfigOptionResult { request_id, response, .. } => {
            QueueOwnerMessage::SetConfigOptionResult { request_id, owner_generation, response }
        }
        QueueOwnerMessage::Error {
            request_id,
            code,
            detail_code,
            origin,
            message,
            retryable,
            acp,
            output_already_emitted,
            ..
        } => QueueOwnerMessage::Error {
            request_id,
            owner_generation,
            code,
            detail_code,
            origin,
            message,
            retryable,
            acp,
            output_already_emitted,
        },
    }
}

#[cfg(unix)]
fn make_queue_owner_error(
    request_id: String,
    message: impl Into<String>,
    detail_code: impl Into<String>,
    retryable: Option<bool>,
    owner_generation: Option<u64>,
) -> QueueOwnerMessage {
    QueueOwnerMessage::Error {
        request_id,
        owner_generation,
        code: OutputErrorCode::Runtime,
        detail_code: Some(detail_code.into()),
        origin: OutputErrorOrigin::Queue,
        message: message.into(),
        retryable,
        acp: None,
        output_already_emitted: None,
    }
}

#[cfg(unix)]
fn make_queue_owner_error_from_connection_error(
    request_id: String,
    error: &QueueConnectionError,
    default_detail_code: impl Into<String>,
    owner_generation: Option<u64>,
) -> QueueOwnerMessage {
    QueueOwnerMessage::Error {
        request_id,
        owner_generation,
        code: error.output_code().unwrap_or(OutputErrorCode::Runtime),
        detail_code: Some(
            error
                .detail_code()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| default_detail_code.into()),
        ),
        origin: error.origin().unwrap_or(OutputErrorOrigin::Queue),
        message: error.message().to_string(),
        retryable: error.retryable(),
        acp: error.acp().cloned(),
        output_already_emitted: None,
    }
}

#[cfg(all(test, unix))]
#[path = "queue_ipc_server_tests.rs"]
mod queue_ipc_server_tests;

#[cfg(windows)]
impl SessionQueueOwner {
    pub async fn start(
        _lease: QueueOwnerSocketLease,
        _control_handlers: Arc<dyn QueueOwnerControlHandlers>,
        _options: SessionQueueOwnerOptions,
    ) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "queue owner server is not supported on windows",
        ))
    }

    pub async fn close(self) -> io::Result<()> {
        Ok(())
    }

    pub async fn next_task(&self, _timeout_ms: Option<u64>) -> Option<QueueTask> {
        None
    }
}
