//! LLM 步骤运行器模块
//!
//! 本模块提供与大型语言模型（LLM）交互的核心功能，负责执行 LLM 调用并处理流式响应。
//! 主要职责包括：
//! - 管理 LLM 调用的生命周期，包括流式响应的收集和处理
//! - 处理推理内容（reasoning）和普通文本内容的分离与格式化
//! - 实现带重试机制的 LLM 调用策略，提高调用的可靠性
//! - 管理工具调用的收集和传递
//!
//! # 核心组件
//!
//! - [`LlmStep`]: 封装单次 LLM 调用的完整结果
//! - [`run_llm_step`]: 执行单次 LLM 调用
//! - [`run_llm_step_with_retry`]: 执行带重试机制的 LLM 调用
//!
//! # 流式处理策略
//!
//! 本模块会立即发送各内容通道的首个非空增量，后续流式内容刷新基于两个维度：
//! - 时间间隔：至少间隔 33ms 刷新一次，避免过于频繁的事件发送
//! - 内容长度：累积至少 1024 字符后刷新，减少小包传输的开销
//!
//! # 推理内容处理
//!
//! 模块会自动检测并格式化推理内容（模型思考过程），使用标准化的标签包裹，
//! 确保推理内容与普通文本内容清晰分离。

use super::types::{StreamEvent, ToolCallStatus, ToolEvent, ToolEventKind};
use super::utils::now_ms;
use crate::app::agent::session::llm;
use crate::app::agent::session::message::AssistantError;
use crate::app::agent::tools::ToolSpec;
use crate::session::ui_types as models;
use serde_json::Value;
use std::time::Duration;
use web_time::Instant;

const DEFAULT_LLM_STEP_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
// ACP 一次性会话由远端代理自行执行工具；一次工具调用（如 cargo llvm-cov、长测试）
// 可能数分钟内不产生任何流事件，空窗时间远超模型 token 间隔。这里按分钟级命令的空窗
// 设置上限，避免把仍在执行的长命令误判为“模型响应超时”。
const ONE_SHOT_ACP_STEP_IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// LLM 步骤执行结果
///
/// 封装单次 LLM 调用的完整输出结果，包括令牌使用统计、完成原因、推理内容、
/// 文本输出、工具调用以及完整的消息历史。
///
/// # 字段说明
///
/// * `usage` - 令牌使用统计，包括输入和输出令牌数量
/// * `finish_reason` - 模型完成响应的原因（如 "stop"、"length" 等）
/// * `reasoning_content` - 模型的推理/思考过程内容
/// * `text` - 模型生成的文本内容
/// * `tool_calls` - 模型请求执行的工具调用列表
/// * `full_messages` - 完整的消息历史记录，用于上下文追溯
///
/// # 示例
///
/// ```ignore
/// let step = run_llm_step(&messages, &system, model, &tools, &mut on_event)?;
/// println!("生成的文本: {}", step.text);
/// println!("使用了 {} 个令牌", step.usage.total_tokens);
/// ```
#[derive(Debug, Clone)]
pub(crate) struct LlmStep {
    /// 令牌使用统计信息
    pub(crate) usage: models::TokenUsage,
    /// 响应完成的原因（可选）
    pub(crate) finish_reason: Option<String>,
    /// 模型的推理/思考内容
    pub(crate) reasoning_content: String,
    /// 模型生成的主要文本内容
    pub(crate) text: String,
    /// 模型请求执行的工具调用列表
    pub(crate) tool_calls: Vec<llm::ToolCall>,
    /// 完整的消息历史，包含所有上下文信息
    pub(crate) full_messages: Vec<serde_json::Value>,
}

fn is_acp_request(options: &Value) -> bool {
    options.get("acp_test").and_then(Value::as_bool).unwrap_or(false)
        || options
            .get("acp_agent")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
}

pub(super) fn is_acp_retryable_error(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("模型响应超时")
        || normalized.contains("timed out")
        || normalized.contains("timeout")
        || normalized.contains("acp session changed:")
        || normalized.contains("acp agent disconnected during request")
        || normalized.contains("queue owner disconnected before prompt completion")
        || normalized.contains("error decoding response body")
        || normalized.contains("eof while parsing a value")
}

fn llm_error_message(error: &str) -> String {
    serde_json::from_str::<Value>(error)
        .ok()
        .and_then(|value| value.get("message").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| error.to_string())
}

/// Context overflow needs a compaction retry in the caller, never the generic retry loop.
fn replay_is_unsafe(error: &str) -> bool {
    matches!(serde_json::from_str::<AssistantError>(error), Ok(AssistantError::APIError { metadata: Some(metadata), .. }) if metadata.get("replay_safe").is_some_and(|value| value == "false"))
}

pub(crate) fn is_prompt_too_long_error(error: &str) -> bool {
    if replay_is_unsafe(error) {
        return false;
    }
    let normalized = llm_error_message(error).to_ascii_lowercase();
    [
        "context length",
        "context window",
        "prompt too long",
        "too many tokens",
        "context overflow",
        "context_length_exceeded",
        "input too long",
        "input length exceeds",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

/// A range error alone is ambiguous (empty input also violates [1, N]).
/// Recover only when this nonempty request demonstrably exceeds its reported bound.
pub(crate) fn input_length_range_max(error: &str) -> Option<u64> {
    let normalized = llm_error_message(error).to_ascii_lowercase();
    let range = normalized.split_once("range of input length should be [")?.1.split_once(']')?.0;
    let (min, max) = range.split_once(',')?;
    let min = min.trim().parse::<u64>().ok()?;
    let max = max.trim().parse::<u64>().ok()?;
    (min == 1 && max > min).then_some(max)
}

pub(crate) fn is_request_too_long_error(error: &str, messages: &[Value]) -> bool {
    if replay_is_unsafe(error) {
        return false;
    }
    if is_prompt_too_long_error(error) {
        return true;
    }
    let Some(max) = input_length_range_max(error) else { return false };
    // Count content/arguments, not JSON envelope bytes: an empty field must never
    // be "repaired" by deleting unrelated history. Bytes are a conservative upper
    // bound when a provider does not report whether its limit uses chars or tokens.
    fn text_bytes(value: &Value) -> u64 {
        match value {
            Value::String(text) => text.len() as u64,
            Value::Array(items) => {
                items.iter().fold(0_u64, |n, item| n.saturating_add(text_bytes(item)))
            }
            Value::Object(fields) => {
                fields.values().fold(0_u64, |n, item| n.saturating_add(text_bytes(item)))
            }
            _ => 0,
        }
    }
    if messages.iter().any(|item| {
        text_bytes(&item["content"]) == 0 && item["tool_calls"].as_array().is_none_or(Vec::is_empty)
    }) {
        return false;
    }
    messages.iter().fold(0_u64, |n, item| {
        n.saturating_add(text_bytes(&item["content"]))
            .saturating_add(text_bytes(&item["tool_calls"]))
    }) > max
}

#[cfg(test)]
fn llm_step_retry_allowed(error: &str, is_acp: bool) -> bool {
    super::recovery::retry_allowed(error, is_acp)
}

fn llm_step_idle_timeout(options: &Value) -> Duration {
    if options.get("acp_close_client_after_request").and_then(Value::as_bool) == Some(true) {
        return ONE_SHOT_ACP_STEP_IDLE_TIMEOUT;
    }

    DEFAULT_LLM_STEP_IDLE_TIMEOUT
}

const STREAM_CONSUMER_STOPPED_ERR: &str = "stream consumer stopped";

fn request_stream_abort(abort_tx: &tokio::sync::watch::Sender<bool>) {
    let _ = abort_tx.send(true);
}

fn emit_stream_event(
    on_event: &mut dyn FnMut(StreamEvent) -> bool,
    abort_tx: &tokio::sync::watch::Sender<bool>,
    event: StreamEvent,
) -> Result<(), String> {
    if on_event(event) {
        return Ok(());
    }

    request_stream_abort(abort_tx);
    Err(STREAM_CONSUMER_STOPPED_ERR.to_string())
}

#[cfg(test)]
fn spawn_llm_stream_thread(
    session_id: String,
    messages: Vec<serde_json::Value>,
    system: Vec<String>,
    model: Option<String>,
    options: serde_json::Value,
    tools: std::collections::HashMap<String, ToolSpec>,
    abort_rx: tokio::sync::watch::Receiver<bool>,
    tx: std::sync::mpsc::Sender<llm::StreamEvent>,
) -> std::thread::JoinHandle<()> {
    if std::env::var_os("VW_RECOVERY_WINDOW_HTTP").is_some() {
        return spawn_real_llm_stream_thread(
            session_id, messages, system, model, options, tools, abort_rx, tx,
        );
    }
    llm_runner_tests::record_test_llm_messages(session_id.clone(), messages, options);
    llm_runner_tests::spawn_test_llm_stream_thread(&session_id, tools, abort_rx, tx)
}

#[cfg(not(test))]
use spawn_real_llm_stream_thread as spawn_llm_stream_thread;

fn spawn_real_llm_stream_thread(
    session_id: String,
    messages: Vec<serde_json::Value>,
    system: Vec<String>,
    model: Option<String>,
    options: serde_json::Value,
    tools: std::collections::HashMap<String, ToolSpec>,
    abort_rx: tokio::sync::watch::Receiver<bool>,
    tx: std::sync::mpsc::Sender<llm::StreamEvent>,
) -> std::thread::JoinHandle<()> {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let project =
        crate::project::instance::project().map(|project| crate::project::instance::Context {
            directory: crate::project::instance::directory(),
            worktree: crate::project::instance::worktree(),
            project,
        });
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = tx.send(llm::StreamEvent::Error(AssistantError::Unknown {
                    message: error.to_string(),
                }));
                return;
            }
        };
        runtime.block_on(crate::session::ui_store::with_tenant_scope(tenant, async move {
            let request = async move {
                llm::stream_chat_with_tools_for_session_with_abort(
                    &session_id,
                    messages,
                    system,
                    model,
                    options,
                    tools,
                    Some(abort_rx),
                    true,
                    move |event| {
                        let _ = tx.send(event);
                    },
                );
            };
            match project {
                Some(context) => crate::project::instance::provide_context(context, request).await,
                None => request.await,
            }
        }));
    })
}

fn collect_llm_step_from_stream(
    rx: std::sync::mpsc::Receiver<llm::StreamEvent>,
    on_event: &mut dyn FnMut(StreamEvent) -> bool,
    abort_tx: &tokio::sync::watch::Sender<bool>,
    idle_timeout: std::time::Duration,
    cancellation: Option<&tokio_util::sync::CancellationToken>,
) -> Result<LlmStep, String> {
    /// 流式内容刷新的最小时间间隔（约 30 FPS）
    const FLUSH_MIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(33);
    /// 流式内容刷新的最小字符数
    const FLUSH_MIN_CHARS: usize = 1024;

    let mut reasoning_content = String::new(); // 推理内容缓冲区
    let mut text = String::new(); // 普通文本内容缓冲区
    let mut usage = models::TokenUsage::default(); // 令牌使用统计
    let mut finish_reason: Option<String> = None; // 完成原因
    let mut tool_calls: Vec<llm::ToolCall> = Vec::new(); // 工具调用列表
    let mut full_messages: Vec<serde_json::Value> = Vec::new(); // 完整消息历史

    // === 状态追踪变量 ===
    let mut pending = String::new(); // 待刷新的普通文本缓冲区
    let mut pending_reasoning = String::new(); // 待刷新的推理文本缓冲区
    let mut last_flush = Instant::now(); // 上次普通文本刷新时间
    let mut last_reasoning_flush = Instant::now(); // 上次推理文本刷新时间
    let mut content_started = false; // 标记是否开始接收普通内容
    let mut text_delta_emitted = false; // 标记是否已立即输出首个普通文本增量
    let mut reasoning_delta_emitted = false; // 标记是否已立即输出首个推理增量
    let mut thinking_open = false; // 标记 thinking 标签是否已打开
    let mut thinking_emitted = false; // 标记是否已输出推理内容

    // 短轮询只用于取消；模型空闲超时仍按最后一条事件计时。
    let mut last_event = Instant::now();
    // === 主事件处理循环 ===
    loop {
        if cancellation.is_some_and(|token| token.is_cancelled()) {
            request_stream_abort(abort_tx);
            return Err("执行已取消".into());
        }
        // 尝试接收事件，设置超时以检测无响应情况
        let ev = match rx.recv_timeout(idle_timeout.min(Duration::from_millis(50))) {
            Ok(v) => {
                last_event = Instant::now();
                v
            }
            // 超时处理：刷新所有待输出内容并返回错误
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                if last_event.elapsed() < idle_timeout =>
            {
                continue;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // 刷新待输出的普通文本
                if !pending.is_empty() {
                    let chunk = std::mem::take(&mut pending);
                    emit_stream_event(on_event, abort_tx, StreamEvent::Delta(chunk))?;
                }
                // 刷新待输出的推理文本
                if !pending_reasoning.is_empty() {
                    let chunk = std::mem::take(&mut pending_reasoning);
                    emit_stream_event(on_event, abort_tx, StreamEvent::Delta(chunk))?;
                }
                // 如果 thinking 标签仍然打开，则关闭它
                if thinking_open {
                    emit_stream_event(
                        on_event,
                        abort_tx,
                        StreamEvent::Delta("</think>\n\n".to_string()),
                    )?;
                }
                request_stream_abort(abort_tx);
                return Err("模型响应超时".to_string());
            }
            // 发送端断开，表示流结束
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };

        // 根据事件类型进行相应处理
        match ev {
            // 处理普通文本增量
            llm::StreamEvent::Delta(delta) => {
                if delta.is_empty() {
                    continue;
                }
                // 检测是否首次接收到普通内容
                if !content_started {
                    content_started = true;
                    // 如果之前有推理内容，需要关闭 thinking 标签
                    if thinking_open {
                        if !pending_reasoning.is_empty() {
                            let chunk = std::mem::take(&mut pending_reasoning);
                            emit_stream_event(on_event, abort_tx, StreamEvent::Delta(chunk))?;
                        }
                        emit_stream_event(
                            on_event,
                            abort_tx,
                            StreamEvent::Delta("</think>\n\n".to_string()),
                        )?;
                        thinking_open = false;
                    }
                }
                // 累积普通文本
                text.push_str(&delta);
                pending.push_str(&delta);
                // 首个非空普通文本不等待合并窗口；后续才按长度或时间节流。
                if !text_delta_emitted
                    || pending.len() >= FLUSH_MIN_CHARS
                    || last_flush.elapsed() >= FLUSH_MIN_INTERVAL
                {
                    let chunk = std::mem::take(&mut pending);
                    last_flush = Instant::now();
                    emit_stream_event(on_event, abort_tx, StreamEvent::Delta(chunk))?;
                    text_delta_emitted = true;
                }
            }
            // 处理推理内容增量
            llm::StreamEvent::ReasoningDelta(delta) => {
                if delta.is_empty() {
                    continue;
                }
                // 累积推理内容
                reasoning_content.push_str(&delta);
                // 如果还未开始接收普通内容，则流式输出推理内容
                if !content_started {
                    // 如果 thinking 标签未打开，则打开它
                    if !thinking_open {
                        pending_reasoning.push_str("<think>");
                        thinking_open = true;
                        thinking_emitted = true;
                    }
                    pending_reasoning.push_str(&delta);
                    // 首个非空推理增量不等待合并窗口；后续才按长度或时间节流。
                    if !reasoning_delta_emitted
                        || pending_reasoning.len() >= FLUSH_MIN_CHARS
                        || last_reasoning_flush.elapsed() >= FLUSH_MIN_INTERVAL
                    {
                        let chunk = std::mem::take(&mut pending_reasoning);
                        last_reasoning_flush = Instant::now();
                        emit_stream_event(on_event, abort_tx, StreamEvent::Delta(chunk))?;
                        reasoning_delta_emitted = true;
                    }
                }
            }
            // 处理工具调用
            llm::StreamEvent::ToolCalls(calls) => {
                tool_calls = calls;
            }
            // ACP 工具已在远端执行；只转成类型化展示事件，不能混入最终答复或再次执行。
            llm::StreamEvent::RemoteToolUpdate { call_id, name, is_initial, payload } => {
                if !pending.is_empty() {
                    let chunk = std::mem::take(&mut pending);
                    emit_stream_event(on_event, abort_tx, StreamEvent::Delta(chunk))?;
                }
                let status = match payload.get("status").and_then(Value::as_str) {
                    Some("running") => ToolCallStatus::Running,
                    Some("waiting") => ToolCallStatus::Waiting,
                    Some("completed") => ToolCallStatus::Completed,
                    Some("error" | "failed") => ToolCallStatus::Failed,
                    Some("denied") => ToolCallStatus::Denied,
                    Some("cancelled") => ToolCallStatus::Cancelled,
                    Some(other) => ToolCallStatus::Other(other.to_string()),
                    None => ToolCallStatus::Other("unknown".to_string()),
                };
                let kind = match (&status, is_initial) {
                    (_, true) => ToolEventKind::Start,
                    (ToolCallStatus::Running | ToolCallStatus::Waiting, false) => {
                        ToolEventKind::Update
                    }
                    _ => ToolEventKind::Complete,
                };
                emit_stream_event(
                    on_event,
                    abort_tx,
                    StreamEvent::Tool(ToolEvent {
                        call_id,
                        name,
                        kind,
                        status,
                        occurred_ms: now_ms(),
                        payload,
                    }),
                )?;
            }
            // 处理完整消息历史
            llm::StreamEvent::FullMessages(msgs) => {
                full_messages = msgs;
            }
            // 处理完成事件
            llm::StreamEvent::Done { usage: u, finish_reason: f } => {
                usage = u;
                finish_reason = f;
                break;
            }
            // 处理错误事件
            llm::StreamEvent::Error(mut e) => {
                // 刷新所有待输出内容
                if !pending.is_empty() {
                    let chunk = std::mem::take(&mut pending);
                    emit_stream_event(on_event, abort_tx, StreamEvent::Delta(chunk))?;
                }
                if !pending_reasoning.is_empty() {
                    let chunk = std::mem::take(&mut pending_reasoning);
                    emit_stream_event(on_event, abort_tx, StreamEvent::Delta(chunk))?;
                }
                if thinking_open {
                    emit_stream_event(
                        on_event,
                        abort_tx,
                        StreamEvent::Delta("</think>\n\n".to_string()),
                    )?;
                }
                // 工具调用已经进入响应，禁止重新生成这一调用。
                if !tool_calls.is_empty() {
                    llm::prevent_error_replay(&mut e);
                }
                // 将错误转换为字符串并返回
                return Err(
                    serde_json::to_string(&e).unwrap_or_else(|_| "模型调用失败".to_string())
                );
            }
        }
    }

    // === 流结束后的收尾处理 ===

    // 刷新剩余的待输出普通文本
    if !pending.is_empty() {
        emit_stream_event(on_event, abort_tx, StreamEvent::Delta(pending))?;
    }

    // 刷新剩余的待输出推理文本
    if !pending_reasoning.is_empty() {
        emit_stream_event(on_event, abort_tx, StreamEvent::Delta(pending_reasoning))?;
    }

    // 关闭仍打开的 thinking 标签
    if thinking_open {
        emit_stream_event(on_event, abort_tx, StreamEvent::Delta("</think>\n\n".to_string()))?;
    }

    // 如果推理内容未在流中输出，则在此处统一输出
    // 这种情况发生在推理内容在普通内容开始后才到达
    if !thinking_emitted && !reasoning_content.trim().is_empty() {
        emit_stream_event(
            on_event,
            abort_tx,
            StreamEvent::Delta(format!("\n\n<think>{}</think>\n\n", reasoning_content.trim_end())),
        )?;
    }

    // 推理内容只是中间过程，不能作为最终答复。否则模型仅输出“接下来继续……”
    // 之类的思考后以 stop 结束时，会被上层误判为任务已经完成。
    if text.trim().is_empty() && tool_calls.is_empty() {
        return Err("模型未返回内容".to_string());
    }

    // 构造并返回完整的 LLM 步骤结果
    Ok(LlmStep { usage, finish_reason, reasoning_content, text, tool_calls, full_messages })
}

/// 执行单次 LLM 调用并处理流式响应
///
/// 此函数是 LLM 交互的核心入口，负责：
/// 1. 准备工具规范并过滤允许的工具
/// 2. 在独立线程中启动 LLM 流式调用
/// 3. 接收并处理流式事件
/// 4. 管理内容的缓冲和刷新策略
/// 5. 处理推理内容和普通内容的分离
/// 6. 收集并返回完整的调用结果
///
/// # 参数
///
/// * `messages` - 对话消息历史，包含用户和助手的历史交互
/// * `system` - 系统提示词列表，用于设定模型行为
/// * `model` - 指定使用的模型名称（可选，使用默认模型）
/// * `allowed_tools` - 允许调用的工具 ID 集合
/// * `on_event` - 流式事件回调函数，返回 `false` 可中断处理
///
/// # 返回值
///
/// 成功时返回 [`LlmStep`] 包含完整的调用结果，失败时返回错误信息字符串。
///
/// # 错误情况
///
/// - 模型响应超时（默认 90 秒无响应）
/// - 模型调用失败（网络错误、API 错误等）
/// - 模型未返回任何内容（文本和工具调用都为空）
///
/// # 流式处理策略
///
/// 采用双重缓冲机制：
/// - **时间维度**：最小刷新间隔 33ms，避免事件过于频繁
/// - **内容维度**：最小刷新字符数 1024，减少小包传输
///
/// # 推理内容处理
///
/// 当模型返回推理内容时，会自动：
/// 1. 在推理内容前添加 `<think>` 开始标签
/// 2. 流式输出推理过程
/// 3. 在开始返回普通内容时添加 `</think>` 结束标签
/// 4. 如果推理内容未流式输出，则在最后统一输出
///
/// UI 侧同时兼容部分 API 返回的 `<thinking>...</thinking>` 别名标签。
///
/// # 示例
///
/// ```ignore
/// let messages = vec![json!({"role": "user", "content": "你好"})];
/// let system = vec!["你是一个有帮助的助手".to_string()];
/// let tools = std::collections::HashSet::from(["search"]);
///
/// let result = run_llm_step(
///     &messages,
///     &system,
///     Some("gpt-4".to_string()),
///     &tools,
///     &mut |event| {
///         if let StreamEvent::Delta(text) = event {
///             print!("{}", text);
///         }
///         true
///     },
/// )?;
/// ```
pub(crate) fn run_llm_step(
    session_id: &str,
    messages: &[serde_json::Value],
    system: &[String],
    model: Option<String>,
    options: &serde_json::Value,
    available_tools: &[ToolSpec],
    allowed_tools: &std::collections::HashSet<String>,
    on_event: &mut dyn FnMut(StreamEvent) -> bool,
    cancellation: Option<&tokio_util::sync::CancellationToken>,
) -> Result<LlmStep, String> {
    // 创建用于跨线程通信的通道
    let (tx, rx) = std::sync::mpsc::channel::<llm::StreamEvent>();
    let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);

    // 克隆数据以便在独立线程中使用
    let session_id = session_id.to_string();
    let messages = messages.to_vec();
    let system = system.to_vec();
    let idle_timeout = llm_step_idle_timeout(options);
    let options = options.clone();
    let available_tools = available_tools.to_vec();

    // 获取工具规范并根据允许列表过滤
    let tools = available_tools
        .into_iter()
        .filter(|s| allowed_tools.contains(&s.id)) // 仅保留允许的工具
        .map(|s| (s.id.clone(), s))
        .collect::<std::collections::HashMap<_, _>>();

    let stream_task =
        spawn_llm_stream_thread(session_id, messages, system, model, options, tools, abort_rx, tx);

    let result = collect_llm_step_from_stream(rx, on_event, &abort_tx, idle_timeout, cancellation);
    if result.is_err() {
        request_stream_abort(&abort_tx);
    }
    stream_task.join().map_err(|_| "model stream worker crashed".to_string())?;
    result
}

/// 执行带重试机制的 LLM 调用
///
/// 此函数在 [`run_llm_step`] 的基础上添加了自动重试机制，当 LLM 调用失败时会自动重试，
/// 每次重试之间采用指数退避策略，以避免对服务器造成过大压力。
///
/// # 参数
///
/// * `messages` - 对话消息历史，包含用户和助手的历史交互
/// * `system` - 系统提示词列表，用于设定模型行为
/// * `model` - 指定使用的模型名称（可选，使用默认模型）
/// * `allowed_tools` - 允许调用的工具 ID 集合
/// * `on_event` - 流式事件回调函数，返回 `false` 可中断处理
/// * `additional_retries` - 首次调用失败后的最大额外重试次数
/// * `initial_backoff_ms` - 首次重试前的等待时间（毫秒）
///
/// # 返回值
///
/// 成功时返回 [`LlmStep`] 包含完整的调用结果，失败时返回最后一次的错误信息字符串。
///
/// # 重试策略
///
/// - 采用指数退避策略：首次重试使用配置的初始等待，之后每次翻倍
/// - 最大退避时间：5000ms（5 秒）
/// - 退避公式：`min(5000, initial_backoff_ms * 2^retry_index)`
///
/// # 示例
///
/// ```ignore
/// let messages = vec![json!({"role": "user", "content": "你好"})];
/// let system = vec!["你是一个有帮助的助手".to_string()];
/// let tools = std::collections::HashSet::from(["search"]);
///
/// // 首次调用失败后最多额外重试 2 次
/// let result = run_llm_step_with_retry(
///     &messages,
///     &system,
///     Some("gpt-4".to_string()),
///     &tools,
///     &mut |event| {
///         if let StreamEvent::Delta(text) = event {
///             print!("{}", text);
///         }
///         true
///     },
///     2,
///     500,
/// )?;
/// ```
///
/// # 注意事项
///
/// - 当 `additional_retries` 为 0 时仍会执行首次调用
/// - 已输出内容或工具事件后停止自动重放，保留消费者收到的内容
/// - 退避期间会阻塞当前线程
pub(crate) fn run_llm_step_with_retry(
    session_id: &str,
    messages: &[serde_json::Value],
    system: &[String],
    model: Option<String>,
    options: &serde_json::Value,
    available_tools: &[ToolSpec],
    allowed_tools: &std::collections::HashSet<String>,
    on_event: &mut dyn FnMut(StreamEvent) -> bool,
    additional_retries: u32,
    initial_backoff_ms: u64,
    cancellation: Option<&tokio_util::sync::CancellationToken>,
) -> Result<LlmStep, String> {
    let mut budget = super::recovery::RetryBudget::new(additional_retries, initial_backoff_ms);
    run_llm_step_with_budget(
        session_id,
        messages,
        system,
        model,
        options,
        available_tools,
        allowed_tools,
        on_event,
        &mut budget,
        cancellation,
    )
}

pub(crate) fn run_llm_step_with_budget(
    session_id: &str,
    messages: &[Value],
    system: &[String],
    model: Option<String>,
    options: &Value,
    available_tools: &[ToolSpec],
    allowed_tools: &std::collections::HashSet<String>,
    on_event: &mut dyn FnMut(StreamEvent) -> bool,
    budget: &mut super::recovery::RetryBudget,
    cancellation: Option<&tokio_util::sync::CancellationToken>,
) -> Result<LlmStep, String> {
    budget.run(is_acp_request(options), cancellation, on_event, |sink| {
        run_llm_step(
            session_id,
            messages,
            system,
            model.clone(),
            options,
            available_tools,
            allowed_tools,
            sink,
            cancellation,
        )
    })
}

const MAX_PROVIDER_RETRY_BACKOFF_MS: u64 = 5_000;

pub(crate) fn provider_retry_backoff_ms(initial_backoff_ms: u64, retry_index: u32) -> u64 {
    let multiplier = 1u64.checked_shl(retry_index).unwrap_or(u64::MAX);
    initial_backoff_ms.saturating_mul(multiplier).min(MAX_PROVIDER_RETRY_BACKOFF_MS)
}
#[cfg(test)]
#[path = "llm_runner_tests.rs"]
mod llm_runner_tests;

#[cfg(test)]
pub(crate) use llm_runner_tests::{
    ChannelCancelStream, enqueue_test_llm_stream, register_channel_cancel_stream,
    take_test_llm_messages, take_test_llm_stream_tool_ids, test_llm_stream_guard,
};
