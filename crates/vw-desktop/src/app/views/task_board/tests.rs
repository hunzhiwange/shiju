use crate::app::App;
use crate::app::state::TaskBoardViewMode;
use crate::app::task::Task;
use iced::{Background, Color, Theme};

fn root_child_count(app: &App) -> usize {
    super::view(app).as_widget().children().len()
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("tests"));
}

#[test]
fn view_builds_default_kanban_surface() {
    let (app, _task) = App::new();

    assert_eq!(app.task_board_view_mode, TaskBoardViewMode::Kanban);
    assert!(!app.task_board_worktree_pixel_office);
    assert_eq!(root_child_count(&app), 2);
}

#[test]
fn view_builds_staff_surface() {
    let (mut app, _task) = App::new();
    app.task_board_view_mode = TaskBoardViewMode::Staff;

    assert_eq!(root_child_count(&app), 2);
}

#[test]
fn view_builds_pixel_office_scroll_surface() {
    let (mut app, _task) = App::new();
    app.task_board_worktree_pixel_office = true;

    assert_eq!(app.task_board_view_mode, TaskBoardViewMode::Kanban);
    assert_eq!(root_child_count(&app), 2);
}

#[test]
fn view_builds_create_panel_instead_of_log_detail() {
    let (mut app, _task) = App::new();
    app.task_board_create_modal_open = true;
    app.task_board_viewing_logs = Some(Task::new(1));

    assert_eq!(root_child_count(&app), 2);
}

#[test]
fn view_builds_log_detail_with_dismiss_layer() {
    let (mut app, _task) = App::new();
    app.task_board_viewing_logs = Some(Task::new(1));

    assert_eq!(root_child_count(&app), 3);
}

#[test]
fn view_builds_settings_modal_overlay() {
    let (mut app, _task) = App::new();
    app.task_board_settings_modal_open = true;

    assert_eq!(root_child_count(&app), 2);
}

#[test]
fn task_board_container_style_uses_light_background() {
    let style = super::task_board_container_style(&Theme::Light);

    assert_eq!(style.background, Some(Background::Color(Color::from_rgba8(246, 248, 252, 0.98))));
}

#[test]
fn task_board_container_style_uses_dark_background() {
    let theme = Theme::Dark;
    let style = super::task_board_container_style(&theme);

    assert_eq!(
        style.background,
        Some(Background::Color(theme.extended_palette().background.base.color.scale_alpha(0.94)))
    );
}
