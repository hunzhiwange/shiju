//! CLI 输出渲染行为的单元测试。

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::MAIN_SEPARATOR;

use serde_json::{Value, json};
use vw_api_types::tools::{RenderHintDto, ToolResultDto};

use crate::cli::output_render::{
    SessionConnectionStatus, agent_session_id_payload, conversation_history_entries,
    format_prompt_session_banner_line, print_cancel_result_by_format,
    print_closed_session_by_format, print_created_session_banner, print_ensured_session_by_format,
    print_new_session_by_format, print_prompt_session_banner, print_queued_prompt_by_format,
    print_session_details_by_format, print_session_history_by_format, print_sessions_by_format,
    print_set_config_option_result_by_format, print_set_mode_result_by_format,
    print_set_model_result_by_format, write_cancel_result_by_format,
    write_closed_session_by_format, write_created_session_banner, write_ensured_session_by_format,
    write_new_session_by_format, write_queued_prompt_by_format, write_session_details_by_format,
    write_session_history_by_format, write_sessions_by_format,
    write_set_config_option_result_by_format, write_set_mode_result_by_format,
    write_set_model_result_by_format,
};
use crate::{
    OutputFormat, SESSION_RECORD_SCHEMA, SessionAcpxState, SessionAgentContent,
    SessionAgentMessage, SessionEnqueueResult, SessionEventLog, SessionMessage,
    SessionMessageImage, SessionMessageImageSize, SessionRecord, SessionThinking,
    SessionTokenUsage, SessionToolResult, SessionToolResultContent, SessionToolUse,
    SessionUserContent, SessionUserMessage,
};

fn sample_record() -> SessionRecord {
    let mut tool_results = HashMap::new();
    tool_results.insert(
        "tool-1".to_string(),
        SessionToolResult {
            tool_use_id: "tool-1".to_string(),
            tool_name: "shell".to_string(),
            is_error: false,
            content: SessionToolResultContent::Text("done".to_string()),
            output: None,
            result: None,
        },
    );

    SessionRecord {
        schema: SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: "record-1".to_string(),
        acp_session_id: "session-1".to_string(),
        agent_session_id: Some("agent-1".to_string()),
        agent_command: "agent".to_string(),
        agent_config: None,
        cwd: "/tmp/project".to_string(),
        name: Some("workspace".to_string()),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_used_at: "2026-01-01T00:02:00Z".to_string(),
        last_seq: 0,
        last_request_id: None,
        event_log: SessionEventLog {
            active_path: "/tmp/project/active.jsonl".to_string(),
            segment_count: 1,
            max_segment_bytes: 1024,
            max_segments: 4,
            last_write_at: None,
            last_write_error: None,
        },
        closed: Some(false),
        closed_at: None,
        pid: Some(42),
        agent_started_at: Some("2026-01-01T00:00:00Z".to_string()),
        last_prompt_at: Some("2026-01-01T00:01:00Z".to_string()),
        last_agent_exit_code: None,
        last_agent_exit_signal: None,
        last_agent_exit_at: None,
        last_agent_disconnect_reason: None,
        protocol_version: None,
        agent_capabilities: None,
        title: Some("Demo".to_string()),
        messages: vec![
            SessionMessage::User(SessionUserMessage {
                id: "user-1".to_string(),
                content: vec![
                    SessionUserContent::Text("hello".to_string()),
                    SessionUserContent::Mention(crate::SessionMention {
                        uri: "file:///tmp/project/readme.md".to_string(),
                        content: "README".to_string(),
                    }),
                ],
            }),
            SessionMessage::Agent(SessionAgentMessage {
                content: vec![
                    SessionAgentContent::Text("hi".to_string()),
                    SessionAgentContent::ToolUse(SessionToolUse {
                        id: "tool-1".to_string(),
                        name: "shell".to_string(),
                        raw_input: "{\"cmd\":\"pwd\"}".to_string(),
                        input: Value::Null,
                        is_input_complete: true,
                        thought_signature: None,
                    }),
                ],
                tool_results,
                reasoning_details: None,
            }),
            SessionMessage::Resume,
        ],
        updated_at: "2026-01-01T00:03:00Z".to_string(),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: HashMap::new(),
        vwacp: Some(SessionAcpxState {
            current_mode_id: Some("focus".to_string()),
            desired_mode_id: Some("focus".to_string()),
            current_model_id: Some("model-a".to_string()),
            available_models: Some(vec!["model-a".to_string(), "model-b".to_string()]),
            available_commands: None,
            config_options: None,
            desired_config_options: None,
            session_options: None,
        }),
    }
}

fn rendered_output(output: Vec<u8>) -> String {
    String::from_utf8(output).unwrap()
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("write failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct NewlineRejectingWriter {
    output: Vec<u8>,
}

impl Write for NewlineRejectingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf == b"\n" {
            return Err(io::Error::other("newline rejected"));
        }

        self.output.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn conversation_history_entries_summarize_user_and_agent_messages() {
    let entries = conversation_history_entries(&sample_record());

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].role, "user");
    assert_eq!(entries[0].text_preview, "hello\nREADME");
    assert_eq!(entries[1].role, "assistant");
    assert_eq!(entries[1].text_preview, "hi\n[tool:shell]\ndone");
}

#[test]
fn conversation_history_entries_summarize_media_and_render_hint_fallbacks() {
    let mut record = sample_record();
    let mut tool_results = HashMap::new();
    tool_results.insert(
        "b-tool".to_string(),
        SessionToolResult {
            tool_use_id: "b-tool".to_string(),
            tool_name: "image".to_string(),
            is_error: false,
            content: SessionToolResultContent::Image(SessionMessageImage {
                source: "file:///tmp/image.png".to_string(),
                size: Some(SessionMessageImageSize { width: 10, height: 20 }),
            }),
            output: None,
            result: None,
        },
    );
    tool_results.insert(
        "a-tool".to_string(),
        SessionToolResult {
            tool_use_id: "a-tool".to_string(),
            tool_name: "empty-text".to_string(),
            is_error: false,
            content: SessionToolResultContent::Text("   ".to_string()),
            output: None,
            result: Some(ToolResultDto {
                tool_use_id: Some("a-tool".to_string()),
                tool_id: Some("empty-text".into()),
                success: Some(true),
                content: Vec::new(),
                data: Value::Null,
                model_result: Value::Null,
                render_hint: Some(RenderHintDto {
                    summary: Some("  fallback summary  ".to_string()),
                    ..RenderHintDto::default()
                }),
                permission_request: None,
                context_updates: Vec::new(),
                extra_messages: Vec::new(),
                telemetry: None,
            }),
        },
    );
    record.messages = vec![
        SessionMessage::User(SessionUserMessage {
            id: "user-2".to_string(),
            content: vec![
                SessionUserContent::Text("   ".to_string()),
                SessionUserContent::Mention(crate::SessionMention {
                    uri: "file:///tmp/project/empty.md".to_string(),
                    content: "  ".to_string(),
                }),
                SessionUserContent::Image(SessionMessageImage {
                    source: "file:///tmp/project/screenshot.png".to_string(),
                    size: None,
                }),
            ],
        }),
        SessionMessage::Agent(SessionAgentMessage {
            content: vec![
                SessionAgentContent::Text(" ".to_string()),
                SessionAgentContent::Thinking(SessionThinking {
                    text: "hidden".to_string(),
                    signature: None,
                }),
                SessionAgentContent::RedactedThinking("redacted".to_string()),
                SessionAgentContent::ToolUse(SessionToolUse {
                    id: "tool-use".to_string(),
                    name: "search".to_string(),
                    raw_input: "{}".to_string(),
                    input: Value::Null,
                    is_input_complete: true,
                    thought_signature: None,
                }),
            ],
            tool_results,
            reasoning_details: None,
        }),
        SessionMessage::User(SessionUserMessage {
            id: "empty-user".to_string(),
            content: vec![SessionUserContent::Text(" ".to_string())],
        }),
    ];

    let entries = conversation_history_entries(&record);

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].text_preview, "file:///tmp/project/empty.md\n[image]");
    assert_eq!(
        entries[1].text_preview,
        "[thinking]\n[thinking redacted]\n[tool:search]\nfallback summary\n[image]"
    );
}

#[test]
fn conversation_history_entries_skip_empty_tool_result_fallbacks() {
    let mut record = sample_record();
    let mut tool_results = HashMap::new();
    tool_results.insert(
        "tool".to_string(),
        SessionToolResult {
            tool_use_id: "tool".to_string(),
            tool_name: "empty".to_string(),
            is_error: false,
            content: SessionToolResultContent::Text(" ".to_string()),
            output: None,
            result: Some(ToolResultDto {
                tool_use_id: Some("tool".to_string()),
                tool_id: Some("empty".into()),
                success: Some(true),
                content: Vec::new(),
                data: Value::Null,
                model_result: Value::Null,
                render_hint: Some(RenderHintDto {
                    summary: Some(" ".to_string()),
                    ..RenderHintDto::default()
                }),
                permission_request: None,
                context_updates: Vec::new(),
                extra_messages: Vec::new(),
                telemetry: None,
            }),
        },
    );
    record.messages = vec![SessionMessage::Agent(SessionAgentMessage {
        content: Vec::new(),
        tool_results,
        reasoning_details: None,
    })];

    assert!(conversation_history_entries(&record).is_empty());
}

#[test]
fn write_sessions_by_format_renders_all_formats() {
    let mut open = sample_record();
    open.name = None;
    let mut closed = sample_record();
    closed.vwacp_record_id = "record-closed".to_string();
    closed.name = Some("closed-name".to_string());
    closed.closed = Some(true);
    let sessions = vec![open, closed];

    let mut json_output = Vec::new();
    write_sessions_by_format(&mut json_output, &sessions, OutputFormat::Json).unwrap();
    let json_value: Value = serde_json::from_slice(&json_output).unwrap();
    assert_eq!(json_value[0]["vwacpRecordId"], "record-1");
    assert_eq!(json_value[1]["vwacpRecordId"], "record-closed");

    let mut quiet_output = Vec::new();
    write_sessions_by_format(&mut quiet_output, &sessions, OutputFormat::Quiet).unwrap();
    assert_eq!(rendered_output(quiet_output), "record-1\nrecord-closed [closed]\n");

    let mut text_output = Vec::new();
    write_sessions_by_format(&mut text_output, &sessions, OutputFormat::Text).unwrap();
    assert_eq!(
        rendered_output(text_output),
        "record-1\t-\t/tmp/project\t2026-01-01T00:02:00Z\nrecord-closed [closed]\tclosed-name\t/tmp/project\t2026-01-01T00:02:00Z\n"
    );
}

#[test]
fn write_sessions_by_format_reports_empty_text_list() {
    let mut output = Vec::new();

    write_sessions_by_format(&mut output, &[], OutputFormat::Text).unwrap();

    assert_eq!(rendered_output(output), "No sessions\n");
}

#[test]
fn write_sessions_by_format_propagates_writer_errors() {
    let mut writer = FailingWriter;

    let error =
        write_sessions_by_format(&mut writer, &[sample_record()], OutputFormat::Text).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::Other);
}

#[test]
fn write_closed_session_by_format_renders_all_formats() {
    let record = sample_record();

    let mut json_output = Vec::new();
    write_closed_session_by_format(&mut json_output, &record, OutputFormat::Json).unwrap();
    let json_value: Value = serde_json::from_slice(&json_output).unwrap();
    assert_eq!(json_value["action"], "session_closed");
    assert_eq!(json_value["vwacpRecordId"], "record-1");
    assert_eq!(json_value["agentSessionId"], "agent-1");

    let mut quiet_output = Vec::new();
    write_closed_session_by_format(&mut quiet_output, &record, OutputFormat::Quiet).unwrap();
    assert!(quiet_output.is_empty());

    let mut text_output = Vec::new();
    write_closed_session_by_format(&mut text_output, &record, OutputFormat::Text).unwrap();
    assert_eq!(rendered_output(text_output), "record-1\n");
}

#[test]
fn write_new_session_by_format_renders_created_and_replaced() {
    let record = sample_record();
    let mut replaced = sample_record();
    replaced.vwacp_record_id = "old-record".to_string();

    let mut json_output = Vec::new();
    write_new_session_by_format(&mut json_output, &record, Some(&replaced), OutputFormat::Json)
        .unwrap();
    let json_value: Value = serde_json::from_slice(&json_output).unwrap();
    assert_eq!(json_value["action"], "session_ensured");
    assert_eq!(json_value["created"], true);
    assert_eq!(json_value["replacedSessionId"], "old-record");

    let mut quiet_output = Vec::new();
    write_new_session_by_format(&mut quiet_output, &record, None, OutputFormat::Quiet).unwrap();
    assert_eq!(rendered_output(quiet_output), "record-1\n");

    let mut text_output = Vec::new();
    write_new_session_by_format(&mut text_output, &record, Some(&replaced), OutputFormat::Text)
        .unwrap();
    assert_eq!(rendered_output(text_output), "record-1\t(replaced old-record)\n");

    let mut plain_output = Vec::new();
    write_new_session_by_format(&mut plain_output, &record, None, OutputFormat::Text).unwrap();
    assert_eq!(rendered_output(plain_output), "record-1\n");
}

#[test]
fn write_ensured_session_by_format_renders_created_and_existing() {
    let record = sample_record();

    let mut json_output = Vec::new();
    write_ensured_session_by_format(&mut json_output, &record, false, OutputFormat::Json).unwrap();
    let json_value: Value = serde_json::from_slice(&json_output).unwrap();
    assert_eq!(json_value["created"], false);
    assert_eq!(json_value["name"], "workspace");

    let mut quiet_output = Vec::new();
    write_ensured_session_by_format(&mut quiet_output, &record, true, OutputFormat::Quiet).unwrap();
    assert_eq!(rendered_output(quiet_output), "record-1\n");

    let mut created_output = Vec::new();
    write_ensured_session_by_format(&mut created_output, &record, true, OutputFormat::Text)
        .unwrap();
    assert_eq!(rendered_output(created_output), "record-1\t(created)\n");

    let mut existing_output = Vec::new();
    write_ensured_session_by_format(&mut existing_output, &record, false, OutputFormat::Text)
        .unwrap();
    assert_eq!(rendered_output(existing_output), "record-1\t(existing)\n");
}

#[test]
fn write_queued_prompt_by_format_renders_all_formats() {
    let result = SessionEnqueueResult {
        queued: true,
        session_id: "record-1".to_string(),
        request_id: "request-1".to_string(),
    };

    let mut json_output = Vec::new();
    write_queued_prompt_by_format(&mut json_output, &result, OutputFormat::Json).unwrap();
    let json_value: Value = serde_json::from_slice(&json_output).unwrap();
    assert_eq!(json_value["action"], "prompt_queued");
    assert_eq!(json_value["requestId"], "request-1");

    let mut quiet_output = Vec::new();
    write_queued_prompt_by_format(&mut quiet_output, &result, OutputFormat::Quiet).unwrap();
    assert!(quiet_output.is_empty());

    let mut text_output = Vec::new();
    write_queued_prompt_by_format(&mut text_output, &result, OutputFormat::Text).unwrap();
    assert_eq!(rendered_output(text_output), "[queued] request-1\n");
}

#[test]
fn write_session_details_by_format_renders_json_payload() {
    let record = sample_record();
    let mut output = Vec::new();

    write_session_details_by_format(&mut output, &record, OutputFormat::Json).unwrap();

    let value: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["action"], "session_details");
    assert_eq!(value["session"]["vwacpRecordId"], "record-1");
    assert_eq!(value["session"]["agentCommand"], "agent");
}

#[test]
fn write_session_details_by_format_renders_text_and_quiet_payloads() {
    let mut record = sample_record();
    record.agent_session_id = None;
    record.name = None;
    record.title = None;
    record.closed = Some(true);
    record.closed_at = Some("2026-01-01T00:04:00Z".to_string());
    record.last_agent_exit_code = Some(2);
    record.last_agent_exit_signal = Some("SIGTERM".to_string());
    record.last_agent_disconnect_reason = Some("shutdown".to_string());
    record.vwacp = None;

    let mut quiet_output = Vec::new();
    write_session_details_by_format(&mut quiet_output, &record, OutputFormat::Quiet).unwrap();
    assert_eq!(rendered_output(quiet_output), "record-1\n");

    let mut text_output = Vec::new();
    write_session_details_by_format(&mut text_output, &record, OutputFormat::Text).unwrap();
    let rendered = rendered_output(text_output);
    assert!(rendered.contains("session: record-1\n"));
    assert!(rendered.contains("agentSessionId: -\n"));
    assert!(rendered.contains("status: closed\n"));
    assert!(rendered.contains("model: -\n"));
    assert!(rendered.contains("closedAt: 2026-01-01T00:04:00Z\n"));
    assert!(rendered.contains("lastAgentExitCode: 2\n"));
    assert!(rendered.contains("lastAgentExitSignal: SIGTERM\n"));
    assert!(rendered.contains("lastAgentDisconnectReason: shutdown\n"));
}

#[test]
fn write_session_details_by_format_renders_full_text_state() {
    let record = sample_record();
    let mut output = Vec::new();

    write_session_details_by_format(&mut output, &record, OutputFormat::Text).unwrap();

    let rendered = rendered_output(output);
    assert!(rendered.contains("session: record-1\n"));
    assert!(rendered.contains("acpSessionId: session-1\n"));
    assert!(rendered.contains("agentSessionId: agent-1\n"));
    assert!(rendered.contains("agent: agent\n"));
    assert!(rendered.contains("cwd: /tmp/project\n"));
    assert!(rendered.contains("name: workspace\n"));
    assert!(rendered.contains("title: Demo\n"));
    assert!(rendered.contains("status: open\n"));
    assert!(rendered.contains("model: model-a\n"));
    assert!(rendered.contains("mode: focus\n"));
    assert!(rendered.contains("availableModels: model-a, model-b\n"));
    assert!(rendered.contains("createdAt: 2026-01-01T00:00:00Z\n"));
    assert!(rendered.contains("lastUsedAt: 2026-01-01T00:02:00Z\n"));
    assert!(rendered.contains("updatedAt: 2026-01-01T00:03:00Z\n"));
    assert!(rendered.contains("lastPromptAt: 2026-01-01T00:01:00Z\n"));
    assert!(rendered.contains("pid: 42\n"));
    assert!(rendered.contains("closedAt: -\n"));
    assert!(rendered.contains("lastAgentExitCode: -\n"));
    assert!(rendered.contains("lastAgentExitSignal: -\n"));
    assert!(rendered.contains("lastAgentDisconnectReason: -\n"));
}

#[test]
fn write_session_history_by_format_limits_text_output() {
    let record = sample_record();
    let mut output = Vec::new();

    write_session_history_by_format(&mut output, &record, OutputFormat::Text, 1).unwrap();

    assert_eq!(String::from_utf8(output).unwrap(), "[assistant] hi\n[tool:shell]\ndone\n");
}

#[test]
fn write_session_history_by_format_keeps_all_entries_when_limit_exceeds_history() {
    let record = sample_record();
    let mut output = Vec::new();

    write_session_history_by_format(&mut output, &record, OutputFormat::Text, 10).unwrap();

    assert_eq!(
        rendered_output(output),
        "[user] hello\nREADME\n[assistant] hi\n[tool:shell]\ndone\n"
    );
}

#[test]
fn write_session_history_by_format_renders_json_quiet_and_empty_text() {
    let record = sample_record();

    let mut json_output = Vec::new();
    write_session_history_by_format(&mut json_output, &record, OutputFormat::Json, 10).unwrap();
    let json_value: Value = serde_json::from_slice(&json_output).unwrap();
    assert_eq!(json_value["action"], "session_history");
    assert_eq!(json_value["entries"].as_array().unwrap().len(), 2);

    let mut quiet_output = Vec::new();
    write_session_history_by_format(&mut quiet_output, &record, OutputFormat::Quiet, 1).unwrap();
    assert_eq!(rendered_output(quiet_output), "hi\n[tool:shell]\ndone\n");

    let mut empty_record = sample_record();
    empty_record.messages.clear();
    let mut empty_output = Vec::new();
    write_session_history_by_format(&mut empty_output, &empty_record, OutputFormat::Text, 1)
        .unwrap();
    assert_eq!(rendered_output(empty_output), "No conversation history\n");
}

#[test]
fn write_session_history_by_format_handles_zero_limit() {
    let record = sample_record();

    let mut text_output = Vec::new();
    write_session_history_by_format(&mut text_output, &record, OutputFormat::Text, 0).unwrap();
    assert_eq!(rendered_output(text_output), "No conversation history\n");

    let mut json_output = Vec::new();
    write_session_history_by_format(&mut json_output, &record, OutputFormat::Json, 0).unwrap();
    let json_value: Value = serde_json::from_slice(&json_output).unwrap();
    assert_eq!(json_value["entries"].as_array().unwrap().len(), 0);
}

#[test]
fn format_prompt_session_banner_line_renders_status_and_routing() {
    let record = sample_record();

    let same = format_prompt_session_banner_line(
        &record,
        "/tmp/project",
        SessionConnectionStatus::Connected,
    );
    assert_eq!(same, "[vwacp] session workspace (record-1) · /tmp/project · agent connected");

    let nested = format_prompt_session_banner_line(
        &record,
        "/tmp/project/src",
        SessionConnectionStatus::NeedsReconnect,
    );
    assert_eq!(
        nested,
        format!(
            "[vwacp] session workspace (record-1) · /tmp/project (routed from .{MAIN_SEPARATOR}src) · agent needs reconnect"
        )
    );

    let mut unnamed = sample_record();
    unnamed.name = None;
    let sibling = format_prompt_session_banner_line(
        &unnamed,
        "/tmp/other",
        SessionConnectionStatus::Connected,
    );
    assert_eq!(
        sibling,
        format!(
            "[vwacp] session cwd (record-1) · /tmp/project (routed from ..{MAIN_SEPARATOR}other) · agent connected"
        )
    );
}

#[test]
fn format_prompt_session_banner_line_resolves_relative_cwd() {
    let mut record = sample_record();
    record.cwd = "workspace".to_string();

    let rendered =
        format_prompt_session_banner_line(&record, "workspace", SessionConnectionStatus::Connected);
    let expected_cwd = std::env::current_dir().unwrap().join("workspace");

    assert_eq!(
        rendered,
        format!(
            "[vwacp] session workspace (record-1) · {} · agent connected",
            expected_cwd.display()
        )
    );
}

#[test]
fn session_connection_status_display_renders_stable_labels() {
    assert_eq!(SessionConnectionStatus::Connected.to_string(), "connected");
    assert_eq!(SessionConnectionStatus::NeedsReconnect.to_string(), "needs reconnect");
}

#[test]
fn write_created_session_banner_renders_label_agent_and_cwd() {
    let record = sample_record();
    let mut output = Vec::new();

    write_created_session_banner(&mut output, &record, "codex").unwrap();

    assert_eq!(
        rendered_output(output),
        "[vwacp] created session workspace (record-1)\n[vwacp] agent: codex\n[vwacp] cwd: /tmp/project\n"
    );
}

#[test]
fn write_created_session_banner_uses_cwd_label_when_name_is_missing() {
    let mut record = sample_record();
    record.name = None;
    let mut output = Vec::new();

    write_created_session_banner(&mut output, &record, "codex").unwrap();

    assert!(rendered_output(output).starts_with("[vwacp] created session cwd (record-1)\n"));
}

#[test]
fn agent_session_id_payload_normalizes_supported_shapes() {
    let string_payload = agent_session_id_payload(Some("agent-1"));
    assert_eq!(string_payload.agent_session_id.as_deref(), Some("agent-1"));

    let empty_payload = agent_session_id_payload(Some(" "));
    assert_eq!(empty_payload.agent_session_id, None);

    let none_payload = agent_session_id_payload(None);
    assert_eq!(none_payload.agent_session_id, None);
}

#[test]
fn agent_session_id_payload_serializes_absent_id_as_empty_object() {
    let payload = agent_session_id_payload(Some(" "));

    assert_eq!(serde_json::to_value(payload).unwrap(), json!({}));
}

#[test]
fn write_cancel_result_by_format_renders_all_formats() {
    let mut json_output = Vec::new();
    write_cancel_result_by_format(&mut json_output, true, OutputFormat::Json).unwrap();
    let json_value: Value = serde_json::from_slice(&json_output).unwrap();
    assert_eq!(json_value, json!({"action": "cancel", "cancelled": true}));

    let mut quiet_output = Vec::new();
    write_cancel_result_by_format(&mut quiet_output, true, OutputFormat::Quiet).unwrap();
    assert!(quiet_output.is_empty());

    let mut cancelled_output = Vec::new();
    write_cancel_result_by_format(&mut cancelled_output, true, OutputFormat::Text).unwrap();
    assert_eq!(rendered_output(cancelled_output), "Cancelled\n");

    let mut not_cancelled_output = Vec::new();
    write_cancel_result_by_format(&mut not_cancelled_output, false, OutputFormat::Text).unwrap();
    assert_eq!(rendered_output(not_cancelled_output), "Not cancelled\n");
}

#[test]
fn write_cancel_result_by_format_propagates_json_writer_errors() {
    let mut writer = FailingWriter;

    let error = write_cancel_result_by_format(&mut writer, true, OutputFormat::Json).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::Other);
}

#[test]
fn write_output_helpers_propagate_text_writer_errors() {
    let record = sample_record();
    let result = SessionEnqueueResult {
        queued: true,
        session_id: "record-1".to_string(),
        request_id: "request-1".to_string(),
    };

    let error = write_closed_session_by_format(&mut FailingWriter, &record, OutputFormat::Text)
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);

    let error = write_new_session_by_format(&mut FailingWriter, &record, None, OutputFormat::Quiet)
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);

    let error =
        write_ensured_session_by_format(&mut FailingWriter, &record, true, OutputFormat::Text)
            .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);

    let error =
        write_queued_prompt_by_format(&mut FailingWriter, &result, OutputFormat::Text).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);

    let error = write_session_details_by_format(&mut FailingWriter, &record, OutputFormat::Quiet)
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);

    let error =
        write_session_history_by_format(&mut FailingWriter, &record, OutputFormat::Quiet, 1)
            .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);

    let error = write_created_session_banner(&mut FailingWriter, &record, "codex").unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);

    let error =
        write_cancel_result_by_format(&mut FailingWriter, false, OutputFormat::Text).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);

    let error = write_set_mode_result_by_format(&mut FailingWriter, "focus", OutputFormat::Text)
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);

    let error = write_set_model_result_by_format(&mut FailingWriter, "model-a", OutputFormat::Text)
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);

    let error = write_set_config_option_result_by_format(
        &mut FailingWriter,
        "approval",
        "strict",
        OutputFormat::Text,
    )
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);
}

#[test]
fn write_sessions_by_format_propagates_json_writer_errors() {
    let error =
        write_sessions_by_format(&mut FailingWriter, &[sample_record()], OutputFormat::Json)
            .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::Other);
}

#[test]
fn write_sessions_by_format_propagates_trailing_newline_errors_after_json() {
    let mut writer = NewlineRejectingWriter::default();

    let error =
        write_sessions_by_format(&mut writer, &[sample_record()], OutputFormat::Json).unwrap_err();

    assert_eq!(error.to_string(), "newline rejected");
    let value: Value = serde_json::from_slice(&writer.output).unwrap();
    assert_eq!(value[0]["vwacpRecordId"], "record-1");
}

#[tokio::test]
async fn print_output_wrappers_return_ok() {
    let record = sample_record();
    let result = SessionEnqueueResult {
        queued: true,
        session_id: "record-1".to_string(),
        request_id: "request-1".to_string(),
    };

    print_sessions_by_format(std::slice::from_ref(&record), OutputFormat::Quiet).unwrap();
    print_closed_session_by_format(&record, OutputFormat::Quiet).unwrap();
    print_new_session_by_format(&record, None, OutputFormat::Quiet).unwrap();
    print_ensured_session_by_format(&record, false, OutputFormat::Quiet).unwrap();
    print_queued_prompt_by_format(&result, OutputFormat::Quiet).unwrap();
    print_session_details_by_format(&record, OutputFormat::Quiet).unwrap();
    print_session_history_by_format(&record, OutputFormat::Quiet, 1).unwrap();
    print_created_session_banner(&record, "codex", OutputFormat::Quiet, false).unwrap();
    print_cancel_result_by_format(true, OutputFormat::Quiet).unwrap();
    print_set_mode_result_by_format("focus", OutputFormat::Quiet).unwrap();
    print_set_model_result_by_format("model-a", OutputFormat::Quiet).unwrap();
    print_set_config_option_result_by_format("approval", "strict", OutputFormat::Quiet).unwrap();
    print_prompt_session_banner(&record, "/tmp/project", OutputFormat::Quiet, false).await.unwrap();
    print_prompt_session_banner(&record, "/tmp/project", OutputFormat::Json, true).await.unwrap();
    print_prompt_session_banner(&record, "/tmp/project", OutputFormat::Text, false).await.unwrap();
}

#[test]
fn write_set_mode_model_and_config_results_render_all_formats() {
    let mut mode_json = Vec::new();
    write_set_mode_result_by_format(&mut mode_json, "focus", OutputFormat::Json).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&mode_json).unwrap(),
        json!({"action": "set_mode", "mode": "focus"})
    );

    let mut mode_quiet = Vec::new();
    write_set_mode_result_by_format(&mut mode_quiet, "focus", OutputFormat::Quiet).unwrap();
    assert!(mode_quiet.is_empty());

    let mut mode_text = Vec::new();
    write_set_mode_result_by_format(&mut mode_text, "focus", OutputFormat::Text).unwrap();
    assert_eq!(rendered_output(mode_text), "Mode set to focus\n");

    let mut model_json = Vec::new();
    write_set_model_result_by_format(&mut model_json, "model-a", OutputFormat::Json).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&model_json).unwrap(),
        json!({"action": "set_model", "model": "model-a"})
    );

    let mut model_quiet = Vec::new();
    write_set_model_result_by_format(&mut model_quiet, "model-a", OutputFormat::Quiet).unwrap();
    assert!(model_quiet.is_empty());

    let mut model_text = Vec::new();
    write_set_model_result_by_format(&mut model_text, "model-a", OutputFormat::Text).unwrap();
    assert_eq!(rendered_output(model_text), "Model set to model-a\n");

    let mut config_json = Vec::new();
    write_set_config_option_result_by_format(
        &mut config_json,
        "approval",
        "strict",
        OutputFormat::Json,
    )
    .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&config_json).unwrap(),
        json!({"action": "set_config_option", "config": "approval", "value": "strict"})
    );

    let mut config_quiet = Vec::new();
    write_set_config_option_result_by_format(
        &mut config_quiet,
        "approval",
        "strict",
        OutputFormat::Quiet,
    )
    .unwrap();
    assert!(config_quiet.is_empty());

    let mut config_text = Vec::new();
    write_set_config_option_result_by_format(
        &mut config_text,
        "approval",
        "strict",
        OutputFormat::Text,
    )
    .unwrap();
    assert_eq!(rendered_output(config_text), "Config option approval set to strict\n");
}
