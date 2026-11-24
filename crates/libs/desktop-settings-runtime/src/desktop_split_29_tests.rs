use crate::hooks::update;
use crate::messages::HooksMessage;
use crate::state::HooksSettingsState;

#[test]
fn enabled_toggle_updates_only_enabled_and_clears_save_error() {
    let mut app = HooksSettingsState {
        enabled: true,
        command_logger: true,
        save_error: Some("old".to_string()),
        ..Default::default()
    };

    let _ = update(&mut app, HooksMessage::EnabledToggled(false));

    assert!(!app.enabled);
    assert!(app.command_logger);
    assert!(app.save_error.is_none());
}

#[test]
fn command_logger_toggle_updates_only_command_logger_and_clears_save_error() {
    let mut app = HooksSettingsState {
        enabled: false,
        command_logger: false,
        save_error: Some("old".to_string()),
        ..Default::default()
    };

    let _ = update(&mut app, HooksMessage::CommandLoggerToggled(true));

    assert!(!app.enabled);
    assert!(app.command_logger);
    assert!(app.save_error.is_none());
}
