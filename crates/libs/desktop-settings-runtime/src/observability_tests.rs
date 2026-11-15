use super::*;
fn app() -> ObservabilitySettingsState { Default::default() }
#[test]
fn backend_normalizer_accepts_supported_values_and_rejects_unknown_values() {
    for (input, expected) in
        [(" NONE ", "none"), ("Log", "log"), (" PROMETHEUS", "prometheus"), ("otel ", "otel")]
    {
        assert_eq!(normalize_observability_backend(input), expected);
    }

    assert_eq!(normalize_observability_backend(""), "none");
    assert_eq!(normalize_observability_backend("unknown"), "none");
}
#[test]
fn runtime_trace_mode_normalizer_accepts_supported_values_and_rejects_unknown_values() {
    for (input, expected) in [(" NONE ", "none"), ("Rolling", "rolling"), (" FULL", "full")] {
        assert_eq!(normalize_runtime_trace_mode(input), expected);
    }

    assert_eq!(normalize_runtime_trace_mode(""), "none");
    assert_eq!(normalize_runtime_trace_mode("unknown"), "none");
}
#[test]
fn backend_change_normalizes_value_and_clears_stale_error() {
    let mut app = app();

    app.save_error = Some("old error".to_string());
    let _ =
        update(&mut app, SettingsMessage::ObservabilityBackendChanged(" PROMETHEUS ".to_string()));
    assert_eq!(app.backend, "prometheus");
    assert!(app.save_error.is_none());

    app.save_error = Some("another error".to_string());
    let _ = update(&mut app, SettingsMessage::ObservabilityBackendChanged("invalid".to_string()));
    assert_eq!(app.backend, "none");
    assert!(app.save_error.is_none());
}
#[test]
fn otel_text_changes_preserve_input_and_clear_stale_error() {
    let mut app = app();

    app.save_error = Some("endpoint error".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::ObservabilityOtelEndpointChanged(
            " http://collector.example:4318 ".to_string(),
        ),
    );
    assert_eq!(app.otel_endpoint_input, " http://collector.example:4318 ");
    assert!(app.save_error.is_none());

    app.save_error = Some("service error".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::ObservabilityOtelServiceNameChanged(" desktop-agent ".to_string()),
    );
    assert_eq!(app.otel_service_name_input, " desktop-agent ");
    assert!(app.save_error.is_none());
}
#[test]
fn runtime_trace_mode_change_normalizes_value_and_clears_stale_error() {
    let mut app = app();

    app.save_error = Some("old error".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::ObservabilityRuntimeTraceModeChanged(" ROLLING ".to_string()),
    );
    assert_eq!(app.runtime_trace_mode, "rolling");
    assert!(app.save_error.is_none());

    app.save_error = Some("another error".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::ObservabilityRuntimeTraceModeChanged("invalid".to_string()),
    );
    assert_eq!(app.runtime_trace_mode, "none");
    assert!(app.save_error.is_none());
}
#[test]
fn runtime_trace_path_change_preserves_input_and_clears_stale_error() {
    let mut app = app();
    app.save_error = Some("old error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::ObservabilityRuntimeTracePathChanged(" traces/runtime.jsonl ".to_string()),
    );

    assert_eq!(app.runtime_trace_path_input, " traces/runtime.jsonl ");
    assert!(app.save_error.is_none());
}
#[test]
fn runtime_trace_max_entries_change_clamps_bounds_and_clears_stale_error() {
    let mut app = app();

    for (input, expected) in
        [(0, 1), (1, 1), (42_000, 42_000), (100_000, 100_000), (100_001, 100_000)]
    {
        app.save_error = Some("old error".to_string());
        let _ =
            update(&mut app, SettingsMessage::ObservabilityRuntimeTraceMaxEntriesChanged(input));

        assert_eq!(app.runtime_trace_max_entries, expected);
        assert!(app.save_error.is_none());
    }
}
#[test]
fn help_messages_toggle_modal_without_clearing_save_error() {
    let mut app = app();
    app.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::ObservabilityHelpOpen);
    assert!(app.show_help_modal);
    assert_eq!(app.save_error.as_deref(), Some("old error"));

    let _ = update(&mut app, SettingsMessage::ObservabilityHelpClose);
    assert!(!app.show_help_modal);
    assert_eq!(app.save_error.as_deref(), Some("old error"));
}
