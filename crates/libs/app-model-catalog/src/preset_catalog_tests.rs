use super::preset_catalog::{universal_preset, universal_presets};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        ProviderProtocol,
        universal_sync::UniversalSyncError,
    },
};

#[test]
fn catalog_presets_universal_loads_universal_endpoints() {
    assert_eq!(
        universal_presets().iter().map(|preset| preset.provider_type.as_str()).collect::<Vec<_>>(),
        ["newapi", "custom_gateway"]
    );

    for key in ["newapi", "custom_gateway"] {
        let preset = universal_preset(key).unwrap();
        assert!(
            preset.default_apps.claude && preset.default_apps.codex && preset.default_apps.gemini
        );
        let claude = preset.default_models.claude.as_ref().unwrap();
        assert_eq!(claude.model.as_deref(), Some("claude-sonnet-5"));
        assert_eq!(claude.haiku_model.as_deref(), Some("claude-haiku-4-5-20251001"));
        assert_eq!(claude.sonnet_model.as_deref(), Some("claude-sonnet-5"));
        assert_eq!(claude.opus_model.as_deref(), Some("claude-opus-5"));
        let codex = preset.default_models.codex.as_ref().unwrap();
        assert_eq!(codex.model.as_deref(), Some("gpt-5.6-sol"));
        assert_eq!(codex.reasoning_effort.as_deref(), Some("high"));
        let gemini = preset.default_models.gemini.as_ref().unwrap();
        assert_eq!(gemini.model.as_deref(), Some("gemini-3.6-flash"));

        for (app, protocol, endpoint) in [
            (AppId::Claude, ProviderProtocol::Anthropic, "https://gateway.example/"),
            (AppId::Codex, ProviderProtocol::OpenaiResponses, "https://gateway.example/v1"),
            (AppId::Gemini, ProviderProtocol::Gemini, "https://gateway.example/"),
        ] {
            assert_eq!(
                preset.endpoint(app, "https://gateway.example/").unwrap(),
                (protocol, endpoint.into())
            );
        }
    }
}

#[test]
fn catalog_presets_universal_keeps_application_presets_out_of_the_universal_catalog() {
    for key in ["NewAPI", "custom", "jiekou", "ppio"] {
        assert!(universal_preset(key).is_none(), "{key}");
    }
    let preset = universal_preset("newapi").unwrap();
    assert!(matches!(
        preset.endpoint(AppId::ClaudeDesktop, "https://gateway.example"),
        Err(UniversalSyncError::UnsupportedApp)
    ));
}

#[test]
fn catalog_presets_universal_requires_a_valid_deployment_endpoint() {
    let newapi = universal_preset("newapi").unwrap();
    let custom = universal_preset("custom_gateway").unwrap();
    assert_eq!(newapi.website_url.as_deref(), Some("https://www.newapi.pro"));
    assert!(!newapi.is_custom_template);
    assert!(custom.is_custom_template);
    assert!(custom.website_url.is_none());

    for preset in [newapi, custom] {
        for invalid in [
            "",
            "http://gateway.example",
            "https://user:password@gateway.example",
            "https://gateway.example?token=fixture",
        ] {
            assert!(matches!(
                preset.endpoint(AppId::Codex, invalid),
                Err(UniversalSyncError::InvalidProfile)
            ));
        }
    }
}
