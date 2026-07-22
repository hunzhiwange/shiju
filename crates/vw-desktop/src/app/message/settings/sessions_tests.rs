use super::*;
use crate::app::models::{ChatMessage, ChatRole, TokenUsage};
use vw_shared::session::info::{Info, TimeInfo};

fn app() -> App {
    let mut app = App::new().0;
    app.active_session_id = None;
    app.sessions.clear();
    app.session_runtime_states.clear();
    app.chat.clear();
    app.usage = TokenUsage::default();
    app
}

fn session(id: &str) -> Info {
    Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: format!("/workspace/{id}"),
        parent_id: None,
        summary: None,
        share: None,
        title: id.to_string(),
        version: "0.0.0".to_string(),
        time: TimeInfo { created: 1, updated: 2, compacting: None, archived: None },
        permission: None,
        revert: None,
    }
}

fn chat_message(content: &str) -> ChatMessage {
    ChatMessage { role: ChatRole::User, content: content.to_string(), think_timing: Vec::new() }
}

#[test]
fn session_delete_active_only_session_clears_conversation_and_usage() {
    let mut app = app();
    app.active_session_id = Some("only".to_string());
    app.sessions.push(session("only"));
    app.get_session_runtime_mut("only").is_requesting = true;
    app.chat.push(chat_message("hello"));
    app.usage =
        TokenUsage { input_tokens: 12, output_tokens: 7, cached_tokens: 3, reasoning_tokens: 2 };

    let _task = update(&mut app, SettingsMessage::SessionDelete("only".to_string()));

    assert!(app.sessions.is_empty());
    assert!(!app.session_runtime_states.contains_key("only"));
    assert_eq!(app.active_session_id, None);
    assert!(app.chat.is_empty());
    assert_eq!(app.usage, TokenUsage::default());
}

#[test]
fn session_delete_active_session_switches_to_first_remaining_and_syncs_preferences() {
    let mut app = app();
    app.active_session_id = Some("deleted".to_string());
    app.sessions = vec![session("deleted"), session("next"), session("later")];
    app.get_session_runtime_mut("deleted").is_requesting = true;
    let next_runtime = app.get_session_runtime_mut("next");
    next_runtime.model = "next-model".to_string();
    next_runtime.auto_model = false;
    next_runtime.has_unseen_success = true;
    app.model = "old-model".to_string();
    app.auto_model = true;
    app.chat.push(chat_message("preserved until load completes"));
    app.usage.input_tokens = 9;

    let _task = update(&mut app, SettingsMessage::SessionDelete("deleted".to_string()));

    assert_eq!(
        app.sessions.iter().map(|session| session.id.as_str()).collect::<Vec<_>>(),
        vec!["next", "later"]
    );
    assert_eq!(app.active_session_id.as_deref(), Some("next"));
    assert!(!app.session_runtime_states.contains_key("deleted"));
    assert!(!app.get_session_runtime("next").has_unseen_success);
    assert_eq!(app.model, "next-model");
    assert!(!app.auto_model);
    assert_eq!(app.chat[0].content, "preserved until load completes");
    assert_eq!(app.usage.input_tokens, 9);
}

#[test]
fn session_delete_inactive_session_preserves_active_context() {
    let mut app = app();
    app.active_session_id = Some("active".to_string());
    app.sessions = vec![session("active"), session("inactive")];
    app.get_session_runtime_mut("active").is_requesting = true;
    app.get_session_runtime_mut("inactive").is_requesting = true;
    app.chat.push(chat_message("keep me"));
    app.usage.output_tokens = 5;

    let _task = update(&mut app, SettingsMessage::SessionDelete("inactive".to_string()));

    assert_eq!(
        app.sessions.iter().map(|session| session.id.as_str()).collect::<Vec<_>>(),
        vec!["active"]
    );
    assert_eq!(app.active_session_id.as_deref(), Some("active"));
    assert!(app.session_runtime_states.contains_key("active"));
    assert!(!app.session_runtime_states.contains_key("inactive"));
    assert_eq!(app.chat[0].content, "keep me");
    assert_eq!(app.usage.output_tokens, 5);
}

#[test]
fn session_delete_missing_session_removes_orphan_runtime_without_changing_context() {
    let mut app = app();
    app.active_session_id = Some("active".to_string());
    app.sessions.push(session("active"));
    app.get_session_runtime_mut("orphan").is_requesting = true;
    app.chat.push(chat_message("keep me"));

    let _task = update(&mut app, SettingsMessage::SessionDelete("orphan".to_string()));

    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.sessions[0].id, "active");
    assert_eq!(app.active_session_id.as_deref(), Some("active"));
    assert!(!app.session_runtime_states.contains_key("orphan"));
    assert_eq!(app.chat[0].content, "keep me");
}

#[test]
fn session_delete_missing_active_session_switches_to_first_available_session() {
    let mut app = app();
    app.active_session_id = Some("missing".to_string());
    app.sessions = vec![session("next"), session("later")];
    let next_runtime = app.get_session_runtime_mut("next");
    next_runtime.model = "next-model".to_string();
    next_runtime.auto_model = false;
    next_runtime.has_unseen_success = true;
    app.model = "old-model".to_string();
    app.auto_model = true;

    let _task = update(&mut app, SettingsMessage::SessionDelete("missing".to_string()));

    assert_eq!(
        app.sessions.iter().map(|session| session.id.as_str()).collect::<Vec<_>>(),
        vec!["next", "later"]
    );
    assert_eq!(app.active_session_id.as_deref(), Some("next"));
    assert!(!app.get_session_runtime("next").has_unseen_success);
    assert_eq!(app.model, "next-model");
    assert!(!app.auto_model);
}

#[test]
fn session_copy_leaves_local_session_state_unchanged_until_task_completes() {
    let mut app = app();
    app.active_session_id = Some("active".to_string());
    app.sessions.push(session("active"));
    app.get_session_runtime_mut("active").is_requesting = true;
    app.chat.push(chat_message("unchanged"));

    let _task = update(&mut app, SettingsMessage::SessionCopy("active".to_string()));

    assert_eq!(app.active_session_id.as_deref(), Some("active"));
    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.sessions[0].id, "active");
    assert!(app.get_session_runtime("active").is_requesting);
    assert_eq!(app.chat[0].content, "unchanged");
}

#[test]
fn unrelated_settings_message_is_ignored() {
    let mut app = app();
    app.active_session_id = Some("active".to_string());
    app.sessions.push(session("active"));
    app.chat.push(chat_message("unchanged"));
    app.usage.cached_tokens = 4;

    let _task = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert_eq!(app.active_session_id.as_deref(), Some("active"));
    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.chat[0].content, "unchanged");
    assert_eq!(app.usage.cached_tokens, 4);
}
