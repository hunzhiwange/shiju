//! 将网关事件规整为类型化事件。

use crate::types::*;

/// 将 `chat.tool` JSON 规整为封闭的类型化工具事件。
pub fn normalize_chat_tool_event(payload: &serde_json::Value) -> Option<GatewayChatToolEvent> {
    if payload.get("type").and_then(serde_json::Value::as_str) != Some("chat.tool") {
        return None;
    }

    let step_index = u32::try_from(payload.get("step_index")?.as_u64()?).ok()?;
    let call_id = payload.get("call_id")?.as_str()?.to_string();
    let name = payload.get("name")?.as_str()?.to_string();
    let kind = match payload.get("kind")?.as_str()? {
        "start" => GatewayChatToolEventKind::Start,
        "update" => GatewayChatToolEventKind::Update,
        "complete" => GatewayChatToolEventKind::Complete,
        _ => return None,
    };
    let status = match payload.get("status")?.as_str()? {
        "running" => GatewayChatToolStatus::Running,
        "waiting" => GatewayChatToolStatus::Waiting,
        "completed" => GatewayChatToolStatus::Completed,
        "failed" => GatewayChatToolStatus::Failed,
        "denied" => GatewayChatToolStatus::Denied,
        "cancelled" => GatewayChatToolStatus::Cancelled,
        other => GatewayChatToolStatus::Other(other.to_string()),
    };
    let occurred_ms = payload.get("occurred_ms")?.as_u64()?;
    let event_payload = payload.get("payload")?.clone();

    Some(GatewayChatToolEvent {
        step_index,
        call_id,
        name,
        kind,
        status,
        occurred_ms,
        payload: event_payload,
    })
}

/// 将原始网关流事件规整为 typed 结果。
///
/// 当前只补齐新 TUI 已确认会消费的基础事件：
/// - delta
/// - context_loaded
/// - step_start / step_finish
/// - done / error
///
/// 未知事件会降级为 `Unknown`，但不会再把原始 JSON 暴露给 CLI runtime。
pub fn normalize_chat_stream_event(event: GatewayChatStreamEvent) -> GatewayTypedChatStreamEvent {
    match event {
        GatewayChatStreamEvent::Delta(delta) => GatewayTypedChatStreamEvent::Delta(delta),
        GatewayChatStreamEvent::Done { finish_reason, usage, message_id, parent_message_id } => {
            GatewayTypedChatStreamEvent::Done {
                finish_reason,
                usage: normalize_chat_usage(usage),
                message_id,
                parent_message_id,
            }
        }
        GatewayChatStreamEvent::Error(error) => GatewayTypedChatStreamEvent::Error(error),
        GatewayChatStreamEvent::Other(payload) => {
            match payload.get("type").and_then(serde_json::Value::as_str) {
                Some("chat.model_recovery") => {
                    match serde_json::from_value::<GatewayModelRecoveryEvent>(payload.clone()) {
                        Ok(state) => GatewayTypedChatStreamEvent::ModelRecovery(state),
                        Err(_) => GatewayTypedChatStreamEvent::Unknown {
                            event_type: Some("chat.model_recovery".into()),
                        },
                    }
                }
                Some("chat.context_loaded") => GatewayTypedChatStreamEvent::ContextLoaded {
                    paths: payload
                        .get("paths")
                        .and_then(serde_json::Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::trim)
                        .filter(|path| !path.is_empty())
                        .map(ToOwned::to_owned)
                        .collect(),
                },
                Some("chat.step_start") => {
                    GatewayTypedChatStreamEvent::StepStart(parse_step_start_payload(&payload))
                }
                Some("chat.step_finish") => {
                    GatewayTypedChatStreamEvent::StepFinish(parse_step_finish_payload(&payload))
                }
                Some("chat.post_tool_round") => GatewayTypedChatStreamEvent::PostToolRound(
                    parse_post_tool_round_payload(&payload),
                ),
                Some("chat.todo_updated") => GatewayTypedChatStreamEvent::TodoUpdated {
                    session_id: payload
                        .get("session_id")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                },
                Some("chat.question_raised") => GatewayTypedChatStreamEvent::QuestionRaised {
                    session_id: payload
                        .get("session_id")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                },
                Some("chat.question_resolved") => GatewayTypedChatStreamEvent::QuestionResolved {
                    session_id: payload
                        .get("session_id")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                },
                Some("chat.usage_updated") => GatewayTypedChatStreamEvent::UsageUpdated {
                    session_id: payload
                        .get("session_id")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                    usage: payload
                        .get("usage")
                        .cloned()
                        .and_then(|value| serde_json::from_value::<GatewayChatUsage>(value).ok())
                        .unwrap_or_default(),
                },
                Some("chat.title_updated") => GatewayTypedChatStreamEvent::TitleUpdated {
                    session_id: payload
                        .get("session_id")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                    title: payload
                        .get("title")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                },
                Some("chat.session_updated") => GatewayTypedChatStreamEvent::SessionUpdated {
                    session_id: payload
                        .get("session_id")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                    title: payload
                        .get("session")
                        .and_then(|session| session.get("title"))
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                },
                other => GatewayTypedChatStreamEvent::Unknown {
                    event_type: other.map(ToOwned::to_owned),
                },
            }
        }
    }
}
fn normalize_chat_usage(value: Option<serde_json::Value>) -> Option<GatewayChatUsage> {
    value.and_then(|value| serde_json::from_value::<GatewayChatUsage>(value).ok())
}

fn parse_step_start_payload(payload: &serde_json::Value) -> GatewayChatStepStartEvent {
    GatewayChatStepStartEvent {
        step_index: payload
            .get("step_index")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or_default(),
        created_ms: payload
            .get("created_ms")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default(),
        model: payload.get("model").and_then(serde_json::Value::as_str).map(ToOwned::to_owned),
    }
}

fn parse_step_finish_payload(payload: &serde_json::Value) -> GatewayChatStepFinishEvent {
    GatewayChatStepFinishEvent {
        step_index: payload
            .get("step_index")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or_default(),
        finished_ms: payload
            .get("finished_ms")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default(),
        usage: payload
            .get("usage")
            .cloned()
            .and_then(|value| serde_json::from_value::<GatewayChatUsage>(value).ok())
            .unwrap_or_default(),
        finish_reason: payload
            .get("finish_reason")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        model: payload.get("model").and_then(serde_json::Value::as_str).map(ToOwned::to_owned),
    }
}

fn parse_post_tool_round_payload(payload: &serde_json::Value) -> GatewayChatPostToolRoundEvent {
    GatewayChatPostToolRoundEvent {
        step_index: payload
            .get("step_index")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or_default(),
    }
}
