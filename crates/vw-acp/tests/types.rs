//! 验证 ACP 会话类型与 TypeScript 兼容的 JSON 形状。
//!
//! 这些断言保护跨语言契约：枚举使用 TS 风格 union，记录结构保留 camelCase
//! 字段名，权限模式序列化为既有字面量。

use std::collections::HashMap;

use serde_json::json;
use vw_acp::{
    PermissionMode, SESSION_RECORD_SCHEMA, SessionAgentContent, SessionAgentMessage,
    SessionEventLog, SessionMessage, SessionRecord, SessionTokenUsage, SessionToolResult,
    SessionToolResultContent, SessionUserContent, SessionUserMessage,
};
use vw_api_tools::{ToolResultContentDto, ToolResultDto};

#[test]
fn session_message_serializes_ts_style_union_shapes() {
    // 同时覆盖用户消息、助手消息和工具结果，避免只验证浅层枚举标签。
    let user_message = SessionMessage::User(SessionUserMessage {
        id: "user-1".to_string(),
        content: vec![SessionUserContent::Text("hello".to_string())],
    });
    let mut tool_results = HashMap::new();
    tool_results.insert(
        "tool-1".to_string(),
        SessionToolResult {
            tool_use_id: "tool-1".to_string(),
            tool_name: "apply_patch".to_string(),
            is_error: false,
            content: SessionToolResultContent::Text("Updated 1 file".to_string()),
            output: None,
            result: Some(ToolResultDto {
                tool_use_id: Some("tool-1".to_string()),
                tool_id: Some("apply_patch".into()),
                success: Some(true),
                content: vec![ToolResultContentDto::Text {
                    text: "patched src/main.rs".to_string(),
                }],
                data: json!({"changed_files": ["src/main.rs"]}),
                model_result: json!("patched src/main.rs"),
                render_hint: None,
                permission_request: None,
                context_updates: Vec::new(),
                extra_messages: Vec::new(),
                telemetry: None,
            }),
        },
    );
    let agent_message = SessionMessage::Agent(SessionAgentMessage {
        content: vec![SessionAgentContent::Text("world".to_string())],
        tool_results,
        reasoning_details: None,
    });

    assert_eq!(
        serde_json::to_value(&user_message).unwrap(),
        json!({
            "User": {
                "id": "user-1",
                "content": [
                    {
                        "Text": "hello"
                    }
                ]
            }
        })
    );
    assert_eq!(
        serde_json::to_value(&agent_message).unwrap(),
        json!({
            "Agent": {
                "content": [
                    {
                        "Text": "world"
                    }
                ],
                "tool_results": {
                    "tool-1": {
                        "tool_use_id": "tool-1",
                        "tool_name": "apply_patch",
                        "is_error": false,
                        "content": {
                            "Text": "Updated 1 file"
                        },
                        "result": {
                            "tool_use_id": "tool-1",
                            "tool_id": "apply_patch",
                            "success": true,
                            "content": [
                                {
                                    "type": "text",
                                    "text": "patched src/main.rs"
                                }
                            ],
                            "data": {
                                "changed_files": ["src/main.rs"]
                            },
                            "model_result": "patched src/main.rs"
                        }
                    }
                }
            }
        })
    );
    assert_eq!(serde_json::to_value(SessionMessage::Resume).unwrap(), json!("Resume"));
}

#[test]
fn session_record_serializes_ts_field_names() {
    // SessionRecord 的 Rust 字段名是 snake_case，但外部 API 仍依赖 TS 端的
    // camelCase 名称；这里固定该边界形状，避免持久化/前端读取回归。
    let record = SessionRecord {
        schema: SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: "record-1".to_string(),
        acp_session_id: "session-1".to_string(),
        agent_session_id: None,
        agent_command: "acp-agent".to_string(),
        agent_config: None,
        cwd: "/tmp/project".to_string(),
        name: Some("demo".to_string()),
        created_at: "2026-04-03T00:00:00Z".to_string(),
        last_used_at: "2026-04-03T00:01:00Z".to_string(),
        last_seq: 12,
        last_request_id: Some("req-1".to_string()),
        event_log: SessionEventLog {
            active_path: "/tmp/project/.vwacp/session.log".to_string(),
            segment_count: 1,
            max_segment_bytes: 1024,
            max_segments: 8,
            last_write_at: Some("2026-04-03T00:01:00Z".to_string()),
            last_write_error: None,
        },
        closed: Some(false),
        closed_at: None,
        pid: Some(321),
        agent_started_at: Some("2026-04-03T00:00:10Z".to_string()),
        last_prompt_at: None,
        last_agent_exit_code: None,
        last_agent_exit_signal: None,
        last_agent_exit_at: None,
        last_agent_disconnect_reason: None,
        protocol_version: Some(1),
        agent_capabilities: None,
        title: Some("ACP Demo".to_string()),
        messages: vec![],
        updated_at: "2026-04-03T00:01:00Z".to_string(),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: HashMap::new(),
        vwacp: None,
    };

    let value = serde_json::to_value(&record).unwrap();

    assert_eq!(value["vwacpRecordId"], json!("record-1"));
    assert_eq!(value["acpSessionId"], json!("session-1"));
    assert_eq!(value["agentCommand"], json!("acp-agent"));
    assert_eq!(value["createdAt"], json!("2026-04-03T00:00:00Z"));
    assert_eq!(value["lastUsedAt"], json!("2026-04-03T00:01:00Z"));
    assert_eq!(value["lastSeq"], json!(12));
    assert_eq!(value["eventLog"]["active_path"], json!("/tmp/project/.vwacp/session.log"));
}

#[test]
fn permission_mode_matches_ts_literals() {
    assert_eq!(serde_json::to_string(&PermissionMode::ApproveReads).unwrap(), "\"approve-reads\"");
}
