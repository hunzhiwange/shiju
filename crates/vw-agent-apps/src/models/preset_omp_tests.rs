use super::{
    preset_catalog::{omp_preset, omp_presets, pi_preset},
    preset_pi::PiPresetProtocol::{OpenaiCompletions, OpenaiResponses},
};
use serde_json::{Value, json, to_value};
use vw_api_types::{agent_apps::AppId, app_providers::omp::OmpProfile};

#[test]
fn catalog_presets_omp_derives_omp_without_pi_mutation() {
    let key = "cc-switch-ppio";
    let endpoint = "https://api.ppio.com/openai/v1";
    let id = "deepseek/deepseek-v4.1-flash";
    let pi = &pi_preset(AppId::Pi, key).unwrap().settings_config;
    let pi_model = pi.model_at_deployment(endpoint, OpenaiCompletions, id).unwrap();
    let before = to_value(pi_model).unwrap();
    let omp_model = omp_preset(AppId::Omp, key).and_then(|preset| {
        preset.settings_config.model_at_deployment(endpoint, OpenaiCompletions, id)
    });
    assert_eq!(
        omp_model.map(|model| to_value(model).unwrap()),
        Some(json!({
            "id": id,
            "name": "DeepSeek V4.1 Flash",
            "reasoning": true,
            "input": ["text"],
            "contextWindow": 1_048_576,
            "maxTokens": 393_216,
            "compat": {
                "supportsStore": false,
                "supportsDeveloperRole": false,
                "maxTokensField": "max_tokens",
                "thinkingFormat": "zai",
                "reasoningContentField": "reasoning_content",
                "requiresReasoningContentForToolCalls": true,
                "requiresReasoningContentForAllAssistantTurns": true,
                "allowsSyntheticReasoningContentForToolCalls": false
            }
        }))
    );
    assert_eq!(before["compat"]["thinkingFormat"], "deepseek");
    assert_eq!(before["compat"]["requiresReasoningContentOnAssistantMessages"], true);
    assert_eq!(to_value(pi_model).unwrap(), before);
    assert_eq!(pi.name, "PPIO");
}

#[test]
fn catalog_presets_omp_keeps_exact_deployments_isolated() {
    let key = "cc-switch-tokenfactory";
    let endpoint = "https://api.tokenfactory.cn/v1";
    let go_endpoint = "https://opencode.ai/zen/go/v1";
    let id = "glm-5.3";
    let plan = omp_preset(AppId::Omp, key).map(|preset| &preset.settings_config);
    assert_eq!(
        plan.and_then(|config| config.model_at_deployment(endpoint, OpenaiCompletions, id))
            .map(|model| model.capabilities.max_tokens),
        Some(128_000)
    );
    let plan = plan.unwrap();
    let go = &omp_preset(AppId::Omp, "cc-switch-open-code-go").unwrap().settings_config;
    assert!(plan.model_at_deployment(go_endpoint, OpenaiCompletions, id).is_none());
    assert!(go.model_at_deployment(endpoint, OpenaiCompletions, id).is_none());
    assert!(plan.model_at_deployment(endpoint, OpenaiResponses, id).is_none());
    assert!(plan.model_at_deployment(endpoint, OpenaiCompletions, "GLM-5.3").is_none());
    assert_eq!(
        go.model_at_deployment(go_endpoint, OpenaiCompletions, id).unwrap().capabilities.max_tokens,
        131_072
    );
    assert!(omp_preset(AppId::Pi, key).is_none());
    assert!(pi_preset(AppId::Omp, key).is_none());
}

#[test]
fn catalog_presets_omp_keeps_only_native_compat_controls() {
    let config = |key| omp_preset(AppId::Omp, key).map(|preset| &preset.settings_config);
    let kimi = config("cc-switch-kimi").and_then(|config| {
        config.model_at_deployment("https://api.moonshot.cn/v1", OpenaiCompletions, "kimi-k3")
    });
    assert_eq!(
        kimi.map(|model| to_value(model).unwrap()["compat"].clone()),
        Some(json!({
            "supportsStore": false,
            "supportsDeveloperRole": false,
            "supportsReasoningEffort": true,
            "maxTokensField": "max_tokens",
            "supportsStrictMode": false,
            "thinkingFormat": "openai",
            "reasoningContentField": "reasoning_content",
            "requiresReasoningContentForToolCalls": true,
            "requiresReasoningContentForAllAssistantTurns": true,
            "allowsSyntheticReasoningContentForToolCalls": false
        }))
    );
    let tencent = config("cc-switch-tencent-token-plan").unwrap();
    let model = tencent
        .model_at_deployment(
            "https://api.lkeap.cloud.tencent.com/plan/v3",
            OpenaiCompletions,
            "deepseek-v4-flash-202605",
        )
        .unwrap();
    assert_eq!(
        to_value(model).unwrap()["compat"],
        json!({
            "supportsStore": false,
            "supportsDeveloperRole": false,
            "maxTokensField": "max_tokens",
            "thinkingFormat": "zai"
        })
    );
}

#[test]
fn catalog_presets_omp_lists_complete_native_catalog() {
    let rows: Vec<_> = omp_presets(AppId::Omp).collect();
    assert_eq!(rows.len(), 115);
    assert_eq!(rows[0].provider_key, "cc-switch-kimi");
    assert_eq!(rows.iter().map(|row| row.settings_config.models.len()).sum::<usize>(), 416);
    for row in rows {
        let mut declaration = to_value(&row.settings_config).unwrap();
        assert!(declaration.get("name").is_none(), "{}", row.provider_key);
        for model in declaration["models"].as_array().unwrap() {
            assert!(model.get("thinkingLevelMap").is_none(), "{}", row.provider_key);
            if let Some(compat) = model.get("compat").and_then(Value::as_object) {
                assert!(!compat.is_empty(), "{}", row.provider_key);
                for key in [
                    "forceAdaptiveThinking",
                    "requiresReasoningContentOnAssistantMessages",
                    "deferredToolsMode",
                ] {
                    assert!(!compat.contains_key(key), "{} {key}", row.provider_key);
                }
                assert_ne!(compat.get("thinkingFormat"), Some(&json!("deepseek")));
            }
        }
        declaration["auth"] = json!("none");
        crate::providers::omp::validate(&OmpProfile { name: row.name.into(), declaration })
            .unwrap_or_else(|error| {
                panic!("invalid native preset {}: {error:?}", row.provider_key)
            });
    }
}
