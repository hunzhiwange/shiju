use super::*;
use crate::app::{App, Message, message};
use iced::Element;
use iced::widget::text;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn settings_message(message: Message) -> message::SettingsMessage {
    match message {
        Message::Settings(settings) => settings,
        _ => panic!("expected settings message"),
    }
}

#[test]
fn field_row_accepts_agents_ipc_control() {
    keep_element(field_row("数据库路径", "多个进程共享 SQLite 文件。", text("control")));
}

#[test]
fn view_builds_default_enabled_and_slider_boundary_states() {
    let mut app = test_app();
    keep_element(view(&app));

    app.agents_ipc_settings.enabled = true;
    app.agents_ipc_settings.db_path_input = "/tmp/vw-agents.db".to_string();
    app.agents_ipc_settings.staleness_secs = 1;
    keep_element(view(&app));

    app.agents_ipc_settings.staleness_secs = 86_400;
    keep_element(view(&app));
}

#[test]
fn enabled_control_maps_both_values_into_settings_messages() {
    for expected in [false, true] {
        match settings_message(enabled_toggled(expected)) {
            message::SettingsMessage::AgentsIpcEnabledToggled(value) => {
                assert_eq!(value, expected);
            }
            _ => panic!("expected agents IPC enabled message"),
        }
    }
}

#[test]
fn db_path_control_preserves_user_input_in_settings_message() {
    let expected = "  /tmp/共享 agents.db  ".to_string();

    match settings_message(db_path_changed(expected.clone())) {
        message::SettingsMessage::AgentsIpcDbPathChanged(value) => {
            assert_eq!(value, expected);
        }
        _ => panic!("expected agents IPC database path message"),
    }
}

#[test]
fn staleness_slider_rounds_values_into_settings_messages() {
    match settings_message(staleness_secs_changed(299.49)) {
        message::SettingsMessage::AgentsIpcStalenessSecsChanged(value) => {
            assert_eq!(value, 299);
        }
        _ => panic!("expected agents IPC staleness message"),
    }

    match settings_message(staleness_secs_changed(299.5)) {
        message::SettingsMessage::AgentsIpcStalenessSecsChanged(value) => {
            assert_eq!(value, 300);
        }
        _ => panic!("expected agents IPC staleness message"),
    }

    match settings_message(staleness_secs_changed(86_400.0)) {
        message::SettingsMessage::AgentsIpcStalenessSecsChanged(value) => {
            assert_eq!(value, 86_400);
        }
        _ => panic!("expected agents IPC staleness message"),
    }
}

#[test]
fn overlays_return_dialog_or_help_modal() {
    let mut app = test_app();
    keep_element(view_overlays(&app, text("dialog").into()));

    app.agents_ipc_settings.show_help_modal = true;
    keep_element(view_overlays(&app, text("dialog").into()));
}
