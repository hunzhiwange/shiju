//! 验证项目列表和 Worktree 选项的界面状态。
//!
//! 项目夹具主动清空最近记录与编辑状态，用于隔离缺失路径和确认框边界。

use super::*;
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

#[test]
fn worktree_row_builds_fill_width_help_label_and_control_layout() {
    let app = test_app();
    let element = current_project_worktree(&app, I18n::new(app.desktop_locale));

    assert_eq!(element.as_widget().size(), Size::new(Length::Fill, Length::Shrink));

    let container_children = element.as_widget().children();
    assert_eq!(container_children.len(), 2);
    assert_eq!(container_children[0].children.len(), 2);
    assert!(container_children[1].children.is_empty());
}
