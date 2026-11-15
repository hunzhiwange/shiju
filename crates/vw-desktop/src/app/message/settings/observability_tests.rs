// 验证可观测性后端、追踪模式及相关文本字段更新。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
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
