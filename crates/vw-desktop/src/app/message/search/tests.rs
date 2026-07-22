#![allow(unused_must_use)]
use super::*;
use crate::app::models::{ChatMessage, ChatRole};

fn chat_message(content: &str) -> ChatMessage {
    ChatMessage { role: ChatRole::User, content: content.to_string(), think_timing: Vec::new() }
}

fn app() -> App {
    App::new().0
}

fn session_info(id: &str, directory: &str) -> vw_shared::session::info::Info {
    vw_shared::session::info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: id.to_string(),
        version: "1".to_string(),
        time: vw_shared::session::info::TimeInfo {
            created: 1,
            updated: 1,
            compacting: None,
            archived: None,
        },
        permission: None,
        revert: None,
    }
}

async fn collect_task_messages(task: Task<Message>) -> Vec<Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

#[test]
fn input_changed_updates_text_overlay_and_refreshes_cache() {
    let mut app = app();
    app.project_path = Some("/workspace".to_string());
    app.set_file_index("/workspace", vec!["src/query.rs".to_string(), "src/other.rs".to_string()]);

    update(&mut app, SearchMessage::InputChanged("  query  ".to_string()));
    assert_eq!(app.search_text, "  query  ");
    assert!(app.show_search_overlay);
    assert_eq!(app.search_panel_file_cache_query, "query");
    assert_eq!(app.cached_search_panel_file_results(), ["src/query.rs"]);

    update(&mut app, SearchMessage::InputChanged("   ".to_string()));
    assert_eq!(app.search_text, "   ");
    assert!(!app.show_search_overlay);
    assert!(app.search_panel_file_cache_query.is_empty());
    assert_eq!(app.cached_search_panel_file_results(), ["src/query.rs", "src/other.rs"]);
}

#[test]
fn open_clears_previous_query_and_shows_overlay() {
    let mut app = app();
    app.project_path = Some("/workspace".to_string());
    app.set_file_index("/workspace", vec!["src/lib.rs".to_string()]);
    app.search_text = "missing".to_string();
    app.refresh_search_panel_file_cache();
    assert!(app.cached_search_panel_file_results().is_empty());

    update(&mut app, SearchMessage::Open);

    assert!(app.search_text.is_empty());
    assert!(app.show_search_overlay);
    assert_eq!(app.cached_search_panel_file_results(), ["src/lib.rs"]);
}

#[test]
fn toggle_sets_overlay_visibility_directly() {
    let mut app = app();

    update(&mut app, SearchMessage::Toggle(true));
    assert!(app.show_search_overlay);

    update(&mut app, SearchMessage::Toggle(false));
    assert!(!app.show_search_overlay);
}

#[test]
fn select_file_sets_file_input_and_hides_overlay() {
    let mut app = app();
    app.show_search_overlay = true;

    update(&mut app, SearchMessage::SelectFile("/tmp/data.json".to_string()));

    assert_eq!(app.file_url_input, "/tmp/data.json");
    assert!(!app.show_search_overlay);
}

#[test]
fn select_session_without_known_project_prepares_active_session_state() {
    let mut app = app();
    app.show_search_overlay = true;
    app.active_session_view_state.updated_ms = 123;
    app.active_session_view_state.base_ready = true;
    app.active_session_view_state.ui_preparing = false;
    app.active_session_id = Some("session-1".to_string());
    app.get_session_runtime_mut("session-1").is_requesting = true;
    app.chat = vec![chat_message("cached message")];
    app.cache_active_session_chat();
    app.active_session_id = Some("previous-session".to_string());
    app.get_session_runtime_mut("previous-session").is_requesting = true;
    app.chat = vec![chat_message("previous message")];

    update(&mut app, SearchMessage::SelectSession("session-1".to_string()));

    assert!(!app.show_search_overlay);
    assert_eq!(app.active_session_id.as_deref(), Some("session-1"));
    assert_eq!(app.usage, crate::app::models::TokenUsage::default());
    assert_eq!(app.active_session_view_state.updated_ms, 0);
    assert!(app.active_session_view_state.ui_preparing);
    assert!(!app.active_session_view_state.base_ready);
    assert_eq!(app.chat.len(), 1);
    assert_eq!(app.chat[0].content, "cached message");
    assert_eq!(
        app.cached_chat_messages("previous-session").unwrap()[0].content,
        "previous message"
    );
    assert!(!app.active_session_view_state.preparing_chat_ui_chunks.is_empty());
    assert!(app.active_session_view_state.pinned_chat_ui_chunk_start.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn select_session_with_known_directory_routes_to_project_session() {
    let mut app = app();
    app.show_search_overlay = true;
    app.sessions = vec![session_info("session-1", "/workspace")];

    let messages = collect_task_messages(update(
        &mut app,
        SearchMessage::SelectSession("session-1".to_string()),
    ))
    .await;

    assert!(!app.show_search_overlay);
    assert!(app.active_session_id.is_none());
    assert!(matches!(
        messages.as_slice(),
        [Message::Project(
            crate::app::message::project::ProjectMessage::OpenProjectSessionPressed(path, id)
        )] if path == "/workspace" && id == "session-1"
    ));
}

#[test]
fn select_session_rejects_blank_known_directory_and_uses_direct_load() {
    let mut app = app();
    app.sessions = vec![session_info("session-1", " \t ")];

    update(&mut app, SearchMessage::SelectSession("session-1".to_string()));

    assert_eq!(app.active_session_id.as_deref(), Some("session-1"));
    assert!(app.active_session_view_state.ui_preparing);
}

#[test]
fn select_session_without_cached_chat_skips_chunk_prewarm_state() {
    let mut app = app();
    app.active_session_id = Some("previous-session".to_string());
    app.chat.clear();

    update(&mut app, SearchMessage::SelectSession("empty-session".to_string()));

    assert_eq!(app.active_session_id.as_deref(), Some("empty-session"));
    assert!(app.chat.is_empty());
    assert!(app.active_session_view_state.ui_preparing);
    assert!(app.active_session_view_state.preparing_chat_ui_chunks.is_empty());
    assert!(app.active_session_view_state.pinned_chat_ui_chunk_start.is_none());
}

#[test]
fn select_project_hides_overlay_before_returning_open_task() {
    let mut app = app();
    app.show_search_overlay = true;

    update(&mut app, SearchMessage::SelectProject("/tmp/project".to_string()));

    assert!(!app.show_search_overlay);
}
