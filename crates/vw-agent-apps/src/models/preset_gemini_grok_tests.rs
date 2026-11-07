use super::{
    preset_catalog::{gemini_grok_preset, gemini_grok_presets},
    preset_gemini_grok::GeminiGrokPresetCategory,
    preset_search::PresetPlan,
};
use std::collections::HashSet;
use vw_api_types::{agent_apps::AppId, app_providers::ProviderProtocol};

#[test]
fn catalog_presets_gemini_grok_preserves_native_ids() {
    for (app, name, protocol, model) in [
        (AppId::Gemini, "OpenModel", ProviderProtocol::Gemini, "gemini-4-argon"),
        (AppId::Gemini, "CherryIN", ProviderProtocol::Gemini, "google/gemini-3.8-flash"),
        (AppId::GrokBuild, "OpenRouter", ProviderProtocol::OpenaiResponses, "x-ai/grok-4.7"),
        (AppId::GrokBuild, "TheRouter", ProviderProtocol::OpenaiResponses, "xai/grok-4.7"),
    ] {
        assert_eq!(
            gemini_grok_preset(app, name).map(|preset| (preset.protocol, preset.model.as_deref())),
            Some((Some(protocol), Some(model))),
            "{app:?} {name} must retain its deployment protocol and exact model ID",
        );
    }
}

#[test]
fn catalog_presets_gemini_grok_isolates_gemini_deployments() {
    let preset = gemini_grok_preset(AppId::Gemini, "TeamoRouter").expect("Gemini TeamoRouter");
    assert_eq!(preset.base_url.as_deref(), Some("https://api.teamorouter.cn"));
    assert_eq!(
        preset.endpoint_candidates,
        ["https://api.teamorouter.cn", "https://api.teamorouter.com"]
    );
    assert!(preset.bearer_api_key);
    assert!(gemini_grok_preset(AppId::Gemini, "xAI (Grok)").is_none());
    assert!(gemini_grok_preset(AppId::Gemini, "Grok Official").is_none());
}

#[test]
fn catalog_presets_gemini_grok_isolates_grok_deployments() {
    let preset = gemini_grok_preset(AppId::GrokBuild, "TeamoRouter").expect("Grok TeamoRouter");
    assert_eq!(preset.base_url.as_deref(), Some("https://api.teamorouter.cn/v1"));
    assert_eq!(
        preset.endpoint_candidates,
        ["https://api.teamorouter.cn/v1", "https://api.teamorouter.com/v1"]
    );
    assert!(!preset.bearer_api_key);
    assert!(gemini_grok_preset(AppId::Gemini, "OspreyAI").is_some());
    assert!(gemini_grok_preset(AppId::GrokBuild, "OspreyAI").is_none());
    assert!(gemini_grok_preset(AppId::GrokBuild, "Google Official").is_none());
    assert!(gemini_grok_preset(AppId::Codex, "TeamoRouter").is_none());

    let payg = gemini_grok_preset(AppId::GrokBuild, "Compshare").unwrap();
    let coding = gemini_grok_preset(AppId::GrokBuild, "Compshare Coding Plan").unwrap();
    assert_eq!(payg.family.as_deref(), Some("compshare"));
    assert_eq!(coding.family, payg.family);
    assert_eq!(
        (payg.plan_key, coding.plan_key),
        (Some(PresetPlan::Payg), Some(PresetPlan::CodingPlan))
    );
    assert_eq!(payg.base_url.as_deref(), Some("https://api.modelverse.cn/v1"));
    assert_eq!(coding.base_url.as_deref(), Some("https://cp.compshare.cn/v1"));
}

#[test]
fn catalog_presets_gemini_grok_keeps_official_account_seeds_unconfigured() {
    for (app, name, id, auth_key) in [
        (AppId::Gemini, "Google Official", "gemini-official", Some("google-official")),
        (AppId::GrokBuild, "Grok Official", "grokbuild-official", None),
    ] {
        let preset = gemini_grok_preset(app, name).expect("missing official account preset");
        assert_eq!(preset.category, Some(GeminiGrokPresetCategory::Official));
        assert_eq!(preset.official_provider_id.as_deref(), Some(id));
        assert_eq!(preset.official_auth_key.as_deref(), auth_key);
        assert_eq!((preset.protocol, &preset.base_url, &preset.model), (None, &None, &None));
        assert!(preset.endpoint_candidates.is_empty());
        assert!(!preset.bearer_api_key);
    }

    let api_key = gemini_grok_preset(AppId::GrokBuild, "xAI (Grok)").unwrap();
    assert!(api_key.official_provider_id.is_none());
    assert_eq!(api_key.base_url.as_deref(), Some("https://api.x.ai/v1"));
}

#[test]
fn catalog_presets_gemini_grok_lists_all_source_rows() {
    for (app, first, count, protocol) in [
        (AppId::Gemini, "Google Official", 50, ProviderProtocol::Gemini),
        (AppId::GrokBuild, "Grok Official", 55, ProviderProtocol::OpenaiResponses),
    ] {
        let rows = gemini_grok_presets(app).collect::<Vec<_>>();
        assert_eq!(rows.len(), count, "{app:?}");
        assert_eq!(rows[0].name, first);
        assert_eq!(rows[1].name, "PackyCode");
        let mut names = HashSet::new();
        for preset in rows {
            assert_eq!(preset.app_id, app);
            assert!(names.insert(&preset.name), "{app:?}: {}", preset.name);
            if preset.name != first {
                assert_eq!(preset.protocol, Some(protocol));
                assert!(preset.model.as_ref().is_some_and(|model| !model.is_empty()));
                assert!(preset.base_url.as_ref().is_some_and(|url| url.starts_with("https://")));
                assert!(preset.official_provider_id.is_none());
            }
        }
    }
    for excluded in ["OpenAI Official", "OpenCode Go", "SiliconFlow"] {
        assert!(gemini_grok_preset(AppId::GrokBuild, excluded).is_none());
    }
}
