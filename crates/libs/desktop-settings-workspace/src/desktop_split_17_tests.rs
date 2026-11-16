//! Project list shape uses only the domain view and explicit presentation callbacks.
use crate::system_settings_projects::{self as domain, Event, ViewState};
use desktop_i18n::I18n;
use iced::{Element, Theme, advanced::widget::Tree, widget::text};
use std::collections::HashMap;

#[derive(Default)]
struct Projects {
    project_path: Option<String>,
    project_worktree_enabled: HashMap<String, bool>,
    recent_projects: Vec<String>,
    recent_projects_edits: Vec<String>,
    recent_project_delete_confirm_idx: Option<usize>,
}
fn test_app() -> Projects {
    Projects::default()
}
fn tree_node_count(tree: &Tree) -> usize {
    1 + tree.children.iter().map(tree_node_count).sum::<usize>()
}
fn avatar<'a>(_: &str, _: &str, _: Option<&str>, _: Option<&str>, size: f32) -> Element<'a, Event> {
    iced::widget::Space::new().width(size).height(size).into()
}
fn help<'a>(
    _: I18n,
    _: &Theme,
    label: &'a str,
    _: &'a str,
    width: f32,
    control: Element<'a, Event>,
) -> iced::widget::Container<'a, Event> {
    desktop_ui_controls::system_settings_common::panels::settings_flexible_row_with_label(
        text(label),
        width,
        control,
    )
}
fn view_shape(app: &Projects) -> (usize, usize) {
    let theme = Theme::Light;
    let state = ViewState {
        project_path: &app.project_path,
        project_worktree_enabled: &app.project_worktree_enabled,
        project_tools_menu_path: &None,
        recent_projects: &app.recent_projects,
        recent_projects_edits: &app.recent_projects_edits,
        recent_project_delete_confirm_idx: app.recent_project_delete_confirm_idx,
        recent_project_query: "",
        desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
        app_theme: &theme,
    };
    let element = domain::view(&state, |event| event, avatar, help, &|_| (None, None));
    let children = element.as_widget().children();
    (children.len(), children.iter().map(tree_node_count).sum())
}

#[test]
fn view_without_project_builds_static_sections_and_open_project_action() {
    let app = test_app();

    assert_eq!(view_shape(&app).0, 4);
}

#[test]
fn view_with_project_builds_worktree_toggle_for_default_and_enabled_states() {
    let mut app = test_app();
    let project_path = "/tmp/vibe-window-project".to_string();
    app.project_path = Some(project_path.clone());

    assert_eq!(view_shape(&app).0, 4);

    app.project_worktree_enabled.insert(project_path, true);

    assert_eq!(view_shape(&app).0, 4);
}

#[test]
fn project_paths_drive_the_list_even_without_custom_names() {
    let mut app = test_app();
    let empty = view_shape(&app);
    app.recent_projects_edits = vec!["Orphan name".into()];
    assert_eq!(view_shape(&app), empty);

    app.recent_projects = vec!["/tmp/one".into(), "/tmp/two".into()];
    assert!(view_shape(&app).1 > empty.1);
}

#[test]
fn delete_confirmation_only_expands_matching_recent_project() {
    let mut ordinary = test_app();
    ordinary.recent_projects = vec!["/tmp/vibe-window-one".to_string()];
    ordinary.recent_projects_edits = vec!["One".to_string()];
    let ordinary_shape = view_shape(&ordinary);

    let mut matching = test_app();
    matching.recent_projects = ordinary.recent_projects.clone();
    matching.recent_projects_edits = ordinary.recent_projects_edits.clone();
    matching.recent_project_delete_confirm_idx = Some(0);
    let matching_shape = view_shape(&matching);

    let mut out_of_range = test_app();
    out_of_range.recent_projects = ordinary.recent_projects;
    out_of_range.recent_projects_edits = ordinary.recent_projects_edits;
    out_of_range.recent_project_delete_confirm_idx = Some(1);
    let out_of_range_shape = view_shape(&out_of_range);

    assert_eq!(matching_shape.0, ordinary_shape.0);
    assert!(matching_shape.1 > ordinary_shape.1);
    assert_eq!(out_of_range_shape, ordinary_shape);
}

#[test]
fn delete_confirmation_expands_only_the_selected_card_in_multiple_projects() {
    let mut ordinary = test_app();
    ordinary.recent_projects = vec!["/tmp/one".to_string(), "/tmp/two".to_string()];
    ordinary.recent_projects_edits = vec!["One".to_string(), "Two".to_string()];
    let ordinary_shape = view_shape(&ordinary);

    let mut first = test_app();
    first.recent_projects = ordinary.recent_projects.clone();
    first.recent_projects_edits = ordinary.recent_projects_edits.clone();
    first.recent_project_delete_confirm_idx = Some(0);

    let mut second = ordinary;
    second.recent_project_delete_confirm_idx = Some(1);

    assert_eq!(view_shape(&first), view_shape(&second));
    assert!(view_shape(&second).1 > ordinary_shape.1);
}

#[test]
fn current_project_allows_delete_confirmation() {
    let mut ordinary = test_app();
    ordinary.recent_projects = vec!["/tmp/current".to_string()];
    ordinary.recent_projects_edits = vec!["Current".to_string()];
    ordinary.project_path = Some("/tmp/current".to_string());
    let ordinary_shape = view_shape(&ordinary);

    let mut confirming = test_app();
    confirming.recent_projects = vec!["/tmp/current".to_string()];
    confirming.recent_projects_edits = vec!["Current".to_string()];
    confirming.project_path = Some("/tmp/current".to_string());
    confirming.recent_project_delete_confirm_idx = Some(0);

    assert!(ordinary_shape.1 > view_shape(&test_app()).1);
    assert!(view_shape(&confirming).1 > ordinary_shape.1);
}
