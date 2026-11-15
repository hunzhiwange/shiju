//! 验证桌面数据适配后的搜索分组、空态与模态布局。
//! 无匹配、选中索引和长结果列表用于覆盖滚动与焦点反馈。

use super::search_panel::{overlay, view};
use crate::app::App;
use iced::Length;

fn session(id: &str, title: &str, directory: &str) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project-1".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "0.0.0".to_string(),
        time: session_info::TimeInfo {
            created: 0,
            updated: 0,
            compacting: None,
            archived: None,
        },
        permission: None,
        fork_execution_started: None,
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

#[test]
fn view_blank_query_returns_full_size_empty_surface() {
    let mut app = App::new().0;
    app.search.search_text = " \t ".to_string();

    let search_view = view(&app);

    assert_eq!(search_view.as_widget().size().width, Length::Fill);
    assert_eq!(search_view.as_widget().size().height, Length::Fill);
    assert!(search_view.as_widget().children().is_empty());
}

#[test]
fn view_groups_file_project_and_session_matches() {
    let mut app = App::new().0;
    app.search.search_text = "needle".to_string();
    app.project_path = Some("/repo".to_string());
    app.set_file_index("/repo", vec!["src/needle.rs".to_string()]);
    app.recent_projects = vec!["/workspace/needle-project".to_string()];
    app.sessions = vec![session("session-1", "Needle session", "/workspace/demo")];

    assert_eq!(search_view_content_len(&app), 6);
}

#[test]
fn view_non_blank_query_without_matches_renders_empty_results() {
    let mut app = App::new().0;
    app.search.search_text = "missing".to_string();

    assert_eq!(search_view_content_len(&app), 0);
}

#[test]
fn view_limits_project_rows_to_eight_case_insensitively() {
    let mut app = App::new().0;
    app.search.search_text = "  NEEDLE  ".to_string();
    app.recent_projects =
        (0..10).map(|index| format!("/workspace/needle-project-{index}")).collect();

    assert_eq!(search_view_content_len(&app), 9);
}

#[test]
fn view_session_labels_render_only_non_blank_metadata() {
    let mut app = App::new().0;
    app.search.search_text = "needle".to_string();
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
