use super::{
    preset_catalog::{pi_model_capabilities, pi_preset, pi_presets},
    preset_pi::{PiModelInput, PiPreset, PiPresetProtocol},
    preset_search::PresetPlan,
};
use serde_json::json;
use std::collections::{BTreeMap, HashSet};
use vw_api_types::agent_apps::AppId;

fn pi(key: &str) -> &'static PiPreset {
    pi_preset(AppId::Pi, key).unwrap_or_else(|| panic!("missing Pi preset {key}"))
}

#[test]
fn catalog_presets_pi_materializes_local_override() {
    let model = pi_preset(AppId::Pi, "cc-switch-tokenfactory").and_then(|preset| {
        preset.settings_config.model_at_deployment(
            "https://api.tokenfactory.cn/v1",
            PiPresetProtocol::OpenaiCompletions,
            "deepseek-v4.1-flash",
        )
    });
    assert_eq!(
        model.map(|model| serde_json::to_value(model).unwrap()),
        Some(json!({
            "id": "deepseek-v4.1-flash",
            "name": "DeepSeek V4.1 Flash",
            "reasoning": true,
            "input": ["text", "image"],
            "contextWindow": 1_048_576,
            "maxTokens": 384_000
        }))
    );
    let base = pi_model_capabilities("deepseek/deepseek-v4-flash").unwrap();
    assert_eq!(base.input, [PiModelInput::Text]);
    assert_eq!(base.context_window, 1_000_000);
}

#[test]
fn catalog_presets_pi_keeps_exact_deployments_isolated() {
    let plan = &pi("cc-switch-tokenfactory").settings_config;
    let protocol = PiPresetProtocol::OpenaiCompletions;
    let id = "glm-5.3";
    let endpoint = "https://api.tokenfactory.cn/v1";
    assert_eq!(
        plan.model_at_deployment(endpoint, protocol, id).unwrap().capabilities.max_tokens,
        128_000
    );
    assert!(plan.model_at_deployment(endpoint, PiPresetProtocol::OpenaiResponses, id).is_none());
    let go = &pi("cc-switch-open-code-go").settings_config;
    assert!(plan.model_at_deployment(&go.base_url, protocol, id).is_none());
    assert!(go.model_at_deployment(endpoint, protocol, id).is_none());
    assert_eq!(
        go.model_at_deployment(&go.base_url, protocol, id).unwrap().capabilities.max_tokens,
        131_072
    );
    assert!(pi_preset(AppId::OpenCode, "cc-switch-tokenfactory").is_none());
}

#[test]
fn catalog_presets_pi_retains_native_protocols_and_plan_roots() {
    use PiPresetProtocol::*;
    assert_eq!(pi("cc-switch-right-code").settings_config.api, OpenaiResponses);
    assert_eq!(pi("cc-switch-aws-bedrock").settings_config.api, BedrockConverseStream);
    let plans: BTreeMap<_, _> = pi_presets(AppId::Pi)
        .filter(|row| row.family.as_deref() == Some("volcengine"))
        .map(|row| (row.settings_config.base_url.as_str(), row.plan_key.unwrap()))
        .collect();
    assert_eq!(
        plans,
        BTreeMap::from([
            ("https://ark.cn-beijing.volces.com/api/plan/v3", PresetPlan::AgentPlan),
            ("https://ark.cn-beijing.volces.com/api/coding/v3", PresetPlan::CodingPlan),
            ("https://ark.cn-beijing.volces.com/api/v3", PresetPlan::Payg),
        ])
    );
}

#[test]
fn catalog_presets_pi_keeps_reviewed_compat_and_capabilities() {
    let kimi = &pi("cc-switch-kimi").settings_config;
    let model = kimi
        .model_at_deployment(
            "https://api.moonshot.cn/v1",
            PiPresetProtocol::OpenaiCompletions,
            "kimi-k3",
        )
        .unwrap();
    let fields = serde_json::to_value(model).unwrap();
    assert_eq!(
        fields["compat"],
        json!({
            "supportsStore": false,
            "supportsDeveloperRole": false,
            "supportsReasoningEffort": true,
            "maxTokensField": "max_tokens",
            "supportsStrictMode": false,
            "thinkingFormat": "openai",
            "requiresReasoningContentOnAssistantMessages": true,
            "deferredToolsMode": "kimi"
        })
    );
    let deepseek = &pi("cc-switch-deep-seek").settings_config.models;
    assert_eq!(deepseek.len(), 1);
    assert_eq!(deepseek[0].id, "deepseek-flash");
    assert_eq!(deepseek[0].capabilities.input, [PiModelInput::Text, PiModelInput::Image]);
}

#[test]
fn catalog_presets_pi_lists_complete_managed_catalog() {
    let rows: Vec<_> = pi_presets(AppId::Pi).collect();
    assert_eq!(rows.len(), 115);
    assert_eq!(rows[0].provider_key, "cc-switch-kimi");
    assert_eq!(rows.iter().map(|row| row.settings_config.models.len()).sum::<usize>(), 416);
    let mut keys = HashSet::new();
    for row in rows {
        assert!(row.provider_key.starts_with("cc-switch-"));
        assert!(keys.insert(&row.provider_key));
        assert!(!row.settings_config.models.is_empty());
        let mut ids = HashSet::new();
        for model in &row.settings_config.models {
            assert!(ids.insert(&model.id));
            assert!(!model.id.is_empty());
            assert!(!model.capabilities.name.is_empty());
            assert!(!model.capabilities.input.is_empty());
            assert!(model.capabilities.context_window > 0);
            assert!(model.capabilities.max_tokens > 0);
        }
    }
    assert!(pi_presets(AppId::Omp).next().is_none());
}
