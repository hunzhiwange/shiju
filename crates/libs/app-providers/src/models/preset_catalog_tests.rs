use super::preset_catalog::universal_preset;
use crate::providers::{universal, universal_sync, update};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        ProviderProtocol,
        create::ProviderSecretWrite,
        universal::{UniversalConfig, UniversalProviderWrite},
use vw_api_app_identity::{AppId};
    },
};

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
