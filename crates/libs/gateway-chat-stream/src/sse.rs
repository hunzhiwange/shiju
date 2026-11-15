//! SSE 帧边界与事件解析。

use crate::types::*;

#[derive(Debug, PartialEq, Eq)]
pub struct GatewaySseFrame {
    pub id: Option<u64>,
    pub data: String,
}

/// 将网关返回的原始 JSON 事件映射为稳定的流式事件枚举。
pub(crate) fn parse_stream_event(payload: serde_json::Value) -> GatewayChatStreamEvent {
    match payload.get("type").and_then(serde_json::Value::as_str).unwrap_or_default() {
        "chat.delta" => GatewayChatStreamEvent::Delta(
            payload
                .get("delta")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ),
        "chat.done" => GatewayChatStreamEvent::Done {
            finish_reason: payload
                .get("finish_reason")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            usage: payload.get("usage").cloned(),
            message_id: payload
                .get("message_id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            parent_message_id: payload
                .get("parent_message_id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
        },
        "chat.error" => GatewayChatStreamEvent::Error(
            payload
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("gateway stream failed")
                .to_string(),
        ),
        _ => GatewayChatStreamEvent::Other(payload),
    }
}

/// 从缓冲区中提取一个完整的 SSE 帧，保留未消费部分供后续继续解析。
pub fn take_next_sse_event(buffer: &mut String) -> Option<String> {
    let separator_len = if let Some(idx) = buffer.find("\r\n\r\n") {
        Some((idx, 4))
    } else {
        buffer.find("\n\n").map(|idx| (idx, 2))
    }?;

    let (idx, sep_len) = separator_len;
    let frame = buffer[..idx].to_string();
    buffer.drain(..idx + sep_len);
    Some(frame)
}

pub fn parse_sse_frame(frame: &str) -> Result<Option<GatewaySseFrame>, String> {
    let mut id = None;
    let data = frame
        .lines()
        .filter_map(|line| {
            let line = line.trim_end_matches('\r');
            if let Some(value) = line.strip_prefix("id:") {
                id = Some(value.trim().parse::<u64>().map_err(|_| {
                    "gateway stream SSE id must be a non-negative integer".to_string()
                }));
                None
            } else {
                line.strip_prefix("data:").map(|value| Ok(value.trim_start()))
            }
        })
        .collect::<Result<Vec<_>, String>>()?
        .join("\n");

    if data.is_empty() {
        return Ok(None);
    }
    Ok(Some(GatewaySseFrame { id: id.transpose()?, data }))
}

pub(crate) fn required_event_id(id: Option<u64>, resumable: bool) -> Result<u64, String> {
    match (id, resumable) {
        (Some(id), _) => Ok(id),
        (None, true) => Err("gateway resumable stream event is missing SSE id".to_string()),
        (None, false) => Ok(0),
    }
}

/// 返回事件对应的稳定日志名称，便于统计与调试。
pub(crate) fn event_name(event: &GatewayChatStreamEvent) -> &'static str {
    match event {
        GatewayChatStreamEvent::Delta(_) => "chat.delta",
        GatewayChatStreamEvent::Done { .. } => "chat.done",
        GatewayChatStreamEvent::Error(_) => "chat.error",
        GatewayChatStreamEvent::Other(_) => "other",
    }
}

pub(crate) fn is_terminal_stream_event(event: &GatewayChatStreamEvent) -> bool {
    matches!(event, GatewayChatStreamEvent::Done { .. } | GatewayChatStreamEvent::Error(_))
}
