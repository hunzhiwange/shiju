//! 会话对话模型持久化测试。
//!
//! 该文件验证 ACP 会话更新被转换为可持久化的对话 DTO 后，仍满足持久化 key
//! 策略，并保留工具结果的结构化渲染信息。

use std::collections::HashMap;

use agent_client_protocol::{
    ContentBlock, ContentChunk, Plan, SessionNotification, SessionUpdate, TextContent,
};
use serde_json::json;
use vw_api_types::tools::{StructuredPatchHunkDto, ToolResultContentDto, ToolResultDto};

use super::*;
use crate::{
    ClientOperation, ClientOperationMethod, ClientOperationStatus, SessionAgentContent,
    SessionAgentMessage, SessionMention, SessionMessage, SessionMessageImage, SessionStateOptions,
    SessionThinking, SessionToolResult, SessionToolResultContent, SessionToolUse,
    SessionUserContent, SessionUserMessage, find_persisted_key_policy_violations,
    parse_prompt_source,
};

const T0: &str = "2026-04-21T00:00:00Z";
const T1: &str = "2026-04-21T00:00:01Z";

fn tool_call_update_notification(raw_output: serde_json::Value) -> SessionNotification {
    notification(json!({
        "sessionUpdate": "tool_call_update",
        "toolCallId": "tool-1",
        "kind": "apply_patch",
        "title": "apply_patch",
        "status": "completed",
        "rawOutput": raw_output
    }))
}

fn notification(update: serde_json::Value) -> SessionNotification {
    serde_json::from_value(json!({
        "sessionId": "session-1",
        "update": update
    }))
    .expect("valid ACP session notification")
}

fn content_block(value: serde_json::Value) -> ContentBlock {
    serde_json::from_value(value).expect("valid ACP content block")
}

fn assert_rfc3339_timestamp(value: &str) {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .expect("timestamp is RFC3339");
}

fn text_from_tool_result(content: &SessionToolResultContent) -> &str {
    match content {
        SessionToolResultContent::Text(text) => text,
        other => panic!("expected text tool result, got {other:?}"),
    }
}

fn last_agent(conversation: &SessionConversation) -> &SessionAgentMessage {
    match conversation.messages.last() {
        Some(SessionMessage::Agent(agent)) => agent,
        other => panic!("expected trailing agent message, got {other:?}"),
    }
}

fn long_text(len: usize) -> String {
    "x".repeat(len)
}

fn populated_state() -> SessionAcpxState {
    SessionAcpxState {
        current_mode_id: Some("plan".to_string()),
        desired_mode_id: Some("code".to_string()),
        current_model_id: Some("model-a".to_string()),
        available_models: Some(vec!["model-a".to_string(), "model-b".to_string()]),
        available_commands: Some(vec!["existing".to_string()]),
        config_options: Some(
            serde_json::from_value(json!([{
                "id": "existing",
                "name": "Existing",
                "type": "select",
                "currentValue": "existing",
                "options": [{"value":"existing","name":"Existing"}]
            }]))
            .expect("valid config options"),
        ),
        desired_config_options: None,
        session_options: Some(SessionStateOptions {
            model: Some("model-a".to_string()),
            allowed_tools: Some(vec!["shell".to_string()]),
            max_turns: Some(3),
        }),
    }
}

#[test]
fn create_and_clone_conversation_fall_back_to_current_timestamp() {
    let conversation = create_session_conversation(None);
    assert!(conversation.messages.is_empty());
    assert_rfc3339_timestamp(&conversation.updated_at);

    let cloned = clone_session_conversation(None);
    assert!(cloned.messages.is_empty());
    assert_rfc3339_timestamp(&cloned.updated_at);
}

#[test]
fn record_session_update_persists_tool_result_dto() {
    let mut conversation = create_session_conversation(Some(T0));
    let notification = tool_call_update_notification(json!({
        "tool_use_id": "tool-1",
        "success": true,
        "content": [
            {
                "type": "structured_patch",
                "hunks": [
                    {
                        "path": "src/main.rs",
                        "header": "@@ -1,1 +1,1 @@",
                        "lines": ["-old", "+new"]
                    }
                ]
            }
        ],
        "data": {
            "changed_files": ["src/main.rs"]
        },
        "model_result": "patched src/main.rs",
        "render_hint": {
            "kind": "structured_patch",
            "summary": "Updated 1 file",
            "metadata": {
                "outputPath": "src/main.rs"
            }
        }
    }));

    let _state = record_session_update(&mut conversation, None, &notification, Some(T1));

    let agent = last_agent(&conversation);
    let tool_result = agent.tool_results.get("tool-1").expect("tool result persisted");

    assert_eq!(tool_result.tool_use_id, "tool-1");
    assert_eq!(tool_result.tool_name, "apply_patch");
    assert!(!tool_result.is_error);
    assert!(matches!(
        &tool_result.content,
        SessionToolResultContent::Text(text) if text == "Updated 1 file"
    ));

    let stored = tool_result.result.as_ref().expect("structured dto persisted");
    assert_eq!(stored.tool_use_id.as_deref(), Some("tool-1"));
    assert_eq!(stored.tool_id.as_ref().map(|tool_id| tool_id.as_ref()), Some("apply_patch"));
    assert_eq!(
        stored.render_hint.as_ref().and_then(|hint| hint.summary.as_deref()),
        Some("Updated 1 file")
    );

    let serialized = serde_json::to_value(&conversation).expect("conversation serializes");
    let violations = find_persisted_key_policy_violations(&serialized);
    assert!(violations.is_empty(), "unexpected persisted key policy violations: {violations:?}");
}

#[test]
fn append_legacy_history_skips_blank_entries_and_updates_timestamp() {
    let mut conversation = create_session_conversation(Some(T0));
    append_legacy_history(
        &mut conversation,
        &[
            LegacyHistoryEntry {
                role: LegacyHistoryRole::User,
                timestamp: T1.to_string(),
                text_preview: "  hello  ".to_string(),
            },
            LegacyHistoryEntry {
                role: LegacyHistoryRole::Assistant,
                timestamp: "2026-04-21T00:00:02Z".to_string(),
                text_preview: "answer".to_string(),
            },
            LegacyHistoryEntry {
                role: LegacyHistoryRole::User,
                timestamp: "2026-04-21T00:00:03Z".to_string(),
                text_preview: "   ".to_string(),
            },
        ],
    );

    assert_eq!(conversation.messages.len(), 2);
    assert_eq!(conversation.updated_at, "2026-04-21T00:00:02Z");
    match &conversation.messages[0] {
        SessionMessage::User(user) => {
            assert!(matches!(&user.content[0], SessionUserContent::Text(text) if text == "hello"));
        }
        other => panic!("expected user message, got {other:?}"),
    }
    match &conversation.messages[1] {
        SessionMessage::Agent(agent) => {
            assert!(
                matches!(&agent.content[0], SessionAgentContent::Text(text) if text == "answer")
            );
            assert!(agent.tool_results.is_empty());
        }
        other => panic!("expected agent message, got {other:?}"),
    }
}

#[test]
fn append_legacy_history_ignores_empty_input_and_blank_previews() {
    let mut conversation = create_session_conversation(Some(T0));

    append_legacy_history(&mut conversation, &[]);
    append_legacy_history(
        &mut conversation,
        &[
            LegacyHistoryEntry {
                role: LegacyHistoryRole::User,
                timestamp: T1.to_string(),
                text_preview: String::new(),
            },
            LegacyHistoryEntry {
                role: LegacyHistoryRole::Assistant,
                timestamp: "2026-04-21T00:00:02Z".to_string(),
                text_preview: " \n\t ".to_string(),
            },
        ],
    );

    assert!(conversation.messages.is_empty());
    assert_eq!(conversation.updated_at, T0);
}

#[test]
fn clone_helpers_preserve_or_create_values() {
    let conversation = create_session_conversation(Some(T0));
    assert_eq!(clone_session_conversation(Some(&conversation)), conversation);
    assert!(clone_session_conversation(None).messages.is_empty());

    let state = SessionAcpxState {
        current_mode_id: Some("plan".to_string()),
        desired_mode_id: Some("code".to_string()),
        current_model_id: Some("model-a".to_string()),
        available_models: Some(vec!["model-a".to_string()]),
        available_commands: Some(vec!["run".to_string()]),
        config_options: None,
        desired_config_options: None,
        session_options: None,
    };
    assert_eq!(clone_session_vwacp_state(Some(&state)), Some(state));
    assert_eq!(clone_session_vwacp_state(None), None);
}

#[test]
fn record_prompt_submission_converts_supported_content_blocks() {
    let mut conversation = create_session_conversation(Some(T0));
    let prompt = parse_prompt_source(
        r#"[
            {"type":"text","text":"hello"},
            {"type":"resource_link","uri":"file:///a.md","name":"A","title":"A"},
            {"type":"resource","resource":{"uri":"file:///b.md","text":"inline text"}},
            {"type":"resource","resource":{"uri":"file:///c.md","text":""}},
            {"type":"image","mimeType":"image/png","data":"QUJDRA=="}
        ]"#,
    )
    .expect("valid prompt");

    record_prompt_submission(&mut conversation, &prompt, Some(T1));

    assert_eq!(conversation.updated_at, T1);
    let user = match conversation.messages.last() {
        Some(SessionMessage::User(user)) => user,
        other => panic!("expected user message, got {other:?}"),
    };
    assert_eq!(user.content.len(), 5);
    assert!(matches!(&user.content[0], SessionUserContent::Text(text) if text == "hello"));
    assert_eq!(
        user.content[1],
        SessionUserContent::Mention(SessionMention {
            uri: "file:///a.md".to_string(),
            content: "A".to_string()
        })
    );
    assert!(matches!(&user.content[2], SessionUserContent::Text(text) if text == "inline text"));
    assert_eq!(
        user.content[3],
        SessionUserContent::Mention(SessionMention {
            uri: "file:///c.md".to_string(),
            content: "file:///c.md".to_string()
        })
    );
    assert_eq!(
        user.content[4],
        SessionUserContent::Image(SessionMessageImage {
            source: "QUJDRA==".to_string(),
            size: None
        })
    );
}

#[test]
fn record_prompt_submission_ignores_empty_and_unsupported_content() {
    let mut conversation = create_session_conversation(Some(T0));
    let empty_prompt = parse_prompt_source("[]").expect("valid empty prompt");

    record_prompt_submission(&mut conversation, &empty_prompt, Some(T1));

    assert!(conversation.messages.is_empty());
    assert_eq!(conversation.updated_at, T0);

    let audio_prompt = vec![content_block(json!({
        "type": "audio",
        "mimeType": "audio/wav",
        "data": "AAAA"
    }))];

    record_prompt_submission(&mut conversation, &audio_prompt, Some(T1));

    assert!(conversation.messages.is_empty());
    assert_eq!(conversation.updated_at, T0);
}

#[test]
fn record_prompt_submission_uses_timestamp_fallback_and_trims_text() {
    let mut conversation = create_session_conversation(Some(T0));
    let source = long_text(MAX_RUNTIME_AGENT_TEXT_CHARS + 1);
    let prompt = parse_prompt_source(&source).expect("valid text prompt");

    record_prompt_submission(&mut conversation, &prompt, None);

    assert_ne!(conversation.updated_at, T0);
    assert_rfc3339_timestamp(&conversation.updated_at);
    let user = match conversation.messages.last() {
        Some(SessionMessage::User(user)) => user,
        other => panic!("expected user message, got {other:?}"),
    };
    assert_eq!(user.content.len(), 1);
    assert!(
        matches!(&user.content[0], SessionUserContent::Text(text) if text.len() == MAX_RUNTIME_AGENT_TEXT_CHARS && text.ends_with("..."))
    );
}

#[test]
fn record_text_prompt_submission_ignores_empty_prompt() {
    let mut conversation = create_session_conversation(Some(T0));

    record_text_prompt_submission(&mut conversation, "", Some(T1));
    record_text_prompt_submission(&mut conversation, " \n\t ", Some(T1));

    assert!(conversation.messages.is_empty());
    assert_eq!(conversation.updated_at, T0);
}

#[test]
fn record_session_update_syncs_valid_notifications_and_preserves_existing_state() {
    let mut conversation = create_session_conversation(Some(T0));
    let base_state = populated_state();
    let mut state = record_session_update(
        &mut conversation,
        Some(&base_state),
        &notification(json!({
            "sessionUpdate": "user_message_chunk",
            "content": {"type":"text","text":"user"}
        })),
        Some(T1),
    );

    assert_eq!(state, base_state);
    assert_eq!(conversation.updated_at, T1);
    assert_eq!(conversation.cumulative_token_usage, SessionTokenUsage::default());
    assert!(conversation.request_token_usage.is_empty());
    let user_id = match &conversation.messages[0] {
        SessionMessage::User(user) => {
            assert!(matches!(&user.content[0], SessionUserContent::Text(text) if text == "user"));
            user.id.clone()
        }
        other => panic!("expected user message, got {other:?}"),
    };

    state = record_session_update(
        &mut conversation,
        Some(&state),
        &notification(json!({
            "sessionUpdate": "agent_message_chunk",
            "content": {"type":"text","text":"agent"}
        })),
        Some("2026-04-21T00:00:02Z"),
    );
    assert_eq!(state, base_state);
    assert_eq!(conversation.updated_at, "2026-04-21T00:00:02Z");

    state = record_session_update(
        &mut conversation,
        Some(&state),
        &notification(json!({
            "sessionUpdate": "agent_thought_chunk",
            "content": {"type":"text","text":"thought"}
        })),
        Some("2026-04-21T00:00:03Z"),
    );
    assert_eq!(state, base_state);
    assert_eq!(conversation.updated_at, "2026-04-21T00:00:03Z");
    let agent = last_agent(&conversation);
    assert!(matches!(&agent.content[0], SessionAgentContent::Text(text) if text == "agent"));
    assert!(matches!(
        &agent.content[1],
        SessionAgentContent::Thinking(thinking) if thinking.text == "thought"
    ));

    state = record_session_update(
        &mut conversation,
        Some(&state),
        &notification(json!({
            "sessionUpdate": "usage_update",
            "used": 21,
            "size": 100,
            "_meta": {
                "usage": {
                    "inputTokens": 13,
                    "outputTokens": 8,
                    "cachedWriteTokens": 5,
                    "cachedReadTokens": 3
                }
            }
        })),
        Some("2026-04-21T00:00:04Z"),
    );
    assert_eq!(state, base_state);
    assert_eq!(conversation.updated_at, "2026-04-21T00:00:04Z");
    assert_eq!(conversation.cumulative_token_usage.input_tokens, Some(13));
    assert_eq!(conversation.cumulative_token_usage.output_tokens, Some(8));
    assert_eq!(conversation.cumulative_token_usage.cache_creation_input_tokens, Some(5));
    assert_eq!(conversation.cumulative_token_usage.cache_read_input_tokens, Some(3));
    assert_eq!(
        conversation.request_token_usage.get(&user_id),
        Some(&conversation.cumulative_token_usage)
    );

    state = record_session_update(
        &mut conversation,
        Some(&state),
        &notification(json!({
            "sessionUpdate": "session_info_update",
            "title": "Synced",
            "updatedAt": "2026-04-21T00:00:05Z"
        })),
        None,
    );
    assert_eq!(state, base_state);
    assert_eq!(conversation.title.as_deref(), Some("Synced"));
    assert_eq!(conversation.updated_at, "2026-04-21T00:00:05Z");

    state = record_session_update(
        &mut conversation,
        Some(&state),
        &notification(json!({
            "sessionUpdate": "available_commands_update",
            "availableCommands": [
                {"name":" run ","description":"Run command"},
                {"name":"test","description":"Test command"}
            ]
        })),
        Some("2026-04-21T00:00:06Z"),
    );
    let expected_commands = SessionAcpxState {
        available_commands: Some(vec!["run".to_string(), "test".to_string()]),
        ..base_state.clone()
    };
    assert_eq!(state, expected_commands);
    assert_eq!(conversation.updated_at, "2026-04-21T00:00:06Z");

    state = record_session_update(
        &mut conversation,
        Some(&state),
        &notification(json!({
            "sessionUpdate": "current_mode_update",
            "currentModeId": "focus"
        })),
        Some("2026-04-21T00:00:07Z"),
    );
    let expected_mode = SessionAcpxState {
        current_mode_id: Some("focus".to_string()),
        ..expected_commands.clone()
    };
    assert_eq!(state, expected_mode);
    assert_eq!(conversation.updated_at, "2026-04-21T00:00:07Z");

    let config_options = json!([{
        "id": "model",
        "name": "Model",
        "type": "select",
        "currentValue": "slow",
        "options": [{"value":"slow","name":"Slow"}]
    }]);
    state = record_session_update(
        &mut conversation,
        Some(&state),
        &notification(json!({
            "sessionUpdate": "config_option_update",
            "configOptions": config_options.clone()
        })),
        Some("2026-04-21T00:00:08Z"),
    );
    let expected_config = SessionAcpxState {
        config_options: Some(serde_json::from_value(config_options).expect("valid config options")),
        ..expected_mode
    };
    assert_eq!(state, expected_config);
    assert_eq!(conversation.updated_at, "2026-04-21T00:00:08Z");
    assert_eq!(conversation.messages.len(), 2);
    assert_eq!(
        conversation.request_token_usage.get(&user_id),
        Some(&conversation.cumulative_token_usage)
    );
}

#[test]
fn record_session_update_ignores_invalid_unsupported_unknown_and_empty_payloads() {
    let mut conversation = create_session_conversation(Some(T0));
    let state = populated_state();

    let invalid = serde_json::from_value::<SessionNotification>(json!({
        "sessionId": "session-1",
        "update": {
            "sessionUpdate": "agent_message_chunk",
            "content": {"type":"text","text": 7}
        }
    }));
    assert!(invalid.is_err());
    assert_eq!(conversation.updated_at, T0);
    assert!(conversation.messages.is_empty());

    let returned = record_session_update(
        &mut conversation,
        Some(&state),
        &notification(json!({
            "sessionUpdate": "user_message_chunk",
            "content": {"type":"audio","mimeType":"audio/wav","data":"AAAA"}
        })),
        Some(T1),
    );
    assert_eq!(returned, state);
    assert_eq!(conversation.updated_at, T1);
    assert!(conversation.messages.is_empty());
    assert_eq!(conversation.cumulative_token_usage, SessionTokenUsage::default());
    assert!(conversation.request_token_usage.is_empty());

    let unknown = SessionNotification::new("session-1", SessionUpdate::Plan(Plan::new(Vec::new())));
    let returned = record_session_update(
        &mut conversation,
        Some(&returned),
        &unknown,
        Some("2026-04-21T00:00:02Z"),
    );
    assert_eq!(returned, state);
    assert_eq!(conversation.updated_at, "2026-04-21T00:00:02Z");
    assert!(conversation.messages.is_empty());

    let returned = record_session_update(
        &mut conversation,
        Some(&returned),
        &notification(json!({
            "sessionUpdate": "session_info_update"
        })),
        Some("2026-04-21T00:00:03Z"),
    );
    assert_eq!(returned, state);
    assert_eq!(conversation.updated_at, "2026-04-21T00:00:03Z");
    assert_eq!(conversation.title, None);
    assert!(conversation.messages.is_empty());
    assert_eq!(conversation.cumulative_token_usage, SessionTokenUsage::default());
    assert!(conversation.request_token_usage.is_empty());
}

#[test]
fn record_session_update_appends_user_agent_and_thinking_chunks() {
    let mut conversation = create_session_conversation(Some(T0));

    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "user_message_chunk",
            "content": {"type":"text","text":"user"}
        })),
        Some(T1),
    );
    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "agent_message_chunk",
            "content": {"type":"text","text":"first"}
        })),
        Some("2026-04-21T00:00:02Z"),
    );
    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "agent_message_chunk",
            "content": {"type":"resource_link","uri":"file:///result.md","name":"result"}
        })),
        Some("2026-04-21T00:00:03Z"),
    );
    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "agent_thought_chunk",
            "content": {"type":"resource","resource":{"uri":"file:///thought.md","text":"file:///thought.md"}}
        })),
        Some("2026-04-21T00:00:04Z"),
    );
    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "agent_thought_chunk",
            "content": {"type":"text","text":" more"}
        })),
        Some("2026-04-21T00:00:05Z"),
    );

    assert_eq!(conversation.messages.len(), 2);
    match &conversation.messages[0] {
        SessionMessage::User(user) => {
            assert!(matches!(&user.content[0], SessionUserContent::Text(text) if text == "user"));
        }
        other => panic!("expected user message, got {other:?}"),
    }
    let agent = last_agent(&conversation);
    assert_eq!(agent.content.len(), 2);
    assert!(matches!(&agent.content[0], SessionAgentContent::Text(text) if text == "firstresult"));
    assert!(matches!(
        &agent.content[1],
        SessionAgentContent::Thinking(thinking) if thinking.text == "file:///thought.md more"
    ));
}

#[test]
fn record_session_update_ignores_blank_agent_chunks_and_unknown_updates() {
    let mut conversation = create_session_conversation(Some(T0));

    let state = record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "agent_message_chunk",
            "content": {"type":"text","text":"   "}
        })),
        Some(T1),
    );
    let next_state = state.clone();

    assert_eq!(conversation.updated_at, T1);
    assert!(conversation.messages.is_empty());
    assert_eq!(next_state, state);
}

#[test]
fn record_session_update_records_usage_for_last_user_message() {
    let mut conversation = create_session_conversation(Some(T0));
    record_text_prompt_submission(&mut conversation, "question", Some(T1));
    let user_id = match &conversation.messages[0] {
        SessionMessage::User(user) => user.id.clone(),
        other => panic!("expected user message, got {other:?}"),
    };

    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "usage_update",
            "used": 18,
            "size": 100,
            "_meta": {
                "usage": {
                    "inputTokens": 11,
                    "output_tokens": 7,
                    "cachedWriteTokens": 3,
                    "cachedReadTokens": 2
                }
            },
            "inputTokens": -1
        })),
        Some("2026-04-21T00:00:02Z"),
    );

    assert_eq!(conversation.cumulative_token_usage.input_tokens, Some(11));
    assert_eq!(conversation.cumulative_token_usage.output_tokens, Some(7));
    assert_eq!(conversation.cumulative_token_usage.cache_creation_input_tokens, Some(3));
    assert_eq!(conversation.cumulative_token_usage.cache_read_input_tokens, Some(2));
    assert_eq!(
        conversation.request_token_usage.get(&user_id),
        Some(&conversation.cumulative_token_usage)
    );
}

#[test]
fn record_session_update_ignores_empty_usage_payload() {
    let mut conversation = create_session_conversation(Some(T0));

    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "usage_update",
            "used": 0,
            "size": 100,
            "inputTokens": -1,
            "outputTokens": null
        })),
        Some(T1),
    );

    assert_eq!(conversation.cumulative_token_usage, SessionTokenUsage::default());
    assert!(conversation.request_token_usage.is_empty());
}

#[test]
fn record_session_update_syncs_session_info_commands_modes_and_config() {
    let mut conversation = create_session_conversation(Some(T0));
    let mut state = record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "session_info_update",
            "title": "Chat",
            "updatedAt": "2026-04-21T00:00:09Z"
        })),
        None,
    );
    assert_eq!(conversation.title.as_deref(), Some("Chat"));
    assert_eq!(conversation.updated_at, "2026-04-21T00:00:09Z");

    state = record_session_update(
        &mut conversation,
        Some(&state),
        &notification(json!({
            "sessionUpdate": "available_commands_update",
            "availableCommands": [
                {"name":" run ","description":"Run command"},
                {"name":" ","description":"Blank command"}
            ]
        })),
        Some(T1),
    );
    assert_eq!(state.available_commands, Some(vec!["run".to_string()]));

    state = record_session_update(
        &mut conversation,
        Some(&state),
        &notification(json!({
            "sessionUpdate": "current_mode_update",
            "currentModeId": "plan"
        })),
        Some(T1),
    );
    assert_eq!(state.current_mode_id.as_deref(), Some("plan"));

    state = record_session_update(
        &mut conversation,
        Some(&state),
        &notification(json!({
            "sessionUpdate": "config_option_update",
            "configOptions": [{
                "id": "model",
                "name": "Model",
                "type": "select",
                "currentValue": "fast",
                "options": [{"value":"fast","name":"Fast"}]
            }]
        })),
        Some(T1),
    );
    assert_eq!(state.config_options.as_ref().map(Vec::len), Some(1));
}

#[test]
fn record_client_operation_updates_timestamp_and_preserves_state() {
    let mut conversation = create_session_conversation(Some(T0));
    let state = SessionAcpxState {
        current_mode_id: Some("plan".to_string()),
        desired_mode_id: None,
        current_model_id: None,
        available_models: None,
        available_commands: None,
        config_options: None,
        desired_config_options: None,
        session_options: None,
    };
    let operation = ClientOperation {
        method: ClientOperationMethod::TerminalWaitForExit,
        status: ClientOperationStatus::Completed,
        summary: "done".to_string(),
        details: Some("exit 0".to_string()),
        timestamp: T1.to_string(),
    };

    let returned = record_client_operation(&mut conversation, Some(&state), &operation, Some(T1));

    assert_eq!(returned, state);
    assert_eq!(conversation.updated_at, T1);
}

#[test]
fn tool_call_update_creates_and_updates_tool_use_and_result() {
    let mut conversation = create_session_conversation(Some(T0));

    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "tool_call",
            "toolCallId": "tool-2",
            "kind": "shell",
            "title": "shell",
            "status": "in_progress",
            "rawInput": {"command":"echo hi"}
        })),
        Some(T1),
    );
    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "tool_call_update",
            "toolCallId": "tool-2",
            "title": "Shell",
            "status": "failed",
            "rawOutput": {"stderr":"boom"}
        })),
        Some(T1),
    );

    let agent = last_agent(&conversation);
    match &agent.content[0] {
        SessionAgentContent::ToolUse(tool) => {
            assert_eq!(tool.id, "tool-2");
            assert_eq!(tool.name, "Shell");
            assert_eq!(tool.raw_input, r#"{"command":"echo hi"}"#);
            assert!(tool.is_input_complete);
        }
        other => panic!("expected tool use, got {other:?}"),
    }

    let result = agent.tool_results.get("tool-2").expect("tool result");
    assert_eq!(result.tool_name, "Shell");
    assert!(result.is_error);
    assert_eq!(text_from_tool_result(&result.content), "boom");
    assert_eq!(result.output.as_ref(), Some(&json!({"stderr":"boom"})));
}

#[test]
fn tool_call_update_preserves_existing_result_fields() {
    let mut conversation = create_session_conversation(Some(T0));

    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "tool_call_update",
            "toolCallId": "tool-3",
            "kind": "grep",
            "title": "grep",
            "status": "completed",
            "rawOutput": {"content":"found"}
        })),
        Some(T1),
    );
    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "tool_call_update",
            "toolCallId": "tool-3",
            "status": "in_progress"
        })),
        Some(T1),
    );

    let agent = last_agent(&conversation);
    let result = agent.tool_results.get("tool-3").expect("tool result");
    assert_eq!(result.tool_name, "grep");
    assert!(!result.is_error);
    assert_eq!(text_from_tool_result(&result.content), "found");
    assert_eq!(result.output.as_ref(), Some(&json!({"content":"found"})));
}

#[test]
fn tool_call_update_handles_null_input_and_missing_id() {
    let mut conversation = create_session_conversation(Some(T0));

    let mut agent = SessionAgentMessage {
        content: Vec::new(),
        tool_results: HashMap::new(),
        reasoning_details: None,
    };
    let missing_id = json!({
        "sessionUpdate": "tool_call_update",
        "kind": "shell",
        "status": "completed",
        "rawInput": null
    });
    apply_tool_call_update(&mut agent, missing_id.as_object().expect("update object"));
    assert!(agent.content.is_empty());
    assert!(conversation.messages.is_empty());

    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "tool_call_update",
            "toolCallId": "tool-4",
            "status": "completed",
            "rawInput": null
        })),
        Some(T1),
    );
    let agent = last_agent(&conversation);
    match &agent.content[0] {
        SessionAgentContent::ToolUse(tool) => {
            assert_eq!(tool.name, "tool_call");
            assert_eq!(tool.input, json!({}));
            assert_eq!(tool.raw_input, "{}");
        }
        other => panic!("expected tool use, got {other:?}"),
    }
}

#[test]
fn extract_tool_result_dto_text_prefers_summary_and_content_fallbacks() {
    let with_summary: ToolResultDto = serde_json::from_value(json!({
        "content": [{"type":"text","text":"content"}],
        "render_hint": {"summary":"summary"}
    }))
    .expect("dto");
    assert_eq!(extract_tool_result_dto_text(&with_summary).as_deref(), Some("summary"));

    let with_json: ToolResultDto = serde_json::from_value(json!({
        "content": [{"type":"json","value":{"message":"json message"}}]
    }))
    .expect("dto");
    assert_eq!(extract_tool_result_dto_text(&with_json).as_deref(), Some("json message"));

    let with_patch: ToolResultDto = serde_json::from_value(json!({
        "content": [{
            "type":"structured_patch",
            "hunks":[
                {"path":"a.rs","header":"@@ -1 +1 @@","lines":["-a","+b"]},
                {"path":"   ","header":"ignored","lines":["ignored"]},
                {"path":"b.rs","header":"","lines":["+c"]}
            ]
        }]
    }))
    .expect("dto");
    assert_eq!(
        extract_tool_result_dto_text(&with_patch).as_deref(),
        Some("--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n-a\n+b\n--- a/b.rs\n+++ b/b.rs\n+c\n")
    );

    let with_model_result: ToolResultDto = serde_json::from_value(json!({
        "model_result": {"output": "model"}
    }))
    .expect("dto");
    assert_eq!(extract_tool_result_dto_text(&with_model_result).as_deref(), Some("model"));

    let with_data: ToolResultDto = serde_json::from_value(json!({
        "data": {"value": "data"}
    }))
    .expect("dto");
    assert_eq!(extract_tool_result_dto_text(&with_data).as_deref(), Some("data"));
}

#[test]
fn extract_tool_result_helpers_cover_scalar_array_and_empty_values() {
    assert_eq!(extract_tool_result_text(&serde_json::Value::Null), None);
    assert_eq!(extract_tool_result_text(&json!("text")).as_deref(), Some("text"));
    assert_eq!(extract_tool_result_text(&json!(42)).as_deref(), Some("42"));
    assert_eq!(extract_tool_result_text(&json!(true)).as_deref(), Some("true"));
    assert_eq!(extract_tool_result_text(&json!([null, {"stdout":"out"}])).as_deref(), Some("out"));
    assert_eq!(extract_tool_result_text(&json!({"unknown":"value"})), None);
    assert_eq!(structured_patch_diff_text(&[]), None);
}

#[test]
fn parse_tool_result_dto_fills_missing_ids_and_tool_kind() {
    let parsed = parse_tool_result_dto(
        &json!({
            "success": false,
            "content": [{"type":"text","text":"failed"}]
        }),
        "tool-5",
        Some(" shell "),
    )
    .expect("dto parsed");

    assert_eq!(parsed.tool_use_id.as_deref(), Some("tool-5"));
    assert_eq!(parsed.tool_id.as_ref().map(|id| id.as_ref()), Some("shell"));
    assert_eq!(parsed.success, Some(false));
    assert!(parse_tool_result_dto(&json!("not a dto"), "tool-5", Some("shell")).is_none());
}

#[test]
fn status_helpers_classify_complete_and_error_values() {
    assert!(status_indicates_complete(Some("completed")));
    assert!(status_indicates_complete(Some("DONE")));
    assert!(status_indicates_complete(Some("success")));
    assert!(status_indicates_complete(Some("failed")));
    assert!(status_indicates_complete(Some("error")));
    assert!(status_indicates_complete(Some("cancelled")));
    assert!(!status_indicates_complete(Some("running")));
    assert!(!status_indicates_complete(None));

    assert!(status_indicates_error(Some("FAILED")));
    assert!(status_indicates_error(Some("error")));
    assert!(!status_indicates_error(Some("completed")));
    assert!(!status_indicates_error(None));
}

#[test]
fn trim_conversation_for_runtime_limits_messages_text_tool_io_and_usage_entries() {
    let mut messages = Vec::new();
    for index in 0..(MAX_RUNTIME_MESSAGES + 1) {
        messages.push(SessionMessage::User(SessionUserMessage {
            id: format!("user-{index:03}"),
            content: vec![SessionUserContent::Text(long_text(MAX_RUNTIME_AGENT_TEXT_CHARS + 1))],
        }));
    }
    messages.push(SessionMessage::Agent(SessionAgentMessage {
        content: vec![
            SessionAgentContent::Text(long_text(MAX_RUNTIME_AGENT_TEXT_CHARS + 1)),
            SessionAgentContent::Thinking(SessionThinking {
                text: long_text(MAX_RUNTIME_THINKING_CHARS + 1),
                signature: Some("sig".to_string()),
            }),
            SessionAgentContent::ToolUse(SessionToolUse {
                id: "tool".to_string(),
                name: "shell".to_string(),
                raw_input: long_text(MAX_RUNTIME_TOOL_IO_CHARS + 1),
                input: json!({}),
                is_input_complete: false,
                thought_signature: None,
            }),
            SessionAgentContent::RedactedThinking("redacted".to_string()),
        ],
        tool_results: HashMap::from([(
            "tool".to_string(),
            SessionToolResult {
                tool_use_id: "tool".to_string(),
                tool_name: "shell".to_string(),
                is_error: false,
                content: SessionToolResultContent::Text(long_text(MAX_RUNTIME_TOOL_IO_CHARS + 1)),
                output: Some(json!(long_text(MAX_RUNTIME_TOOL_IO_CHARS + 1))),
                result: None,
            },
        )]),
        reasoning_details: None,
    }));

    let request_token_usage = (0..(MAX_RUNTIME_REQUEST_TOKEN_USAGE + 1))
        .map(|index| {
            (
                format!("user-{index:03}"),
                SessionTokenUsage {
                    input_tokens: Some(index as i64),
                    output_tokens: None,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                },
            )
        })
        .collect();
    let mut conversation = SessionConversation {
        title: None,
        messages,
        updated_at: T0.to_string(),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage,
    };

    trim_conversation_for_runtime(&mut conversation);

    assert_eq!(conversation.messages.len(), MAX_RUNTIME_MESSAGES);
    assert_eq!(conversation.request_token_usage.len(), MAX_RUNTIME_REQUEST_TOKEN_USAGE);
    assert!(!conversation.request_token_usage.contains_key("user-000"));
    match &conversation.messages[0] {
        SessionMessage::User(user) => {
            assert!(
                matches!(&user.content[0], SessionUserContent::Text(text) if text.len() == MAX_RUNTIME_AGENT_TEXT_CHARS)
            );
        }
        other => panic!("expected user message, got {other:?}"),
    }
    let agent = last_agent(&conversation);
    assert!(
        matches!(&agent.content[0], SessionAgentContent::Text(text) if text.len() == MAX_RUNTIME_AGENT_TEXT_CHARS)
    );
    assert!(
        matches!(&agent.content[1], SessionAgentContent::Thinking(thinking) if thinking.text.len() == MAX_RUNTIME_THINKING_CHARS)
    );
    assert!(
        matches!(&agent.content[2], SessionAgentContent::ToolUse(tool) if tool.raw_input.len() == MAX_RUNTIME_TOOL_IO_CHARS)
    );
    let result = agent.tool_results.get("tool").expect("tool result");
    assert_eq!(text_from_tool_result(&result.content).len(), MAX_RUNTIME_TOOL_IO_CHARS);
    assert!(
        matches!(result.output.as_ref(), Some(serde_json::Value::String(text)) if text.len() == MAX_RUNTIME_TOOL_IO_CHARS)
    );
}

#[test]
fn trim_runtime_text_handles_boundary_and_unicode() {
    assert_eq!(trim_runtime_text("abc", 3), "abc");
    assert_eq!(trim_runtime_text("abcd", 3), "...");
    assert_eq!(trim_runtime_text("你好世界", 4), "你好世界");
    assert_eq!(trim_runtime_text("你好世界啊", 4), "你...");
}

#[test]
fn content_block_conversion_handles_resource_link_resource_and_image_fallbacks() {
    let link_with_name = content_block(json!({
        "type": "resource_link",
        "uri": "file:///name.md",
        "name": "Name",
        "title": "   "
    }));
    assert_eq!(extract_text(&link_with_name).as_deref(), Some("Name"));
    assert_eq!(
        content_to_user_content(&link_with_name),
        Some(SessionUserContent::Mention(SessionMention {
            uri: "file:///name.md".to_string(),
            content: "Name".to_string()
        }))
    );

    let link_with_uri = content_block(json!({
        "type": "resource_link",
        "uri": "file:///uri.md",
        "name": "   ",
        "title": null
    }));
    assert_eq!(extract_text(&link_with_uri).as_deref(), Some("file:///uri.md"));
    assert_eq!(
        content_to_user_content(&link_with_uri),
        Some(SessionUserContent::Mention(SessionMention {
            uri: "file:///uri.md".to_string(),
            content: "file:///uri.md".to_string()
        }))
    );

    let resource_with_text = content_block(json!({
        "type": "resource",
        "resource": {"uri": "file:///inline.md", "text": "inline"}
    }));
    assert_eq!(extract_text(&resource_with_text).as_deref(), Some("inline"));
    assert_eq!(
        content_to_user_content(&resource_with_text),
        Some(SessionUserContent::Text("inline".to_string()))
    );

    let resource_with_blank_text = content_block(json!({
        "type": "resource",
        "resource": {"uri": "file:///fallback.md", "text": "   "}
    }));
    assert_eq!(extract_text(&resource_with_blank_text).as_deref(), Some("file:///fallback.md"));
    assert_eq!(
        content_to_user_content(&resource_with_blank_text),
        Some(SessionUserContent::Mention(SessionMention {
            uri: "file:///fallback.md".to_string(),
            content: "file:///fallback.md".to_string()
        }))
    );

    let resource_with_blob = content_block(json!({
        "type": "resource",
        "resource": {"uri": "file:///asset.bin", "blob": "QUJD"}
    }));
    assert_eq!(
        content_to_user_content(&resource_with_blob),
        Some(SessionUserContent::Mention(SessionMention {
            uri: "file:///asset.bin".to_string(),
            content: "file:///asset.bin".to_string()
        }))
    );

    let image = content_block(json!({
        "type": "image",
        "mimeType": "image/png",
        "data": ""
    }));
    assert_eq!(extract_text(&image), None);
    assert_eq!(
        content_to_user_content(&image),
        Some(SessionUserContent::Image(SessionMessageImage { source: String::new(), size: None }))
    );
}

#[test]
fn content_block_conversion_rejects_unsupported_blocks() {
    let unsupported = content_block(json!({
        "type":"audio",
        "mimeType":"audio/wav",
        "data":"AAAA"
    }));
    assert_eq!(extract_text(&unsupported), None);
    assert_eq!(content_to_user_content(&unsupported), None);
}

#[test]
fn notification_with_invalid_inner_content_is_ignored() {
    let conversation = create_session_conversation(Some(T0));
    let invalid_content = json!({"type":"text","text": 7});

    assert!(serde_json::from_value::<ContentBlock>(invalid_content).is_err());

    assert!(conversation.messages.is_empty());
    assert_eq!(conversation.updated_at, T0);
}

#[test]
fn record_session_update_handles_manual_session_update_variants() {
    let mut conversation = create_session_conversation(Some(T0));
    let notification = SessionNotification::new(
        "session-1",
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "hello from typed update",
        )))),
    );

    record_session_update(&mut conversation, None, &notification, Some(T1));

    let agent = last_agent(&conversation);
    assert!(matches!(
        &agent.content[0],
        SessionAgentContent::Text(text) if text == "hello from typed update"
    ));
}

#[test]
fn dto_content_text_skips_blank_and_uses_next_available_source() {
    let result = ToolResultDto {
        tool_use_id: None,
        tool_id: None,
        success: None,
        content: vec![
            ToolResultContentDto::Text { text: "   ".to_string() },
            ToolResultContentDto::Json { value: json!({"summary":"json summary"}) },
        ],
        data: serde_json::Value::Null,
        model_result: serde_json::Value::Null,
        render_hint: None,
        permission_request: None,
        context_updates: Vec::new(),
        extra_messages: Vec::new(),
        telemetry: None,
    };

    assert_eq!(extract_tool_result_dto_text(&result).as_deref(), Some("json summary"));
}

#[test]
fn structured_patch_diff_text_omits_empty_path_only_hunks() {
    let hunks = vec![StructuredPatchHunkDto {
        header: "@@ -1 +1 @@".to_string(),
        path: Some("   ".to_string()),
        old_start: None,
        old_lines: None,
        new_start: None,
        new_lines: None,
        lines: vec!["+ignored".to_string()],
    }];

    assert_eq!(structured_patch_diff_text(&hunks), None);
}

#[test]
fn append_agent_helpers_trim_existing_content_and_ignore_blank_thinking() {
    let mut agent = SessionAgentMessage {
        content: Vec::new(),
        tool_results: HashMap::new(),
        reasoning_details: None,
    };

    append_agent_thinking(&mut agent, " \n\t ");
    assert!(agent.content.is_empty());

    append_agent_text(&mut agent, "start");
    append_agent_text(&mut agent, &long_text(MAX_RUNTIME_AGENT_TEXT_CHARS + 1));
    assert!(
        matches!(&agent.content[0], SessionAgentContent::Text(text) if text.len() == MAX_RUNTIME_AGENT_TEXT_CHARS && text.ends_with("..."))
    );

    append_agent_thinking(&mut agent, "think");
    append_agent_thinking(&mut agent, &long_text(MAX_RUNTIME_THINKING_CHARS + 1));
    assert!(
        matches!(&agent.content[1], SessionAgentContent::Thinking(thinking) if thinking.text.len() == MAX_RUNTIME_THINKING_CHARS && thinking.text.ends_with("..."))
    );
}

#[test]
fn tool_result_text_helpers_cover_nested_empty_and_serialized_fallbacks() {
    assert_eq!(
        extract_tool_result_text(&json!({"summary":"   ","output":"used"})).as_deref(),
        Some("used")
    );
    assert_eq!(extract_tool_result_text(&json!([null, {"unknown":"value"}])), None);

    let fallback = to_tool_result_content(&json!({"unknown":"value"}));
    assert_eq!(text_from_tool_result(&fallback), r#"{"unknown":"value"}"#);

    let empty: ToolResultDto = serde_json::from_value(json!({})).expect("dto");
    assert_eq!(
        to_tool_result_content_from_dto(&empty),
        SessionToolResultContent::Text(String::new())
    );
}

#[test]
fn extract_tool_result_dto_text_handles_text_patch_and_empty_fallbacks() {
    let with_text: ToolResultDto = serde_json::from_value(json!({
        "content": [{"type":"text","text":"plain"}]
    }))
    .expect("dto");
    assert_eq!(extract_tool_result_dto_text(&with_text).as_deref(), Some("plain"));

    let with_blank_summary: ToolResultDto = serde_json::from_value(json!({
        "content": [{"type":"text","text":"content"}],
        "render_hint": {"summary":"   "}
    }))
    .expect("dto");
    assert_eq!(extract_tool_result_dto_text(&with_blank_summary).as_deref(), Some("content"));

    let with_json_miss_then_text: ToolResultDto = serde_json::from_value(json!({
        "content": [
            {"type":"json","value":{"unused":"value"}},
            {"type":"text","text":"after json"}
        ]
    }))
    .expect("dto");
    assert_eq!(
        extract_tool_result_dto_text(&with_json_miss_then_text).as_deref(),
        Some("after json")
    );

    let with_empty_patch_then_data: ToolResultDto = serde_json::from_value(json!({
        "content": [{"type":"structured_patch","hunks":[]}],
        "data": {"message":"from data"}
    }))
    .expect("dto");
    assert_eq!(
        extract_tool_result_dto_text(&with_empty_patch_then_data).as_deref(),
        Some("from data")
    );

    let empty: ToolResultDto = serde_json::from_value(json!({})).expect("dto");
    assert_eq!(extract_tool_result_dto_text(&empty), None);
}

#[test]
fn structured_patch_diff_text_groups_multiple_hunks_for_same_path() {
    let hunks = vec![
        StructuredPatchHunkDto {
            header: "@@ ignored @@".to_string(),
            path: None,
            old_start: None,
            old_lines: None,
            new_start: None,
            new_lines: None,
            lines: vec!["+ignored".to_string()],
        },
        StructuredPatchHunkDto {
            header: "@@ -1 +1 @@".to_string(),
            path: Some("src/lib.rs".to_string()),
            old_start: None,
            old_lines: None,
            new_start: None,
            new_lines: None,
            lines: vec!["-old".to_string(), "+new".to_string()],
        },
        StructuredPatchHunkDto {
            header: "@@ -3 +3 @@".to_string(),
            path: Some("src/lib.rs".to_string()),
            old_start: None,
            old_lines: None,
            new_start: None,
            new_lines: None,
            lines: vec!["+again".to_string()],
        },
    ];

    assert_eq!(
        structured_patch_diff_text(&hunks).as_deref(),
        Some("--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1 @@\n-old\n+new\n@@ -3 +3 @@\n+again\n")
    );
}

#[test]
fn parse_tool_result_dto_rejects_plain_objects_and_invalid_shapes() {
    assert!(parse_tool_result_dto(&json!({"message":"plain"}), "tool-6", Some("shell")).is_none());
    assert!(
        parse_tool_result_dto(
            &json!({"content":[{"type":"unknown","text":"bad"}]}),
            "tool-6",
            Some("shell"),
        )
        .is_none()
    );

    let parsed = parse_tool_result_dto(
        &json!({
            "tool_use_id": "   ",
            "tool_id": "existing",
            "content": [{"type":"text","text":"ok"}]
        }),
        "tool-6",
        Some("shell"),
    )
    .expect("dto parsed");
    assert_eq!(parsed.tool_use_id.as_deref(), Some("tool-6"));
    assert_eq!(parsed.tool_id.as_ref().map(|tool_id| tool_id.as_ref()), Some("existing"));

    let without_kind =
        parse_tool_result_dto(&json!({"success": true}), "tool-7", Some(" ")).expect("dto parsed");
    assert_eq!(without_kind.tool_id, None);
}

#[test]
fn tool_call_update_uses_kind_raw_string_and_result_success_without_status() {
    let mut conversation = create_session_conversation(Some(T0));

    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "tool_call_update",
            "toolCallId": "tool-8",
            "kind": " shell ",
            "title": "shell",
            "rawInput": "echo hi",
            "rawOutput": {
                "success": false,
                "content": [{"type":"text","text":"failed"}]
            }
        })),
        Some(T1),
    );

    let agent = last_agent(&conversation);
    match &agent.content[0] {
        SessionAgentContent::ToolUse(tool) => {
            assert_eq!(tool.name, "shell");
            assert_eq!(tool.raw_input, "echo hi");
            assert!(!tool.is_input_complete);
        }
        other => panic!("expected tool use, got {other:?}"),
    }
    let result = agent.tool_results.get("tool-8").expect("tool result");
    assert_eq!(result.tool_name, "shell");
    assert!(result.is_error);
    assert_eq!(text_from_tool_result(&result.content), "failed");
    assert_eq!(result.result.as_ref().and_then(|result| result.success), Some(false));
}

#[test]
fn tool_call_update_with_only_raw_input_does_not_create_result() {
    let mut agent = SessionAgentMessage {
        content: Vec::new(),
        tool_results: HashMap::new(),
        reasoning_details: None,
    };
    let update = json!({
        "sessionUpdate": "tool_call_update",
        "toolCallId": "tool-9",
        "rawInput": {"query":"value"}
    });

    apply_tool_call_update(&mut agent, update.as_object().expect("update object"));

    assert_eq!(agent.content.len(), 1);
    assert!(agent.tool_results.is_empty());
    assert!(
        matches!(&agent.content[0], SessionAgentContent::ToolUse(tool) if tool.raw_input == r#"{"query":"value"}"#)
    );
}

#[test]
fn usage_update_handles_direct_fields_without_user_message() {
    let mut conversation = create_session_conversation(Some(T0));

    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "usage_update",
            "used": 22,
            "size": 100,
            "_meta": {
                "usage": {
                    "input_tokens": 4,
                    "outputTokens": 5,
                    "cache_creation_input_tokens": 6,
                    "cacheReadInputTokens": 7
                }
            }
        })),
        Some(T1),
    );

    assert_eq!(conversation.cumulative_token_usage.input_tokens, Some(4));
    assert_eq!(conversation.cumulative_token_usage.output_tokens, Some(5));
    assert_eq!(conversation.cumulative_token_usage.cache_creation_input_tokens, Some(6));
    assert_eq!(conversation.cumulative_token_usage.cache_read_input_tokens, Some(7));
    assert!(conversation.request_token_usage.is_empty());

    let huge = json!({"huge": u64::MAX, "text": "7"});
    let source = huge.as_object().expect("object");
    assert_eq!(number_field(source, &["huge"]), None);
    assert_eq!(number_field(source, &["text"]), None);
}

#[test]
fn usage_update_does_not_treat_non_protocol_top_level_fields_as_token_breakdown() {
    let mut conversation = create_session_conversation(Some(T0));
    record_text_prompt_submission(&mut conversation, "question", Some(T1));
    let user_id = match &conversation.messages[0] {
        SessionMessage::User(user) => user.id.clone(),
        other => panic!("expected user message, got {other:?}"),
    };

    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "usage_update",
            "used": 15,
            "size": 100,
            "inputTokens": 9,
            "output_tokens": 6,
            "cachedWriteTokens": 4,
            "cache_read_input_tokens": 1
        })),
        Some("2026-04-21T00:00:02Z"),
    );

    assert_eq!(conversation.cumulative_token_usage, SessionTokenUsage::default());
    assert!(!conversation.request_token_usage.contains_key(&user_id));
}

#[test]
fn usage_update_ignores_meta_without_usage_object() {
    let mut conversation = create_session_conversation(Some(T0));

    record_session_update(
        &mut conversation,
        None,
        &notification(json!({
            "sessionUpdate": "usage_update",
            "used": 0,
            "size": 100,
            "_meta": {"other": {"inputTokens": 3}}
        })),
        Some(T1),
    );

    assert_eq!(conversation.cumulative_token_usage, SessionTokenUsage::default());
    assert!(conversation.request_token_usage.is_empty());
}

#[test]
fn number_field_reads_first_present_non_negative_key() {
    let source = json!({"a": -5, "b": 12, "c": 30});
    let object = source.as_object().expect("object");
    assert_eq!(number_field(object, &["missing", "a", "b"]), Some(12));
    assert_eq!(number_field(object, &["c"]), Some(30));
    assert_eq!(number_field(object, &["a"]), None);
    assert_eq!(number_field(object, &["missing"]), None);
}

#[test]
fn number_field_skips_unsigned_values_that_exceed_i64() {
    let source = json!({"too_large": u64::MAX, "fallback": 7});
    let object = source.as_object().expect("object");

    assert_eq!(number_field(object, &["too_large", "fallback"]), Some(7));
    assert_eq!(number_field(object, &["too_large"]), None);
}

#[test]
fn ensure_vwacp_state_returns_defaults_when_absent() {
    let state = ensure_vwacp_state(None);
    assert_eq!(state.current_mode_id, None);
    assert_eq!(state.desired_mode_id, None);
    assert_eq!(state.current_model_id, None);
    assert_eq!(state.available_models, None);
    assert_eq!(state.available_commands, None);
    assert_eq!(state.config_options, None);
    assert_eq!(state.session_options, None);

    let populated = populated_state();
    assert_eq!(ensure_vwacp_state(Some(&populated)), populated);
}

#[test]
fn ensure_agent_message_reuses_trailing_agent_and_starts_new_after_user() {
    let mut conversation = create_session_conversation(Some(T0));

    let first = ensure_agent_message(&mut conversation) as *mut SessionAgentMessage;
    let again = ensure_agent_message(&mut conversation) as *mut SessionAgentMessage;
    assert_eq!(first, again, "trailing agent message should be reused");
    assert_eq!(conversation.messages.len(), 1);

    conversation.messages.push(SessionMessage::User(SessionUserMessage {
        id: "user-x".to_string(),
        content: vec![SessionUserContent::Text("ask".to_string())],
    }));
    ensure_agent_message(&mut conversation);
    assert_eq!(conversation.messages.len(), 3);
    assert!(matches!(conversation.messages.last(), Some(SessionMessage::Agent(_))));
}

#[test]
fn last_user_message_id_returns_latest_user_and_none_when_absent() {
    let mut conversation = create_session_conversation(Some(T0));
    assert_eq!(last_user_message_id(&conversation), None);

    conversation.messages.push(SessionMessage::User(SessionUserMessage {
        id: "user-old".to_string(),
        content: vec![SessionUserContent::Text("old".to_string())],
    }));
    conversation.messages.push(SessionMessage::Agent(SessionAgentMessage {
        content: Vec::new(),
        tool_results: HashMap::new(),
        reasoning_details: None,
    }));
    conversation.messages.push(SessionMessage::User(SessionUserMessage {
        id: "user-new".to_string(),
        content: vec![SessionUserContent::Text("new".to_string())],
    }));

    assert_eq!(last_user_message_id(&conversation).as_deref(), Some("user-new"));
}

#[test]
fn ensure_tool_use_content_reuses_matching_entry() {
    let mut agent = SessionAgentMessage {
        content: Vec::new(),
        tool_results: HashMap::new(),
        reasoning_details: None,
    };

    let created = ensure_tool_use_content(&mut agent, "tool-a") as *mut SessionToolUse;
    let reused = ensure_tool_use_content(&mut agent, "tool-a") as *mut SessionToolUse;
    assert_eq!(created, reused, "same id should reuse the existing tool use");
    assert_eq!(agent.content.len(), 1);

    ensure_tool_use_content(&mut agent, "tool-b");
    assert_eq!(agent.content.len(), 2);
}

#[test]
fn to_raw_input_covers_string_null_none_and_object_variants() {
    let long = long_text(MAX_RUNTIME_TOOL_IO_CHARS + 1);
    let long_value = json!(long);
    let trimmed = to_raw_input(Some(&long_value));
    assert_eq!(trimmed.chars().count(), MAX_RUNTIME_TOOL_IO_CHARS);
    assert!(trimmed.ends_with("..."));

    assert_eq!(to_raw_input(Some(&serde_json::Value::Null)), "{}");
    assert_eq!(to_raw_input(None), "{}");
    assert_eq!(to_raw_input(Some(&json!({"key":"value"}))), r#"{"key":"value"}"#);
}

#[test]
fn state_updates_handle_empty_commands_title_clear_and_invalid_config() {
    let mut conversation = create_session_conversation(Some(T0));
    conversation.title = Some("Old".to_string());
    let state = populated_state();

    let mut returned = record_session_update(
        &mut conversation,
        Some(&state),
        &notification(json!({
            "sessionUpdate": "session_info_update",
            "title": null
        })),
        None,
    );
    assert_eq!(conversation.title, None);
    assert_eq!(conversation.updated_at, T0);
    assert_eq!(returned, state);

    returned = record_session_update(
        &mut conversation,
        Some(&returned),
        &notification(json!({
            "sessionUpdate": "available_commands_update",
            "availableCommands": []
        })),
        Some(T1),
    );
    assert_eq!(returned.available_commands, Some(Vec::new()));

    returned = record_session_update(
        &mut conversation,
        Some(&returned),
        &notification(json!({
            "sessionUpdate": "current_mode_update",
            "currentModeId": ""
        })),
        Some(T1),
    );
    assert_eq!(returned.current_mode_id.as_deref(), Some(""));

    returned = record_session_update(
        &mut conversation,
        Some(&returned),
        &notification(json!({
            "sessionUpdate": "config_option_update",
            "configOptions": []
        })),
        Some(T1),
    );
    assert_eq!(returned.config_options, Some(Vec::new()));
}

#[test]
fn record_client_operation_without_timestamp_uses_current_time() {
    let mut conversation = create_session_conversation(Some(T0));
    let operation = ClientOperation {
        method: ClientOperationMethod::TerminalWaitForExit,
        status: ClientOperationStatus::Completed,
        summary: "done".to_string(),
        details: None,
        timestamp: T1.to_string(),
    };

    let returned = record_client_operation(&mut conversation, None, &operation, None);

    assert_eq!(returned, ensure_vwacp_state(None));
    assert_rfc3339_timestamp(&conversation.updated_at);
    assert_ne!(conversation.updated_at, T0);
}

#[test]
fn trim_conversation_for_runtime_preserves_resume_and_non_string_outputs() {
    let output = json!({"nested":"value"});
    let mut conversation = SessionConversation {
        title: None,
        messages: vec![
            SessionMessage::Resume,
            SessionMessage::Agent(SessionAgentMessage {
                content: Vec::new(),
                tool_results: HashMap::from([(
                    "tool".to_string(),
                    SessionToolResult {
                        tool_use_id: "tool".to_string(),
                        tool_name: "shell".to_string(),
                        is_error: false,
                        content: SessionToolResultContent::Text("ok".to_string()),
                        output: Some(output.clone()),
                        result: None,
                    },
                )]),
                reasoning_details: None,
            }),
        ],
        updated_at: T0.to_string(),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: HashMap::new(),
    };

    trim_conversation_for_runtime(&mut conversation);

    assert!(matches!(conversation.messages[0], SessionMessage::Resume));
    let agent = last_agent(&conversation);
    let result = agent.tool_results.get("tool").expect("tool result");
    assert_eq!(result.output.as_ref(), Some(&output));
}
