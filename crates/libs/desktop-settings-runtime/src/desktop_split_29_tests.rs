use crate::hooks::update;
use crate::messages::HooksMessage;
use crate::state::HooksSettingsState;

#[test]
fn enabled_toggle_updates_only_enabled_and_clears_save_error() {
    let mut app = HooksSettingsState::default();
    app.enabled = true;
    app.command_logger = true;
    app.save_error = Some("old".to_string());

    let _ = update(&mut app, HooksMessage::EnabledToggled(false));

    assert!(!app.enabled);
    assert!(app.command_logger);
    assert!(app.save_error.is_none());
}

#[test]
fn command_logger_toggle_updates_only_command_logger_and_clears_save_error() {
    let mut app = HooksSettingsState::default();
    app.enabled = false;
    app.command_logger = false;
    app.save_error = Some("old".to_string());

    let _ = update(&mut app, HooksMessage::CommandLoggerToggled(true));

    assert!(!app.enabled);
    assert!(app.command_logger);
    assert!(app.save_error.is_none());
}
