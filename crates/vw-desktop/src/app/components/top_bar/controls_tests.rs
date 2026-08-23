//! 验证顶部栏窗口控件在不同平台与窗口状态下的行为。
//!
//! 平台与窗口状态分开构造，确保最小化、最大化和关闭动作保持稳定映射。

use super::controls;
use crate::app::state::WorkspaceMode;
use crate::app::{App, Message, Screen};
use iced::{Element, Length, Size};

#[cfg(not(target_arch = "wasm32"))]
const PROJECT_CONTROL_COUNT: usize = 6;
#[cfg(target_arch = "wasm32")]
const PROJECT_CONTROL_COUNT: usize = 5;
#[cfg(not(target_arch = "wasm32"))]
const WORK_PROJECT_CONTROL_COUNT: usize = 4;
#[cfg(target_arch = "wasm32")]
const WORK_PROJECT_CONTROL_COUNT: usize = 3;

fn test_app() -> App {
    let (app, _task) = App::new();
    app
}

fn assert_project_toolbar_shape(element: &Element<'_, Message>) {
    assert_eq!(element.as_widget().size(), Size::new(Length::Shrink, Length::Shrink));

    let controls = element.as_widget().children();
    assert_eq!(controls.len(), PROJECT_CONTROL_COUNT);
    assert!(controls.iter().all(|control| control.children.len() == 2));
}

fn assert_work_project_toolbar_shape(element: &Element<'_, Message>) {
    assert_eq!(element.as_widget().size(), Size::new(Length::Shrink, Length::Shrink));

    let controls = element.as_widget().children();
    assert_eq!(controls.len(), WORK_PROJECT_CONTROL_COUNT);
    assert!(controls.iter().all(|control| control.children.len() == 2));
}

#[test]
fn settings_button_builds_compact_tooltip_control() {
    let element = controls::settings_button();

    assert_eq!(element.as_widget().size(), Size::new(Length::Shrink, Length::Fixed(24.0)));
    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn project_view_tools_returns_empty_space_for_every_non_project_screen() {
    let mut app = test_app();
    let non_project_screens = [
        Screen::Home,
        Screen::Preview,
        Screen::Apps,
        Screen::Usage,
        Screen::Knowledge,
        Screen::WorkflowTool,
        Screen::WorkflowManager,
        Screen::TaskBoard,
    ];

    for screen in non_project_screens {
        app.screen = screen;
        let element = controls::project_view_tools(&app);

        assert_eq!(element.as_widget().size(), Size::new(Length::Shrink, Length::Shrink));
        assert!(element.as_widget().children().is_empty());
    }
}

#[test]
fn project_view_tools_builds_project_controls_when_panels_are_hidden() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.show_settings = false;
    app.terminal.is_visible = false;
    app.show_diff = false;
    app.show_file_manager = false;

    let element = controls::project_view_tools(&app);

    assert_project_toolbar_shape(&element);
}

#[test]
fn project_view_tools_builds_project_controls_when_panels_are_visible() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.show_settings = true;
    app.terminal.is_visible = true;
    app.show_diff = true;
    app.show_file_manager = true;
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.task_pet_window_id = Some(iced::window::Id::unique());
    }

    let element = controls::project_view_tools(&app);

    assert_project_toolbar_shape(&element);
}

#[test]
fn work_project_tools_hide_code_only_terminal_and_review_controls() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.workspace_mode = WorkspaceMode::Work;
    app.terminal.is_visible = true;
    app.show_diff = true;

    let element = controls::project_view_tools(&app);

    assert_work_project_toolbar_shape(&element);
}
