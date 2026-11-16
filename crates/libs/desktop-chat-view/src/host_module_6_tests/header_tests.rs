//! 验证聊天标题栏的会话状态、工作区标记、返回操作与主题样式。
//! fixture 组合近期会话、项目缓存和运行态队列，覆盖父会话回退查找与空标识拒绝。

use super::header_fixture::HeaderFixture as App;
use crate::header::*;
use desktop_assets::assets::Icon;
use iced::{Background, Color, Element, Theme};
type Message = HeaderAction;
use desktop_session::message::chat::request::AcpHistoryReplayMode;
use desktop_session::message::chat::request::{ChatSendBehavior, QueueItem};
type SessionRuntimeState = desktop_session::session::agent::SessionRuntimeState<()>;
use iced::widget::button;
use session_info::{Info, TimeInfo};

fn test_app() -> App {
    App::new().0
}

fn active_header_app() -> App {
    let mut app = test_app();
    app.active_session_id = Some("active".to_string());
    app.project_path = Some("/workspace/main".to_string());
    app.sessions.push(session("active", "/workspace/main"));
    app
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        _ => panic!("expected color background"),
    }
}

fn session(id: &str, directory: &str) -> Info {
    Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: format!("{id} title"),
        version: "1".to_string(),
        time: TimeInfo { created: 1, updated: 2, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

fn queue_item(query: &str) -> QueueItem {
    QueueItem {
        created_ms: 1,
        query: query.to_string(),
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
        acp_recent_count: 3,
        full_access_enabled: true,
        send_behavior: ChatSendBehavior::Queue,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
    }
}

#[test]
fn half_fullscreen_control_is_available_in_code_and_work_modes() {
    assert!(show_half_fullscreen_control());
    assert!(show_half_fullscreen_control());
}

#[test]
fn active_session_directory_returns_none_without_matching_session() {
    let mut app = test_app();

    assert_eq!(active_session_directory(&app.input()), None);

    app.active_session_id = Some("missing".to_string());
    app.sessions.push(session("other", "/workspace/other"));

    assert_eq!(active_session_directory(&app.input()), None);
}

#[test]
fn active_session_directory_returns_active_session_directory() {
    let mut app = test_app();
    app.active_session_id = Some("active".to_string());
    app.sessions.push(session("inactive", "/workspace/inactive"));
    app.sessions.push(session("active", "/workspace/active"));

    assert_eq!(active_session_directory(&app.input()).as_deref(), Some("/workspace/active"));
}

#[test]
fn workspace_badge_state_labels_primary_named_and_fallback_workspaces() {
    let mut app = test_app();
    app.project_path = Some("/workspace/main".to_string());
    app.active_session_id = Some("primary".to_string());
    app.sessions.push(session("primary", "/workspace/main"));

    assert_eq!(session_workspace_badge_state(&app.input()), Some(("主工作区".to_string(), true)));

    app.active_session_id = Some("named".to_string());
    app.sessions.push(session("named", "/workspace/worktrees/feature-a"));
    assert_eq!(session_workspace_badge_state(&app.input()), Some(("feature-a".to_string(), false)));

    app.active_session_id = Some("root".to_string());
    app.sessions.push(session("root", "/"));
    assert_eq!(session_workspace_badge_state(&app.input()), Some(("工作区".to_string(), false)));
}

#[test]
fn workspace_badge_element_builds_only_when_active_session_has_directory() {
    let app = test_app();
    assert!(session_workspace_badge(&app.input()).is_none());

    let app = active_header_app();
    let badge = session_workspace_badge(&app.input()).expect("active session should show badge");
    keep_element(badge);
}

#[test]
fn session_status_defaults_when_no_active_session() {
    let app = test_app();
    let (is_running, queued, has_unseen_success, breathe, color) = get_session_status(&app.input());

    assert!(!is_running);
    assert_eq!(queued, 0);
    assert!(!has_unseen_success);
    assert_eq!(breathe, 0.0);
    assert_eq!(color, Color::TRANSPARENT);
}

#[test]
fn session_status_defaults_runtime_flags_when_active_session_has_no_runtime_state() {
    let mut app = test_app();
    app.active_session_id = Some("active-without-runtime".to_string());

    let (is_running, queued, has_unseen_success, breathe, color) = get_session_status(&app.input());

    assert!(!is_running);
    assert_eq!(queued, 0);
    assert!(!has_unseen_success);
    assert!((0.0..=1.0).contains(&breathe));
    assert_ne!(color, Color::TRANSPARENT);
}

#[test]
fn session_status_reads_runtime_flags_queue_and_animation_color() {
    let mut app = test_app();
    app.active_session_id = Some("active".to_string());

    let mut runtime = SessionRuntimeState::default();
    runtime.request_lifecycle.is_requesting = true;
    runtime.has_unseen_success = true;
    runtime.request_lifecycle.queue.push(queue_item("first"));
    runtime.request_lifecycle.queue.push(queue_item("second"));
    app.session_runtime_states.insert("active".to_string(), runtime);

    let (is_running, queued, has_unseen_success, breathe, color) = get_session_status(&app.input());

    assert!(is_running);
    assert_eq!(queued, 2);
    assert!(has_unseen_success);
    assert!((0.0..=1.0).contains(&breathe));
    assert_ne!(color, Color::TRANSPARENT);
}

#[test]
fn session_actions_elements_build_for_empty_and_active_session() {
    let mut app = test_app();

    keep_element(session_actions_button("重命名", Some(Message::None)));
    keep_element(session_actions_popover(&app.input()));

    app.active_session_id = Some("active".to_string());
    keep_element(session_actions_popover(&app.input()));
}

#[test]
fn session_actions_popover_contains_compact_and_existing_actions() {
    let mut app = test_app();
    app.active_session_id = Some("active".to_string());

    let popover = session_actions_popover(&app.input());
    let children = popover.as_widget().children();

    assert_eq!(children.len(), 4);
}

#[test]
fn header_icon_button_builds_tooltip_button() {
    keep_element(header_icon_button(Icon::Gear, "设置", Message::None));
}

#[test]
fn chat_header_view_builds_default_session_variants() {
    let app = test_app();
    keep_element(chat_header_view(&app.input()));

    let active = active_header_app();
    keep_element(chat_header_view(&active.input()));

    let mut running = active_header_app();
    let mut running_runtime = SessionRuntimeState::default();
    running_runtime.request_lifecycle.is_requesting = true;
    running_runtime.request_lifecycle.queue.push(queue_item("queued"));
    running.session_runtime_states.insert("active".to_string(), running_runtime);
    keep_element(chat_header_view(&running.input()));

    let mut queued = active_header_app();
    let mut queued_runtime = SessionRuntimeState::default();
    queued_runtime.request_lifecycle.queue.push(queue_item("queued"));
    queued.session_runtime_states.insert("active".to_string(), queued_runtime);
    keep_element(chat_header_view(&queued.input()));

    let mut success = active_header_app();
    let mut success_runtime = SessionRuntimeState::default();
    success_runtime.has_unseen_success = true;
    success.session_runtime_states.insert("active".to_string(), success_runtime);
    success.show_session_actions_popover = true;
    keep_element(chat_header_view(&success.input()));
}

#[test]
fn chat_header_places_mode_control_before_title() {
    let source = include_str!("../header.rs");
    assert!(source.contains("row![status_icon, mode_control, title_element]"));

    let mut app = active_header_app();
    keep_element(chat_header_view(&app.input()));
    app.chat_panel_mode = true;
    keep_element(chat_header_view(&app.input()));

    for theme in [Theme::Light, Theme::Dark] {
        let active = chat_mode_button_style(&theme, button::Status::Active, true);
        let inactive = chat_mode_button_style(&theme, button::Status::Active, false);
        assert_ne!(background_color(active.background), background_color(inactive.background));
    }
}

#[test]
fn chat_header_view_keeps_container_shape_across_fullscreen_modes() {
    let mut app = active_header_app();
    let normal = chat_header_view(&app.input());
    assert!(!normal.as_widget().children().is_empty());
    drop(normal);

    app.fullscreen = true;
    app.show_session_actions_popover = true;
    let fullscreen = chat_header_view(&app.input());
    assert!(!fullscreen.as_widget().children().is_empty());
}

#[test]
fn return_to_parent_session_pressed_is_none_without_parent() {
    let app = active_header_app();
    assert!(return_to_parent_session_pressed(&app.input()).is_none());

    let mut app = test_app();
    app.active_session_id = Some("orphan".to_string());
    assert!(return_to_parent_session_pressed(&app.input()).is_none());
}

#[test]
fn return_to_parent_session_pressed_emits_open_parent_when_parent_id_present() {
    let mut app = test_app();
    let parent = session("parent-session", "/workspace/main");
    let mut child = session("child", "/workspace/main");
    child.parent_id = Some(parent.id.clone());
    app.project_path = Some("/workspace/main".to_string());
    app.active_session_id = Some(parent.id.clone());
    app.sessions = vec![parent, child];

    app.active_session_id = Some("child".to_string());
    assert_eq!(app.active_session_id.as_deref(), Some("child"));

    match return_to_parent_session_pressed(&app.input()) {
        Some(HeaderAction::OpenParent(path, id)) => {
            assert_eq!(path, "/workspace/main");
            assert_eq!(id, "parent-session");
        }
        other => panic!("expected OpenProjectSessionPressed, got {other:?}"),
    }
}

#[test]
fn return_to_parent_session_pressed_uses_project_sessions_cache_when_needed() {
    let mut app = test_app();
    app.project_path = Some("/workspace/main".to_string());
    app.active_session_id = Some("child".to_string());
    let mut child = session("child", "/workspace/child");
    child.parent_id = Some("parent-session".to_string());
    app.project_sessions.insert("/workspace/main".to_string(), vec![child]);

    match return_to_parent_session_pressed(&app.input()) {
        Some(HeaderAction::OpenParent(path, id)) => {
            assert_eq!(path, "/workspace/main");
            assert_eq!(id, "parent-session");
        }
        other => panic!("expected OpenProjectSessionPressed, got {other:?}"),
    }
}

#[test]
fn return_to_parent_session_pressed_ignores_blank_parent_id() {
    let mut app = active_header_app();
    let mut child = session("child", "/workspace/main");
    child.parent_id = Some("   ".to_string());
    app.active_session_id = Some("child".to_string());
    app.sessions.push(child);

    assert!(return_to_parent_session_pressed(&app.input()).is_none());
}

#[test]
fn chat_header_view_builds_with_return_to_parent_control() {
    let mut app = active_header_app();
    let mut child = session("child", "/workspace/main");
    child.parent_id = Some("parent-session".to_string());
    app.active_session_id = Some("child".to_string());
    app.sessions.push(child);

    keep_element(chat_header_view(&app.input()));
}

#[test]
fn header_session_menu_exposes_compact_current_session_action() {
    let app = active_header_app();

    assert!(matches!(
        session_compact_message(&app.input()),
        Some(HeaderAction::Compact {
            session_id,
            project_path,
        }) if session_id == "active" && project_path == "/workspace/main"
    ));
}
