use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

#[test]
fn coordination_updates_accept_in_range_values_and_clear_stale_errors() {
    let mut app = app();

    app.coordination_settings.save_error = Some("stale".to_string());
    let _ = update(&mut app, SettingsMessage::CoordinationEnabledToggled(false));
    assert!(!app.coordination_settings.enabled);
    assert!(app.coordination_settings.save_error.is_none());

    app.coordination_settings.save_error = Some("stale".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::CoordinationLeadAgentChanged(" delegate-main ".to_string()),
    );
    assert_eq!(app.coordination_settings.lead_agent_input, " delegate-main ");
    assert!(app.coordination_settings.save_error.is_none());

    app.coordination_settings.save_error = Some("stale".to_string());
    let _ = update(&mut app, SettingsMessage::CoordinationMaxInboxMessagesPerAgentChanged(512));
    assert_eq!(app.coordination_settings.max_inbox_messages_per_agent, 512);
    assert!(app.coordination_settings.save_error.is_none());

    app.coordination_settings.save_error = Some("stale".to_string());
    let _ = update(&mut app, SettingsMessage::CoordinationMaxDeadLettersChanged(256));
    assert_eq!(app.coordination_settings.max_dead_letters, 256);
    assert!(app.coordination_settings.save_error.is_none());

    app.coordination_settings.save_error = Some("stale".to_string());
    let _ = update(&mut app, SettingsMessage::CoordinationMaxContextEntriesChanged(1_024));
    assert_eq!(app.coordination_settings.max_context_entries, 1_024);
    assert!(app.coordination_settings.save_error.is_none());

    app.coordination_settings.save_error = Some("stale".to_string());
    let _ = update(&mut app, SettingsMessage::CoordinationMaxSeenMessageIdsChanged(8_192));
    assert_eq!(app.coordination_settings.max_seen_message_ids, 8_192);
    assert!(app.coordination_settings.save_error.is_none());
}

#[test]
fn coordination_numeric_updates_clamp_both_safe_bounds() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::CoordinationMaxInboxMessagesPerAgentChanged(0));
    assert_eq!(app.coordination_settings.max_inbox_messages_per_agent, 1);
    let _ =
        update(&mut app, SettingsMessage::CoordinationMaxInboxMessagesPerAgentChanged(u32::MAX));
    assert_eq!(app.coordination_settings.max_inbox_messages_per_agent, 10_000);

    let _ = update(&mut app, SettingsMessage::CoordinationMaxDeadLettersChanged(0));
    assert_eq!(app.coordination_settings.max_dead_letters, 1);
    let _ = update(&mut app, SettingsMessage::CoordinationMaxDeadLettersChanged(u32::MAX));
    assert_eq!(app.coordination_settings.max_dead_letters, 10_000);

    let _ = update(&mut app, SettingsMessage::CoordinationMaxContextEntriesChanged(0));
    assert_eq!(app.coordination_settings.max_context_entries, 1);
    let _ = update(&mut app, SettingsMessage::CoordinationMaxContextEntriesChanged(u32::MAX));
    assert_eq!(app.coordination_settings.max_context_entries, 20_000);

    let _ = update(&mut app, SettingsMessage::CoordinationMaxSeenMessageIdsChanged(0));
    assert_eq!(app.coordination_settings.max_seen_message_ids, 1);
    let _ = update(&mut app, SettingsMessage::CoordinationMaxSeenMessageIdsChanged(u32::MAX));
    assert_eq!(app.coordination_settings.max_seen_message_ids, 100_000);
}

#[test]
fn coordination_save_clears_error_without_changing_values() {
    let mut app = app();
    app.coordination_settings.enabled = false;
    app.coordination_settings.lead_agent_input = " lead ".to_string();
    app.coordination_settings.max_inbox_messages_per_agent = 321;
    app.coordination_settings.max_dead_letters = 654;
    app.coordination_settings.max_context_entries = 987;
    app.coordination_settings.max_seen_message_ids = 4_321;
    app.coordination_settings.save_error = Some("save failed".to_string());

    let _ = update(&mut app, SettingsMessage::CoordinationSave);

    assert!(!app.coordination_settings.enabled);
    assert_eq!(app.coordination_settings.lead_agent_input, " lead ");
    assert_eq!(app.coordination_settings.max_inbox_messages_per_agent, 321);
    assert_eq!(app.coordination_settings.max_dead_letters, 654);
    assert_eq!(app.coordination_settings.max_context_entries, 987);
    assert_eq!(app.coordination_settings.max_seen_message_ids, 4_321);
    assert!(app.coordination_settings.save_error.is_none());
}

#[test]
fn coordination_help_messages_only_toggle_modal() {
    let mut app = app();
    app.coordination_settings.enabled = false;
    app.coordination_settings.save_error = Some("unchanged".to_string());

    let _ = update(&mut app, SettingsMessage::CoordinationHelpOpen);
    assert!(app.coordination_settings.show_help_modal);
    assert!(!app.coordination_settings.enabled);
    assert_eq!(app.coordination_settings.save_error.as_deref(), Some("unchanged"));

    let _ = update(&mut app, SettingsMessage::CoordinationHelpClose);
    assert!(!app.coordination_settings.show_help_modal);
    assert!(!app.coordination_settings.enabled);
    assert_eq!(app.coordination_settings.save_error.as_deref(), Some("unchanged"));
}

#[test]
fn unrelated_message_leaves_coordination_state_unchanged() {
    let mut app = app();
    app.coordination_settings.enabled = false;
    app.coordination_settings.lead_agent_input = "lead-before".to_string();
    app.coordination_settings.max_inbox_messages_per_agent = 111;
    app.coordination_settings.max_dead_letters = 222;
    app.coordination_settings.max_context_entries = 333;
    app.coordination_settings.max_seen_message_ids = 444;
    app.coordination_settings.show_help_modal = true;
    app.coordination_settings.save_error = Some("unchanged".to_string());

    let _ = update(&mut app, SettingsMessage::SystemHelpClose);

    assert!(!app.coordination_settings.enabled);
    assert_eq!(app.coordination_settings.lead_agent_input, "lead-before");
    assert_eq!(app.coordination_settings.max_inbox_messages_per_agent, 111);
    assert_eq!(app.coordination_settings.max_dead_letters, 222);
    assert_eq!(app.coordination_settings.max_context_entries, 333);
    assert_eq!(app.coordination_settings.max_seen_message_ids, 444);
    assert!(app.coordination_settings.show_help_modal);
    assert_eq!(app.coordination_settings.save_error.as_deref(), Some("unchanged"));
}
