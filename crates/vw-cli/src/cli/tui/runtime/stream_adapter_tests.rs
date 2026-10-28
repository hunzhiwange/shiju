use super::stream_adapter::{UiRuntimeEvent, adapt_gateway_stream_event};
use crate::cli::tui::model::UiMessage;
use serde_json::json;
use vw_gateway_client::GatewayChatStreamEvent;

#[test]
fn model_recovery_reports_progress_without_ending_the_turn() {
    for (phase, completed) in
        [("waiting", false), ("recovered", true), ("stopped", true), ("cancelled", true)]
    {
        let event = GatewayChatStreamEvent::Other(json!({
            "type": "chat.model_recovery", "session_id": "session-recovery",
            "attempt": 1, "max_attempts": 3, "wait_ms": 500,
            "phase": phase, "reason": "模型不可用", "next_action": "检查模型配置"
        }));
        let UiRuntimeEvent::Message(UiMessage::Progress(progress)) =
            adapt_gateway_stream_event(event)
        else {
            panic!("recovery must remain a progress event: {phase}");
        };
        assert_eq!(progress.completed, completed);
        assert_eq!(progress.current, Some(1));
        assert_eq!(progress.total, Some(3));
        assert!(progress.label.contains("检查模型配置"));
    }
}
