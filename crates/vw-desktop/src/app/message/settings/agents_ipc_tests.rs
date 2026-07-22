use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

#[test]
fn enabled_toggle_updates_only_enabled_and_clears_save_error() {
    let mut app = app();
    app.agents_ipc_settings.enabled = true;
    app.agents_ipc_settings.db_path_input = " ipc.db ".to_string();
    app.agents_ipc_settings.staleness_secs = 321;
    app.agents_ipc_settings.show_help_modal = true;
    app.agents_ipc_settings.save_error = Some("stale".to_string());

    let _ = update(&mut app, SettingsMessage::AgentsIpcEnabledToggled(false));

    assert!(!app.agents_ipc_settings.enabled);
    assert_eq!(app.agents_ipc_settings.db_path_input, " ipc.db ");
    assert_eq!(app.agents_ipc_settings.staleness_secs, 321);
    assert!(app.agents_ipc_settings.show_help_modal);
    assert!(app.agents_ipc_settings.save_error.is_none());
}

#[test]
fn db_path_change_preserves_user_input_and_clears_save_error() {
    let mut app = app();
    app.agents_ipc_settings.enabled = false;
    app.agents_ipc_settings.staleness_secs = 456;
    app.agents_ipc_settings.save_error = Some("stale".to_string());

    let _ =
        update(&mut app, SettingsMessage::AgentsIpcDbPathChanged("  /tmp/agents.db  ".to_string()));

    assert_eq!(app.agents_ipc_settings.db_path_input, "  /tmp/agents.db  ");
    assert!(!app.agents_ipc_settings.enabled);
    assert_eq!(app.agents_ipc_settings.staleness_secs, 456);
    assert!(app.agents_ipc_settings.save_error.is_none());
}

#[test]
fn blank_db_path_change_preserves_edit_buffer_for_default_path_persistence() {
    let mut app = app();
    app.agents_ipc_settings.db_path_input = "custom.db".to_string();
    app.agents_ipc_settings.save_error = Some("stale".to_string());

    let task = update(&mut app, SettingsMessage::AgentsIpcDbPathChanged(" \n\t ".to_string()));

    assert_eq!(app.agents_ipc_settings.db_path_input, " \n\t ");
    assert!(app.agents_ipc_settings.save_error.is_none());
    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

#[test]
fn staleness_change_clamps_both_bounds_and_accepts_in_range_value() {
    let mut app = app();

    app.agents_ipc_settings.save_error = Some("lower".to_string());
    let _ = update(&mut app, SettingsMessage::AgentsIpcStalenessSecsChanged(0));
    assert_eq!(app.agents_ipc_settings.staleness_secs, 1);
    assert!(app.agents_ipc_settings.save_error.is_none());

    app.agents_ipc_settings.save_error = Some("upper".to_string());
    let _ = update(&mut app, SettingsMessage::AgentsIpcStalenessSecsChanged(u64::MAX));
    assert_eq!(app.agents_ipc_settings.staleness_secs, 86_400);
    assert!(app.agents_ipc_settings.save_error.is_none());

    let _ = update(&mut app, SettingsMessage::AgentsIpcStalenessSecsChanged(600));
    assert_eq!(app.agents_ipc_settings.staleness_secs, 600);
}

#[test]
fn save_clears_error_without_changing_editable_state() {
    let mut app = app();
    app.agents_ipc_settings.enabled = false;
    app.agents_ipc_settings.db_path_input = "  custom.db  ".to_string();
    app.agents_ipc_settings.staleness_secs = 777;
    app.agents_ipc_settings.show_help_modal = true;
    app.agents_ipc_settings.save_error = Some("save failed".to_string());

    let _ = update(&mut app, SettingsMessage::AgentsIpcSave);

    assert!(!app.agents_ipc_settings.enabled);
    assert_eq!(app.agents_ipc_settings.db_path_input, "  custom.db  ");
    assert_eq!(app.agents_ipc_settings.staleness_secs, 777);
    assert!(app.agents_ipc_settings.show_help_modal);
    assert!(app.agents_ipc_settings.save_error.is_none());
}

#[test]
fn help_messages_toggle_only_modal_visibility() {
    let mut app = app();
    app.agents_ipc_settings.enabled = false;
    app.agents_ipc_settings.db_path_input = "keep.db".to_string();
    app.agents_ipc_settings.staleness_secs = 888;
    app.agents_ipc_settings.save_error = Some("keep".to_string());

    let _ = update(&mut app, SettingsMessage::AgentsIpcHelpOpen);
    assert!(app.agents_ipc_settings.show_help_modal);

    let _ = update(&mut app, SettingsMessage::AgentsIpcHelpClose);
    assert!(!app.agents_ipc_settings.show_help_modal);
    assert!(!app.agents_ipc_settings.enabled);
    assert_eq!(app.agents_ipc_settings.db_path_input, "keep.db");
    assert_eq!(app.agents_ipc_settings.staleness_secs, 888);
    assert_eq!(app.agents_ipc_settings.save_error.as_deref(), Some("keep"));
}

#[test]
fn unrelated_message_preserves_all_agents_ipc_state() {
    let mut app = app();
    app.agents_ipc_settings.enabled = false;
    app.agents_ipc_settings.db_path_input = "unchanged.db".to_string();
    app.agents_ipc_settings.staleness_secs = 999;
    app.agents_ipc_settings.show_help_modal = true;
    app.agents_ipc_settings.save_error = Some("unchanged".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(!app.agents_ipc_settings.enabled);
    assert_eq!(app.agents_ipc_settings.db_path_input, "unchanged.db");
    assert_eq!(app.agents_ipc_settings.staleness_secs, 999);
    assert!(app.agents_ipc_settings.show_help_modal);
    assert_eq!(app.agents_ipc_settings.save_error.as_deref(), Some("unchanged"));
}
