use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

fn has_scheduled_action(task: Task<Message>) -> bool {
    task.units() > 0
}

#[test]
fn enabled_toggle_updates_state_and_schedules_persistence() {
    let mut app = app();

    let enabled_task = update(&mut app, SettingsMessage::HeartbeatEnabledToggled(true));

    assert!(app.heartbeat_settings.enabled);
    assert!(has_scheduled_action(enabled_task));

    let disabled_task = update(&mut app, SettingsMessage::HeartbeatEnabledToggled(false));

    assert!(!app.heartbeat_settings.enabled);
    assert!(has_scheduled_action(disabled_task));
}

#[test]
fn interval_change_clamps_bounds_and_preserves_in_range_values() {
    let mut app = app();

    for (input, expected) in [(0, 1), (1, 1), (37, 37), (1_440, 1_440), (u32::MAX, 1_440)] {
        let task = update(&mut app, SettingsMessage::HeartbeatIntervalChanged(input));

        assert_eq!(app.heartbeat_settings.interval_minutes, expected);
        assert!(has_scheduled_action(task));
    }
}

#[test]
fn text_changes_preserve_exact_input_and_schedule_persistence() {
    let mut app = app();

    let message_task =
        update(&mut app, SettingsMessage::HeartbeatMessageChanged("  ping  ".to_string()));
    assert_eq!(app.heartbeat_settings.message_input, "  ping  ");
    assert!(has_scheduled_action(message_task));

    let target_task =
        update(&mut app, SettingsMessage::HeartbeatTargetChanged("  room  ".to_string()));
    assert_eq!(app.heartbeat_settings.target_input, "  room  ");
    assert!(has_scheduled_action(target_task));

    let to_task = update(&mut app, SettingsMessage::HeartbeatToChanged("   ".to_string()));
    assert_eq!(app.heartbeat_settings.to_input, "   ");
    assert!(has_scheduled_action(to_task));
}

#[test]
fn save_schedules_persistence_without_changing_ui_state() {
    let mut app = app();
    app.heartbeat_settings.enabled = true;
    app.heartbeat_settings.interval_minutes = 0;
    app.heartbeat_settings.message_input = " message ".to_string();
    app.heartbeat_settings.target_input = " target ".to_string();
    app.heartbeat_settings.to_input = " to ".to_string();
    app.heartbeat_settings.show_help_modal = true;

    let task = update(&mut app, SettingsMessage::HeartbeatSave);

    assert!(app.heartbeat_settings.enabled);
    assert_eq!(app.heartbeat_settings.interval_minutes, 0);
    assert_eq!(app.heartbeat_settings.message_input, " message ");
    assert_eq!(app.heartbeat_settings.target_input, " target ");
    assert_eq!(app.heartbeat_settings.to_input, " to ");
    assert!(app.heartbeat_settings.show_help_modal);
    assert!(has_scheduled_action(task));
}

#[test]
fn help_messages_only_toggle_modal_and_do_not_schedule_persistence() {
    let mut app = app();
    app.heartbeat_settings.enabled = true;
    app.heartbeat_settings.interval_minutes = 45;
    app.heartbeat_settings.message_input = "message".to_string();

    let open_task = update(&mut app, SettingsMessage::HeartbeatHelpOpen);

    assert!(app.heartbeat_settings.show_help_modal);
    assert!(app.heartbeat_settings.enabled);
    assert_eq!(app.heartbeat_settings.interval_minutes, 45);
    assert_eq!(app.heartbeat_settings.message_input, "message");
    assert!(!has_scheduled_action(open_task));

    let close_task = update(&mut app, SettingsMessage::HeartbeatHelpClose);

    assert!(!app.heartbeat_settings.show_help_modal);
    assert!(app.heartbeat_settings.enabled);
    assert_eq!(app.heartbeat_settings.interval_minutes, 45);
    assert_eq!(app.heartbeat_settings.message_input, "message");
    assert!(!has_scheduled_action(close_task));
}

#[test]
fn unrelated_settings_message_is_ignored_without_scheduling_work() {
    let mut app = app();
    app.heartbeat_settings.enabled = true;
    app.heartbeat_settings.interval_minutes = 75;
    app.heartbeat_settings.message_input = "message".to_string();
    app.heartbeat_settings.target_input = "target".to_string();
    app.heartbeat_settings.to_input = "to".to_string();
    app.heartbeat_settings.show_help_modal = true;
    app.heartbeat_settings.save_error = Some("unchanged".to_string());

    let task = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(app.heartbeat_settings.enabled);
    assert_eq!(app.heartbeat_settings.interval_minutes, 75);
    assert_eq!(app.heartbeat_settings.message_input, "message");
    assert_eq!(app.heartbeat_settings.target_input, "target");
    assert_eq!(app.heartbeat_settings.to_input, "to");
    assert!(app.heartbeat_settings.show_help_modal);
    assert_eq!(app.heartbeat_settings.save_error.as_deref(), Some("unchanged"));
    assert!(!has_scheduled_action(task));
}

#[test]
fn persistence_updates_leave_existing_save_error_unchanged() {
    let mut app = app();
    app.heartbeat_settings.save_error = Some("existing failure".to_string());

    let task =
        update(&mut app, SettingsMessage::HeartbeatMessageChanged("next heartbeat".to_string()));

    assert_eq!(app.heartbeat_settings.message_input, "next heartbeat");
    assert_eq!(app.heartbeat_settings.save_error.as_deref(), Some("existing failure"));
    assert!(has_scheduled_action(task));
}
