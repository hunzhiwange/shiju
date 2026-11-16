//! ACP 事件映射领域断言；真实宿主运行与取消验收仍在宿主注册。
use crate::backend::map_runtime_event;
use serde_json::json;
use acp_server::server::{BackendToolStatus, BackendEvent};
use session_port::{PromptEvent, ToolCallStatus, ToolEvent, ToolEventKind};

#[test]
fn backend_maps_runtime_events_without_parsing_display_text() {
    let payload = json!({
        "status": "completed",
        "input": {"token": "[REDACTED]"},
        "output": {"ok": true}
    });
    let event = PromptEvent::Tool(ToolEvent {
        call_id: "call-1".to_string(),
        name: "shell".to_string(),
        kind: ToolEventKind::Complete,
        status: ToolCallStatus::Completed,
        occurred_ms: 1,
        payload: payload.clone(),
    });

    let mapped = map_runtime_event(event).expect("mapped event");
    assert!(matches!(
        mapped,
        BackendEvent::ToolComplete {
            call_id,
            status: BackendToolStatus::Completed,
            raw_output: Some(output),
            ..
        } if call_id == "call-1" && output == json!({"ok": true})
    ));
}
