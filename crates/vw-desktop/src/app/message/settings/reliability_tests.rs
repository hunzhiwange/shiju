use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

#[test]
fn reliability_updates_accept_in_range_values_and_clear_stale_errors() {
    let mut app = app();

    app.reliability_settings.save_error = Some("stale".to_string());
    let _ = update(&mut app, SettingsMessage::ReliabilityProviderRetriesChanged(7));
    assert_eq!(app.reliability_settings.provider_retries, 7);
    assert!(app.reliability_settings.save_error.is_none());

    app.reliability_settings.save_error = Some("stale".to_string());
    let _ = update(&mut app, SettingsMessage::ReliabilityProviderBackoffMsChanged(1_500));
    assert_eq!(app.reliability_settings.provider_backoff_ms, 1_500);
    assert!(app.reliability_settings.save_error.is_none());

    app.reliability_settings.save_error = Some("stale".to_string());
    let _ = update(&mut app, SettingsMessage::ReliabilitySchedulerPollSecsChanged(30));
    assert_eq!(app.reliability_settings.scheduler_poll_secs, 30);
    assert!(app.reliability_settings.save_error.is_none());

    app.reliability_settings.save_error = Some("stale".to_string());
    let _ = update(&mut app, SettingsMessage::ReliabilitySchedulerRetriesChanged(8));
    assert_eq!(app.reliability_settings.scheduler_retries, 8);
    assert!(app.reliability_settings.save_error.is_none());
}

#[test]
fn reliability_updates_clamp_numeric_values_to_safe_bounds() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::ReliabilityProviderRetriesChanged(u32::MAX));
    assert_eq!(app.reliability_settings.provider_retries, 20);
    let _ = update(&mut app, SettingsMessage::ReliabilityProviderRetriesChanged(0));
    assert_eq!(app.reliability_settings.provider_retries, 0);

    let _ = update(&mut app, SettingsMessage::ReliabilityProviderBackoffMsChanged(u64::MAX));
    assert_eq!(app.reliability_settings.provider_backoff_ms, 60_000);
    let _ = update(&mut app, SettingsMessage::ReliabilityProviderBackoffMsChanged(0));
    assert_eq!(app.reliability_settings.provider_backoff_ms, 0);

    let _ = update(&mut app, SettingsMessage::ReliabilitySchedulerPollSecsChanged(0));
    assert_eq!(app.reliability_settings.scheduler_poll_secs, 1);
    let _ = update(&mut app, SettingsMessage::ReliabilitySchedulerPollSecsChanged(u64::MAX));
    assert_eq!(app.reliability_settings.scheduler_poll_secs, 3600);

    let _ = update(&mut app, SettingsMessage::ReliabilitySchedulerRetriesChanged(u32::MAX));
    assert_eq!(app.reliability_settings.scheduler_retries, 20);
    let _ = update(&mut app, SettingsMessage::ReliabilitySchedulerRetriesChanged(0));
    assert_eq!(app.reliability_settings.scheduler_retries, 0);
}

#[test]
fn channel_initial_backoff_clamps_and_preserves_maximum_invariant() {
    let mut app = app();
    app.reliability_settings.channel_max_backoff_secs = 60;

    let _ = update(&mut app, SettingsMessage::ReliabilityChannelInitialBackoffSecsChanged(0));
    assert_eq!(app.reliability_settings.channel_initial_backoff_secs, 1);
    assert_eq!(app.reliability_settings.channel_max_backoff_secs, 60);

    let _ = update(&mut app, SettingsMessage::ReliabilityChannelInitialBackoffSecsChanged(30));
    assert_eq!(app.reliability_settings.channel_initial_backoff_secs, 30);
    assert_eq!(app.reliability_settings.channel_max_backoff_secs, 60);

    let _ =
        update(&mut app, SettingsMessage::ReliabilityChannelInitialBackoffSecsChanged(u64::MAX));
    assert_eq!(app.reliability_settings.channel_initial_backoff_secs, 3600);
    assert_eq!(app.reliability_settings.channel_max_backoff_secs, 3600);
}

#[test]
fn channel_max_backoff_uses_safe_minimum_current_initial_and_upper_bound() {
    let mut app = app();

    app.reliability_settings.channel_initial_backoff_secs = 0;
    let _ = update(&mut app, SettingsMessage::ReliabilityChannelMaxBackoffSecsChanged(0));
    assert_eq!(app.reliability_settings.channel_max_backoff_secs, 1);

    app.reliability_settings.channel_initial_backoff_secs = 40;
    let _ = update(&mut app, SettingsMessage::ReliabilityChannelMaxBackoffSecsChanged(20));
    assert_eq!(app.reliability_settings.channel_max_backoff_secs, 40);

    let _ = update(&mut app, SettingsMessage::ReliabilityChannelMaxBackoffSecsChanged(90));
    assert_eq!(app.reliability_settings.channel_max_backoff_secs, 90);

    let _ = update(&mut app, SettingsMessage::ReliabilityChannelMaxBackoffSecsChanged(u64::MAX));
    assert_eq!(app.reliability_settings.channel_max_backoff_secs, 3600);
}

#[test]
fn reliability_save_clears_error_without_changing_values() {
    let mut app = app();
    app.reliability_settings.provider_retries = 9;
    app.reliability_settings.provider_backoff_ms = 2_000;
    app.reliability_settings.channel_initial_backoff_secs = 4;
    app.reliability_settings.channel_max_backoff_secs = 120;
    app.reliability_settings.scheduler_poll_secs = 45;
    app.reliability_settings.scheduler_retries = 6;
    app.reliability_settings.save_error = Some("save failed".to_string());

    let _ = update(&mut app, SettingsMessage::ReliabilitySave);

    assert_eq!(app.reliability_settings.provider_retries, 9);
    assert_eq!(app.reliability_settings.provider_backoff_ms, 2_000);
    assert_eq!(app.reliability_settings.channel_initial_backoff_secs, 4);
    assert_eq!(app.reliability_settings.channel_max_backoff_secs, 120);
    assert_eq!(app.reliability_settings.scheduler_poll_secs, 45);
    assert_eq!(app.reliability_settings.scheduler_retries, 6);
    assert!(app.reliability_settings.save_error.is_none());
}

#[test]
fn reliability_help_messages_only_toggle_modal() {
    let mut app = app();
    app.reliability_settings.save_error = Some("unchanged".to_string());

    let _ = update(&mut app, SettingsMessage::ReliabilityHelpOpen);
    assert!(app.reliability_settings.show_help_modal);
    assert_eq!(app.reliability_settings.save_error.as_deref(), Some("unchanged"));

    let _ = update(&mut app, SettingsMessage::ReliabilityHelpClose);
    assert!(!app.reliability_settings.show_help_modal);
    assert_eq!(app.reliability_settings.save_error.as_deref(), Some("unchanged"));
}

#[test]
fn reliability_update_ignores_unrelated_message() {
    let mut app = app();
    app.reliability_settings.provider_retries = 11;
    app.reliability_settings.show_help_modal = true;
    app.reliability_settings.save_error = Some("unchanged".to_string());

    let _ = update(&mut app, SettingsMessage::HeartbeatHelpOpen);

    assert_eq!(app.reliability_settings.provider_retries, 11);
    assert!(app.reliability_settings.show_help_modal);
    assert_eq!(app.reliability_settings.save_error.as_deref(), Some("unchanged"));
}

#[test]
fn reliability_changes_schedule_persistence_tasks() {
    let mut app = app();

    for message in [
        SettingsMessage::ReliabilityProviderRetriesChanged(3),
        SettingsMessage::ReliabilityChannelInitialBackoffSecsChanged(5),
        SettingsMessage::ReliabilitySave,
    ] {
        let task = update(&mut app, message);
        assert!(iced_runtime::task::into_stream(task.collect()).is_some());
    }
}
