use crate::messages::SettingsMessage;
use crate::observability::update;
#[test]
fn update_ignores_unrelated_messages() {
    let mut app = crate::state::ObservabilitySettingsState::default();
    app.backend = "otel".to_string();
    app.otel_endpoint_input = "http://collector.example".to_string();
    app.show_help_modal = true;
    app.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::ReliabilityHelpClose);

    assert_eq!(app.backend, "otel");
    assert_eq!(app.otel_endpoint_input, "http://collector.example");
    assert!(app.show_help_modal);
    assert_eq!(app.save_error.as_deref(), Some("old error"));
}
