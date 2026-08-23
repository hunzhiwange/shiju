//! 验证 ACP 错误载荷提取、深度限制与资源不存在判定的兼容边界。
//!
//! 构造的协议载荷与错误链用于固定归一化优先级和诊断信息边界。

use serde_json::json;

use super::*;

// 下列行为用例覆盖已知错误、缺失字段、嵌套载荷和安全回退。
#[test]
fn extract_acp_error_reads_direct_and_nested_payloads() {
    let direct = json!({
        "code": -32001,
        "message": "  missing session  ",
        "data": { "sessionId": "abc" }
    });
    let nested = json!({ "error": { "cause": direct.clone() } });

    let payload = extract_acp_error(&nested).expect("nested ACP error should parse");

    assert_eq!(payload.code, -32001);
    assert_eq!(payload.message, "missing session");
    assert_eq!(payload.data, Some(json!({ "sessionId": "abc" })));
}

#[test]
fn extract_acp_error_reads_each_supported_nested_key() {
    for key in ["error", "acp", "cause"] {
        let value = json!({
            key: {
                "code": -32002,
                "message": "missing",
            }
        });

        let payload = extract_acp_error(&value).expect("nested ACP error should parse");

        assert_eq!(payload.code, -32002);
        assert_eq!(payload.message, "missing");
        assert_eq!(payload.data, None);
    }
}

#[test]
fn extract_acp_error_prefers_direct_payload_over_nested_payload() {
    let value = json!({
        "code": 1,
        "message": "direct",
        "error": {
            "code": -32001,
            "message": "nested",
        }
    });

    let payload = extract_acp_error(&value).expect("direct ACP error should parse");

    assert_eq!(payload.code, 1);
    assert_eq!(payload.message, "direct");
}

#[test]
fn extract_acp_error_rejects_missing_or_empty_fields() {
    for value in [
        json!("plain"),
        json!({ "code": -32001 }),
        json!({ "message": "missing" }),
        json!({ "code": -32001, "message": 123 }),
        json!({ "code": "bad", "message": "missing" }),
        json!({ "code": -32001, "message": "   " }),
        json!({ "error": "not an object" }),
    ] {
        assert!(extract_acp_error(&value).is_none());
    }
}

#[test]
fn extract_acp_error_honors_depth_limit_boundary() {
    let mut value = json!({
        "code": -32001,
        "message": "missing session"
    });
    for _ in 0..5 {
        value = json!({ "error": value });
    }
    assert!(extract_acp_error(&value).is_some());

    value = json!({ "error": value });
    assert!(extract_acp_error(&value).is_none());
}

#[test]
fn format_unknown_error_message_prefers_non_empty_message() {
    assert_eq!(format_unknown_error_message(&json!("plain")), "plain");
    assert_eq!(format_unknown_error_message(&json!("")), "\"\"");
    assert_eq!(format_unknown_error_message(&json!(false)), "false");
    assert_eq!(format_unknown_error_message(&json!({ "message": "from object" })), "from object");
    assert_eq!(format_unknown_error_message(&json!({ "message": "" })), r#"{"message":""}"#);
    assert_eq!(format_unknown_error_message(&json!({ "message": 123 })), r#"{"message":123}"#);
}

#[test]
fn resource_not_found_detects_codes_messages_and_nested_hints() {
    assert!(is_acp_resource_not_found_error(&json!({ "code": -32002, "message": "missing" })));
    assert!(is_acp_resource_not_found_error(&json!({
        "code": 1,
        "message": "Session abc_123 not found"
    })));
    assert!(is_acp_resource_not_found_error(&json!({
        "error": {
            "code": 1,
            "message": "runtime",
            "data": { "details": ["unknown session"] }
        }
    })));
    assert!(is_acp_resource_not_found_error(&json!({ "message": "Resource not found" })));
    assert!(is_acp_resource_not_found_error(&json!({ "message": "invalid session identifier" })));
    assert!(!is_acp_resource_not_found_error(&json!({ "code": 1, "message": "other" })));
    assert!(!is_acp_resource_not_found_error(&json!({ "message": "other" })));
    assert!(!is_acp_resource_not_found_error(&json!(123)));
}

#[test]
fn session_not_found_pattern_rejects_incomplete_or_unsafe_shapes() {
    for value in [
        "session",
        "session abc missing",
        "session not found",
        "session '' not found",
        "session abc.def not found",
    ] {
        assert!(!session_not_found_pattern(value));
    }

    assert!(session_not_found_pattern("session `abc-123` not found"));
    assert!(session_not_found_pattern("SESSION 'ABC_123' NOT FOUND"));
}

#[test]
fn resource_not_found_hint_obeys_scan_depth_limit() {
    assert!(has_session_not_found_hint(&json!({ "a": [{ "b": "unknown session" }] }), 0));
    assert!(!has_session_not_found_hint(&json!("unknown session"), 5));
    assert!(!has_session_not_found_hint(&json!(false), 0));
}
