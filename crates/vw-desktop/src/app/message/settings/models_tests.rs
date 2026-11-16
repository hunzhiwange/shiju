// 验证模型摘要对提供商连接状态、能力与排序的映射。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::state::{ModelSummary, ProviderModelsSummary};
use model_types::{
    ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost,
    ModelCostCache, ModelLimit, ProviderSource,
};
use serde_json::{Value, json};
use std::collections::HashMap;

fn app() -> App {
    App::new().0
}

fn model(provider_id: &str, id: &str, name: &str, status: &str) -> Model {
    Model {
        id: id.to_string(),
        provider_id: provider_id.to_string(),
        api: ApiInfo {
            protocols: None,
            id: id.to_string(),
            url: "https://example.test".to_string(),
            adapter: "openai-compatible".to_string(),
        },
        name: name.to_string(),
        family: None,
        capabilities: Capabilities {
            native_tool_search: false,
            catalog_execution: None,
            temperature: true,
            reasoning: false,
            reasoning_options: None,
            attachment: false,
            toolcall: true,
            input: CapabilityIO { text: true, audio: false, image: true, video: false, pdf: true },
            output: CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            interleaved: InterleavedCapability::Field { field: "content".to_string() },
        },
        cost: ModelCost {
            input: 1.0,
            output: 2.0,
            cache: ModelCostCache { read: 0.5, write: 0.75 },
            experimental_over_200k: None,
        },
        limit: ModelLimit { context: 200_000, input: Some(100_000), output: 20_000 },
        status: status.to_string(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: "2026-01-01".to_string(),
        variants: HashMap::new(),
    }
}

fn provider(
    id: &str,
    name: &str,
    source: ProviderSource,
    key: Option<&str>,
    models: Vec<Model>,
) -> Info {
    Info {
        id: id.to_string(),
        name: name.to_string(),
        source,
        env: Vec::new(),
        key: key.map(ToString::to_string),
        options: HashMap::new(),
        models: models.into_iter().map(|model| (model.id.clone(), model)).collect(),
    }
}

fn provider_summary_with_detail(detail: Value) -> ProviderModelsSummary {
    ProviderModelsSummary {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        models: vec![ModelSummary {
            id: "gpt-5".to_string(),
            name: "GPT-5".to_string(),
            enabled: true,
            toolcall: true,
            attachment: true,
            context_limit: 200_000,
            detail,
        }],
    }
}

fn provider_summary() -> ProviderModelsSummary {
    provider_summary_with_detail(json!({
        "id": "gpt-5",
        "name": "GPT-5",
        "status": "active",
        "release_date": "2026-01-01",
        "api": { "url": "https://api.example.com", "adapter": "openai" },
        "limit": { "context": 200000, "input": 100000, "output": 20000 },
        "capabilities": {
            "toolcall": true,
            "attachment": true,
            "reasoning": true,
            "temperature": true,
            "input": { "text": true, "audio": false, "image": true, "video": false, "pdf": true },
            "output": { "text": true, "audio": false, "image": false, "video": false, "pdf": false },
            "interleaved": { "field": "content" }
        },
        "cost": { "input": 1.0, "output": 2.0, "cache": { "read": 0.5, "write": 0.75 } },
        "headers": { "x-test-header": "configured" },
        "options": { "reasoning_effort": "high" },
        "variants": { "fast": { "tier": "low-latency" } }
    }))
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。

#[test]
fn model_toggle_updates_locally_and_starts_save_without_reloading() {
    let mut app = app();
    app.auto_model = false;
    app.model = "another-model".to_string();
    app.model_settings.save_error = Some(("stale".to_string()).into());
    app.model_settings.providers = vec![provider_summary()];
    let _ = update(
        &mut app,
        SettingsMessage::ModelDetailOpen("openai".to_string(), "gpt-5".to_string()),
    );

    let _ = update(
        &mut app,
        SettingsMessage::ModelToggle("openai".to_string(), "gpt-5".to_string(), false),
    );

    assert!(!app.model_settings.loading);
    assert!(app.model_settings.saving);
    assert!(app.model_settings.save_error.is_none());
    assert!(app.model_settings.detail_modal.is_none());
    assert!(!app.model_settings.providers[0].models[0].enabled);
    assert_eq!(app.model_settings.providers[0].models[0].detail["status"], "disabled");
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

    assert!(!app.model_settings.loading);
    assert!(app.model_settings.saving);
    assert!(app.model_settings.providers[0].models[0].enabled);
    assert_eq!(app.model_settings.providers[0].models[0].detail["status"], "active");
    assert!(!app.auto_model);
    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

#[test]
fn provider_changes_update_available_models_without_opening_model_settings() {
    let mut app = app();
    for message in [
        SettingsMessage::CustomProviderSaveDone
            as fn(super::super::types::ProvidersLoaded) -> SettingsMessage,
        SettingsMessage::ProviderConnectDone,
        SettingsMessage::ProvidersRefreshed,
        SettingsMessage::ProviderModelsSyncDone,
        SettingsMessage::ProviderDisconnectDone,
    ] {
        let providers = HashMap::from([(
            "custom".to_string(),
            provider(
                "custom",
                "Custom",
                ProviderSource::Api,
                None,
                vec![model("custom", "latest", "Latest", "active")],
            ),
        )]);
        let _ = super::super::providers::update(
            &mut app,
            message(Ok((providers, Vec::new(), false, Vec::new()))),
        );
        assert_eq!(app.model_settings.providers[0].models[0].id, "latest");
        assert!(app.model_settings.providers[0].models[0].enabled);

        let _ = super::super::providers::update(
            &mut app,
            message(Ok((HashMap::new(), Vec::new(), false, Vec::new()))),
        );
        assert!(app.model_settings.providers.is_empty());
    }
}
