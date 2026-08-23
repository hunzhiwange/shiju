//! 验证项目启动设置的脚本编辑、工作树开关和条件布局。
//!
//! 工作树开关控制条件区块，编辑器动作则要求无损封装到项目消息。

use super::*;
use crate::app::message::project::ProjectMessage;

fn test_app() -> App {
    let (app, _task) = App::new();
    app
}

fn unwrap_project_message(message: Message) -> ProjectMessage {
    let Message::Project(message) = message else {
        panic!("expected a project message");
    };
    message
}

#[test]
fn start_script_editor_action_preserves_editor_action() {
    assert!(matches!(
        unwrap_project_message(start_script_editor_action(
            iced::widget::text_editor::Action::SelectAll,
        )),
        ProjectMessage::ProjectEditStartScriptEditorAction(
            iced::widget::text_editor::Action::SelectAll,
        )
    ));
}

#[test]
fn worktree_toggled_preserves_enabled_value() {
    for enabled in [false, true] {
        assert!(matches!(
            unwrap_project_message(worktree_toggled(enabled)),
            ProjectMessage::ProjectEditWorktreeToggled(value) if value == enabled
        ));
    }
}

#[test]
fn launch_tab_builds_when_worktree_is_disabled() {
    let mut app = test_app();
    app.project_edit_start_script = "bun install\nbun dev".to_string();
    app.project_edit_start_script_editor =
        iced::widget::text_editor::Content::with_text(&app.project_edit_start_script);
    app.project_edit_worktree_enabled = false;

    let element = launch_tab(&app);

    std::hint::black_box(element);
}

#[test]
fn launch_tab_builds_when_worktree_is_enabled() {
    let mut app = test_app();
    app.project_edit_start_script = String::new();
    app.project_edit_start_script_editor = iced::widget::text_editor::Content::new();
    app.project_edit_worktree_enabled = true;

    let element = launch_tab(&app);

    std::hint::black_box(element);
}
