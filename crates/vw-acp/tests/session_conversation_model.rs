//! 验证 ACP 会话对话模型的创建、更新、用量记录与运行时裁剪。
//!
//! 会话记录既要持久化给 UI 回放，也要裁剪后喂给运行时；这些测试确保用户消息、
//! agent 增量、模式状态和 token usage 在同一模型中按预期落位。

use std::collections::HashMap;

use agent_client_protocol::SessionNotification;
use serde_json::json;
use vw_acp::{
    SessionAcpxState, SessionAgentContent, SessionConversation, SessionMessage, SessionTokenUsage,
    create_session_conversation, record_session_update, record_text_prompt_submission,
    trim_conversation_for_runtime,
};

/// 构造空 ACP 扩展状态，作为 session update 应用时的基线。
fn empty_state() -> SessionAcpxState {
    SessionAcpxState {
        current_mode_id: None,
        desired_mode_id: None,
        current_model_id: None,
        available_models: None,
        available_commands: None,
        config_options: None,
        desired_config_options: None,
        session_options: None,
    }
}

/// 用给定 update 负载构造协议通知，保证测试输入保持真实 ACP 外层形状。
fn sample_notification(value: serde_json::Value) -> SessionNotification {
    serde_json::from_value(json!({
        "sessionId": "session-1",
        "update": value
    }))
    .unwrap()
}

/// 新会话对话应初始化为可持久化、无消息且用量清零的形状。
#[test]
fn create_session_conversation_initializes_runtime_shape() {
    let conversation = create_session_conversation(Some("2026-04-04T00:00:00Z"));
    assert_eq!(conversation.title, None);
    assert_eq!(conversation.updated_at, "2026-04-04T00:00:00Z");
    assert!(conversation.messages.is_empty());
    assert_eq!(conversation.cumulative_token_usage, SessionTokenUsage::default());
}

/// 文本 prompt 提交应追加用户消息并刷新会话更新时间。
#[test]
fn record_text_prompt_submission_adds_user_message() {
    let mut conversation = create_session_conversation(Some("2026-04-04T00:00:00Z"));
    record_text_prompt_submission(&mut conversation, "hello world", Some("2026-04-04T00:00:01Z"));

    assert_eq!(conversation.updated_at, "2026-04-04T00:00:01Z");
    assert!(matches!(
        &conversation.messages[0],
        SessionMessage::User(user) if !user.id.is_empty()
            && matches!(user.content.first(), Some(vw_acp::SessionUserContent::Text(text)) if text == "hello world")
    ));
}

/// agent 文本增量和模式更新应分别进入消息列表与 ACP 状态。
#[test]
fn record_session_update_appends_agent_text_and_updates_mode() {
    let mut conversation = create_session_conversation(Some("2026-04-04T00:00:00Z"));
    let notification = sample_notification(json!({
        "sessionUpdate": "agent_message_chunk",
        "content": {
            "type": "text",
            "text": "partial "
        }
    }));
    let _ = record_session_update(
        &mut conversation,
        Some(&empty_state()),
        &notification,
        Some("2026-04-04T00:00:02Z"),
    );

    let mode_notification = sample_notification(json!({
        "sessionUpdate": "current_mode_update",
        "currentModeId": "focus"
    }));
    let state = record_session_update(
        &mut conversation,
        Some(&empty_state()),
        &mode_notification,
        Some("2026-04-04T00:00:03Z"),
    );

    assert_eq!(state.current_mode_id.as_deref(), Some("focus"));
    assert!(matches!(
        &conversation.messages[0],
        SessionMessage::Agent(agent)
            if matches!(agent.content.first(), Some(SessionAgentContent::Text(text)) if text == "partial ")
    ));
}

/// usage update 应累计到会话总量，并归因到最近一次用户请求。
#[test]
fn record_session_update_tracks_usage_for_last_user_message() {
    let mut conversation = SessionConversation {
        title: None,
        messages: vec![SessionMessage::User(vw_acp::SessionUserMessage {
            id: "user-1".to_string(),
            content: vec![vw_acp::SessionUserContent::Text("prompt".to_string())],
        })],
        updated_at: "2026-04-04T00:00:00Z".to_string(),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: HashMap::new(),
    };

    let notification = sample_notification(json!({
        "sessionUpdate": "usage_update",
        "used": 1,
        "size": 2,
        "_meta": {
            "usage": {
                "inputTokens": 10,
                "outputTokens": 4,
                "cachedReadTokens": 2
            }
        }
    }));
    let _ = record_session_update(
        &mut conversation,
        Some(&empty_state()),
        &notification,
        Some("2026-04-04T00:00:04Z"),
    );

    assert_eq!(conversation.cumulative_token_usage.input_tokens, Some(10));
    assert_eq!(conversation.request_token_usage.get("user-1").unwrap().output_tokens, Some(4));
}

/// 运行时裁剪需要限制消息数量，避免历史会话无限膨胀后拖慢模型调用。
#[test]
fn trim_conversation_for_runtime_limits_messages() {
    let mut conversation = create_session_conversation(Some("2026-04-04T00:00:00Z"));
    for index in 0..205 {
        conversation.messages.push(SessionMessage::User(vw_acp::SessionUserMessage {
            id: format!("user-{index}"),
            content: vec![vw_acp::SessionUserContent::Text("x".repeat(10))],
        }));
    }

    trim_conversation_for_runtime(&mut conversation);
    assert_eq!(conversation.messages.len(), 200);
}
