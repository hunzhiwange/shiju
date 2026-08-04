use crate::app::models::{
    ChatMessage, ChatRole, ChatSession, ChatSessionMeta, ChatSessionStep, TokenUsage,
};

fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage { role, content: content.to_string(), model: None, think_timing: Vec::new(), processing_timing: None }
}

fn session_meta(id: &str, last_content: Option<&str>) -> ChatSessionMeta {
    ChatSessionMeta {
        id: id.to_string(),
        title: format!("Session {id}"),
        updated_ms: 20,
        message_count: 2,
        call_count: 1,
        last_content: last_content.map(str::to_string),
    }
}

fn session_step(index: u32, usage: TokenUsage) -> ChatSessionStep {
    ChatSessionStep {
        index,
        started_ms: u64::from(index),
        finished_ms: None,
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage,
        cost_usd: None,
        finish_reason: None,
        model: None,
    }
}

fn session_info(id: &str) -> vw_shared::session::info::Info {
    vw_shared::session::info::Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project-1".to_string(),
        directory: "/workspace/project".to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: "Session".to_string(),
        version: "1".to_string(),
        time: vw_shared::session::info::TimeInfo {
            created: 10,
            updated: 20,
            compacting: None,
            archived: None,
        },
        permission: None,
        revert: None,
    }
}

#[test]
fn shared_chat_messages_preserves_message_order_and_content() {
    let shared = super::shared_chat_messages(vec![
        message(ChatRole::User, "question"),
        message(ChatRole::Assistant, "answer"),
    ]);

    assert_eq!(shared.len(), 2);
    assert_eq!(shared[0].role, ChatRole::User);
    assert_eq!(shared[0].content, "question");
    assert_eq!(shared[1].role, ChatRole::Assistant);
    assert_eq!(shared[1].content, "answer");
}

#[test]
fn shared_chat_messages_accepts_an_empty_history() {
    let shared = super::shared_chat_messages(Vec::new());

    assert!(shared.is_empty());
}

#[test]
fn build_session_previews_filters_missing_and_blank_content() {
    let previews = super::build_session_previews(vec![
        session_meta("missing", None),
        session_meta("blank", Some(" \n\t ")),
        session_meta("visible", Some("content")),
    ]);

    assert_eq!(previews.len(), 1);
    assert_eq!(previews.get("visible").map(String::as_str), Some("content"));
}

#[test]
fn build_session_previews_trims_edges_and_flattens_newlines() {
    let previews =
        super::build_session_previews(vec![session_meta("session-1", Some("  first\nsecond  "))]);

    assert_eq!(previews.get("session-1").map(String::as_str), Some("first second"));
}

#[test]
fn build_session_previews_truncates_long_content_to_display_limit() {
    let content = "x".repeat(121);
    let expected = format!("{}…", "x".repeat(119));
    let previews = super::build_session_previews(vec![session_meta("session-1", Some(&content))]);

    assert_eq!(previews.get("session-1"), Some(&expected));
}

#[test]
fn build_session_previews_keeps_content_at_exact_display_limit() {
    let content = "x".repeat(120);
    let previews = super::build_session_previews(vec![session_meta("session-1", Some(&content))]);

    assert_eq!(previews.get("session-1"), Some(&content));
}

#[test]
fn build_session_previews_keeps_last_value_for_duplicate_session_id() {
    let previews = super::build_session_previews(vec![
        session_meta("session-1", Some("older")),
        session_meta("session-1", Some("newer")),
    ]);

    assert_eq!(previews.len(), 1);
    assert_eq!(previews.get("session-1").map(String::as_str), Some("newer"));
}

#[test]
fn session_usage_returns_zeroes_for_session_without_steps() {
    let session = ChatSession {
        id: "session-1".to_string(),
        title: "Session".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 10,
        updated_ms: 20,
    };

    assert_eq!(super::session_usage(&session), TokenUsage::default());
}

#[test]
fn session_usage_sums_every_token_category_across_steps() {
    let session = ChatSession {
        id: "session-1".to_string(),
        title: "Session".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: vec![
            session_step(
                1,
                TokenUsage {
                    input_tokens: 10,
                    output_tokens: 20,
                    cached_tokens: 3,
                    reasoning_tokens: 4,
                },
            ),
            session_step(
                2,
                TokenUsage {
                    input_tokens: 5,
                    output_tokens: 7,
                    cached_tokens: 11,
                    reasoning_tokens: 13,
                },
            ),
        ],
        created_ms: 10,
        updated_ms: 20,
    };

    assert_eq!(
        super::session_usage(&session),
        TokenUsage { input_tokens: 15, output_tokens: 27, cached_tokens: 14, reasoning_tokens: 17 }
    );
}

#[test]
fn reload_sessions_for_project_clears_current_session_state_before_loading() {
    let (mut app, _startup_task) = crate::app::App::new();
    app.session_previews.insert("session-1".to_string(), "preview".to_string());
    app.archived_session_ids.insert("session-1".to_string());
    app.sessions.push(session_info("session-1"));
    app.active_session_id = Some("session-1".to_string());
    app.chat.push(message(ChatRole::User, "stale message"));
    app.usage =
        TokenUsage { input_tokens: 10, output_tokens: 20, cached_tokens: 3, reasoning_tokens: 4 };
    app.chat_visible_text_cache.push(Some("stale text".to_string()));
    app.chat_message_expanded.insert(0);

    let _reload_task = app.reload_sessions_for_project(None);

    assert!(app.session_previews.is_empty());
    assert!(app.archived_session_ids.is_empty());
    assert!(app.sessions.is_empty());
    assert_eq!(app.active_session_id, None);
    assert!(app.chat.is_empty());
    assert_eq!(app.usage, TokenUsage::default());
    assert!(app.chat_visible_text_cache.is_empty());
    assert!(app.chat_message_expanded.is_empty());
}

#[test]
fn reload_sessions_for_project_with_path_returns_runnable_task() {
    let (mut app, _startup_task) = crate::app::App::new();

    let reload_task = app.reload_sessions_for_project(Some("/workspace/project".to_string()));

    assert!(iced_runtime::task::into_stream(reload_task.collect()).is_some());
}
