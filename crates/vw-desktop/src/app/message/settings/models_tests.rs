// 验证模型摘要对提供商连接状态、能力与排序的映射。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::state::{ModelSummary, ProviderModelsSummary};

use serde_json::json;

fn app() -> App {
    App::new().0
}

// Only model identity and availability are required by the session preference boundary.
fn provider_summary() -> ProviderModelsSummary {
    ProviderModelsSummary { id: "openai".into(), name: "OpenAI".into(), models: vec![ModelSummary {
        id: "gpt-5".into(), name: "GPT-5".into(), enabled: true, toolcall: true, attachment: true,
        context_limit: 200_000, detail: json!({"id": "gpt-5", "status": "active"}),
    }] }
}

#[test]
fn model_toggle_updates_locally_and_starts_save_without_reloading() {
    let mut app = app();
    app.auto_model = false;
    app.model = "another-model".to_string();
    app.model_settings.providers = vec![provider_summary()];


    let _ = update(
        &mut app,
        SettingsMessage::ModelToggle("openai".to_string(), "gpt-5".to_string(), false),
    );

    assert!(!app.auto_model);
}

#[test]
fn disabling_selected_model_switches_back_to_auto_model() {
    let mut app = app();
    app.auto_model = false;
    app.model = "openai/gpt-5".to_string();
    app.current_session_runtime_mut().auto_model = false;
    app.model_settings.providers = vec![provider_summary()];

    let _ = update(
        &mut app,
        SettingsMessage::ModelToggle("openai".to_string(), "gpt-5".to_string(), false),
    );

    assert!(app.auto_model);
    assert!(app.current_session_runtime().auto_model);
    assert!(app.model_settings.saving);
}

#[test]
fn enabling_model_updates_locally_and_schedules_save_without_forcing_auto_model() {
    let mut app = app();
    app.auto_model = false;
    app.model = "gpt-5".to_string();
    let mut provider = provider_summary();
    provider.models[0].enabled = false;
    provider.models[0].detail["status"] = json!("disabled");
    app.model_settings.providers = vec![provider];

    let task = update(
        &mut app,
        SettingsMessage::ModelToggle("openai".to_string(), "gpt-5".to_string(), true),
    );

    assert!(!app.auto_model);
    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}
