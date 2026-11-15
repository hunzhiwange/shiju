//! 覆盖 ACP 端口辅助函数的兼容性行为。
//!
//! 这些测试集中验证运行时 session id 归一化、读类输出抑制、事件日志路径生成
//! 以及 JSON-RPC 错误响应。端口层是协议边界，断言固定输出形状可以防止客户端
//! 兼容性被内部错误模型调整意外破坏。

use std::path::PathBuf;

use serde_json::json;
use vw_acp::{
    AGENT_SESSION_ID_META_KEYS, BuildJsonRpcErrorParams, DEFAULT_EVENT_MAX_SEGMENTS,
    DEFAULT_EVENT_SEGMENT_MAX_BYTES, OutputErrorCode, OutputErrorOrigin,
    RUNTIME_SESSION_ID_META_KEYS, ReadLikeToolDescriptor, SUPPRESSED_READ_OUTPUT,
    build_json_rpc_error_response, extract_runtime_session_id, is_read_like_tool,
    normalize_runtime_session_id, output_error_jsonrpc_code, safe_session_id, session_base_dir,
    session_event_active_path, session_event_lock_path, session_event_log,
    session_event_segment_path,
};

/// 运行时 session id helper 需要沿用 agent session id 的元数据 key 与归一化规则。
#[test]
fn runtime_session_id_helpers_delegate_to_agent_session_id_logic() {
    assert_eq!(RUNTIME_SESSION_ID_META_KEYS, AGENT_SESSION_ID_META_KEYS);
    assert_eq!(
        normalize_runtime_session_id(&json!(" session-123 ")),
        Some("session-123".to_string())
    );
    assert_eq!(
        extract_runtime_session_id(&json!({
            "sessionId": " session-123 "
        })),
        Some("session-123".to_string())
    );
}

/// 读类工具输出可能包含大文件或敏感内容，因此端口层需要稳定识别并抑制展示。
#[test]
fn read_output_suppression_detects_read_like_tools() {
    assert_eq!(SUPPRESSED_READ_OUTPUT, "[read output suppressed]");
    assert!(is_read_like_tool(&ReadLikeToolDescriptor {
        title: None,
        kind: Some("read".to_string()),
    }));
    assert!(is_read_like_tool(&ReadLikeToolDescriptor {
        title: Some("Open: package.json".to_string()),
        kind: None,
    }));
    assert!(!is_read_like_tool(&ReadLikeToolDescriptor {
        title: Some("Write: package.json".to_string()),
        kind: Some("edit".to_string()),
    }));
}

/// 事件日志路径必须对 session id 做安全编码，避免路径分隔符逃逸日志目录。
#[test]
fn session_event_log_helpers_build_expected_paths() {
    let home_dir = PathBuf::from("/tmp/vwacp-home");
    let session_id = "session 1/中文";
    let sessions_dir =
        config_runtime_paths::home_config_dir(&home_dir).join("acp").join("sessions");
    let active_path = sessions_dir.join("session%201%2F%E4%B8%AD%E6%96%87.stream.ndjson");

    assert_eq!(safe_session_id(session_id), "session%201%2F%E4%B8%AD%E6%96%87");
    assert_eq!(session_base_dir(&home_dir), sessions_dir.clone());
    assert_eq!(session_event_active_path(session_id, &home_dir), active_path.clone());
    assert_eq!(
        session_event_segment_path(session_id, 2, &home_dir),
        sessions_dir.join("session%201%2F%E4%B8%AD%E6%96%87.stream.2.ndjson")
    );
    assert_eq!(
        session_event_lock_path(session_id, &home_dir),
        sessions_dir.join("session%201%2F%E4%B8%AD%E6%96%87.stream.lock")
    );

    let log = session_event_log(session_id, &home_dir);
    assert_eq!(log.active_path, active_path.to_string_lossy());
    assert_eq!(log.segment_count, DEFAULT_EVENT_MAX_SEGMENTS);
    assert_eq!(log.max_segment_bytes, DEFAULT_EVENT_SEGMENT_MAX_BYTES);
    assert_eq!(log.max_segments, DEFAULT_EVENT_MAX_SEGMENTS);
    assert_eq!(log.last_write_at, None);
    assert_eq!(log.last_write_error, None);
}

/// 当 ACP 原始错误负载可用时，JSON-RPC 响应应优先保持对端协议给出的形状。
#[test]
fn jsonrpc_error_response_prefers_acp_payload_when_available() {
    let response = build_json_rpc_error_response(BuildJsonRpcErrorParams {
        id: Some(json!(7)),
        output_code: OutputErrorCode::Runtime,
        detail_code: Some("IGNORED".to_string()),
        origin: Some(OutputErrorOrigin::Runtime),
        message: "wrapper".to_string(),
        retryable: Some(true),
        timestamp: Some("2026-04-03T00:00:00Z".to_string()),
        session_id: Some("session-123".to_string()),
        acp: Some(vw_acp::OutputErrorAcpPayload {
            code: -32002,
            message: "Session missing".to_string(),
            data: Some(json!({ "sessionId": "session-123" })),
        }),
    });

    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, json!(7));
    assert_eq!(response.error.code, -32002);
    assert_eq!(response.error.message, "Session missing");
    assert_eq!(response.error.data, Some(json!({ "sessionId": "session-123" })));
}

/// 没有 ACP 原始负载时，端口层应把统一错误模型转换为客户端可消费的 JSON-RPC data。
#[test]
fn jsonrpc_error_response_builds_fallback_data_for_normalized_errors() {
    assert_eq!(output_error_jsonrpc_code(OutputErrorCode::Usage), -32602);
    assert_eq!(output_error_jsonrpc_code(OutputErrorCode::Timeout), -32070);

    let response = build_json_rpc_error_response(BuildJsonRpcErrorParams {
        id: None,
        output_code: OutputErrorCode::PermissionDenied,
        detail_code: Some("POLICY_DENIED".to_string()),
        origin: Some(OutputErrorOrigin::Cli),
        message: "permission denied".to_string(),
        retryable: Some(false),
        timestamp: Some("2026-04-03T00:00:00Z".to_string()),
        session_id: Some("session-123".to_string()),
        acp: None,
    });

    assert_eq!(
        serde_json::to_value(response).unwrap(),
        json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": {
                "code": -32071,
                "message": "permission denied",
                "data": {
                    "vwacpCode": "PERMISSION_DENIED",
                    "detailCode": "POLICY_DENIED",
                    "origin": "cli",
                    "retryable": false,
                    "timestamp": "2026-04-03T00:00:00Z",
                    "sessionId": "session-123"
                }
            }
        })
    );
}
