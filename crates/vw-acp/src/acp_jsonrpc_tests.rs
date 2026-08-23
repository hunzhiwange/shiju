//! 验证 ACP JSON-RPC 消息形状、通知识别与停止原因解析契约。
//!
//! 最小 JSON 与协议类型 fixture 用于固定线格式和输入校验边界。

use serde_json::json;

use super::*;

// 下列行为用例覆盖合法往返、字段默认值和无效形状拒绝。
#[test]
fn json_rpc_message_accepts_requests_notifications_and_responses() {
    assert!(is_acp_json_rpc_message(&json!({
        "jsonrpc": "2.0",
        "method": "session/new",
        "id": "1"
    })));
    assert!(is_acp_json_rpc_message(&json!({
        "jsonrpc": "2.0",
        "method": "session/update"
    })));
    assert!(is_acp_json_rpc_message(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {}
    })));
    assert!(is_acp_json_rpc_message(&json!({
        "jsonrpc": "2.0",
        "id": null,
        "error": { "code": -1, "message": "failed" }
    })));
}

#[test]
fn json_rpc_message_rejects_invalid_shapes() {
    assert!(!is_acp_json_rpc_message(&json!({ "jsonrpc": "1.0", "method": "x" })));
    assert!(!is_acp_json_rpc_message(&json!(null)));
    assert!(!is_acp_json_rpc_message(&json!({ "jsonrpc": "2.0" })));
    assert!(!is_acp_json_rpc_message(&json!({ "jsonrpc": "2.0", "method": "" })));
    assert!(!is_acp_json_rpc_message(&json!({ "jsonrpc": "2.0", "method": 1 })));
    assert!(!is_acp_json_rpc_message(&json!({ "jsonrpc": "2.0", "id": true, "result": {} })));
    assert!(!is_acp_json_rpc_message(&json!({ "jsonrpc": "2.0", "id": {}, "result": {} })));
    assert!(!is_acp_json_rpc_message(&json!({ "jsonrpc": "2.0", "id": [], "result": {} })));
    assert!(!is_acp_json_rpc_message(&json!({ "jsonrpc": "2.0", "id": 1 })));
    assert!(!is_acp_json_rpc_message(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {},
        "error": { "code": -1, "message": "failed" }
    })));
    assert!(!is_acp_json_rpc_message(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": { "code": "bad", "message": "failed" }
    })));
    assert!(!is_acp_json_rpc_message(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": { "code": -1 }
    })));
    assert!(!is_acp_json_rpc_message(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": { "code": -1, "message": 1 }
    })));
    assert!(!is_acp_json_rpc_message(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": "failed"
    })));
    assert!(!is_acp_json_rpc_message(&json!({
        "jsonrpc": "2.0",
        "method": "session/new",
        "id": true
    })));
}

#[test]
fn notification_helpers_require_session_update_payload() {
    let valid = json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "sessionId": "session-1",
            "update": { "sessionUpdate": "plan", "entries": [] }
        }
    });

    assert!(is_json_rpc_notification(&valid));
    assert!(is_session_update_notification(&valid));
    assert!(extract_session_update_notification(&valid).is_some());
    assert!(is_json_rpc_notification(&json!({ "method": "" })));
    assert!(!is_json_rpc_notification(&json!(null)));
    assert!(!is_json_rpc_notification(&json!({ "method": 1 })));
    assert!(!is_json_rpc_notification(&json!({ "method": "session/update", "id": 1 })));
    assert!(!is_session_update_notification(&json!(null)));
    assert!(!is_session_update_notification(&json!({ "method": "session/update", "id": 1 })));
    assert!(
        extract_session_update_notification(&json!({
            "jsonrpc": "2.0",
            "method": "session/cancel"
        }))
        .is_none()
    );
    assert!(
        extract_session_update_notification(&json!({
            "jsonrpc": "2.0",
            "method": "session/update"
        }))
        .is_none()
    );
    assert!(
        extract_session_update_notification(&json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": []
        }))
        .is_none()
    );
    assert!(
        extract_session_update_notification(&json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {
                "update": { "sessionUpdate": "plan", "entries": [] }
            }
        }))
        .is_none()
    );
    assert!(
        extract_session_update_notification(&json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {
                "sessionId": "session-1"
            }
        }))
        .is_none()
    );
    assert!(
        extract_session_update_notification(&json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {
                "sessionId": "session-1",
                "update": []
            }
        }))
        .is_none()
    );
    assert!(
        extract_session_update_notification(&json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": { "sessionId": "session-1", "update": {} }
        }))
        .is_none()
    );
    assert!(
        extract_session_update_notification(&json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {
                "sessionId": "session-1",
                "update": { "sessionUpdate": "not_a_real_update" }
            }
        }))
        .is_none()
    );
}

#[test]
fn parse_prompt_stop_reason_and_error_message() {
    assert_eq!(
        parse_prompt_stop_reason(&json!({ "result": { "stopReason": "end_turn" } })),
        Some("end_turn".to_string())
    );
    assert_eq!(
        parse_json_rpc_error_message(&json!({ "error": { "message": "boom" } })),
        Some("boom".to_string())
    );
    assert!(parse_prompt_stop_reason(&json!(null)).is_none());
    assert!(parse_prompt_stop_reason(&json!({})).is_none());
    assert!(parse_prompt_stop_reason(&json!({ "result": [] })).is_none());
    assert!(parse_prompt_stop_reason(&json!({ "result": {} })).is_none());
    assert!(parse_prompt_stop_reason(&json!({ "result": { "stopReason": 1 } })).is_none());
    assert!(parse_json_rpc_error_message(&json!(null)).is_none());
    assert!(parse_json_rpc_error_message(&json!({})).is_none());
    assert!(parse_json_rpc_error_message(&json!({ "error": [] })).is_none());
    assert!(
        parse_json_rpc_error_message(&json!({
            "error": { "message": 1 }
        }))
        .is_none()
    );
}
