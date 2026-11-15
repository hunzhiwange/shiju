//! SessionTool 创建结果的业务投影。
pub fn session_tool_created_session(
    event: &gateway_chat_stream::GatewayChatToolEvent,
) -> Option<(String, Option<String>)> {
    use gateway_chat_stream::{GatewayChatToolEventKind, GatewayChatToolStatus};

    if event.name != "SessionTool"
        || event.kind != GatewayChatToolEventKind::Complete
        || event.status != GatewayChatToolStatus::Completed
    {
        return None;
    }
    let output = event.payload.get("output")?.as_str()?;
    let output: serde_json::Value = serde_json::from_str(output).ok()?;
    if output.get("action").and_then(serde_json::Value::as_str) != Some("create") {
        return None;
    }
    let session_id = output.get("session_id")?.as_str()?.trim().to_string();
    if session_id.is_empty() {
        return None;
    }
    let directory = output
        .get("directory")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    Some((session_id, directory))
}
