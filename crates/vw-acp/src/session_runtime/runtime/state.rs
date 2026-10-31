//! 会话运行期间的可变状态与输出转发工具。
//!
//! 本模块集中管理提示执行时产生的对话状态、ACP 消息缓存、输出回放和
//! 队列任务格式化器。状态对象在回调中被共享，因此内部使用锁和原子标记
//! 来区分连接阶段、提示阶段以及是否已经产生副作用。

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::perf_metrics::measure_perf;
use crate::session_conversation_model::{
    clone_session_conversation, clone_session_vwacp_state, record_client_operation,
    record_prompt_submission, record_session_update, trim_conversation_for_runtime,
};
use crate::session_event_log::{
    DEFAULT_EVENT_MAX_SEGMENTS, DEFAULT_EVENT_SEGMENT_MAX_BYTES, default_session_event_log,
    session_event_log,
};
use crate::session_events::{SessionEventAppendOptions, SessionEventWriter};
use crate::session_persistence::iso_now;
use crate::session_runtime::lifecycle::apply_conversation;
use crate::types::{
    AcpJsonRpcMessage, AcpMessageCallback, AcpSessionOptions, ClientOperation,
    ClientOperationCallback, OutputErrorOrigin, OutputErrorParams, OutputFormatter,
    OutputFormatterContext, SessionConversation, SessionEventLog, SessionRecord,
    SessionStateOptions, SessionUpdateCallback,
};

use super::SessionRuntimeError;

#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;

/// 单次持久会话提示执行期间共享的运行状态。
///
/// 该结构体缓存对话、vwacp 扩展状态和待落盘消息，并用原子标记记录
/// 当前是否处于提示 turn、是否已经观察到 ACP 消息或副作用。
pub(super) struct RuntimeState {
    conversation: Mutex<SessionConversation>,
    vwacp_state: Mutex<Option<crate::SessionAcpxState>>,
    pending_messages: Mutex<Vec<AcpJsonRpcMessage>>,
    pending_connect_output_messages: Mutex<Vec<AcpJsonRpcMessage>>,
    pending_prompt_output_messages: Mutex<Vec<AcpJsonRpcMessage>>,
    prompt_turn_active: AtomicBool,
    prompt_turn_had_side_effects: AtomicBool,
    saw_acp_message: AtomicBool,
    buffering_connect_output: AtomicBool,
}

impl RuntimeState {
    /// 从持久化记录和待提交提示创建运行状态。
    ///
    /// 创建时会克隆记录中的对话与 vwacp 状态，并立即把本次用户提示写入
    /// 内存对话模型，供后续回调增量更新。
    pub(super) fn new(record: &SessionRecord, prompt: &crate::prompt_content::PromptInput) -> Self {
        let mut conversation = clone_session_conversation(Some(&SessionConversation {
            title: record.title.clone(),
            messages: record.messages.clone(),
            updated_at: record.updated_at.clone(),
            cumulative_token_usage: record.cumulative_token_usage.clone(),
            request_token_usage: record.request_token_usage.clone(),
        }));
        let vwacp_state = clone_session_vwacp_state(record.vwacp.as_ref());
        record_prompt_submission(&mut conversation, prompt, Some(&iso_now()));
        Self {
            conversation: Mutex::new(conversation),
            vwacp_state: Mutex::new(vwacp_state),
            pending_messages: Mutex::new(Vec::new()),
            pending_connect_output_messages: Mutex::new(Vec::new()),
            pending_prompt_output_messages: Mutex::new(Vec::new()),
            prompt_turn_active: AtomicBool::new(false),
            prompt_turn_had_side_effects: AtomicBool::new(false),
            saw_acp_message: AtomicBool::new(false),
            buffering_connect_output: AtomicBool::new(true),
        }
    }

    pub(super) fn take_pending_messages(&self) -> Vec<AcpJsonRpcMessage> {
        let mut guard = self.pending_messages.lock();
        std::mem::take(&mut *guard)
    }

    pub(super) fn take_connect_output_messages(&self) -> Vec<AcpJsonRpcMessage> {
        let mut guard = self.pending_connect_output_messages.lock();
        std::mem::take(&mut *guard)
    }

    pub(super) fn take_prompt_output_messages(&self) -> Vec<AcpJsonRpcMessage> {
        let mut guard = self.pending_prompt_output_messages.lock();
        std::mem::take(&mut *guard)
    }

    pub(super) fn prompt_output_messages(&self) -> &Mutex<Vec<AcpJsonRpcMessage>> {
        &self.pending_prompt_output_messages
    }

    pub(super) fn prompt_turn_had_side_effects(&self) -> bool {
        self.prompt_turn_had_side_effects.load(Ordering::SeqCst)
    }

    pub(super) fn saw_acp_message(&self) -> bool {
        self.saw_acp_message.load(Ordering::SeqCst)
    }

    pub(super) fn mark_prompt_turn_active(&self, active: bool) {
        self.prompt_turn_active.store(active, Ordering::SeqCst);
    }

    pub(super) fn set_buffering_connect_output(&self, buffering: bool) {
        self.buffering_connect_output.store(buffering, Ordering::SeqCst);
    }

    pub(super) fn sync_session_state(&self, record: &SessionRecord) {
        *self.vwacp_state.lock() = clone_session_vwacp_state(record.vwacp.as_ref());
    }

    pub(super) fn apply_to_record(&self, record: &mut SessionRecord) {
        let conversation = self.conversation.lock().clone();
        apply_conversation(record, &conversation);
        record.vwacp = self.vwacp_state.lock().clone();
    }
}

/// 复制一条 ACP JSON-RPC 消息。
///
/// 消息类型可能包含内部所有权结构，因此这里经由 JSON 值做深拷贝；
/// 序列化或反序列化失败时返回 `None`，避免把不可复制消息写入缓存。
pub(super) fn duplicate_acp_message(message: &AcpJsonRpcMessage) -> Option<AcpJsonRpcMessage> {
    serde_json::to_value(message).ok().and_then(|value| serde_json::from_value(value).ok())
}

/// 将运行时输出转发给队列请求方的格式化器。
///
/// 格式化器把 ACP 消息和错误包装成队列所有者消息，并通过后台任务
/// 异步发送，避免运行时回调直接阻塞在 IPC 发送路径上。
pub(super) struct QueueTaskOutputFormatter {
    request_id: String,
    event_tx: Option<mpsc::UnboundedSender<crate::queue_messages::QueueOwnerMessage>>,
    forward_task: Option<JoinHandle<()>>,
    emitted_messages: Arc<AtomicBool>,
}

impl QueueTaskOutputFormatter {
    pub(super) fn new(
        request_id: impl Into<String>,
        task: crate::queue_ipc_server::QueueTask,
    ) -> Self {
        let (event_tx, mut event_rx) =
            mpsc::unbounded_channel::<crate::queue_messages::QueueOwnerMessage>();
        let forward_task = tokio::spawn(async move {
            while let Some(message) = event_rx.recv().await {
                task.send(message).await;
            }
        });
        Self {
            request_id: request_id.into(),
            event_tx: Some(event_tx),
            forward_task: Some(forward_task),
            emitted_messages: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(super) fn has_messages(&self) -> bool {
        self.emitted_messages.load(Ordering::SeqCst)
    }

    pub(super) async fn finish(&mut self) {
        self.event_tx.take();
        if let Some(handle) = self.forward_task.take() {
            let _ = handle.await;
        }
    }
}

impl OutputFormatter for QueueTaskOutputFormatter {
    fn set_context(&mut self, _context: OutputFormatterContext) {}

    fn on_acp_message(&mut self, message: AcpJsonRpcMessage) {
        self.emitted_messages.store(true, Ordering::SeqCst);
        if let Some(event_tx) = &self.event_tx {
            let _ = event_tx.send(crate::queue_messages::QueueOwnerMessage::Event {
                request_id: self.request_id.clone(),
                owner_generation: None,
                message,
            });
        }
    }

    fn on_error(&mut self, params: OutputErrorParams) {
        self.emitted_messages.store(true, Ordering::SeqCst);
        if let Some(event_tx) = &self.event_tx {
            let _ = event_tx.send(crate::queue_messages::QueueOwnerMessage::Error {
                request_id: self.request_id.clone(),
                owner_generation: None,
                code: params.code,
                detail_code: params.detail_code,
                origin: params.origin.unwrap_or(OutputErrorOrigin::Runtime),
                message: params.message,
                retryable: params.retryable,
                acp: params.acp,
                output_already_emitted: None,
            });
        }
    }

    fn flush(&mut self) {}
}

/// 丢弃所有输出的格式化器。
///
/// 非等待型队列任务只需要触发执行，不需要把中间事件返回给提交方。
pub(super) struct DiscardOutputFormatter;

impl OutputFormatter for DiscardOutputFormatter {
    fn set_context(&mut self, _context: OutputFormatterContext) {}

    fn on_acp_message(&mut self, _message: AcpJsonRpcMessage) {}

    fn on_error(&mut self, _params: OutputErrorParams) {}

    fn flush(&mut self) {}
}

/// 将缓存中的输出消息回放到格式化器并立即 flush。
pub(super) fn replay_output_buffer(
    formatter: &mut dyn OutputFormatter,
    output_messages: &Mutex<Vec<AcpJsonRpcMessage>>,
) {
    replay_output_messages(formatter, std::mem::take(&mut *output_messages.lock()));
    formatter.flush();
}

/// 按原顺序把输出消息提交给格式化器。
pub(super) fn replay_output_messages(
    formatter: &mut dyn OutputFormatter,
    messages: Vec<AcpJsonRpcMessage>,
) {
    for message in messages {
        formatter.on_acp_message(message);
    }
}

/// 构建 ACP 客户端回调并把事件同步进运行状态。
///
/// 返回的四个回调分别处理原始 ACP 消息、可展示输出、会话更新和客户端操作。
/// 回调会先更新本地状态，再调用外部回调，确保持久化视图不会落后于
/// 用户自定义观察者。
pub(super) fn build_runtime_state_callbacks(
    state: Arc<RuntimeState>,
    on_acp_message: Option<AcpMessageCallback>,
    on_session_update: Option<SessionUpdateCallback>,
    on_client_operation: Option<ClientOperationCallback>,
) -> (
    Option<AcpMessageCallback>,
    Option<AcpMessageCallback>,
    Option<SessionUpdateCallback>,
    Option<ClientOperationCallback>,
) {
    let on_message_callback = {
        let state = state.clone();
        let on_acp_message = on_acp_message.clone();
        Some(Arc::new(move |direction, message: AcpJsonRpcMessage| {
            state.saw_acp_message.store(true, Ordering::SeqCst);
            if let Some(duplicate) = duplicate_acp_message(&message) {
                state.pending_messages.lock().push(duplicate);
            }
            if let Some(callback) = on_acp_message.as_ref() {
                callback(direction, message);
            }
        }) as AcpMessageCallback)
    };

    let on_output_callback = {
        let state = state.clone();
        Some(Arc::new(move |direction, message: AcpJsonRpcMessage| {
            let duplicate = duplicate_acp_message(&message);
            if state.buffering_connect_output.load(Ordering::SeqCst) {
                if let Some(duplicate) = duplicate {
                    state.pending_connect_output_messages.lock().push(duplicate);
                }
            } else if let Some(duplicate) = duplicate {
                state.pending_prompt_output_messages.lock().push(duplicate);
            }
            if let Some(callback) = on_acp_message.as_ref() {
                callback(direction, message);
            }
        }) as AcpMessageCallback)
    };

    let on_session_update_callback = {
        let state = state.clone();
        Some(Arc::new(move |notification| {
            if state.prompt_turn_active.load(Ordering::SeqCst) {
                state.prompt_turn_had_side_effects.store(true, Ordering::SeqCst);
            }
            {
                let mut conversation = state.conversation.lock();
                // session/load 回放的是已持久化历史，不能追加到当前用户提示之后。
                let mut replay_conversation = clone_session_conversation(None);
                let conversation = if state.buffering_connect_output.load(Ordering::SeqCst) {
                    &mut replay_conversation
                } else {
                    &mut *conversation
                };
                let next_state = {
                    let vwacp_state = state.vwacp_state.lock();
                    record_session_update(
                        conversation,
                        vwacp_state.as_ref(),
                        &notification,
                        Some(&iso_now()),
                    )
                };
                trim_conversation_for_runtime(conversation);
                *state.vwacp_state.lock() = Some(next_state);
            }
            if let Some(callback) = on_session_update.as_ref() {
                callback(notification);
            }
        }) as SessionUpdateCallback)
    };

    let on_client_operation_callback = {
        let state = state.clone();
        Some(Arc::new(move |operation: ClientOperation| {
            if state.prompt_turn_active.load(Ordering::SeqCst) {
                state.prompt_turn_had_side_effects.store(true, Ordering::SeqCst);
            }
            {
                let mut conversation = state.conversation.lock();
                let next_state = {
                    let vwacp_state = state.vwacp_state.lock();
                    record_client_operation(
                        &mut conversation,
                        vwacp_state.as_ref(),
                        &operation,
                        Some(&iso_now()),
                    )
                };
                trim_conversation_for_runtime(&mut conversation);
                *state.vwacp_state.lock() = Some(next_state);
            }
            if let Some(callback) = on_client_operation.as_ref() {
                callback(operation);
            }
        }) as ClientOperationCallback)
    };

    (
        on_message_callback,
        on_output_callback,
        on_session_update_callback,
        on_client_operation_callback,
    )
}

/// 将暂存的 ACP 消息批量写入事件日志。
///
/// 当没有待写消息时直接成功返回；写入失败会转换为 [`SessionRuntimeError`]。
pub(super) async fn flush_pending_messages(
    event_writer: &mut SessionEventWriter,
    state: &RuntimeState,
    checkpoint: bool,
) -> Result<(), SessionRuntimeError> {
    let batch = state.take_pending_messages();
    if batch.is_empty() {
        return Ok(());
    }

    measure_perf("session.events.flush_pending", || async {
        event_writer.append_messages(&batch, SessionEventAppendOptions { checkpoint }).await
    })
    .await
    .map_err(SessionRuntimeError::from_source)
}

/// 构造事件日志元数据的兜底值。
///
/// 默认路径无法生成时使用空路径和默认分段限制，保证会话记录仍有
/// 明确的事件日志结构，而不是留下缺失字段。
pub(super) fn fallback_event_log(session_id: &str) -> SessionEventLog {
    default_session_event_log(session_id).unwrap_or_else(|| SessionEventLog {
        active_path: session_event_log(session_id, PathBuf::new()).active_path,
        segment_count: DEFAULT_EVENT_MAX_SEGMENTS,
        max_segment_bytes: DEFAULT_EVENT_SEGMENT_MAX_BYTES,
        max_segments: DEFAULT_EVENT_MAX_SEGMENTS,
        last_write_at: None,
        last_write_error: None,
    })
}

/// 从会话记录中恢复可传给 ACP 客户端的会话选项。
///
/// 空白模型和空工具列表会被过滤；没有任何有效值时返回 `None`。
pub(super) fn session_options_from_record(record: &SessionRecord) -> Option<AcpSessionOptions> {
    let stored = record.vwacp.as_ref().and_then(|state| state.session_options.as_ref())?;

    let mut session_options = AcpSessionOptions::default();
    if let Some(model) = non_empty_trimmed(stored.model.as_deref()) {
        session_options.model = Some(model);
    }
    if let Some(allowed_tools) = stored.allowed_tools.clone()
        && let Some(allowed_tools) = clone_non_empty_tools(&allowed_tools)
    {
        session_options.allowed_tools = Some(allowed_tools);
    }
    if let Some(max_turns) = stored.max_turns {
        session_options.max_turns = Some(max_turns);
    }

    (session_options.model.is_some()
        || session_options.allowed_tools.is_some()
        || session_options.max_turns.is_some())
    .then_some(session_options)
}

/// 将请求的会话选项持久化到记录的 vwacp 状态中。
///
/// 只有存在有效模型、工具或最大轮数时才写入；否则清除旧的会话选项，
/// 避免空配置被误判为用户显式设置。
pub(super) fn persist_session_options(
    record: &mut SessionRecord,
    options: Option<&AcpSessionOptions>,
) {
    let next = options.map(|options| SessionStateOptions {
        model: non_empty_trimmed(options.model.as_deref()),
        allowed_tools: options
            .allowed_tools
            .as_ref()
            .and_then(|entries| clone_non_empty_tools(entries)),
        max_turns: options.max_turns,
    });

    let has_values = next.as_ref().is_some_and(|options| {
        options.model.is_some()
            || options.allowed_tools.as_ref().is_some_and(|entries| !entries.is_empty())
            || options.max_turns.is_some()
    });

    if has_values {
        let vwacp = record.vwacp.get_or_insert(crate::SessionAcpxState {
            current_mode_id: None,
            desired_mode_id: None,
            current_model_id: None,
            available_models: None,
            available_commands: None,
            config_options: None,
            desired_config_options: None,
            session_options: None,
        });
        vwacp.session_options = next;
        return;
    }

    if let Some(vwacp) = record.vwacp.as_mut() {
        vwacp.session_options = None;
    }
}

/// 修剪字符串并过滤空值。
pub(super) fn non_empty_trimmed(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// 克隆工具列表并移除空白条目。
pub(super) fn clone_non_empty_tools(entries: &[String]) -> Option<Vec<String>> {
    let allowed_tools = entries
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect::<Vec<_>>();
    (!allowed_tools.is_empty()).then_some(allowed_tools)
}
