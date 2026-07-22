use super::*;
use crate::app::state::{AcpHistoryReplayMode, ChatSendBehavior, QueueItem, SessionRuntimeState};
use iced::widget::button;
use vw_shared::session::info::{Info, TimeInfo};

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
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        acp_force_new_session: false,
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 3,
        full_access_enabled: true,
        send_behavior: ChatSendBehavior::Queue,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled: false,
    }
}

#[test]
fn task_475_test_module_is_wired() {
    let path = std::path::Path::new(file!());

    assert!(path.ends_with("header_tests.rs"));
}

#[test]
fn active_session_directory_returns_none_without_matching_session() {
    let mut app = test_app();

    assert_eq!(active_session_directory(&app), None);

    app.active_session_id = Some("missing".to_string());
    app.sessions.push(session("other", "/workspace/other"));

    assert_eq!(active_session_directory(&app), None);
}

#[test]
fn active_session_directory_returns_active_session_directory() {
    let mut app = test_app();
    app.active_session_id = Some("active".to_string());
    app.sessions.push(session("inactive", "/workspace/inactive"));
    app.sessions.push(session("active", "/workspace/active"));

    assert_eq!(active_session_directory(&app).as_deref(), Some("/workspace/active"));
}

#[test]
fn workspace_badge_state_labels_primary_named_and_fallback_workspaces() {
    let mut app = test_app();
    app.project_path = Some("/workspace/main".to_string());
    app.active_session_id = Some("primary".to_string());
    app.sessions.push(session("primary", "/workspace/main"));

    assert_eq!(session_workspace_badge_state(&app), Some(("主工作区".to_string(), true)));

    app.active_session_id = Some("named".to_string());
    app.sessions.push(session("named", "/workspace/worktrees/feature-a"));
    assert_eq!(session_workspace_badge_state(&app), Some(("feature-a".to_string(), false)));

    app.active_session_id = Some("root".to_string());
    app.sessions.push(session("root", "/"));
    assert_eq!(session_workspace_badge_state(&app), Some(("工作区".to_string(), false)));
}

#[test]
fn workspace_badge_element_builds_only_when_active_session_has_directory() {
    let app = test_app();
    assert!(session_workspace_badge(&app).is_none());

    let app = active_header_app();
    let badge = session_workspace_badge(&app).expect("active session should show badge");
    keep_element(badge);
}

#[test]
fn workspace_badge_styles_cover_theme_and_primary_variants() {
    for theme in [Theme::Light, Theme::Dark] {
        for is_primary in [true, false] {
            let style = session_workspace_badge_style(&theme, is_primary);

            assert!(style.background.is_some());
            assert_eq!(style.border.width, 1.0);
            assert_eq!(style.border.radius.top_left, 999.0);
            assert!(style.text_color.is_some());
        }
    }

    let primary = background_color(session_workspace_badge_style(&Theme::Light, true).background);
    let secondary =
        background_color(session_workspace_badge_style(&Theme::Light, false).background);

    assert_ne!(primary, secondary);
}

#[test]
fn session_status_defaults_when_no_active_session() {
    let app = test_app();
    let (is_running, queued, has_unseen_success, breathe, color) = get_session_status(&app);

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

    let (is_running, queued, has_unseen_success, breathe, color) = get_session_status(&app);

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
    runtime.is_requesting = true;
    runtime.has_unseen_success = true;
    runtime.queue.push(queue_item("first"));
    runtime.queue.push(queue_item("second"));
    app.session_runtime_states.insert("active".to_string(), runtime);

    let (is_running, queued, has_unseen_success, breathe, color) = get_session_status(&app);

    assert!(is_running);
    assert_eq!(queued, 2);
    assert!(has_unseen_success);
    assert!((0.0..=1.0).contains(&breathe));
    assert_ne!(color, Color::TRANSPARENT);
}

#[test]
fn status_badge_styles_use_expected_surfaces() {
    let running = running_status_badge_style(Color::from_rgb8(239, 68, 68));
    let queued = queued_status_badge_style();
    let text_style = colored_text_style(Color::from_rgb8(46, 184, 114));

    assert!(running.background.is_some());
    assert!(queued.background.is_some());
    assert_eq!(running.border.radius.top_left, 999.0);
    assert_eq!(queued.text_color, Some(Color::from_rgb8(245, 158, 11)));
    assert_eq!(text_style.color, Some(Color::from_rgb8(46, 184, 114)));
}

#[test]
fn header_surface_style_is_transparent() {
    let style = header_surface_style(&Theme::Dark);

    assert!(style.background.is_none());
    assert_eq!(style.border.width, 0.0);
    assert_eq!(style.border.color, Color::TRANSPARENT);
}

#[test]
fn session_actions_button_style_covers_all_status_and_theme_paths() {
    for theme in [Theme::Light, Theme::Dark] {
        for status in [
            button::Status::Active,
            button::Status::Hovered,
            button::Status::Pressed,
            button::Status::Disabled,
        ] {
            let style = session_actions_button_style(&theme, status);

            assert!(style.background.is_some());
            assert_eq!(style.border.width, 1.0);
            assert_eq!(style.border.radius.top_left, 8.0);
            assert_eq!(style.text_color, theme.palette().text.scale_alpha(0.92));
        }
    }

    let active = background_color(
        session_actions_button_style(&Theme::Light, button::Status::Active).background,
    );
    let hovered = background_color(
        session_actions_button_style(&Theme::Light, button::Status::Hovered).background,
    );
    let pressed_dark = background_color(
        session_actions_button_style(&Theme::Dark, button::Status::Pressed).background,
    );

    assert_eq!(active, Color::TRANSPARENT);
    assert_ne!(hovered, Color::TRANSPARENT);
    assert_ne!(pressed_dark, Color::TRANSPARENT);
}

#[test]
fn session_actions_popover_style_covers_light_and_dark_surfaces() {
    let light = session_actions_popover_style(&Theme::Light);
    let dark = session_actions_popover_style(&Theme::Dark);

    assert!(light.background.is_some());
    assert!(dark.background.is_some());
    assert_eq!(light.border.radius.top_left, 12.0);
    assert_eq!(dark.border.radius.top_left, 12.0);
    assert_ne!(background_color(light.background), background_color(dark.background));
    assert!(dark.shadow.blur_radius > 0.0);
}

#[test]
fn session_actions_elements_build_for_empty_and_active_session() {
    let mut app = test_app();

    keep_element(session_actions_button("重命名", Message::None));
    keep_element(session_actions_popover(&app));

    app.active_session_id = Some("active".to_string());
    keep_element(session_actions_popover(&app));
}

#[test]
fn session_actions_popover_contains_three_action_buttons() {
    let mut app = test_app();
    app.active_session_id = Some("active".to_string());

    let popover = session_actions_popover(&app);
    let children = popover.as_widget().children();

    assert_eq!(children.len(), 3);
}

#[test]
fn header_icon_button_builds_tooltip_button() {
    keep_element(header_icon_button(Icon::Gear, "设置", Message::None));
}

#[test]
fn chat_header_view_builds_default_session_variants() {
    let app = test_app();
    keep_element(chat_header_view(&app));

    let active = active_header_app();
    keep_element(chat_header_view(&active));

    let mut running = active_header_app();
    let mut running_runtime = SessionRuntimeState::default();
    running_runtime.is_requesting = true;
    running_runtime.queue.push(queue_item("queued"));
    running.session_runtime_states.insert("active".to_string(), running_runtime);
    keep_element(chat_header_view(&running));

    let mut queued = active_header_app();
    let mut queued_runtime = SessionRuntimeState::default();
    queued_runtime.queue.push(queue_item("queued"));
    queued.session_runtime_states.insert("active".to_string(), queued_runtime);
    keep_element(chat_header_view(&queued));

    let mut success = active_header_app();
    let mut success_runtime = SessionRuntimeState::default();
    success_runtime.has_unseen_success = true;
    success.session_runtime_states.insert("active".to_string(), success_runtime);
    success.show_session_actions_popover = true;
    keep_element(chat_header_view(&success));
}

#[test]
fn chat_header_view_keeps_container_shape_across_fullscreen_modes() {
    let mut app = active_header_app();
    let normal = chat_header_view(&app);
    assert!(!normal.as_widget().children().is_empty());
    drop(normal);

    app.chat_panel_fullscreen = true;
    app.show_session_actions_popover = true;
    let fullscreen = chat_header_view(&app);
    assert!(!fullscreen.as_widget().children().is_empty());
}
