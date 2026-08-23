//! 验证运行时会话标识的规范化以及候选元数据键优先级。

use super::*;
use serde_json::json;

#[test]
fn normalize_runtime_session_id_trims_non_empty_strings() {
    assert_eq!(normalize_runtime_session_id(&json!(" session-1 ")).as_deref(), Some("session-1"));
    assert_eq!(normalize_runtime_session_id(&json!("   ")), None);
    assert_eq!(normalize_runtime_session_id(&json!(1)), None);
}

#[test]
fn extract_runtime_session_id_uses_supported_meta_keys_in_order() {
    assert_eq!(
        extract_runtime_session_id(&json!({"agentSessionId": " agent ", "sessionId": "runtime"}))
            .as_deref(),
        Some("agent")
    );
    assert_eq!(
        extract_runtime_session_id(&json!({"sessionId": " runtime "})).as_deref(),
        Some("runtime")
    );
}
