use crate::messages::SettingsMessage;
use crate::observability::update;
#[test]
fn update_ignores_unrelated_messages() {
    let mut app = crate::state::ObservabilitySettingsState {
        backend: "otel".to_string(),
        otel_endpoint_input: "http://collector.example".to_string(),
        show_help_modal: true,
        save_error: Some("old error".to_string()),
        ..Default::default()
    };

    let _ = update(&mut app, SettingsMessage::ReliabilityHelpClose);

    assert_eq!(app.backend, "otel");
    assert_eq!(app.otel_endpoint_input, "http://collector.example");
    assert!(app.show_help_modal);
    assert_eq!(app.save_error.as_deref(), Some("old error"));
}
