use crate::models::{ModelMessage, ModelStatusChange};
use crate::state::{ModelSummary, ProviderModelsSummary};
use serde_json::{Value, json};
fn update(
    state: &mut crate::state::ModelSettingsState,
    message: ModelMessage,
) -> iced::Task<ModelMessage> {
    let effect = crate::models::update(state, message);
    crate::model_update::persist_status_effect(
        state,
        effect,
        gateway_endpoint::GatewayEndpoint::default(),
        true,
        "",
    )
    .map(|(_, task)| task)
    .unwrap_or_else(iced::Task::none)
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

#[test]
fn model_toggle_updates_locally_and_starts_save_without_reloading() {
    let mut state = crate::state::ModelSettingsState::default();
    state.save_error = Some(("stale".to_string()).into());
    state.providers = vec![provider_summary()];
    let _ = update(
        &mut state,
        ModelMessage::ModelDetailOpen("openai".to_string(), "gpt-5".to_string()),
    );

    let _ = update(
        &mut state,
        ModelMessage::ModelToggle("openai".to_string(), "gpt-5".to_string(), false),
    );

    assert!(!state.loading);
    assert!(state.saving);
    assert!(state.save_error.is_none());
    assert!(state.detail_modal.is_none());
    assert!(!state.providers[0].models[0].enabled);
    assert_eq!(state.providers[0].models[0].detail["status"], "disabled");
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn enabling_model_updates_locally_and_schedules_save_without_forcing_auto_model() {
    let mut state = crate::state::ModelSettingsState::default();
    let mut provider = provider_summary();
    provider.models[0].enabled = false;
    provider.models[0].detail["status"] = json!("disabled");
    state.providers = vec![provider];

    let task = update(
        &mut state,
        ModelMessage::ModelToggle("openai".to_string(), "gpt-5".to_string(), true),
    );

    assert!(!state.loading);
    assert!(state.saving);
    assert!(state.providers[0].models[0].enabled);
    assert_eq!(state.providers[0].models[0].detail["status"], "active");
    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

#[test]
fn provider_set_all_only_changes_models_in_selected_group() {
    let mut state = crate::state::ModelSettingsState::default();
    let mut anthropic = provider_summary();
    anthropic.id = "anthropic".to_string();
    anthropic.name = "Anthropic".to_string();
    anthropic.models[0].id = "claude-sonnet".to_string();
    anthropic.models[0].name = "Claude Sonnet".to_string();
    state.providers = vec![provider_summary(), anthropic];

    let task = update(&mut state, ModelMessage::ProviderModelsSetAll("openai".to_string(), false));

    assert!(state.saving);
    assert!(!state.providers[0].models[0].enabled);
    assert!(state.providers[1].models[0].enabled);
    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

#[test]
fn provider_reset_optimistically_disables_only_selected_group() {
    let mut state = crate::state::ModelSettingsState::default();
    let mut anthropic = provider_summary();
    anthropic.id = "anthropic".to_string();
    anthropic.models[0].id = "claude-sonnet".to_string();
    state.providers = vec![provider_summary(), anthropic];

    let task = update(&mut state, ModelMessage::ProviderModelsReset("openai".to_string()));

    assert!(state.saving);
    assert!(!state.providers[0].models[0].enabled);
    assert!(state.providers[1].models[0].enabled);
    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

#[test]
fn models_set_all_updates_every_connected_model_with_one_save() {
    let mut state = crate::state::ModelSettingsState::default();
    let mut provider = provider_summary();
    provider.models.push(ModelSummary {
        id: "gpt-4.1".to_string(),
        name: "GPT-4.1".to_string(),
        enabled: false,
        toolcall: true,
        attachment: false,
        context_limit: 128_000,
        detail: json!({ "id": "gpt-4.1", "status": "disabled" }),
    });
    state.providers = vec![provider];

    let task = update(&mut state, ModelMessage::ModelsSetAll(false));

    assert!(state.saving);
    assert!(state.providers[0].models.iter().all(|model| !model.enabled));
    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn model_status_save_error_rolls_back_local_changes() {
    let mut state = crate::state::ModelSettingsState::default();
    state.providers = vec![provider_summary()];
    let change = ModelStatusChange {
        provider_id: "openai".to_string(),
        model_id: "gpt-5".to_string(),
        previous_enabled: true,
        enabled: false,
    };

    let _ = update(
        &mut state,
        ModelMessage::ModelToggle(
            change.provider_id.clone(),
            change.model_id.clone(),
            change.enabled,
        ),
    );
    let _ = update(
        &mut state,
        ModelMessage::ModelStatusesSaved(vec![change], Err("save failed".to_string().into())),
    );

    assert!(!state.saving);
    assert!(state.providers[0].models[0].enabled);
    assert_eq!(state.providers[0].models[0].detail["status"], "active");
    assert_eq!(state.save_error.as_deref(), Some("save failed"));
}

#[test]
fn model_status_save_success_keeps_optimistic_state() {
    let mut state = crate::state::ModelSettingsState::default();
    state.providers = vec![provider_summary()];
    let change = ModelStatusChange {
        provider_id: "openai".to_string(),
        model_id: "gpt-5".to_string(),
        previous_enabled: true,
        enabled: false,
    };

    let _ = update(
        &mut state,
        ModelMessage::ModelToggle(
            change.provider_id.clone(),
            change.model_id.clone(),
            change.enabled,
        ),
    );
    let _ = update(&mut state, ModelMessage::ModelStatusesSaved(vec![change], Ok(())));

    assert!(!state.saving);
    assert!(!state.providers[0].models[0].enabled);
    assert!(state.save_error.is_none());
}

#[test]
fn invalid_routes_block_persistence_and_valid_routes_clear_feedback() {
    let mut state = crate::state::ModelRoutesSettingsState::default();
    state.routes.push(crate::state::ModelRoute {
        pattern: "chat".into(),
        provider: "".into(),
        model: "fixture".into(),
        priority_input: "7".into(),
    });
    assert!(crate::model_routes::prepare_routes(&mut state).is_none());
    assert!(state.persistence.route_invalid);
    assert!(state.save_error.is_some());
    state.routes[0].provider = "provider".into();
    let projection =
        crate::model_routes::prepare_routes(&mut state).expect("valid route projection");
    assert!(!state.persistence.route_invalid);
    assert!(state.save_error.is_none());
    assert_eq!(projection.rules[0].priority, 7);
    assert_eq!(projection.patch["model_routes"][0]["provider"], "provider");
}
