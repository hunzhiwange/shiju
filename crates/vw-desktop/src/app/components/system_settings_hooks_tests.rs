use super::*;
use crate::app::App;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn hooks_message(message: Message) -> HooksMessage {
    match message {
        Message::Settings(message::SettingsMessage::Hooks(hooks)) => hooks,
        _ => panic!("expected hooks settings message"),
    }
}

#[test]
fn system_settings_hooks_tests_are_wired() {
    assert!(module_path!().contains("system_settings_hooks_tests"));
}

#[test]
fn field_row_accepts_hooks_control() {
    keep_element(field_row("钩子", "钩子说明", text("control")));
}

#[test]
fn enabled_toggle_maps_both_values_to_hooks_messages() {
    for expected in [false, true] {
        match hooks_message(enabled_toggled(expected)) {
            HooksMessage::EnabledToggled(value) => assert_eq!(value, expected),
            _ => panic!("expected hooks enabled message"),
        }
    }
}

#[test]
fn command_logger_toggle_maps_both_values_to_hooks_messages() {
    for expected in [false, true] {
        match hooks_message(command_logger_toggled(expected)) {
            HooksMessage::CommandLoggerToggled(value) => assert_eq!(value, expected),
            _ => panic!("expected command logger message"),
        }
    }
}

#[test]
fn view_builds_all_hook_toggle_combinations() {
    for (enabled, command_logger) in [(false, false), (false, true), (true, false), (true, true)] {
        let mut app = test_app();
        app.hooks_settings.enabled = enabled;
        app.hooks_settings.command_logger = command_logger;
        keep_element(view(&app));
    }
}
