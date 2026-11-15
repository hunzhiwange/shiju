use super::*;

#[test]
fn unrelated_settings_message_leaves_security_state_unchanged() {
    let mut app = App::new().0;
    app.security_settings.show_help_modal = true;
    app.security_settings.save_error = Some("unchanged".to_string());

    drop(update(&mut app, SettingsMessage::ReliabilityHelpClose));

    assert!(app.security_settings.show_help_modal);
    assert_eq!(app.security_settings.save_error.as_deref(), Some("unchanged"));
}
