use super::{
    preset_catalog::{kimi_dsh_preset, kimi_dsh_presets},
    preset_kimi_dsh::{
        DshCompatValue, DshPresetCatalog, DshPresetProtocol, DshReasoningEfforts, KimiDshCatalog,
        KimiPresetCatalog, KimiPresetProtocol,
    },
};
use std::collections::HashSet;
use vw_api_types::agent_apps::AppId::{self, Dsh, Kimi};

fn kimi(key: &str) -> &'static KimiPresetCatalog {
    let KimiDshCatalog::Kimi(catalog) = &kimi_dsh_preset(Kimi, key).expect(key).catalog else {
        panic!("unexpected catalog for {key}");
    };
    catalog
}

fn dsh(key: &str) -> &'static DshPresetCatalog {
    let KimiDshCatalog::Dsh(catalog) = &kimi_dsh_preset(Dsh, key).expect(key).catalog else {
        panic!("unexpected catalog for {key}");
    };
    catalog
}

#[test]
fn catalog_presets_kimi_dsh_keeps_account_catalog_empty() {
    let accounts = [(Kimi, "kimi-official-account"), (Dsh, "dsh-official-account")];
    assert_eq!(
        accounts.map(|(app, key)| {
            kimi_dsh_preset(app, key).map(|row| match &row.catalog {
                KimiDshCatalog::Kimi(catalog) => catalog.models.len(),
                KimiDshCatalog::Dsh(catalog) => catalog.models.len(),
            })
        }),
        [Some(0), Some(0)]
    );
    for (app, key) in accounts {
        let row = kimi_dsh_presets(app).next().unwrap();
        assert_eq!(row.provider_key, key);
        assert_eq!(row.category, "official");
        assert!(row.family.is_none());
    }
    assert!(kimi("kimi-official-account").provider.base_url.is_empty());
    assert!(dsh("dsh-official-account").options.base_url.is_empty());
    assert!(kimi_dsh_preset(Kimi, "dsh-official-account").is_none());
    assert!(kimi_dsh_preset(Dsh, "kimi-official-account").is_none());
}

#[test]
fn catalog_presets_kimi_dsh_keeps_kimi_native_capabilities_and_deployments() {
    let preset = kimi("cc-switch-kimi");
    let endpoint = "https://api.moonshot.cn/v1";
    let alias = "cc-switch-kimi/kimi-k3.1-pro";
    let model = preset.model_at_endpoint(endpoint, alias).unwrap();
    assert_eq!(preset.provider.provider_type, KimiPresetProtocol::Kimi);
    assert_eq!(model.model, "kimi-k3.1-pro");
    assert_eq!(model.max_context_size, 1_048_576);
    assert_eq!(
        model.capabilities,
        ["tool_use", "image_in", "thinking", "always_thinking", "dynamically_loaded_tools"]
    );
    assert_eq!(model.support_efforts.as_deref().unwrap(), ["low", "high", "max"]);
    assert_eq!(model.default_effort.as_deref(), Some("high"));
    assert!(preset.model_at_endpoint("https://api.moonshot.ai/v1", alias).is_none());
    assert!(preset.model_at_endpoint(endpoint, "cc-switch-kimi-global/kimi-k3.1-pro").is_none());
    assert!(preset.model_at_endpoint(endpoint, "kimi-k3.1-pro").is_none());
    assert!(
        kimi("cc-switch-kimi-for-coding")
            .model_at_endpoint(
                "https://api.kimi.com/coding/v1",
                "cc-switch-kimi-for-coding/kimi-for-coding"
            )
            .is_some()
    );
    let opus = kimi("cc-switch-packy-code")
        .model_at_endpoint("https://www.packyapi.ai", "cc-switch-packy-code/claude-opus-5-5")
        .unwrap();
    assert_eq!(opus.adaptive_thinking, Some(true));
    assert_eq!(opus.max_output_size, Some(128_000));
}

#[test]
fn catalog_presets_kimi_dsh_keeps_dsh_native_protocol_limits_and_compat() {
    let preset = dsh("cc-switch-kimi");
    assert_eq!(preset.api, DshPresetProtocol::OpenaiCompletions);
    let model = preset.model_at_endpoint("https://api.moonshot.cn/v1", "kimi-k3.1-pro").unwrap();
    assert_eq!((model.limit.context, model.limit.output), (1_048_576, 131_072));
    assert_eq!(model.modalities.input, ["text", "image"]);
    assert_eq!(model.modalities.output, ["text"]);
    assert_eq!(model.reasoning, Some(true));
    let compat = model.compat.as_ref().unwrap();
    assert!(!compat.contains_key("deferredToolsMode"));
    assert_eq!(compat["requiresReasoningContentOnAssistantMessages"], DshCompatValue::Flag(true));
    assert_eq!(compat["maxTokensField"], DshCompatValue::Dialect("max_tokens".into()));
    let DshReasoningEfforts::Levels(levels) = &model.reasoning_efforts else {
        panic!("Kimi Pro supports DSH reasoning levels");
    };
    assert_eq!(levels.keys().map(String::as_str).collect::<Vec<_>>(), ["high", "low", "max"]);
    assert!(!levels.contains_key("off"));
}

#[test]
fn catalog_presets_kimi_dsh_keeps_dsh_deployments_and_effort_defaults_separate() {
    let preset = dsh("cc-switch-kimi-for-coding");
    let id = "kimi-for-coding";
    let model = preset.model_at_endpoint("https://api.kimi.com/coding", id).unwrap();
    assert_eq!(preset.api, DshPresetProtocol::AnthropicMessages);
    assert!(preset.model_at_endpoint("https://api.kimi.com/coding/v1", id).is_none());
    let DshReasoningEfforts::Levels(levels) = &model.reasoning_efforts else {
        panic!("Kimi for Coding supports DSH reasoning levels");
    };
    assert_eq!(levels.get("off"), Some(&None));
    assert_eq!(levels.get("medium").and_then(Option::as_deref), Some("medium"));
    assert_eq!(levels.len(), 5);
    let agent = dsh("cc-switch-agentplan");
    let id = "ark-code-latest";
    let ark = agent.model_at_endpoint("https://ark.cn-beijing.volces.com/api/plan/v3", id).unwrap();
    assert_eq!(ark.reasoning_efforts, DshReasoningEfforts::Flag(false));
    assert!(
        agent.model_at_endpoint("https://ark.cn-beijing.volces.com/api/coding/v3", id).is_none()
    );
}

#[test]
fn catalog_presets_kimi_dsh_lists_source_rows_without_unsupported_transports() {
    for app in [Kimi, Dsh] {
        let rows: Vec<_> = kimi_dsh_presets(app).collect();
        assert_eq!(rows.len(), 115);
        assert_eq!(rows[1].name, "Kimi");
        assert_eq!(rows.last().unwrap().name, "AIComing");
        assert!(rows.iter().all(|row| row.app_id() == app && row.name != "AWS Bedrock"));
        assert_eq!(rows.iter().map(|row| &row.provider_key).collect::<HashSet<_>>().len(), 115);
        let total: usize = rows
            .iter()
            .map(|row| match &row.catalog {
                KimiDshCatalog::Kimi(catalog) => {
                    assert!(catalog.models.iter().all(|(key, model)| {
                        *key == format!("{}/{}", row.provider_key, model.model)
                    }));
                    catalog.models.len()
                }
                KimiDshCatalog::Dsh(catalog) => catalog.models.len(),
            })
            .sum();
        assert_eq!(total, 410);
    }
    assert!(kimi_dsh_presets(AppId::Pi).next().is_none());
}
