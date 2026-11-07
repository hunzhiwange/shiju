use super::preset_catalog::{universal_preset, universal_presets};
use crate::providers::{universal, universal_sync, update};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        ProviderProtocol,
        create::ProviderSecretWrite,
        universal::{UniversalConfig, UniversalProviderWrite},
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

#[test]
fn catalog_presets_universal_matches_native_projection_for_custom_paths() {
    let preset = universal_preset("custom_gateway").unwrap();
    for base_url in ["https://gateway.example/tenant/Case/", "https://gateway.example/v1"] {
        let request = UniversalProviderWrite {
            config: UniversalConfig {
                name: preset.name.clone(),
                provider_type: preset.provider_type.clone(),
                apps: preset.default_apps.clone(),
                base_url: base_url.into(),
                models: preset.default_models.clone(),
                website_url: preset.website_url.clone(),
                notes: None,
                icon: preset.icon.clone(),
                icon_color: preset.icon_color.clone(),
                sort_index: None,
            },
            api_key: ProviderSecretWrite::Replace { value: "fixture-secret".into() },
        };
        let parent = universal::save("fixture", None, &request).unwrap();
        for (app, model) in [
            (AppId::Claude, "claude-sonnet-5"),
            (AppId::Codex, "gpt-5.6-sol"),
            (AppId::Gemini, "gemini-3.6-flash"),
        ] {
            let (protocol, endpoint) = preset.endpoint(app, base_url).unwrap();
            let expected =
                if app == AppId::Codex { base_url.trim_end_matches('/') } else { base_url };
            assert_eq!(endpoint, expected);
            let projected = universal_sync::project("fixture", &parent, app, None).unwrap();
            let profile = update::profile(&projected).unwrap();
            assert_eq!(profile.protocol, protocol);
            assert_eq!(profile.base_url, endpoint);
            assert_eq!(profile.model, model);
        }
    }
}
