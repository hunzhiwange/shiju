//! 验证公开数据类型的稳定字面量、字段形状、默认值与往返序列化。
//!
//! 最小 JSON 与协议类型 fixture 用于固定线格式和输入校验边界。

use std::collections::HashMap;
use std::path::PathBuf;

use agent_client_protocol::StopReason;
use serde_json::json;

use super::*;

fn test_record() -> SessionRecord {
    SessionRecord {
        schema: SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: "record-1".to_string(),
        acp_session_id: "session-1".to_string(),
        agent_session_id: None,
        agent_command: "agent".to_string(),
        agent_config: None,
        cwd: "/tmp/project".to_string(),
        name: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_used_at: "2026-01-01T00:00:00Z".to_string(),
        last_seq: 0,
        last_request_id: None,
        event_log: SessionEventLog {
            active_path: "/tmp/project/events.jsonl".to_string(),
            segment_count: 1,
            max_segment_bytes: 1024,
            max_segments: 4,
            last_write_at: None,
            last_write_error: None,
        },
        closed: Some(false),
        closed_at: None,
        pid: None,
        agent_started_at: None,
        last_prompt_at: None,
        last_agent_exit_code: None,
        last_agent_exit_signal: None,
        last_agent_exit_at: None,
        last_agent_disconnect_reason: None,
        protocol_version: None,
        agent_capabilities: None,
        title: None,
        messages: Vec::new(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: HashMap::new(),
        vwacp: None,
    }
}

// 下列行为用例覆盖合法往返、字段默认值和无效形状拒绝。
#[test]
fn exit_code_constants_match_public_contract() {
    assert_eq!(EXIT_CODE_SUCCESS, 0);
    assert_eq!(EXIT_CODE_ERROR, 1);
    assert_eq!(EXIT_CODE_USAGE, 2);
    assert_eq!(EXIT_CODE_TIMEOUT, 3);
    assert_eq!(EXIT_CODE_NO_SESSION, 4);
    assert_eq!(EXIT_CODE_PERMISSION_DENIED, 5);
    assert_eq!(EXIT_CODE_INTERRUPTED, 130);
}

#[test]
fn enum_serialization_uses_stable_wire_literals() {
    assert_eq!(serde_json::to_value(OutputFormat::Text).unwrap(), json!("text"));
    assert_eq!(serde_json::to_value(OutputFormat::Json).unwrap(), json!("json"));
    assert_eq!(serde_json::to_value(OutputFormat::Quiet).unwrap(), json!("quiet"));

    assert_eq!(serde_json::to_value(PermissionMode::ApproveAll).unwrap(), json!("approve-all"));
    assert_eq!(serde_json::to_value(PermissionMode::ApproveReads).unwrap(), json!("approve-reads"));
    assert_eq!(serde_json::to_value(PermissionMode::DenyAll).unwrap(), json!("deny-all"));
    assert_eq!(PermissionMode::default(), PermissionMode::ApproveAll);

    assert_eq!(serde_json::to_value(AuthPolicy::Skip).unwrap(), json!("skip"));
    assert_eq!(serde_json::to_value(AuthPolicy::Fail).unwrap(), json!("fail"));
    assert_eq!(serde_json::to_value(NonInteractivePermissionPolicy::Deny).unwrap(), json!("deny"));
    assert_eq!(serde_json::to_value(NonInteractivePermissionPolicy::Fail).unwrap(), json!("fail"));
    assert_eq!(serde_json::to_value(SessionResumePolicy::AllowNew).unwrap(), json!("allow-new"));
    assert_eq!(
        serde_json::to_value(SessionResumePolicy::SameSessionOnly).unwrap(),
        json!("same-session-only")
    );
    assert_eq!(serde_json::to_value(OutputStream::Prompt).unwrap(), json!("prompt"));
    assert_eq!(serde_json::to_value(OutputStream::Control).unwrap(), json!("control"));
    assert_eq!(serde_json::to_value(AcpMessageDirection::Outbound).unwrap(), json!("outbound"));
    assert_eq!(serde_json::to_value(AcpMessageDirection::Inbound).unwrap(), json!("inbound"));
    assert_eq!(serde_json::to_value(ClientOperationStatus::Running).unwrap(), json!("running"));
    assert_eq!(serde_json::to_value(ClientOperationStatus::Completed).unwrap(), json!("completed"));
    assert_eq!(serde_json::to_value(ClientOperationStatus::Failed).unwrap(), json!("failed"));
}

#[test]
fn error_and_operation_codes_keep_protocol_names() {
    assert_eq!(serde_json::to_value(OutputErrorCode::NoSession).unwrap(), json!("NO_SESSION"));
    assert_eq!(serde_json::to_value(OutputErrorCode::Timeout).unwrap(), json!("TIMEOUT"));
    assert_eq!(
        serde_json::to_value(OutputErrorCode::PermissionPromptUnavailable).unwrap(),
        json!("PERMISSION_PROMPT_UNAVAILABLE")
    );
    assert_eq!(serde_json::to_value(OutputErrorOrigin::Cli).unwrap(), json!("cli"));
    assert_eq!(serde_json::to_value(OutputErrorOrigin::Runtime).unwrap(), json!("runtime"));
    assert_eq!(
        serde_json::to_value(QueueErrorDetailCode::QueueRequestPayloadInvalidJson).unwrap(),
        json!("QUEUE_REQUEST_PAYLOAD_INVALID_JSON")
    );
    assert_eq!(
        serde_json::to_value(QueueErrorDetailCode::QueueRuntimePromptFailed).unwrap(),
        json!("QUEUE_RUNTIME_PROMPT_FAILED")
    );
    assert_eq!(
        serde_json::to_value(ClientOperationMethod::FsReadTextFile).unwrap(),
        json!("fs/read_text_file")
    );
    assert_eq!(
        serde_json::to_value(ClientOperationMethod::TerminalWaitForExit).unwrap(),
        json!("terminal/wait_for_exit")
    );
}

#[test]
fn optional_fields_are_omitted_from_serialized_output() {
    let acp_payload =
        OutputErrorAcpPayload { code: -32000, message: "agent failed".to_string(), data: None };
    assert_eq!(
        serde_json::to_value(acp_payload).unwrap(),
        json!({
            "code": -32000,
            "message": "agent failed"
        })
    );

    let params = OutputErrorParams {
        code: OutputErrorCode::Runtime,
        detail_code: None,
        origin: Some(OutputErrorOrigin::Acp),
        message: "runtime failed".to_string(),
        retryable: None,
        acp: None,
        timestamp: None,
    };
    assert_eq!(
        serde_json::to_value(params).unwrap(),
        json!({
            "code": "RUNTIME",
            "origin": "acp",
            "message": "runtime failed"
        })
    );

    let operation = ClientOperation {
        method: ClientOperationMethod::TerminalCreate,
        status: ClientOperationStatus::Completed,
        summary: "created terminal".to_string(),
        details: None,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
    };
    assert_eq!(
        serde_json::to_value(operation).unwrap(),
        json!({
            "method": "terminal/create",
            "status": "completed",
            "summary": "created terminal",
            "timestamp": "2026-01-01T00:00:00Z"
        })
    );
}

#[test]
fn dto_structs_use_expected_field_names_and_defaults() {
    assert_eq!(
        serde_json::to_value(OutputFormatterContext { session_id: "session-1".to_string() })
            .unwrap(),
        json!({
            "sessionId": "session-1"
        })
    );

    let policy = OutputPolicy {
        format: OutputFormat::Json,
        json_strict: true,
        suppress_reads: true,
        suppress_non_json_stderr: false,
        queue_error_already_emitted: true,
        suppress_sdk_console_errors: false,
    };
    assert_eq!(
        serde_json::to_value(policy).unwrap(),
        json!({
            "format": "json",
            "jsonStrict": true,
            "suppressReads": true,
            "suppressNonJsonStderr": false,
            "queueErrorAlreadyEmitted": true,
            "suppressSdkConsoleErrors": false
        })
    );

    assert_eq!(
        serde_json::to_value(OutputErrorEmissionPolicy { queue_error_already_emitted: true })
            .unwrap(),
        json!({
            "queueErrorAlreadyEmitted": true
        })
    );

    assert_eq!(
        serde_json::to_value(AcpAgentConfig {
            command: "agent".to_string(),
            args: Vec::new(),
            env: HashMap::new(),
        })
        .unwrap(),
        json!({
            "command": "agent"
        })
    );

    let mut counters = HashMap::new();
    counters.insert("requests".to_string(), 2);
    let mut timings = HashMap::new();
    timings
        .insert("prompt".to_string(), PerfMetricSummary { count: 1, total_ms: 12.5, max_ms: 12.5 });
    let snapshot = PerfMetricsSnapshot { counters, timings, gauges: HashMap::new() };
    assert_eq!(
        serde_json::to_value(snapshot).unwrap(),
        json!({
            "counters": {
                "requests": 2
            },
            "timings": {
                "prompt": {
                    "count": 1,
                    "totalMs": 12.5,
                    "maxMs": 12.5
                }
            },
            "gauges": {}
        })
    );

    let options: AcpSessionOptions = serde_json::from_value(json!({})).unwrap();
    assert_eq!(options, AcpSessionOptions::default());
}

#[test]
fn prompt_request_defaults_to_new_session_and_accepts_strategy_override() {
    let request = PromptRequest::new(PathBuf::from("/tmp/project"), "hello");

    assert_eq!(request.cwd, PathBuf::from("/tmp/project"));
    assert_eq!(request.prompt, "hello");
    assert_eq!(request.session_strategy, SessionStrategy::New);

    let request = request.with_session_strategy(SessionStrategy::Resume("session-1".to_string()));
    assert_eq!(request.session_strategy, SessionStrategy::Resume("session-1".to_string()));
}

#[test]
fn session_send_outcome_serializes_direct_and_queued_results() {
    let direct = SessionSendOutcome::SessionSendResult(Box::new(SessionSendResult {
        stop_reason: StopReason::EndTurn,
        permission_stats: PermissionStats { requested: 2, approved: 1, denied: 1, cancelled: 0 },
        session_id: "session-1".to_string(),
        record: test_record(),
        resumed: true,
        load_error: Some("record was stale".to_string()),
    }));
    let value = serde_json::to_value(direct).unwrap();
    assert_eq!(value["stopReason"], json!("end_turn"));
    assert_eq!(value["permissionStats"]["requested"], json!(2));
    assert_eq!(value["record"]["schema"], json!(SESSION_RECORD_SCHEMA));
    assert_eq!(value["resumed"], json!(true));
    assert_eq!(value["loadError"], json!("record was stale"));

    let queued = SessionSendOutcome::SessionEnqueueResult(SessionEnqueueResult {
        queued: true,
        session_id: "session-1".to_string(),
        request_id: "request-1".to_string(),
    });

    assert_eq!(
        serde_json::to_value(queued).unwrap(),
        json!({
            "queued": true,
            "sessionId": "session-1",
            "requestId": "request-1"
        })
    );
}

#[test]
fn nested_session_message_content_round_trips() {
    let mut tool_results = HashMap::new();
    tool_results.insert(
        "tool-1".to_string(),
        SessionToolResult {
            tool_use_id: "tool-1".to_string(),
            tool_name: "inspect".to_string(),
            is_error: false,
            content: SessionToolResultContent::Image(SessionMessageImage {
                source: "data:image/png;base64,AA==".to_string(),
                size: Some(SessionMessageImageSize { width: 640, height: 480 }),
            }),
            output: Some(json!({ "ok": true })),
            result: None,
        },
    );

    let message = SessionMessage::Agent(SessionAgentMessage {
        content: vec![
            SessionAgentContent::Text("hello".to_string()),
            SessionAgentContent::Thinking(SessionThinking {
                text: "reasoning".to_string(),
                signature: Some("sig-1".to_string()),
            }),
            SessionAgentContent::RedactedThinking("hidden".to_string()),
            SessionAgentContent::ToolUse(SessionToolUse {
                id: "tool-1".to_string(),
                name: "inspect".to_string(),
                raw_input: "{\"path\":\"src\"}".to_string(),
                input: json!({ "path": "src" }),
                is_input_complete: true,
                thought_signature: Some("tool-sig".to_string()),
            }),
        ],
        tool_results,
        reasoning_details: Some(json!({ "provider": "agent" })),
    });

    let value = serde_json::to_value(&message).unwrap();
    assert_eq!(value["Agent"]["content"][1]["Thinking"]["signature"], json!("sig-1"));
    assert_eq!(
        value["Agent"]["tool_results"]["tool-1"]["content"]["Image"]["size"]["width"],
        json!(640)
    );

    let round_trip: SessionMessage = serde_json::from_value(value).unwrap();
    assert_eq!(round_trip, message);
}

#[test]
fn invalid_protocol_literals_are_rejected() {
    assert!(serde_json::from_value::<OutputFormat>(json!("verbose")).is_err());
    assert!(serde_json::from_value::<PermissionMode>(json!("approve_all")).is_err());
    assert!(serde_json::from_value::<OutputErrorCode>(json!("permission_denied")).is_err());
    assert!(serde_json::from_value::<ClientOperationMethod>(json!("terminal/wait")).is_err());
}
