//! CLI legacy processor 入口与迁移对照桥。
//!
//! 当前模块同时承担两类职责：
//! 1. 保留 legacy `session::processor` 的 CLI 包装入口，维持现有交互链路不变。
//! 2. 为 TUI v2 的 gateway-first 迁移补上一层“同请求、同结果结构”的对照桥，
//!    让后续 slice 可以在不改调用方输入模型的前提下，对比 legacy processor
//!    与 `GatewayUiRuntime` 两条链路的终态与输出。
//!
//! 这里刻意只桥接当前已稳定的最小公共面：
//! - 输入沿用 legacy processor 的 `Request`
//! - 输出统一为 delta 聚合文本、usage、step finish 计数与终态枚举
//! - 不在此处追加 shadow 调度、开关配置或 UI 状态逻辑

use std::path::Path;

use crate::app::agent::session::processor as legacy_processor;
use crate::session::ui_types as models;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use vw_gateway_client::vw_api_types::id::SessionId;
use vw_gateway_client::{GatewayChatStreamRequest, GatewayChatUsage};

use super::tui::runtime::gateway::{GatewayUiRuntime, normalize_optional_str_ref};
use super::tui::runtime::stream_adapter::{UiRuntimeEvent, UiRuntimeTerminalEvent};

/// legacy CLI 链路当前暴露给交互层的成功结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionProcessorCliResult {
    pub(crate) output: String,
    pub(crate) usage: models::TokenUsage,
    pub(crate) step_finishes: usize,
}

/// 迁移对照桥统一使用的终态枚举。
///
/// legacy processor 与 gateway runtime 的流式终态并不完全一致：
/// - legacy 只有 done/error
/// - gateway runtime 还会显式区分 cancelled/timeout
///
/// 对照桥在这里先把两边都收口为统一终态，方便后续 shadow compare
/// 或 cutover 前的行为比对复用同一份结构。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionProcessorComparableTerminal {
    Done {
        finish_reason: Option<String>,
        message_id: Option<String>,
        parent_message_id: Option<String>,
    },
    Cancelled {
        reason: Option<String>,
        message_id: Option<String>,
        parent_message_id: Option<String>,
    },
    TimedOut {
        message: String,
        message_id: Option<String>,
        parent_message_id: Option<String>,
    },
    Error(String),
}

/// legacy/gateway 两条链路共享的可比对结果结构。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionProcessorComparableResult {
    pub(crate) output: String,
    pub(crate) usage: models::TokenUsage,
    pub(crate) step_finishes: usize,
    pub(crate) terminal: SessionProcessorComparableTerminal,
}

impl SessionProcessorComparableResult {
    /// 将统一终态结果压回 legacy CLI 现有的成功/失败语义。
    ///
    /// 这样旧调用点仍可保持 `Result<SessionProcessorCliResult>` 不变，
    /// 而新对照桥入口则可以继续保留更细粒度的终态信息。
    pub(crate) fn into_cli_result(self) -> anyhow::Result<SessionProcessorCliResult> {
        let SessionProcessorComparableResult { output, usage, step_finishes, terminal } = self;

        match terminal {
            SessionProcessorComparableTerminal::Done { .. } => {
                Ok(SessionProcessorCliResult { output, usage, step_finishes })
            }
            SessionProcessorComparableTerminal::Cancelled { reason, .. } => Err(anyhow::anyhow!(
                reason.unwrap_or_else(|| "session processor cancelled".to_string())
            )),
            SessionProcessorComparableTerminal::TimedOut { message, .. } => {
                Err(anyhow::anyhow!(message))
            }
            SessionProcessorComparableTerminal::Error(message) => Err(anyhow::anyhow!(message)),
        }
    }
}

/// 运行 legacy processor，并返回可用于对照桥的统一结果结构。
pub(crate) async fn run_session_processor_comparable_for_cli(
    req: legacy_processor::Request,
    delta_tx: Option<mpsc::Sender<String>>,
) -> anyhow::Result<SessionProcessorComparableResult> {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<legacy_processor::StreamEvent>();

    tokio::task::spawn_blocking(move || {
        legacy_processor::run(req, move |ev| event_tx.send(ev).is_ok());
    });

    let mut output = String::new();
    let mut step_finishes = 0usize;

    while let Some(ev) = event_rx.recv().await {
        match ev {
            legacy_processor::StreamEvent::Delta(delta) => {
                output.push_str(&delta);
                if let Some(tx) = delta_tx.as_ref() {
                    let _ = tx.send(delta).await;
                }
            }
            legacy_processor::StreamEvent::Tool(event) => {
                let delta = format!("tool {}\n{}\n", event.name, event.payload);
                output.push_str(&delta);
                if let Some(tx) = delta_tx.as_ref() {
                    let _ = tx.send(delta).await;
                }
            }
            legacy_processor::StreamEvent::Done(done_usage) => {
                return Ok(SessionProcessorComparableResult {
                    output,
                    usage: done_usage,
                    step_finishes,
                    terminal: SessionProcessorComparableTerminal::Done {
                        finish_reason: None,
                        message_id: None,
                        parent_message_id: None,
                    },
                });
            }
            legacy_processor::StreamEvent::Error(err) => {
                return Ok(SessionProcessorComparableResult {
                    output,
                    usage: models::TokenUsage::default(),
                    step_finishes,
                    terminal: SessionProcessorComparableTerminal::Error(err),
                });
            }
            legacy_processor::StreamEvent::StepStart { .. }
            | legacy_processor::StreamEvent::PostToolRound { .. }
            | legacy_processor::StreamEvent::Compaction { .. }
            | legacy_processor::StreamEvent::ModelRecovery(_)
            | legacy_processor::StreamEvent::LlmRawArtifact(_)
            | legacy_processor::StreamEvent::ContextLoaded { .. } => {}
            legacy_processor::StreamEvent::StepFinish { .. } => {
                step_finishes = step_finishes.saturating_add(1);
            }
        }
    }

    anyhow::bail!("session processor exited without terminal event")
}

/// 保留 legacy CLI 现有的 processor 包装入口。
pub(crate) async fn run_session_processor_for_cli(
    req: legacy_processor::Request,
    delta_tx: Option<mpsc::Sender<String>>,
) -> anyhow::Result<SessionProcessorCliResult> {
    run_session_processor_comparable_for_cli(req, delta_tx).await?.into_cli_result()
}

#[cfg_attr(not(test), allow(dead_code))]
/// 通过 `GatewayUiRuntime` 执行一次与 legacy processor 可对照的流式请求。
///
/// 该入口的目标不是立即替换当前 CLI 主流程，而是为后续 shadow compare
/// 与逐步 cutover 留出一条稳定、可复用的桥接入口。
pub(crate) async fn run_gateway_runtime_for_cli(
    runtime: &GatewayUiRuntime,
    req: legacy_processor::Request,
    delta_tx: Option<mpsc::Sender<String>>,
) -> anyhow::Result<SessionProcessorComparableResult> {
    ensure_runtime_directory_matches_request(runtime, &req)?;

    let body = gateway_stream_request_from_processor_request(&req);
    let (bridge_tx, forward_task) = start_gateway_delta_forwarder(delta_tx);
    let mut output = String::new();
    let mut step_finishes = 0usize;

    let terminal = runtime
        .stream_chat(&body, |event| {
            match event {
                UiRuntimeEvent::Delta(delta) => {
                    output.push_str(&delta);
                    if let Some(tx) = bridge_tx.as_ref() {
                        let _ = tx.send(delta);
                    }
                }
                UiRuntimeEvent::StepStart { .. }
                | UiRuntimeEvent::PlanRunTaskStarted { .. }
                | UiRuntimeEvent::ContextLoaded { .. }
                | UiRuntimeEvent::Message(_)
                | UiRuntimeEvent::TaskStateChanged { .. }
                | UiRuntimeEvent::SessionMetadataChanged { .. }
                | UiRuntimeEvent::UsageUpdated { .. }
                | UiRuntimeEvent::PostToolRound
                | UiRuntimeEvent::Terminal(_)
                | UiRuntimeEvent::Unknown { .. } => {}
                UiRuntimeEvent::StepFinish { .. } => {
                    step_finishes = step_finishes.saturating_add(1);
                }
            }
            true
        })
        .await;

    drop(bridge_tx);
    if let Some(task) = forward_task {
        task.await.map_err(|err| anyhow::anyhow!("gateway delta forwarder failed: {err}"))?;
    }

    Ok(SessionProcessorComparableResult {
        output,
        usage: token_usage_from_terminal(&terminal),
        step_finishes,
        terminal: comparable_terminal_from_runtime_terminal(terminal),
    })
}

/// 将 legacy processor 请求转换为 gateway chat stream 请求体。
///
/// 当前桥接只覆盖两条链路的稳定交集：history、query、session、model 与 options。
/// `stream` 和 `persist_app_session_artifacts` 仍保留在 legacy 侧语义中，
/// 不在 gateway 请求体中做伪映射。
pub(crate) fn gateway_stream_request_from_processor_request(
    req: &legacy_processor::Request,
) -> GatewayChatStreamRequest {
    let mut messages = Vec::with_capacity(req.history.len().saturating_add(1));
    for message in &req.history {
        messages.push(gateway_message_from_history(message));
    }
    messages.push(gateway_message_value("user", req.query.as_str()));

    GatewayChatStreamRequest {
        session_id: normalize_optional_str_ref(Some(req.session.as_str())).map(SessionId::from),
        messages,
        system: None,
        model: req.model.clone(),
        reasoning_effort: None,
        agent: None,
        allowed_tools: None,
        acp_agent: None,
        acp_allowed_tools: None,
        options: gateway_request_options(&req.options),
    }
}

/// 要求 runtime 目录上下文与 legacy request root 保持一致，避免静默扩大对照范围。
fn ensure_runtime_directory_matches_request(
    runtime: &GatewayUiRuntime,
    req: &legacy_processor::Request,
) -> anyhow::Result<()> {
    let Some(root) = req.root.as_deref().and_then(|value| normalize_optional_str_ref(Some(value)))
    else {
        return Ok(());
    };

    let request_root = Path::new(root);
    if runtime.directory() != request_root {
        anyhow::bail!(
            "gateway runtime directory does not match legacy processor request root: runtime={}, request={}",
            runtime.directory().display(),
            request_root.display()
        );
    }

    Ok(())
}

/// 为 gateway runtime 的同步回调建立一个异步 delta 转发桥。
fn start_gateway_delta_forwarder(
    delta_tx: Option<mpsc::Sender<String>>,
) -> (Option<mpsc::UnboundedSender<String>>, Option<JoinHandle<()>>) {
    let Some(delta_tx) = delta_tx else {
        return (None, None);
    };

    let (bridge_tx, mut bridge_rx) = mpsc::unbounded_channel::<String>();
    let task = tokio::spawn(async move {
        while let Some(delta) = bridge_rx.recv().await {
            if delta_tx.send(delta).await.is_err() {
                break;
            }
        }
    });

    (Some(bridge_tx), Some(task))
}

/// 将 gateway terminal 中可能附带的 usage 规整为 CLI 共享结构。
fn token_usage_from_terminal(terminal: &UiRuntimeTerminalEvent) -> models::TokenUsage {
    let usage = match terminal {
        UiRuntimeTerminalEvent::Done { usage, .. }
        | UiRuntimeTerminalEvent::Cancelled { usage, .. }
        | UiRuntimeTerminalEvent::TimedOut { usage, .. } => usage.as_ref(),
        UiRuntimeTerminalEvent::Error(_) => None,
    };

    usage.map(token_usage_from_gateway_usage).unwrap_or_default()
}

/// 将 runtime terminal 转为对照桥统一终态。
fn comparable_terminal_from_runtime_terminal(
    terminal: UiRuntimeTerminalEvent,
) -> SessionProcessorComparableTerminal {
    match terminal {
        UiRuntimeTerminalEvent::Done { finish_reason, message_id, parent_message_id, .. } => {
            SessionProcessorComparableTerminal::Done {
                finish_reason,
                message_id,
                parent_message_id,
            }
        }
        UiRuntimeTerminalEvent::Cancelled { reason, message_id, parent_message_id, .. } => {
            SessionProcessorComparableTerminal::Cancelled { reason, message_id, parent_message_id }
        }
        UiRuntimeTerminalEvent::TimedOut { message, message_id, parent_message_id, .. } => {
            SessionProcessorComparableTerminal::TimedOut { message, message_id, parent_message_id }
        }
        UiRuntimeTerminalEvent::Error(message) => {
            SessionProcessorComparableTerminal::Error(message)
        }
    }
}

/// 将 gateway usage 结构转换为 CLI/legacy 共用的 token 统计结构。
fn token_usage_from_gateway_usage(usage: &GatewayChatUsage) -> models::TokenUsage {
    models::TokenUsage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cached_tokens: usage.cached_tokens,
        reasoning_tokens: usage.reasoning_tokens,
    }
}

/// 将 legacy history message 转换为 gateway 兼容消息对象。
fn gateway_message_from_history(message: &models::ChatMessage) -> Value {
    gateway_message_value(gateway_role(message.role), message.content.as_str())
}

/// 构造单条 gateway 兼容消息对象。
fn gateway_message_value(role: &str, content: &str) -> Value {
    serde_json::json!({
        "role": role,
        "content": content,
    })
}

/// 将 legacy UI role 映射为 gateway 消息 role 字符串。
fn gateway_role(role: models::ChatRole) -> &'static str {
    match role {
        models::ChatRole::User => "user",
        models::ChatRole::Assistant => "assistant",
        models::ChatRole::System => "system",
        models::ChatRole::Tool => "tool",
    }
}

/// 仅在 legacy request options 不是 null 时透传给 gateway。
fn gateway_request_options(options: &Value) -> Option<Value> {
    if options.is_null() { None } else { Some(options.clone()) }
}

#[cfg(test)]
#[path = "processor_tests.rs"]
mod tests;
