use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

fn has_scheduled_action(task: Task<Message>) -> bool {
    task.units() > 0
}

#[test]
fn enabled_toggle_updates_only_enabled_and_clears_save_error() {
    let mut app = app();
    app.hooks_settings.enabled = true;
    app.hooks_settings.command_logger = true;
    app.hooks_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::Hooks(HooksMessage::EnabledToggled(false)));

    assert!(!app.hooks_settings.enabled);
    assert!(app.hooks_settings.command_logger);
    assert!(app.hooks_settings.save_error.is_none());
}

#[test]
fn command_logger_toggle_updates_only_command_logger_and_clears_save_error() {
    let mut app = app();
    app.hooks_settings.enabled = false;
    app.hooks_settings.command_logger = false;
    app.hooks_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::Hooks(HooksMessage::CommandLoggerToggled(true)));

    assert!(!app.hooks_settings.enabled);
    assert!(app.hooks_settings.command_logger);
    assert!(app.hooks_settings.save_error.is_none());
}

#[test]
fn unrelated_message_preserves_all_hooks_state() {
    let mut app = app();
    app.hooks_settings.enabled = false;
    app.hooks_settings.command_logger = true;
    app.hooks_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::HeartbeatHelpOpen);

    assert!(!app.hooks_settings.enabled);
    assert!(app.hooks_settings.command_logger);
    assert_eq!(app.hooks_settings.save_error.as_deref(), Some("old"));
}

#[test]
fn hooks_edits_schedule_persistence_while_unrelated_messages_do_not() {
    let mut app = app();

    let enabled_task = update(&mut app, SettingsMessage::Hooks(HooksMessage::EnabledToggled(true)));
    assert!(has_scheduled_action(enabled_task));

    let logger_task =
        update(&mut app, SettingsMessage::Hooks(HooksMessage::CommandLoggerToggled(true)));
    assert!(has_scheduled_action(logger_task));

    let unrelated_task = update(&mut app, SettingsMessage::HeartbeatHelpOpen);
    assert!(!has_scheduled_action(unrelated_task));
}
