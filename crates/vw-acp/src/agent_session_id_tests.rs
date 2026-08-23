//! 验证代理会话标识的规范化、字段优先级与无效元数据处理。

use serde_json::json;

use super::*;

#[test]
fn normalize_agent_session_id_trims_non_empty_strings() {
    assert_eq!(normalize_agent_session_id(&json!("  agent-1  ")), Some("agent-1".to_string()));
    assert!(normalize_agent_session_id(&json!("   ")).is_none());
    assert!(normalize_agent_session_id(&json!(42)).is_none());
}

#[test]
fn extract_agent_session_id_prefers_agent_specific_key() {
    let meta = json!({
        "agentSessionId": " agent-session ",
        "sessionId": "runtime-session"
    });

    assert_eq!(extract_agent_session_id(&meta), Some("agent-session".to_string()));
}

#[test]
fn extract_agent_session_id_falls_back_to_session_id_and_rejects_invalid_meta() {
    assert_eq!(
        extract_agent_session_id(&json!({ "sessionId": " session-1 " })),
        Some("session-1".to_string())
    );
    assert!(extract_agent_session_id(&json!({ "sessionId": "" })).is_none());
    assert!(extract_agent_session_id(&json!(["session-1"])).is_none());
}
