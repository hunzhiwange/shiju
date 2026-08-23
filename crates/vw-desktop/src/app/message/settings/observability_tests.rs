// 验证可观测性后端、追踪模式及相关文本字段更新。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
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
fn update_ignores_unrelated_messages() {
    let mut app = app();
    app.observability_settings.backend = "otel".to_string();
    app.observability_settings.otel_endpoint_input = "http://collector.example".to_string();
    app.observability_settings.show_help_modal = true;
    app.observability_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert_eq!(app.observability_settings.backend, "otel");
    assert_eq!(app.observability_settings.otel_endpoint_input, "http://collector.example");
    assert!(app.observability_settings.show_help_modal);
    assert_eq!(app.observability_settings.save_error.as_deref(), Some("old error"));
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn backend_change_normalizes_value_and_clears_stale_error() {
    let mut app = app();

    app.observability_settings.save_error = Some("old error".to_string());
    let _ =
        update(&mut app, SettingsMessage::ObservabilityBackendChanged(" PROMETHEUS ".to_string()));
    assert_eq!(app.observability_settings.backend, "prometheus");
    assert!(app.observability_settings.save_error.is_none());

    app.observability_settings.save_error = Some("another error".to_string());
    let _ = update(&mut app, SettingsMessage::ObservabilityBackendChanged("invalid".to_string()));
    assert_eq!(app.observability_settings.backend, "none");
    assert!(app.observability_settings.save_error.is_none());
}

#[test]
fn otel_text_changes_preserve_input_and_clear_stale_error() {
    let mut app = app();

    app.observability_settings.save_error = Some("endpoint error".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::ObservabilityOtelEndpointChanged(
            " http://collector.example:4318 ".to_string(),
        ),
    );
    assert_eq!(app.observability_settings.otel_endpoint_input, " http://collector.example:4318 ");
    assert!(app.observability_settings.save_error.is_none());

    app.observability_settings.save_error = Some("service error".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::ObservabilityOtelServiceNameChanged(" desktop-agent ".to_string()),
    );
    assert_eq!(app.observability_settings.otel_service_name_input, " desktop-agent ");
    assert!(app.observability_settings.save_error.is_none());
}

#[test]
fn runtime_trace_mode_change_normalizes_value_and_clears_stale_error() {
    let mut app = app();

    app.observability_settings.save_error = Some("old error".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::ObservabilityRuntimeTraceModeChanged(" ROLLING ".to_string()),
    );
    assert_eq!(app.observability_settings.runtime_trace_mode, "rolling");
    assert!(app.observability_settings.save_error.is_none());

    app.observability_settings.save_error = Some("another error".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::ObservabilityRuntimeTraceModeChanged("invalid".to_string()),
    );
    assert_eq!(app.observability_settings.runtime_trace_mode, "none");
    assert!(app.observability_settings.save_error.is_none());
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn runtime_trace_path_change_preserves_input_and_clears_stale_error() {
    let mut app = app();
    app.observability_settings.save_error = Some("old error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::ObservabilityRuntimeTracePathChanged(" traces/runtime.jsonl ".to_string()),
    );

    assert_eq!(app.observability_settings.runtime_trace_path_input, " traces/runtime.jsonl ");
    assert!(app.observability_settings.save_error.is_none());
}

#[test]
fn runtime_trace_max_entries_change_clamps_bounds_and_clears_stale_error() {
    let mut app = app();

    for (input, expected) in
        [(0, 1), (1, 1), (42_000, 42_000), (100_000, 100_000), (100_001, 100_000)]
    {
        app.observability_settings.save_error = Some("old error".to_string());
        let _ =
            update(&mut app, SettingsMessage::ObservabilityRuntimeTraceMaxEntriesChanged(input));

        assert_eq!(app.observability_settings.runtime_trace_max_entries, expected);
        assert!(app.observability_settings.save_error.is_none());
    }
}

#[test]
fn help_messages_toggle_modal_without_clearing_save_error() {
    let mut app = app();
    app.observability_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::ObservabilityHelpOpen);
    assert!(app.observability_settings.show_help_modal);
    assert_eq!(app.observability_settings.save_error.as_deref(), Some("old error"));

    let _ = update(&mut app, SettingsMessage::ObservabilityHelpClose);
    assert!(!app.observability_settings.show_help_modal);
    assert_eq!(app.observability_settings.save_error.as_deref(), Some("old error"));
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn observability_changes_schedule_persistence_tasks() {
    let mut app = app();

    for message in [
        SettingsMessage::ObservabilityBackendChanged("log".to_string()),
        SettingsMessage::ObservabilityOtelEndpointChanged(String::new()),
        SettingsMessage::ObservabilityRuntimeTracePathChanged(String::new()),
    ] {
        let task = update(&mut app, message);
        assert!(iced_runtime::task::into_stream(task.collect()).is_some());
    }
}
