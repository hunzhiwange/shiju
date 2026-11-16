//! 聊天流局部规则：直接使用领域输入，宿主仅保留跨域装配契约。
use crate::message::chat::request::{AcpHistoryReplayMode, ChatSendBehavior, QueueItem};
use crate::message::chat::stream::projection::{directory_for_save, load_snapshot};
use crate::message::chat::stream::stream_events::{
    leading_guide_count, stream_delta_flush_interval,
};
use session_ui_types::{ChatMessage as ModelChatMessage, ChatRole};
fn chat_message(role: ChatRole, content: &str) -> ModelChatMessage {
    ModelChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}
fn session_info(id: &str, title: &str, directory: &str) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project-1".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "0.0.0".to_string(),
        time: session_info::TimeInfo { created: 10, updated: 20, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}
fn queue_item(send_behavior: ChatSendBehavior) -> QueueItem {
    QueueItem {
        created_ms: 1,
        query: "query".to_string(),
        attachments: Vec::new(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_model_source: Default::default(),
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        send_behavior,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
    }
}
#[test]
fn session_directory_for_save_skips_blank_session_directory() {
    assert_eq!(
        directory_for_save(Some("  \n".into()), Some("/project")).as_deref(),
        Some("/project")
    );
}
#[test]
fn session_directory_for_save_prefers_known_session_directory() {
    assert_eq!(
        directory_for_save(Some("  /session/root  ".into()), Some("/project/root")).as_deref(),
        Some("  /session/root  ")
    );
}
#[test]
fn session_directory_for_save_rejects_blank_fallbacks() {
    assert_eq!(directory_for_save(Some(" \n\t ".into()), Some("   ")).as_deref(), None);
}
#[test]
fn active_stream_delta_flush_interval_is_one_frame() {
    assert_eq!(
        stream_delta_flush_interval(Some("active-session"), "active-session"),
        std::time::Duration::from_millis(16)
    );
}
#[test]
fn background_stream_delta_keeps_existing_interval() {
    assert_eq!(
        stream_delta_flush_interval(Some("active-session"), "background-session"),
        std::time::Duration::from_millis(100)
    );
}
#[test]
fn leading_guide_count_only_counts_initial_guide_items() {
    let guide = queue_item(ChatSendBehavior::Guide);
    let normal = queue_item(ChatSendBehavior::Queue);
    let late_guide = queue_item(ChatSendBehavior::Guide);

    assert_eq!(leading_guide_count(&[guide, normal, late_guide]), 1);
}
#[test]
fn leading_guide_count_handles_empty_and_all_guide_queues() {
    assert_eq!(leading_guide_count(&[]), 0);
    assert_eq!(
        leading_guide_count(&[
            queue_item(ChatSendBehavior::Guide),
            queue_item(ChatSendBehavior::Guide),
        ]),
        2
    );
}
#[test]
fn load_session_or_default_replaces_mismatched_cached_message_ids() {
    let info = session_info("session-1", "Title", "/project");
    let cached = Some((
        vec![chat_message(ChatRole::User, "question"), chat_message(ChatRole::Assistant, "answer")],
        Some(vec![Some("only-one-id".to_string())]),
    ));

    let loaded = load_snapshot("session-1".to_string(), Some(&info), None, cached, 0);

    assert_eq!(loaded.messages.len(), 2);
    assert_eq!(loaded.message_ids, vec![None, None]);
    assert!(loaded.calls.is_empty());
    assert!(loaded.steps.is_empty());
}
#[test]
fn load_session_or_default_keeps_inactive_session_metadata_without_cache() {
    let info = session_info("session-1", "Stored title", "/project");

    let cached = None;
    let loaded = load_snapshot("session-1".to_string(), Some(&info), None, cached, 0);

    assert_eq!(loaded.id, "session-1");
    assert_eq!(loaded.title, "Stored title");
    assert_eq!(loaded.created_ms, 10);
    assert_eq!(loaded.updated_ms, 20);
    assert!(loaded.messages.is_empty());
    assert!(loaded.message_ids.is_empty());
    assert!(loaded.calls.is_empty());
    assert!(loaded.steps.is_empty());
}
#[test]
fn load_session_or_default_preserves_matching_cached_message_ids() {
    let info = session_info("session-1", "Cached", "/project");
    let cached = Some((
        vec![chat_message(ChatRole::User, "cached")],
        Some(vec![Some("cached-id".to_string())]),
    ));

    let loaded = load_snapshot("session-1".to_string(), Some(&info), None, cached, 0);

    assert_eq!(loaded.messages[0].content, "cached");
    assert_eq!(loaded.message_ids, vec![Some("cached-id".to_string())]);
}
