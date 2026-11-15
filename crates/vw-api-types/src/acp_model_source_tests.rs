use vw_api_chat::chat::{ChatOptionsDto, GatewayChatStreamRequest};
use serde_json::json;

#[test]
fn acp_model_source_contract_defaults_and_round_trips_chat_requests() {
    let options: ChatOptionsDto = serde_json::from_value(json!({})).unwrap();
    assert_eq!(serde_json::to_value(options).unwrap()["acp_model_source"], "session_model");
    let request: GatewayChatStreamRequest = serde_json::from_value(json!({
        "messages": [], "acp_model_source": "cli_local", "model": "provider/model"
    }))
    .unwrap();
    let encoded = serde_json::to_value(request).unwrap();
    assert_eq!(encoded["acp_model_source"], "cli_local");
    assert_eq!(encoded["model"], "provider/model");
}

#[test]
fn acp_model_source_contract_rejects_unknown_source() {
    assert!(
        serde_json::from_value::<ChatOptionsDto>(json!({"acp_model_source": "fallback"})).is_err()
    );
    assert!(
        serde_json::from_value::<GatewayChatStreamRequest>(json!({
            "messages": [], "acp_model_source": "fallback"
        }))
        .is_err()
    );
}
