use super::{
    preset_catalog::{gemini_grok_preset, gemini_grok_presets},
    preset_gemini_grok::GeminiGrokPresetCategory,
};
use vw_api_app_identity::AppId;

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

}

#[test]
fn catalog_presets_gemini_grok_only_bundles_official_accounts() {
    for (app, name) in [(AppId::Gemini, "Google Official"), (AppId::GrokBuild, "Grok Official")] {
        let rows: Vec<_> = gemini_grok_presets(app).map(|row| row.name.as_str()).collect();
        assert_eq!(rows, [name]);
        for retired in ["PackyCode", "OpenRouter", "TeamoRouter", "xAI (Grok)"] {
            assert!(gemini_grok_preset(app, retired).is_none());
        }
    }
}
