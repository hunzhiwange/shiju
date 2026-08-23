//! 验证项目设置及最近项目编辑、删除确认和工作树选项的界面状态。
//!
//! 项目夹具主动清空最近记录与编辑状态，用于隔离缺失路径和确认框边界。

use super::*;
use iced::advanced::widget::Tree;
use iced::{Length, Size};

fn test_app() -> App {
    let mut app = App::new().0;
    app.project_path = None;
    app.project_worktree_enabled.clear();
    app.recent_projects.clear();
    app.recent_projects_edits.clear();
    app.recent_project_delete_confirm_idx = None;
    app
}

fn tree_node_count(tree: &Tree) -> usize {
    1 + tree.children.iter().map(tree_node_count).sum::<usize>()
}

fn view_shape(app: &App) -> (usize, usize) {
    let element = view(app);
    let children = element.as_widget().children();
    let node_count = children.iter().map(tree_node_count).sum();

    (children.len(), node_count)
}

#[test]
fn field_row_builds_fill_width_label_description_and_control_layout() {
    let element = field_row("标签", "说明", text("控件"));

    assert_eq!(element.as_widget().size(), Size::new(Length::Fill, Length::Shrink));

    let container_children = element.as_widget().children();
    assert_eq!(container_children.len(), 2);
    assert_eq!(container_children[0].children.len(), 2);
    assert!(container_children[1].children.is_empty());
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
fn recent_project_cards_follow_edit_rows_and_tolerate_missing_paths() {
    let mut app = test_app();
    app.recent_projects = vec!["/tmp/vibe-window-one".to_string()];
    app.recent_projects_edits = vec!["One".to_string(), "Missing path".to_string()];

    assert_eq!(view_shape(&app).0, 6);
}

#[test]
fn recent_project_paths_without_edit_rows_do_not_render_cards() {
    let mut app = test_app();
    app.recent_projects =
        vec!["/tmp/vibe-window-one".to_string(), "/tmp/vibe-window-two".to_string()];

    assert_eq!(view_shape(&app).0, 4);
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

    assert_eq!(ordinary_shape.0, 5);
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
