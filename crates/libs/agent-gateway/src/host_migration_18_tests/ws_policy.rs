use crate::ws::*;
use agent_loop::progress::{DRAFT_PROGRESS_SENTINEL, DRAFT_WS_EVENT_SENTINEL};
use axum::http::{HeaderMap, header};
use vw_api_tools::{ToolResultContentDto, ToolResultDto};
fn extract_ws_bearer_token(headers: &HeaderMap) -> Option<String> { extract_ws_auth_skey(headers) }
#[test]
fn extract_ws_bearer_token_falls_back_when_authorization_is_not_bearer() {
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, "Basic dXNlcjpwYXNz".parse().unwrap());
    headers.insert(
        header::SEC_WEBSOCKET_PROTOCOL,
        "vibewindow.v1, bearer.protocol-token".parse().unwrap(),
    );

    assert_eq!(extract_ws_bearer_token(&headers).as_deref(), Some("protocol-token"));
}

#[test]
fn parse_ws_private_event_uses_plain_payload_defaults() {
    let progress = format!(
        "{DRAFT_WS_EVENT_SENTINEL}{}",
        serde_json::json!({
            "event": "tool_result",
            "success": true,
            "duration_secs": 7,
            "tool_call_id": "call_plain"
        })
    );

    assert_eq!(
        parse_ws_private_event(&progress),
        Some(WsDeltaEvent::ToolResult {
            name: "tool".to_string(),
            success: true,
            duration_secs: Some(7),
            tool_call_id: Some("call_plain".to_string()),
            result: None,
        })
    );
}

#[test]
fn parse_ws_private_event_prefers_explicit_name_over_result_tool_id() {
    let result = ToolResultDto {
        tool_use_id: Some("call_result".to_string()),
        tool_id: Some("file_read".into()),
        success: Some(false),
        content: vec![ToolResultContentDto::Text { text: "denied".to_string() }],
        data: serde_json::Value::Null,
        model_result: serde_json::Value::Null,
        render_hint: None,
        permission_request: None,
        context_updates: Vec::new(),
        extra_messages: Vec::new(),
        telemetry: None,
    };
    let progress = format!(
        "{DRAFT_WS_EVENT_SENTINEL}{}",
        serde_json::json!({
            "event": "tool_result",
            "name": "display_name",
            "result": result,
        })
    );

    assert_eq!(
        parse_ws_private_event(&progress),
        Some(WsDeltaEvent::ToolResult {
            name: "display_name".to_string(),
            success: false,
            duration_secs: None,
            tool_call_id: Some("call_result".to_string()),
            result: Some(result),
        })
    );
}

#[test]
fn parse_ws_delta_event_maps_empty_hint_to_none_and_bad_duration_to_none() {
    assert_eq!(
        parse_ws_delta_event(&format!("{DRAFT_PROGRESS_SENTINEL}⏳ shell:   ")),
        Some(WsDeltaEvent::ToolCall { name: "shell".to_string(), hint: None })
    );
    assert_eq!(
        parse_ws_delta_event(&format!("{DRAFT_PROGRESS_SENTINEL}✅ shell (later)")),
        Some(WsDeltaEvent::ToolResult {
            name: "shell".to_string(),
            success: true,
            duration_secs: None,
            tool_call_id: None,
            result: None,
        })
    );
}


#[test]
fn websocket_entrypoints_and_constants_are_available() {
    assert_eq!(WS_CHAT_SUBPROTOCOL, "vibewindow.v1");
    assert!(EMPTY_WS_RESPONSE_FALLBACK.contains("Tool execution completed"));
}
