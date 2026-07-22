//! 验证 ACP 会话记录在磁盘序列化、反序列化与索引重建时保持兼容形状。
//!
//! 这些测试覆盖持久化边界：写入磁盘使用稳定的 snake_case 字段，
//! 读取时恢复运行时结构，并从会话文件重新生成索引文件。

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use vw_acp::{
    SESSION_RECORD_SCHEMA, SessionAcpxState, SessionEventLog, SessionIndexEntry, SessionMessage,
    SessionRecord, SessionStateOptions, SessionTokenUsage, parse_session_record,
    rebuild_session_index, serialize_session_record_for_disk,
};

/// 为单个测试生成临时目录路径。
///
/// 参数 `name` 用于标识调用场景，返回值尚未创建目录；调用方可按需创建并清理。
fn temp_dir(name: &str) -> PathBuf {
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    std::env::temp_dir().join(format!("vw-acp-{name}-{}-{suffix}", std::process::id()))
}

/// 构造包含主要持久化字段的会话记录样本。
///
/// 返回值刻意包含消息、token 用量和 vwacp 扩展状态，确保序列化测试覆盖
/// 会话恢复所需的嵌套结构。
fn sample_record() -> SessionRecord {
    SessionRecord {
        schema: SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: "record-1".to_string(),
        acp_session_id: "session-1".to_string(),
        agent_session_id: Some(" runtime-session ".trim().to_string()),
        agent_command: "acp-agent".to_string(),
        agent_config: None,
        cwd: "/tmp/project".to_string(),
        name: Some("demo".to_string()),
        created_at: "2026-04-03T00:00:00Z".to_string(),
        last_used_at: "2026-04-03T00:01:00Z".to_string(),
        last_seq: 3,
        last_request_id: Some("req-1".to_string()),
        event_log: SessionEventLog {
            active_path: "/tmp/project/.vwacp/sessions/record-1.stream.ndjson".to_string(),
            segment_count: 1,
            max_segment_bytes: 4096,
            max_segments: 8,
            last_write_at: Some("2026-04-03T00:01:00Z".to_string()),
            last_write_error: None,
        },
        closed: Some(false),
        closed_at: None,
        pid: Some(321),
        agent_started_at: Some("2026-04-03T00:00:10Z".to_string()),
        last_prompt_at: Some("2026-04-03T00:00:20Z".to_string()),
        last_agent_exit_code: None,
        last_agent_exit_signal: None,
        last_agent_exit_at: None,
        last_agent_disconnect_reason: None,
        protocol_version: Some(1),
        agent_capabilities: None,
        title: Some("ACP Demo".to_string()),
        messages: vec![SessionMessage::Resume],
        updated_at: "2026-04-03T00:01:00Z".to_string(),
        cumulative_token_usage: SessionTokenUsage {
            input_tokens: Some(10),
            output_tokens: Some(20),
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        },
        request_token_usage: HashMap::from([(
            "prompt".to_string(),
            SessionTokenUsage {
                input_tokens: Some(4),
                output_tokens: Some(8),
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
            },
        )]),
        vwacp: Some(SessionAcpxState {
            current_mode_id: Some("chat".to_string()),
            desired_mode_id: Some("plan".to_string()),
            current_model_id: Some("claude".to_string()),
            available_models: Some(vec!["claude".to_string(), "gemini".to_string()]),
            available_commands: Some(vec!["/help".to_string()]),
            config_options: None,
            desired_config_options: None,
            session_options: Some(SessionStateOptions {
                model: Some("claude".to_string()),
                allowed_tools: Some(vec!["read".to_string()]),
                max_turns: Some(5),
            }),
        }),
    }
}

#[test]
fn serialize_session_record_for_disk_uses_snake_case_fields() {
    let value = serialize_session_record_for_disk(&sample_record());

    assert_eq!(value["schema"], json!(SESSION_RECORD_SCHEMA));
    assert_eq!(value["vwacp_record_id"], json!("record-1"));
    assert_eq!(value["acp_session_id"], json!("session-1"));
    assert_eq!(value["agent_session_id"], json!("runtime-session"));
    assert_eq!(value["agent_command"], json!("acp-agent"));
    assert_eq!(value["last_used_at"], json!("2026-04-03T00:01:00Z"));
    assert_eq!(
        value["event_log"]["active_path"],
        json!("/tmp/project/.vwacp/sessions/record-1.stream.ndjson")
    );
    assert_eq!(value["messages"], json!(["Resume"]));
    assert!(value.get("closed_at").is_none());
}

#[test]
fn parse_session_record_restores_runtime_shape() {
    let raw = json!({
        "schema": SESSION_RECORD_SCHEMA,
        "vwacp_record_id": "record-1",
        "acp_session_id": "session-1",
        "agent_session_id": " runtime-session ",
        "agent_command": "acp-agent",
        "cwd": "/tmp/project",
        "name": "  demo  ",
        "created_at": "2026-04-03T00:00:00Z",
        "last_used_at": "2026-04-03T00:01:00Z",
        "last_seq": 3,
        "last_request_id": "req-1",
        "event_log": {
            "active_path": "/tmp/project/.vwacp/sessions/record-1.stream.ndjson",
            "segment_count": 1,
            "max_segment_bytes": 4096,
            "max_segments": 8
        },
        "closed": false,
        "pid": 321,
        "agent_started_at": "2026-04-03T00:00:10Z",
        "last_prompt_at": "2026-04-03T00:00:20Z",
        "protocol_version": 1,
        "title": "ACP Demo",
        "messages": ["Resume"],
        "updated_at": "2026-04-03T00:01:00Z",
        "cumulative_token_usage": {
            "input_tokens": 10,
            "output_tokens": 20
        },
        "request_token_usage": {
            "prompt": {
                "input_tokens": 4,
                "output_tokens": 8
            }
        },
        "vwacp": {
            "current_mode_id": "chat",
            "desired_mode_id": "plan",
            "current_model_id": "claude",
            "available_models": ["claude", "gemini"],
            "available_commands": ["/help"],
            "session_options": {
                "model": "claude",
                "allowed_tools": ["read"],
                "max_turns": 5
            }
        }
    });

    let parsed = parse_session_record(&raw).unwrap();

    assert_eq!(parsed.vwacp_record_id, "record-1");
    assert_eq!(parsed.agent_session_id.as_deref(), Some("runtime-session"));
    assert_eq!(parsed.name.as_deref(), Some("demo"));
    assert_eq!(parsed.closed, Some(false));
    assert_eq!(parsed.messages, vec![SessionMessage::Resume]);
    assert_eq!(
        parsed.request_token_usage.get("prompt").and_then(|usage| usage.output_tokens),
        Some(8)
    );
    assert_eq!(
        parsed
            .vwacp
            .as_ref()
            .and_then(|state| state.session_options.as_ref())
            .and_then(|options| options.max_turns),
        Some(5)
    );
}

#[tokio::test]
async fn rebuild_session_index_reads_session_files() {
    let session_dir = temp_dir("session-index");
    fs::create_dir_all(&session_dir).unwrap();

    let record = sample_record();
    let payload = serde_json::to_vec_pretty(&serialize_session_record_for_disk(&record)).unwrap();
    // 真实会话文件通常以换行结尾；这里保留该形状，避免索引重建只适配紧凑 JSON。
    fs::write(session_dir.join("record-1.json"), [&payload[..], b"\n"].concat()).unwrap();

    let index = rebuild_session_index(&session_dir).await.unwrap();

    assert_eq!(index.files, vec!["record-1.json".to_string()]);
    assert_eq!(
        index.entries,
        vec![SessionIndexEntry {
            file: "record-1.json".to_string(),
            vwacp_record_id: "record-1".to_string(),
            acp_session_id: "session-1".to_string(),
            agent_command: "acp-agent".to_string(),
            cwd: "/tmp/project".to_string(),
            name: Some("demo".to_string()),
            closed: false,
            last_used_at: "2026-04-03T00:01:00Z".to_string(),
        }]
    );

    let persisted_index: Value =
        serde_json::from_slice(&fs::read(session_dir.join("index.json")).unwrap()).unwrap();
    assert_eq!(persisted_index["schema"], json!("vwacp.session-index.v1"));

    let _ = fs::remove_dir_all(&session_dir);
}
