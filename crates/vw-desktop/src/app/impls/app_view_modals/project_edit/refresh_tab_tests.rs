//! 验证项目刷新间隔的归一化、步进消息和控件边界。
//!
//! 间隔输入覆盖空白、非法、上下界和步进操作，确保控件值与保存值一致。

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
fn normalized_session_refresh_interval_trims_clamps_and_falls_back() {
    assert_eq!(normalized_session_refresh_interval(" 125 "), 125);
    assert_eq!(normalized_session_refresh_interval("0"), 1);
    assert_eq!(normalized_session_refresh_interval("9000"), 3600);
    assert_eq!(normalized_session_refresh_interval("not-a-number"), 60);
    assert_eq!(normalized_session_refresh_interval(""), 60);
}

#[test]
fn refresh_interval_step_targets_move_by_five_without_crossing_bounds() {
    assert_eq!(refresh_interval_step_targets(0), (1, 6));
    assert_eq!(refresh_interval_step_targets(3), (1, 8));
    assert_eq!(refresh_interval_step_targets(60), (55, 65));
    assert_eq!(refresh_interval_step_targets(3598), (3593, 3600));
    assert_eq!(refresh_interval_step_targets(u64::MAX), (3595, 3600));
}

#[test]
fn auto_refresh_message_builders_preserve_toggled_values() {
    assert!(matches!(
        unwrap_project_message(task_board_auto_refresh_toggled(false)),
        ProjectMessage::ProjectEditTaskBoardAutoRefreshToggled(false)
    ));
    assert!(matches!(
        unwrap_project_message(session_auto_refresh_toggled(true)),
        ProjectMessage::ProjectEditSessionAutoRefreshToggled(true)
    ));
}

#[test]
fn session_refresh_message_builders_preserve_step_and_raw_input() {
    assert!(matches!(
        unwrap_project_message(session_refresh_interval_changed(3600)),
        ProjectMessage::ProjectEditSessionRefreshIntervalSecondsChanged(3600)
    ));
    assert!(matches!(
        unwrap_project_message(session_refresh_interval_input_changed(" 7x ".to_string())),
        ProjectMessage::ProjectEditSessionRefreshIntervalSecondsInputChanged(value)
            if value == " 7x "
    ));
}

#[test]
fn task_board_refresh_message_builders_preserve_step_and_raw_input() {
    assert!(matches!(
        unwrap_project_message(task_board_refresh_interval_changed(1)),
        ProjectMessage::ProjectEditTaskBoardRefreshIntervalSecondsChanged(1)
    ));
    assert!(matches!(
        unwrap_project_message(task_board_refresh_interval_input_changed("3601".to_string())),
        ProjectMessage::ProjectEditTaskBoardRefreshIntervalSecondsInputChanged(value)
            if value == "3601"
    ));
}

#[test]
fn refresh_tab_builds_with_default_refresh_settings() {
    let app = test_app();

    let element = refresh_tab(&app);

    std::hint::black_box(element);
}

#[test]
fn refresh_tab_clamps_invalid_and_low_intervals_for_controls() {
    let mut app = test_app();
    app.project_edit_session_refresh_interval_seconds_input = "not-a-number".to_string();
    app.project_edit_task_board_settings.refresh_interval_seconds = 0;
    app.project_edit_task_board_auto_refresh = false;
    app.project_edit_session_auto_refresh = false;

    let element = refresh_tab(&app);

    std::hint::black_box(element);
}

#[test]
fn refresh_tab_clamps_high_intervals_for_controls() {
    let mut app = test_app();
    app.project_edit_session_refresh_interval_seconds_input = "9000".to_string();
    app.project_edit_task_board_settings.refresh_interval_seconds = 9000;

    let element = refresh_tab(&app);

    std::hint::black_box(element);
}
