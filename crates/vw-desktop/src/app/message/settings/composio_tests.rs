use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

#[test]
fn apply_composio_inputs_trims_non_empty_values() {
    let mut composio = ComposioConfig::default();

    apply_composio_inputs(
        &mut composio,
        true,
        "  secret  ".to_string(),
        "  workspace  ".to_string(),
    );

    assert!(composio.enabled);
    assert_eq!(composio.api_key.as_deref(), Some("secret"));
    assert_eq!(composio.entity_id, "workspace");
}

#[test]
fn apply_composio_inputs_maps_blank_values_to_defaults() {
    let mut composio = ComposioConfig {
        enabled: true,
        api_key: Some("old-secret".to_string()),
        entity_id: "old-workspace".to_string(),
    };

    apply_composio_inputs(&mut composio, false, " \n\t ".to_string(), "   ".to_string());

    assert!(!composio.enabled);
    assert!(composio.api_key.is_none());
    assert_eq!(composio.entity_id, "default");
}

#[test]
fn enabled_toggle_updates_value_and_clears_stale_error() {
    let mut app = app();
    app.composio_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::ComposioEnabledToggled(true));

    assert!(app.composio_settings.enabled);
    assert!(app.composio_settings.save_error.is_none());
}

#[test]
fn api_key_change_preserves_raw_input_and_clears_stale_error() {
    let mut app = app();
    app.composio_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::ComposioApiKeyChanged(" secret ".to_string()));

    assert_eq!(app.composio_settings.api_key_input, " secret ");
    assert!(app.composio_settings.save_error.is_none());
}

#[test]
fn entity_id_change_preserves_raw_input_and_clears_stale_error() {
    let mut app = app();
    app.composio_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::ComposioEntityIdChanged(" workspace ".to_string()));

    assert_eq!(app.composio_settings.entity_id_input, " workspace ");
    assert!(app.composio_settings.save_error.is_none());
}

#[test]
fn unrelated_message_preserves_composio_state() {
    let mut app = app();
    app.composio_settings.enabled = true;
    app.composio_settings.api_key_input = " secret ".to_string();
    app.composio_settings.entity_id_input = " workspace ".to_string();
    app.composio_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::HeartbeatHelpOpen);

    assert!(app.composio_settings.enabled);
    assert_eq!(app.composio_settings.api_key_input, " secret ");
    assert_eq!(app.composio_settings.entity_id_input, " workspace ");
    assert_eq!(app.composio_settings.save_error.as_deref(), Some("old"));
}
