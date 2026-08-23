//! 验证搜索面板的查询结果、导航消息、空态与主题样式。
//! 无匹配、选中索引和长结果列表用于覆盖滚动与焦点反馈。

use super::search_panel::{
    matching_sessions, overlay, result_button_style, search_input_style, view,
};
use crate::app::App;
use iced::widget::{button, text_input};
use iced::{Background, Length, Theme};

fn session(id: &str, title: &str, directory: &str) -> vw_shared::session::info::Info {
    vw_shared::session::info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project-1".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "0.0.0".to_string(),
        time: vw_shared::session::info::TimeInfo {
            created: 0,
            updated: 0,
            compacting: None,
            archived: None,
        },
        permission: None,
        revert: None,
    }
}

fn search_view_content_len(app: &App) -> usize {
    let search_view = view(app);
    search_view.as_widget().children().first().map_or(0, |content| content.children.len())
}

fn session_label_line_count(app: &App) -> usize {
    let search_view = view(app);
    let root = search_view.as_widget().children();
    let content = &root.first().expect("search view should contain results").children;
    let session_button = content.get(1).expect("session heading should precede its button");
    session_button.children.first().map_or(0, |label| label.children.len())
}

fn bg(style: &iced::widget::button::Style) -> iced::Color {
    match style.background {
        Some(Background::Color(color)) => color,
        _ => panic!("button style should use a color background"),
    }
}

#[test]
fn result_button_style_changes_background_by_status() {
    let theme = Theme::Light;

    let active = result_button_style(&theme, button::Status::Active);
    let hovered = result_button_style(&theme, button::Status::Hovered);
    let pressed = result_button_style(&theme, button::Status::Pressed);

    assert_ne!(bg(&active), bg(&hovered));
    assert_ne!(bg(&hovered), bg(&pressed));
    assert_eq!(active.border.width, 0.0);
    assert_eq!(active.border.radius.top_left, 6.0);
}

#[test]
fn result_button_style_uses_base_surface_for_fallback_statuses() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let active = result_button_style(&theme, button::Status::Active);
        let disabled = result_button_style(&theme, button::Status::Disabled);

        assert_eq!(bg(&active), palette.background.base.color);
        assert_eq!(bg(&disabled), palette.background.base.color);
        assert_eq!(active.text_color, palette.background.base.text);
        assert_eq!(disabled.text_color, palette.background.base.text);
    }
}

#[test]
fn search_input_style_tracks_focus_and_hover() {
    let theme = Theme::Light;

    let active = search_input_style(&theme, text_input::Status::Active);
    let hovered = search_input_style(&theme, text_input::Status::Hovered);
    let focused = search_input_style(&theme, text_input::Status::Focused { is_hovered: false });
    let focused_hovered =
        search_input_style(&theme, text_input::Status::Focused { is_hovered: true });

    assert_ne!(active.border.color, hovered.border.color);
    assert_ne!(hovered.border.color, focused.border.color);
    assert_eq!(focused.border.color, focused_hovered.border.color);
    assert_eq!(active.border.width, 1.0);
    assert_eq!(active.border.radius.top_left, 8.0);
}

#[test]
fn matching_sessions_uses_title_preview_and_directory() {
    let mut app = App::new().0;
    app.sessions = vec![
        session("session-1", "Alpha Plan", "/workspace/alpha"),
        session("session-2", "Beta", "/workspace/beta"),
    ];
    app.session_previews = std::collections::HashMap::from([(
        "session-2".to_string(),
        "Discuss release checklist".to_string(),
    )]);

    let title_hits = matching_sessions(&app, "alpha");
    let preview_hits = matching_sessions(&app, "release");
    let directory_hits = matching_sessions(&app, "beta");

    assert_eq!(title_hits[0].id, "session-1");
    assert_eq!(preview_hits[0].id, "session-2");
    assert_eq!(directory_hits[0].id, "session-2");
}

#[test]
fn matching_sessions_rejects_blank_queries() {
    let mut app = App::new().0;
    app.sessions = vec![session("session-1", "Alpha", "/workspace/alpha")];

    assert!(matching_sessions(&app, "").is_empty());
    assert!(matching_sessions(&app, " \t\n ").is_empty());
}

#[test]
fn matching_sessions_matches_id_case_insensitively_and_preserves_fields() {
    let mut app = App::new().0;
    app.sessions = vec![session("Session-ALPHA", "Planning", "/workspace/demo")];
    app.session_previews.insert("Session-ALPHA".to_string(), "Release notes".to_string());

    let matches = matching_sessions(&app, "  session-alpha  ");

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].id, "Session-ALPHA");
    assert_eq!(matches[0].title, "Planning");
    assert_eq!(matches[0].preview.as_deref(), Some("Release notes"));
    assert_eq!(matches[0].directory, "/workspace/demo");
}

#[test]
fn matching_sessions_limits_results_to_first_eight() {
    let mut app = App::new().0;
    app.sessions = (0..10)
        .map(|index| session(&format!("session-{index}"), "Needle", "/workspace/demo"))
        .collect();

    let matches = matching_sessions(&app, "needle");
    let ids = matches.iter().map(|session| session.id.clone()).collect::<Vec<_>>();

    assert_eq!(ids, (0..8).map(|index| format!("session-{index}")).collect::<Vec<_>>());
}

#[test]
fn matching_sessions_preserves_session_order_across_different_matching_fields() {
    let mut app = App::new().0;
    app.sessions = vec![
        session("needle-id", "First", "/workspace/one"),
        session("session-2", "Needle title", "/workspace/two"),
        session("session-3", "Third", "/workspace/needle-directory"),
    ];
    app.session_previews.insert("session-2".to_string(), "unrelated".to_string());

    let ids =
        matching_sessions(&app, "needle").into_iter().map(|session| session.id).collect::<Vec<_>>();

    assert_eq!(ids, ["needle-id", "session-2", "session-3"].map(str::to_string));
}

#[test]
fn view_blank_query_returns_full_size_empty_surface() {
    let mut app = App::new().0;
    app.search_text = " \t ".to_string();

    let search_view = view(&app);

    assert_eq!(search_view.as_widget().size().width, Length::Fill);
    assert_eq!(search_view.as_widget().size().height, Length::Fill);
    assert!(search_view.as_widget().children().is_empty());
}

#[test]
fn view_groups_file_project_and_session_matches() {
    let mut app = App::new().0;
    app.search_text = "needle".to_string();
    app.project_path = Some("/repo".to_string());
    app.set_file_index("/repo", vec!["src/needle.rs".to_string()]);
    app.recent_projects = vec!["/workspace/needle-project".to_string()];
    app.sessions = vec![session("session-1", "Needle session", "/workspace/demo")];

    assert_eq!(search_view_content_len(&app), 6);
}

#[test]
fn view_non_blank_query_without_matches_renders_empty_results() {
    let mut app = App::new().0;
    app.search_text = "missing".to_string();

    assert_eq!(search_view_content_len(&app), 0);
}

#[test]
fn view_limits_project_rows_to_eight_case_insensitively() {
    let mut app = App::new().0;
    app.search_text = "  NEEDLE  ".to_string();
    app.recent_projects =
        (0..10).map(|index| format!("/workspace/needle-project-{index}")).collect();

    assert_eq!(search_view_content_len(&app), 9);
}

#[test]
fn view_session_labels_render_only_non_blank_metadata() {
    let mut app = App::new().0;
    app.search_text = "needle".to_string();
    app.sessions = vec![session("session-1", "Needle session", " \t")];
    app.session_previews.insert("session-1".to_string(), "  ".to_string());
    assert_eq!(session_label_line_count(&app), 1);

    app.sessions[0].directory = "/workspace/demo".to_string();
    app.session_previews.insert("session-1".to_string(), "Release preview".to_string());
    assert_eq!(session_label_line_count(&app), 3);
}

#[test]
fn overlay_contains_scrim_and_modal_layers() {
    let app = App::new().0;

    let search_overlay = overlay(&app);

    assert_eq!(search_overlay.as_widget().children().len(), 2);
}
