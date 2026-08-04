//! gateway stream 到 runtime 事件的适配层。
//!
//! 本模块负责两类事情：
//! - 将 gateway client 暴露的 typed stream 事件转换为 CLI 内部事件
//! - 统一终态控制语义，把 done/error/cancel/timeout 收口为稳定枚举

use vw_gateway_client::{
    GatewayChatStreamEvent, GatewayChatUsage, GatewayTypedChatStreamEvent,
    normalize_chat_stream_event,
};
use vw_gateway_client::vw_api_types::task::TaskPlanRunStreamEventDto;
use serde_json::Value;

use crate::cli::tui_v2::model::{
    UiAttachment, UiAttachmentKind, UiCompactBoundary, UiMessage, UiMessageBase, UiMessageId,
    UiMessageOrigin, UiProgressMessage, UiUnknownMessage,
};

/// CLI 新 TUI 在 runtime 边界统一消费的终态事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UiRuntimeTerminalEvent {
    /// 当前 chat stream 正常结束。
    Done {
        finish_reason: Option<String>,
        usage: Option<GatewayChatUsage>,
        message_id: Option<String>,
        parent_message_id: Option<String>,
    },
    /// 当前 chat stream 被主动取消或被中断终止。
    Cancelled {
        reason: Option<String>,
        usage: Option<GatewayChatUsage>,
        message_id: Option<String>,
        parent_message_id: Option<String>,
    },
    /// 当前 chat stream 以超时结束。
    TimedOut {
        message: String,
        usage: Option<GatewayChatUsage>,
        message_id: Option<String>,
        parent_message_id: Option<String>,
    },
    /// 当前 chat stream 以错误结束。
    Error(String),
}

impl UiRuntimeTerminalEvent {
    /// 基于错误文本归类统一终态语义。
    pub(crate) fn from_error_message(error: String) -> Self {
        let message = normalize_optional_string(Some(error))
            .unwrap_or_else(|| "gateway stream failed".to_string());

        match classify_terminal_marker(Some(&message)) {
            TerminalMarker::Done => Self::Error(message),
            TerminalMarker::Cancelled => Self::Cancelled {
                reason: Some(message),
                usage: None,
                message_id: None,
                parent_message_id: None,
            },
            TerminalMarker::TimedOut => {
                Self::TimedOut { message, usage: None, message_id: None, parent_message_id: None }
            }
        }
    }
}

/// CLI 新 TUI 在 runtime 边界统一消费的事件枚举。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UiRuntimeEvent {
    /// 后台计划运行已分配任务 ID，可用于停止控制面。
    PlanRunTaskStarted { task_id: String },
    /// 助手文本增量。
    Delta(String),
    /// 本轮实际加载的上下文指令文件。
    ContextLoaded { paths: Vec<String> },
    /// gateway 已标准化的非文本内容块。
    Message(UiMessage),
    /// 一个新的推理步骤开始执行。
    StepStart { step_index: u32, created_ms: u64, model: Option<String> },
    /// 一个推理步骤执行完成。
    StepFinish {
        step_index: u32,
        finished_ms: u64,
        usage: GatewayChatUsage,
        finish_reason: Option<String>,
        model: Option<String>,
    },
    /// 当前工具轮次已完成，下一次模型请求尚未开始。
    PostToolRound,
    /// 当前 chat stream 的终态控制事件。
    Terminal(UiRuntimeTerminalEvent),
    /// 会话任务侧数据已变化，需从 runtime 重新同步 question/todo 状态。
    TaskStateChanged { session_id: Option<String> },
    /// 会话元数据已变化，需刷新标题、scope、path 或 preview。
    SessionMetadataChanged { session_id: Option<String>, title: Option<String> },
    /// 用量快照已变化；当前先显式识别，避免误记为 unknown warning。
    UsageUpdated { session_id: Option<String>, usage: GatewayChatUsage },
    /// 当前 runtime 尚未识别的事件类型。
    Unknown { event_type: Option<String> },
}

/// 将计划运行事件投影到既有 TUI 消息与终态管线。
pub(crate) fn adapt_task_plan_run_event(
    event: TaskPlanRunStreamEventDto,
) -> Vec<UiRuntimeEvent> {
    match event {
        TaskPlanRunStreamEventDto::PlanningStarted {
            task_id,
            plan_dir,
            session_id,
        } => vec![
            UiRuntimeEvent::PlanRunTaskStarted { task_id: task_id.clone() },
            plan_run_progress(
                &task_id,
                "planning",
                format!("正在规划：{plan_dir}"),
                None,
                None,
                false,
                Some(session_id),
            ),
        ],
        TaskPlanRunStreamEventDto::SessionDelta { delta, .. } => {
            vec![UiRuntimeEvent::Delta(delta)]
        }
        TaskPlanRunStreamEventDto::SessionWaiting {
            task_id,
            session_id,
            active_descendant_session_ids,
        } => vec![plan_run_progress(
            &task_id,
            "waiting",
            format!(
                "等待 {} 个子智能体完成",
                active_descendant_session_ids.len()
            ),
            None,
            None,
            false,
            Some(session_id),
        )],
        TaskPlanRunStreamEventDto::PlanCreated {
            task_id,
            subtask_count,
            session_id,
            ..
        } => vec![plan_run_progress(
            &task_id,
            "plan-created",
            format!("计划已创建，共 {subtask_count} 个子任务"),
            Some(subtask_count as u64),
            Some(subtask_count as u64),
            true,
            Some(session_id),
        )],
        TaskPlanRunStreamEventDto::BatchStarted {
            task_id,
            batch,
            subtask_ids,
        } => vec![plan_run_progress(
            &task_id,
            &format!("batch-{batch}-started"),
            format!("批次 {batch} 开始，共 {} 个子任务", subtask_ids.len()),
            Some(0),
            Some(subtask_ids.len() as u64),
            false,
            None,
        )],
        TaskPlanRunStreamEventDto::BatchFinished {
            task_id,
            batch,
            subtask_ids,
            success,
        } => vec![plan_run_progress(
            &task_id,
            &format!("batch-{batch}-finished"),
            format!(
                "批次 {batch} {}",
                if success { "已完成" } else { "执行失败" }
            ),
            Some(subtask_ids.len() as u64),
            Some(subtask_ids.len() as u64),
            true,
            None,
        )],
        TaskPlanRunStreamEventDto::GoalTurnStarted {
            task_id,
            turn,
            session_id,
            reason,
        } => vec![plan_run_progress(
            &task_id,
            &format!("goal-turn-{turn}"),
            format!("目标续轮 {turn}：{reason}"),
            Some(u64::from(turn)),
            None,
            false,
            Some(session_id),
        )],
        TaskPlanRunStreamEventDto::GoalEvaluated {
            task_id,
            turn,
            achieved,
            reason,
            session_id,
        } => vec![plan_run_progress(
            &task_id,
            &format!("goal-evaluated-{turn}"),
            format!(
                "目标评估 {turn}：{}（{reason}）",
                if achieved { "已达成" } else { "未达成" }
            ),
            Some(u64::from(turn)),
            None,
            achieved,
            Some(session_id),
        )],
        TaskPlanRunStreamEventDto::SubtaskStarted {
            task_id,
            batch,
            subtask_id,
            title,
            session_id,
            ..
        } => vec![plan_run_progress(
            &task_id,
            &format!("subtask-{subtask_id}-started"),
            format!("批次 {batch} · {title}"),
            None,
            None,
            false,
            Some(session_id),
        )],
        TaskPlanRunStreamEventDto::SubtaskFinished {
            task_id,
            subtask_id,
            title,
            success,
            output,
            error,
            session_id,
            ..
        } => {
            let mut events = vec![plan_run_progress(
                &task_id,
                &format!("subtask-{subtask_id}-finished"),
                format!(
                    "{}：{title}",
                    if success { "子任务完成" } else { "子任务失败" }
                ),
                None,
                None,
                true,
                Some(session_id),
            )];
            if let Some(text) = output.or(error).filter(|text| !text.trim().is_empty()) {
                events.push(UiRuntimeEvent::Delta(format!("{text}\n")));
            }
            events
        }
        TaskPlanRunStreamEventDto::Blocked {
            task_id,
            reason,
            session_id,
            ..
        } => vec![
            plan_run_progress(
                &task_id,
                "blocked",
                format!("计划运行已阻塞：{reason}"),
                None,
                None,
                true,
                Some(session_id),
            ),
            UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Error(format!(
                "计划运行已阻塞：{reason}"
            ))),
        ],
        TaskPlanRunStreamEventDto::Finished {
            task_id,
            status,
            output,
            session_id,
            ..
        } => {
            let mut events = vec![plan_run_progress(
                &task_id,
                "finished",
                format!("计划运行已结束：{status}"),
                None,
                None,
                true,
                session_id,
            )];
            if !output.trim().is_empty() {
                events.push(UiRuntimeEvent::Delta(output));
            }
            events.push(UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Done {
                finish_reason: Some(status),
                usage: None,
                message_id: None,
                parent_message_id: None,
            }));
            events
        }
        TaskPlanRunStreamEventDto::Error { message, .. } => {
            vec![UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Error(message))]
        }
        TaskPlanRunStreamEventDto::Cancelled { .. } => {
            vec![UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Cancelled {
                reason: Some("计划运行已取消".to_string()),
                usage: None,
                message_id: None,
                parent_message_id: None,
            })]
        }
    }
}

fn plan_run_progress(
    task_id: &str,
    phase: &str,
    label: String,
    current: Option<u64>,
    total: Option<u64>,
    completed: bool,
    session_id: Option<String>,
) -> UiRuntimeEvent {
    let mut base =
        UiMessageBase::new(UiMessageId::local(format!("plan-run-{task_id}-{phase}")))
            .with_origin(UiMessageOrigin::Gateway {
                event_type: format!("task_plan_run.{phase}"),
            });
    if let Some(session_id) = session_id {
        base = base.with_session_id(session_id);
    }
    UiRuntimeEvent::Message(UiMessage::Progress(UiProgressMessage {
        base,
        label,
        current,
        total,
        completed,
    }))
}

/// 将 gateway 原始流事件规整为 CLI 内部事件。
pub(crate) fn adapt_gateway_stream_event(event: GatewayChatStreamEvent) -> UiRuntimeEvent {
    if let GatewayChatStreamEvent::Other(value) = &event
        && value.get("type").and_then(Value::as_str) == Some("chat.content")
    {
        return adapt_gateway_content_event(value);
    }

    match normalize_chat_stream_event(event) {
        GatewayTypedChatStreamEvent::Delta(delta) => UiRuntimeEvent::Delta(delta),
        GatewayTypedChatStreamEvent::ContextLoaded { paths } => {
            UiRuntimeEvent::ContextLoaded { paths }
        }
        GatewayTypedChatStreamEvent::StepStart(event) => UiRuntimeEvent::StepStart {
            step_index: event.step_index,
            created_ms: event.created_ms,
            model: event.model,
        },
        GatewayTypedChatStreamEvent::StepFinish(event) => UiRuntimeEvent::StepFinish {
            step_index: event.step_index,
            finished_ms: event.finished_ms,
            usage: event.usage,
            finish_reason: event.finish_reason,
            model: event.model,
        },
        GatewayTypedChatStreamEvent::PostToolRound(_) => UiRuntimeEvent::PostToolRound,
        GatewayTypedChatStreamEvent::Done {
            finish_reason,
            usage,
            message_id,
            parent_message_id,
        } => UiRuntimeEvent::Terminal(terminal_from_done(
            finish_reason,
            usage,
            message_id,
            parent_message_id,
        )),
        GatewayTypedChatStreamEvent::Error(error) => {
            UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::from_error_message(error))
        }
        GatewayTypedChatStreamEvent::TodoUpdated { session_id }
        | GatewayTypedChatStreamEvent::QuestionRaised { session_id }
        | GatewayTypedChatStreamEvent::QuestionResolved { session_id } => {
            UiRuntimeEvent::TaskStateChanged { session_id }
        }
        GatewayTypedChatStreamEvent::TitleUpdated { session_id, title } => {
            UiRuntimeEvent::SessionMetadataChanged {
                session_id,
                title: normalize_optional_string(Some(title)),
            }
        }
        GatewayTypedChatStreamEvent::SessionUpdated { session_id, title } => {
            UiRuntimeEvent::SessionMetadataChanged {
                session_id,
                title: normalize_optional_string(title),
            }
        }
        GatewayTypedChatStreamEvent::UsageUpdated { session_id, usage } => {
            UiRuntimeEvent::UsageUpdated { session_id, usage }
        }
        GatewayTypedChatStreamEvent::Unknown { event_type } => {
            UiRuntimeEvent::Unknown { event_type }
        }
    }
}

fn adapt_gateway_content_event(value: &Value) -> UiRuntimeEvent {
    let content = value.get("content").and_then(Value::as_object);
    let external_kind =
        content.and_then(|content| content.get("type")).and_then(Value::as_str).unwrap_or("unknown");
    let mut base = content_event_base(value, external_kind);

    if let Some(origin) = value.get("origin").and_then(Value::as_object)
        && origin.get("type").and_then(Value::as_str) == Some("command")
        && let Some(name) = origin.get("name").and_then(Value::as_str).and_then(non_empty)
    {
        base.origin = Some(UiMessageOrigin::Command { name: name.to_string() });
    }

    let message = match external_kind {
        "attachment" => {
            let kind = match content
                .and_then(|content| content.get("kind"))
                .and_then(Value::as_str)
            {
                Some("image") => UiAttachmentKind::Image,
                Some("link") => UiAttachmentKind::Link,
                Some("pasted_text") => UiAttachmentKind::PastedText,
                Some("resource") => UiAttachmentKind::Resource,
                _ => UiAttachmentKind::File,
            };
            UiMessage::Attachment(UiAttachment {
                base,
                kind,
                name: content_string(content, "name").unwrap_or_else(|| "attachment".to_string()),
                media_type: content_string(content, "media_type"),
                uri: content_string(content, "uri").unwrap_or_default(),
                alt_text: content_string(content, "alt_text"),
                width_px: content
                    .and_then(|content| content.get("width_px"))
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok()),
                height_px: content
                    .and_then(|content| content.get("height_px"))
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok()),
                size_bytes: content
                    .and_then(|content| content.get("size_bytes"))
                    .and_then(Value::as_u64),
                source: content_string(content, "source"),
                preview: content_string(content, "preview"),
            })
        }
        "progress" => UiMessage::Progress(UiProgressMessage {
            base,
            label: content_string(content, "label").unwrap_or_else(|| "Progress".to_string()),
            current: content
                .and_then(|content| content.get("current"))
                .and_then(Value::as_u64),
            total: content.and_then(|content| content.get("total")).and_then(Value::as_u64),
            completed: content
                .and_then(|content| content.get("completed"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        "compact_boundary" => UiMessage::CompactBoundary(UiCompactBoundary {
            base,
            summary: content_string(content, "summary")
                .unwrap_or_else(|| "Earlier context compacted".to_string()),
            compacted_messages: content
                .and_then(|content| content.get("compacted_messages"))
                .and_then(Value::as_u64)
                .and_then(|count| usize::try_from(count).ok())
                .unwrap_or_default(),
        }),
        _ => UiMessage::Unknown(UiUnknownMessage {
            base,
            external_kind: external_kind.to_string(),
            summary: format!("Unsupported content block: {external_kind}."),
        }),
    };

    UiRuntimeEvent::Message(message)
}

fn content_event_base(value: &Value, external_kind: &str) -> UiMessageBase {
    let message_id = value.get("message_id").and_then(Value::as_str).and_then(non_empty);
    let mut base = match message_id {
        Some(message_id) => UiMessageBase::new(UiMessageId::gateway(message_id)),
        None => UiMessageBase::new(UiMessageId::local(format!("content-{external_kind}"))),
    }
    .with_origin(UiMessageOrigin::Gateway { event_type: "chat.content".to_string() });

    if let Some(parent_id) =
        value.get("parent_message_id").and_then(Value::as_str).and_then(non_empty)
    {
        base = base.with_parent_id(UiMessageId::gateway(parent_id));
    }
    if let Some(created_ms) = value.get("created_ms").and_then(Value::as_u64) {
        base = base.with_created_ms(created_ms);
    }
    base
}

fn content_string(
    content: Option<&serde_json::Map<String, Value>>,
    key: &str,
) -> Option<String> {
    content
        .and_then(|content| content.get(key))
        .and_then(Value::as_str)
        .and_then(non_empty)
        .map(ToOwned::to_owned)
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

/// 将 `chat.done` 事件规整为统一终态事件。
fn terminal_from_done(
    finish_reason: Option<String>,
    usage: Option<GatewayChatUsage>,
    message_id: Option<String>,
    parent_message_id: Option<String>,
) -> UiRuntimeTerminalEvent {
    let finish_reason = normalize_optional_string(finish_reason);
    match classify_terminal_marker(finish_reason.as_deref()) {
        TerminalMarker::Done => {
            UiRuntimeTerminalEvent::Done { finish_reason, usage, message_id, parent_message_id }
        }
        TerminalMarker::Cancelled => UiRuntimeTerminalEvent::Cancelled {
            reason: finish_reason,
            usage,
            message_id,
            parent_message_id,
        },
        TerminalMarker::TimedOut => UiRuntimeTerminalEvent::TimedOut {
            message: finish_reason
                .clone()
                .unwrap_or_else(|| "gateway stream timed out".to_string()),
            usage,
            message_id,
            parent_message_id,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalMarker {
    Done,
    Cancelled,
    TimedOut,
}

/// 基于 finish_reason 或错误文本，归类统一的终态语义。
fn classify_terminal_marker(value: Option<&str>) -> TerminalMarker {
    let Some(value) = normalize_optional_str_ref(value) else {
        return TerminalMarker::Done;
    };

    let normalized = value.to_ascii_lowercase();
    if contains_terminal_keyword(&normalized, &["timeout", "timed out", "deadline exceeded"]) {
        return TerminalMarker::TimedOut;
    }

    if contains_terminal_keyword(
        &normalized,
        &["cancelled", "canceled", "cancel", "interrupted", "aborted"],
    ) {
        return TerminalMarker::Cancelled;
    }

    TerminalMarker::Done
}

/// 将外部传入的字符串归一化为“空白即无值”。
fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

/// 将可选字符串引用归一化为“空白即无值”。
fn normalize_optional_str_ref(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

/// 检查标准化文本中是否包含任一终态关键字。
pub(crate) fn contains_terminal_keyword(value: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| value.contains(keyword))
}
