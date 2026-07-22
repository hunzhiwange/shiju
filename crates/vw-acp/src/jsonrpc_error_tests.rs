use serde_json::{Value, json};

use super::jsonrpc_error::*;
use super::types::{OutputErrorAcpPayload, OutputErrorCode, OutputErrorOrigin};

#[test]
fn output_error_codes_map_to_jsonrpc_codes() {
    let cases = [
        (OutputErrorCode::NoSession, -32002),
        (OutputErrorCode::Timeout, -32070),
        (OutputErrorCode::PermissionDenied, -32071),
        (OutputErrorCode::PermissionPromptUnavailable, -32072),
        (OutputErrorCode::Runtime, -32603),
        (OutputErrorCode::Usage, -32602),
    ];

    for (code, jsonrpc_code) in cases {
        assert_eq!(output_error_jsonrpc_code(code), jsonrpc_code);
    }
}

#[test]
fn fallback_error_response_preserves_metadata() {
    let response = build_json_rpc_error_response(BuildJsonRpcErrorParams {
        id: Some(json!("request-1")),
        output_code: OutputErrorCode::Timeout,
        detail_code: Some("SLOW".to_string()),
        origin: Some(OutputErrorOrigin::Runtime),
        message: "timed out".to_string(),
        retryable: Some(false),
        timestamp: Some("2026-05-24T00:00:00Z".to_string()),
        session_id: Some("session-1".to_string()),
        acp: None,
    });

    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, json!("request-1"));
    assert_eq!(response.error.code, -32070);
    assert_eq!(response.error.message, "timed out");
    assert_eq!(
        response.error.data,
        Some(json!({
            "vwacpCode": "TIMEOUT",
            "detailCode": "SLOW",
            "origin": "runtime",
            "retryable": false,
            "timestamp": "2026-05-24T00:00:00Z",
            "sessionId": "session-1"
        }))
    );
}

#[test]
fn valid_acp_error_takes_precedence_over_fallback_fields() {
    let response = build_json_rpc_error_response(BuildJsonRpcErrorParams {
        id: None,
        output_code: OutputErrorCode::Runtime,
        detail_code: Some("IGNORED".to_string()),
        origin: Some(OutputErrorOrigin::Acp),
        message: "fallback".to_string(),
        retryable: Some(true),
        timestamp: None,
        session_id: None,
        acp: Some(OutputErrorAcpPayload {
            code: -32000,
            message: "agent message".to_string(),
            data: Some(json!({ "methodId": "token" })),
        }),
    });

    assert_eq!(response.id, Value::Null);
    assert_eq!(response.error.code, -32000);
    assert_eq!(response.error.message, "agent message");
    assert_eq!(response.error.data, Some(json!({ "methodId": "token" })));
}

#[test]
fn blank_acp_error_message_uses_fallback_error() {
    let response = build_json_rpc_error_response(BuildJsonRpcErrorParams {
        id: Some(json!(7)),
        output_code: OutputErrorCode::PermissionDenied,
        detail_code: None,
        origin: None,
        message: "permission denied".to_string(),
        retryable: None,
        timestamp: None,
        session_id: None,
        acp: Some(OutputErrorAcpPayload {
            code: -32000,
            message: " \t\n".to_string(),
            data: Some(json!({ "ignored": true })),
        }),
    });

    assert_eq!(response.id, json!(7));
    assert_eq!(response.error.code, -32071);
    assert_eq!(response.error.message, "permission denied");
    assert_eq!(
        response.error.data,
        Some(json!({
            "vwacpCode": "PERMISSION_DENIED"
        }))
    );
}

#[test]
fn jsonrpc_error_object_skips_missing_data_when_serialized() {
    let value = serde_json::to_value(JsonRpcErrorObject {
        code: -32603,
        message: "runtime failed".to_string(),
        data: None,
    })
    .unwrap();

    assert_eq!(
        value,
        json!({
            "code": -32603,
            "message": "runtime failed"
        })
    );
}
