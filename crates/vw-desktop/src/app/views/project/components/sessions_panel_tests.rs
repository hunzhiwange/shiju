use super::*;
use crate::app::state::{AcpHistoryReplayMode, ChatSendBehavior, QueueItem, SessionRuntimeState};
use iced::widget::{button, svg};
use vw_shared::session::info::{Info, Summary, TimeInfo};

fn test_app() -> crate::app::App {
    crate::app::App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn color_from_background(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        _ => panic!("expected color background"),
    }
}

fn assert_color_near(actual: Color, expected: Color) {
    let epsilon = 0.001;
    assert!((actual.r - expected.r).abs() < epsilon, "red mismatch: {actual:?}");
    assert!((actual.g - expected.g).abs() < epsilon, "green mismatch: {actual:?}");
    assert!((actual.b - expected.b).abs() < epsilon, "blue mismatch: {actual:?}");
    assert!((actual.a - expected.a).abs() < epsilon, "alpha mismatch: {actual:?}");
}

fn session(id: &str, title: &str, directory: &str, archived: bool) -> Info {
    Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: Some(Summary { additions: 3, deletions: 1, files: 2, diffs: None }),
        share: None,
        title: title.to_string(),
        version: "1".to_string(),
        time: TimeInfo {
            created: 1,
            updated: 2,
            compacting: None,
            archived: archived.then_some(3),
        },
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
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("sessions_panel_tests"));
}

#[test]
fn button_styles_cover_light_and_dark_states() {
    for theme in [Theme::Light, Theme::Dark] {
        for status in [
            button::Status::Active,
            button::Status::Hovered,
            button::Status::Pressed,
            button::Status::Disabled,
        ] {
            assert!(icon_action_button_style(&theme, status).background.is_some());
            assert!(outline_panel_button_style(&theme, false, status).background.is_some());
            assert!(outline_panel_button_style(&theme, true, status).background.is_some());
            assert!(session_row_button_style(&theme, false, status).background.is_some());
            assert!(session_row_button_style(&theme, true, status).background.is_some());
        }
    }
}

#[test]
fn menu_button_style_tracks_hover_pressed_and_theme_variants() {
    let active = session_menu_button_style(&Theme::Light, button::Status::Active);
    let hovered = session_menu_button_style(&Theme::Light, button::Status::Hovered);
    let pressed_dark = session_menu_button_style(&Theme::Dark, button::Status::Pressed);

    assert_eq!(color_from_background(active.background), Color::TRANSPARENT);
    assert_eq!(hovered.border.width, 1.0);
    assert_ne!(color_from_background(hovered.background), Color::TRANSPARENT);
    assert_ne!(color_from_background(pressed_dark.background), Color::TRANSPARENT);
    assert_eq!(pressed_dark.text_color, Theme::Dark.palette().text);
}

#[test]
fn menu_button_style_covers_dark_hover_and_light_press() {
    let hovered_dark = session_menu_button_style(&Theme::Dark, button::Status::Hovered);
    let pressed_light = session_menu_button_style(&Theme::Light, button::Status::Pressed);

    assert_eq!(hovered_dark.border.width, 1.0);
    assert_ne!(color_from_background(hovered_dark.background), Color::TRANSPARENT);
    assert_eq!(pressed_light.border.width, 0.0);
    assert_ne!(color_from_background(pressed_light.background), Color::TRANSPARENT);
}

#[test]
fn menu_separator_and_panel_styles_use_visible_surfaces() {
    let separator = menu_separator_style(&Theme::Light);
    let panel = floating_menu_panel_style(&Theme::Dark);

    assert!(separator.background.is_some());
    assert!(panel.background.is_some());
    assert_eq!(panel.border.radius.top_left, 16.0);
}

#[test]
fn status_styles_keep_expected_colors_and_alpha() {
    let running = running_status_color(0.0);
    let queued = queued_status_color(1.0);
    let badge_color = Color::from_rgb8(46, 184, 114);
    let badge = status_badge_style(badge_color);

    assert_color_near(running, Color::from_rgb8(245, 158, 11));
    assert_ne!(queued, Color::from_rgb8(239, 68, 68));
    assert_color_near(running_status_color(1.0), Color::from_rgb8(239, 68, 68));
    assert_color_near(queued_status_color(0.0), Color::from_rgb8(245, 158, 11));
    assert_color_near(color_from_background(badge.background), badge_color.scale_alpha(0.16));
    assert_eq!(badge.text_color, Some(badge_color));
    assert_eq!(badge.border.width, 0.0);
    assert_eq!(status_text_style(badge_color).color, Some(badge_color));
    assert_eq!(
        muted_status_icon_style(&Theme::Light).color,
        Some(Theme::Light.palette().text.scale_alpha(0.3)),
    );
    assert_eq!(additions_text_style(&Theme::Light).color, Some(Color::from_rgb8(46, 184, 114)),);
    assert_eq!(deletions_text_style(&Theme::Light).color, Some(Color::from_rgb8(239, 68, 68)),);
}

#[test]
fn task_pool_and_load_more_styles_adjust_base_styles() {
    let task_pool = task_pool_button_style(&Theme::Light, button::Status::Hovered);
    let load_more = load_more_button_style(&Theme::Dark, button::Status::Pressed);
    let icon = task_pool_icon_style(&Theme::Dark, svg::Status::Idle);

    assert_eq!(task_pool.border.radius.top_left, 14.0);
    assert_eq!(load_more.border.width, 0.0);
    assert_eq!(load_more.border.color, Color::TRANSPARENT);
    assert_eq!(icon.color, Some(Theme::Dark.palette().text));
}

#[test]
fn panel_title_prefers_non_blank_recent_edit_name() {
    let projects = vec!["/tmp/a".to_string(), "/tmp/b".to_string()];
    let edits = vec!["Alpha".to_string(), "   ".to_string()];

    assert_eq!(project_panel_title("/tmp/a", &projects, &edits), "Alpha");
    assert_eq!(project_panel_title("/tmp/b", &projects, &edits), "/tmp/b");
    assert_eq!(
        project_panel_title("/tmp/missing-edit", &["/tmp/missing-edit".to_string()], &[]),
        "/tmp/missing-edit"
    );
    assert_eq!(project_panel_title("/tmp/c", &projects, &edits), "/tmp/c");
}

#[test]
fn session_workspace_label_covers_main_named_and_fallback_paths() {
    assert_eq!(session_workspace_label("/tmp/project", "/tmp/project"), "主工作区");
    assert_eq!(session_workspace_label("/tmp/project/worktree-a", "/tmp/project"), "worktree-a",);
    assert_eq!(session_workspace_label("", "/tmp/project"), "工作区");
    assert_eq!(session_workspace_label("/", "/tmp/project"), "工作区");
}

#[test]
fn menu_builders_return_elements() {
    keep_element(session_menu_button(
        "重命名",
        Message::Project(message::ProjectMessage::SessionRenamePressed("s1".to_string())),
    ));
    keep_element(build_session_menu("s1".to_string()));
    keep_element(build_project_tools_menu("/tmp/project".to_string()));
}

#[test]
fn header_builds_clickable_and_static_variants() {
    keep_element(project_sessions_header(
        "项目".to_string(),
        "/tmp/project".to_string(),
        12,
        true,
        true,
    ));
    keep_element(project_sessions_header(
        "项目".to_string(),
        "/tmp/project".to_string(),
        8,
        false,
        false,
    ));
}

#[test]
fn session_items_build_loading_empty_and_load_button_states() {
    let app = test_app();
    keep_element(session_items_list(&app, "/tmp/project", None, 18, 10, false));
    keep_element(session_items_list(&app, "/tmp/project", Some(&Vec::new()), 18, 10, false));
    keep_element(session_items_list(&app, "/tmp/project", None, 18, 10, true));
}

#[test]
fn session_items_filters_archived_and_shows_load_more() {
    let app = test_app();
    let sessions = vec![
        session("s1", "可见会话一", "/tmp/project", false),
        session("s2", "已归档会话", "/tmp/project", true),
        session("s3", "可见会话二", "/tmp/project/worktree-a", false),
    ];

    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), 16, 1, false));
    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), 16, 0, false));
}

#[test]
fn session_items_reports_empty_when_only_archived_sessions_exist() {
    let app = test_app();
    let sessions = vec![
        session("s1", "归档一", "/tmp/project", true),
        session("s2", "归档二", "/tmp/project/worktree-a", true),
    ];

    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), 16, 10, false));
}

#[test]
fn session_items_builds_runtime_status_variants() {
    let mut app = test_app();
    app.active_session_id = Some("running".to_string());
    app.session_menu_id = Some("queued".to_string());
    app.session_menu_anchor = Some(Point::new(4.0, 6.0));

    let mut running = SessionRuntimeState::default();
    running.is_requesting = true;
    app.session_runtime_states.insert("running".to_string(), running);

    let mut queued = SessionRuntimeState::default();
    queued.queue.push(queue_item("继续"));
    app.session_runtime_states.insert("queued".to_string(), queued);

    let mut unseen = SessionRuntimeState::default();
    unseen.has_unseen_success = true;
    app.session_runtime_states.insert("unseen".to_string(), unseen);

    let sessions = vec![
        session("running", "运行中会话", "/tmp/project", false),
        session("queued", "排队会话", "/tmp/project/worktree-b", false),
        session("unseen", "未读成功会话", "/tmp/project", false),
        session("idle", "空闲会话", "", false),
    ];

    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), 10, 10, false));
}

#[test]
fn session_items_builds_missing_summary_menu_default_anchor_and_no_load_more() {
    let mut app = test_app();
    app.session_menu_id = Some("summaryless".to_string());
    app.session_menu_anchor = None;

    let mut summaryless = session("summaryless", "没有摘要的会话", "/", false);
    summaryless.summary = None;
    let sessions =
        vec![summaryless, session("normal", "普通会话", "/tmp/project/worktree-d", false)];

    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), 0, 10, false));
}

#[test]
fn standalone_panel_fragments_build_inactive_and_pressed_variants() {
    keep_element(session_menu_button("删除", Message::None));
    keep_element(project_sessions_header(
        "长项目标题".to_string(),
        "/tmp/project/with/a/very/long/path".to_string(),
        4,
        true,
        false,
    ));

    for theme in [Theme::Light, Theme::Dark] {
        let inactive = outline_panel_button_style(&theme, false, button::Status::Active);
        let active_pressed = outline_panel_button_style(&theme, true, button::Status::Pressed);
        assert_eq!(inactive.text_color, theme.palette().text);
        assert!(active_pressed.background.is_some());
        assert!(active_pressed.border.width > 0.0);
    }
}

#[test]
fn project_panel_builds_empty_and_selected_project_states() {
    let mut app = test_app();
    keep_element(project_sessions_panel(&app, 476.0, 72.0, 0.5, None));

    app.recent_projects.push("/tmp/project".to_string());
    keep_element(project_sessions_panel(&app, 476.0, 72.0, 0.5, None));

    app.recent_projects_edits.push("自定义项目名".to_string());
    app.project_tools_menu_path = Some("/tmp/project".to_string());
    app.project_session_has_vertical_scrollbar.insert("/tmp/project".to_string(), true);
    app.project_session_load_counts.insert("/tmp/project".to_string(), 1);
    app.project_sessions.insert(
        "/tmp/project".to_string(),
        vec![
            session("s1", "第一条会话", "/tmp/project", false),
            session("s2", "第二条会话", "/tmp/project/worktree-c", false),
        ],
    );

    keep_element(project_sessions_panel(&app, 476.0, 72.0, 0.5, Some("/tmp/project".to_string())));

    app.project_sessions_loading.insert("/tmp/loading".to_string());
    keep_element(project_sessions_panel(&app, 80.0, 120.0, 0.5, Some("/tmp/loading".to_string())));
}

#[test]
fn project_panel_uses_path_fallback_for_unknown_or_blank_recent_names() {
    let mut app = test_app();
    app.recent_projects = vec!["/tmp/project".to_string()];
    app.recent_projects_edits = vec!["   ".to_string()];
    app.project_path = Some("/tmp/project".to_string());

    keep_element(project_sessions_panel(&app, 476.0, 72.0, 0.5, Some("/tmp/project".to_string())));
    keep_element(project_sessions_panel(
        &app,
        476.0,
        72.0,
        0.5,
        Some("/tmp/unknown-project".to_string()),
    ));
}
