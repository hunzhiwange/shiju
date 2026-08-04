#![allow(unused_must_use)]
//! 处理聊天流式会话事件。
//! 本模块把网关轮询和流式增量落到会话状态，避免 UI 层理解传输细节。

use super::permissions::{
    advance_permission_requests_after_reply, split_auto_approved_permission_requests,
    sync_permission_requests,
};

fn request(id: &str) -> vw_gateway_client::PendingPermissionRequestDto {
    vw_gateway_client::PendingPermissionRequestDto {
        id: id.to_string(),
        session_id: "session-1".to_string(),
        permission: "shell".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: None,
    }
}

fn chat_message(
    role: crate::app::models::ChatRole,
    content: &str,
) -> crate::app::models::ChatMessage {
    crate::app::models::ChatMessage { role, content: content.to_string(), model: None, think_timing: Vec::new(), processing_timing: None }
}

fn session_info(id: &str, title: &str, directory: &str) -> vw_shared::session::info::Info {
    vw_shared::session::info::Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project-1".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "0.0.0".to_string(),
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
fn sync_permission_requests_preserves_existing_selection_after_sort() {
    let (requests, selected_request_id) = sync_permission_requests(
        Some("perm-2"),
        vec![request("perm-3"), request("perm-2"), request("perm-1")],
    );

    assert_eq!(
        requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["perm-1", "perm-2", "perm-3"]
    );
    assert_eq!(selected_request_id.as_deref(), Some("perm-2"));
}

#[test]
fn advance_permission_requests_after_reply_selects_next_request() {
    let (requests, selected_request_id) = advance_permission_requests_after_reply(
        Some("perm-2"),
        vec![request("perm-1"), request("perm-2"), request("perm-3")],
    );

    assert_eq!(
        requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["perm-1", "perm-3"]
    );
    assert_eq!(selected_request_id.as_deref(), Some("perm-3"));
}

#[test]
fn advance_permission_requests_after_reply_falls_back_to_previous_request() {
    let (requests, selected_request_id) = advance_permission_requests_after_reply(
        Some("perm-3"),
        vec![request("perm-1"), request("perm-2"), request("perm-3")],
    );

    assert_eq!(
        requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["perm-1", "perm-2"]
    );
    assert_eq!(selected_request_id.as_deref(), Some("perm-2"));
}

#[test]
fn split_auto_approved_permission_requests_separates_active_session_requests() {
    let mut other_request = request("perm-2");
    other_request.session_id = "session-2".to_string();

    let (remaining_requests, auto_approve_request_ids) = split_auto_approved_permission_requests(
        Some("session-1"),
        vec![request("perm-1"), other_request, request("perm-3")],
    );

    assert_eq!(
        remaining_requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["perm-2"]
    );
    assert_eq!(auto_approve_request_ids, vec!["perm-1", "perm-3"]);
}

#[test]
fn session_directory_for_save_prefers_known_session_directory_then_project_path() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/project".to_string());
    app.sessions = vec![session_info("s1", "Session", "/session")];

    assert_eq!(super::session_directory_for_save(&app, "s1").as_deref(), Some("/session"));
    assert_eq!(super::session_directory_for_save(&app, "missing").as_deref(), Some("/project"));

    app.project_path = Some("   ".to_string());
    assert_eq!(super::session_directory_for_save(&app, "missing"), None);
}

#[test]
fn load_session_or_default_uses_active_chat_and_steps_for_active_session() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());
    app.sessions = vec![session_info("s1", "Title", "/repo")];
    app.chat = vec![chat_message(crate::app::models::ChatRole::User, "hi")];
    app.chat_message_ids = vec![Some("m1".to_string())];
    app.active_session_view_state.steps = vec![crate::app::models::ChatSessionStep {
        index: 1,
        started_ms: 10,
        finished_ms: None,
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: crate::app::models::TokenUsage::default(),
        cost_usd: None,
        finish_reason: None,
        model: Some("model".to_string()),
    }];

    let loaded = super::load_session_or_default(&app, "s1".to_string());

    assert_eq!(loaded.title, "Title");
    assert_eq!(loaded.messages.len(), app.chat.len());
    assert_eq!(loaded.messages[0].role, crate::app::models::ChatRole::User);
    assert_eq!(loaded.messages[0].content, "hi");
    assert_eq!(loaded.message_ids, app.chat_message_ids);
    assert_eq!(loaded.steps.len(), 1);
    assert_eq!(loaded.created_ms, 10);
    assert_eq!(loaded.updated_ms, 20);
}

#[test]
fn load_session_or_default_uses_cached_chat_for_inactive_session() {
    let (mut app, _task) = crate::app::App::new();
    app.sessions = vec![session_info("s1", "Title", "/repo")];
    app.store_session_chat_snapshot(
        "s1".to_string(),
        crate::app::session::shared_chat_messages(vec![
            chat_message(crate::app::models::ChatRole::User, "hi"),
            chat_message(crate::app::models::ChatRole::Assistant, "hello"),
        ]),
        vec![Some("m1".to_string()), Some("m2".to_string())],
    );

    let loaded = super::load_session_or_default(&app, "s1".to_string());

    assert_eq!(loaded.messages.len(), 2);
    assert_eq!(loaded.message_ids, vec![Some("m1".to_string()), Some("m2".to_string())]);
    assert!(loaded.steps.is_empty());
}

#[test]
fn load_session_or_default_falls_back_when_session_unknown() {
    let (app, _task) = crate::app::App::new();

    let loaded = super::load_session_or_default(&app, "missing".to_string());

    assert_eq!(loaded.id, "missing");
    assert_eq!(loaded.title, "新会话");
    assert!(loaded.messages.is_empty());
    assert!(loaded.message_ids.is_empty());
}

#[test]
fn start_next_noops_for_empty_queue() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());

    super::start_next(&mut app, "s1");

    assert!(app.get_session_runtime("s1").queue.is_empty());
}
