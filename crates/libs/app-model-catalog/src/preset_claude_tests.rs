use super::{
    preset_catalog::{claude_preset, claude_presets, universal_preset},
    preset_claude::{ClaudePreset, ClaudePresetCategory},
};
use std::collections::HashSet;
use vw_api_app_provider_profiles::desktop_mapping::DesktopMappingMode;
use vw_api_app_identity::{AppId};

#[test]
fn catalog_presets_claude_keeps_code_model_roles_and_context_defaults() {
    let Some(ClaudePreset::Code { models, max_context_tokens, auto_compact_window, .. }) =
        claude_preset(AppId::Claude, "Kimi For Coding")
    else {
        panic!("missing Claude Code Kimi preset");
    };
    assert_eq!(
        [
            models.model.as_deref(),
            models.haiku_model.as_deref(),
            models.sonnet_model.as_deref(),
            models.opus_model.as_deref(),
        ],
        [Some("kimi-for-coding"); 4],
    );
    assert_eq!((*max_context_tokens, *auto_compact_window), (Some(262_144), Some(262_144)));

    let Some(ClaudePreset::Desktop { routes, mode, .. }) =
        claude_preset(AppId::ClaudeDesktop, "Kimi For Coding")
    else {
        panic!("missing Claude Desktop Kimi preset");
    };
    assert_eq!(*mode, DesktopMappingMode::Proxy);
    assert_eq!(routes.len(), 3);
    assert_eq!(routes[0].target.model, "claude-sonnet-5");
    assert!(routes.iter().all(|route| route.target.model != "kimi-for-coding"));
}

#[test]
fn catalog_presets_claude_retains_desktop_route_order_and_per_route_windows() {
    let Some(ClaudePreset::Desktop { mode, routes, .. }) =
        claude_preset(AppId::ClaudeDesktop, "DigitalOcean")
    else {
        panic!("missing Claude Desktop DigitalOcean preset");
    };
    assert_eq!(*mode, DesktopMappingMode::Direct);
    assert_eq!(
        routes.iter().map(|route| route.route_id.as_str()).collect::<Vec<_>>(),
        ["claude-sonnet-5", "claude-opus-5", "claude-haiku-4-5"],
    );
    assert!(routes.iter().all(|route| route.route_id == route.target.model));
    assert!(routes[0].target.supports_1m);
    assert!(!routes[2].target.supports_1m);
}

#[test]
fn catalog_presets_claude_does_not_fall_back_to_another_apps_deployment() {
    for app in [AppId::Claude, AppId::ClaudeDesktop] {
        for retired in ["OpenCode Zen", "OpenCode Go", "PackyCode"] {
            assert!(claude_preset(app, retired).is_none());
        }
    }
    assert!(claude_preset(AppId::Claude, "Claude Desktop Official").is_none());
    assert!(claude_preset(AppId::ClaudeDesktop, "Claude Official").is_none());
    assert!(claude_preset(AppId::Claude, "opencode zen").is_none());
    assert!(claude_preset(AppId::Codex, "OpenCode Go").is_none());
    assert!(universal_preset("OpenCode Go").is_none());
}

#[test]
fn catalog_presets_claude_leaves_official_deployments_without_inferred_defaults() {
    let Some(ClaudePreset::Code { provider, models, .. }) =
        claude_preset(AppId::Claude, "Claude Official")
    else {
        panic!("missing official Claude Code preset");
    };
    assert_eq!(provider.category, ClaudePresetCategory::Official);
    assert!(provider.base_url.is_none());
    assert_eq!(models, &Default::default());

    let Some(ClaudePreset::Desktop { provider, routes, .. }) =
        claude_preset(AppId::ClaudeDesktop, "Claude Desktop Official")
    else {
        panic!("missing official Claude Desktop preset");
    };
    assert_eq!(provider.category, ClaudePresetCategory::Official);
    assert!(provider.base_url.is_none());
    assert!(routes.is_empty());
}

#[test]
fn catalog_presets_claude_lists_every_source_row_without_duplicate_names_or_routes() {
    for (app, expected_count) in [(AppId::Claude, 56), (AppId::ClaudeDesktop, 50)] {
        let rows: Vec<_> = claude_presets(app).collect();
        assert_eq!(rows.len(), expected_count);
        let mut names = HashSet::new();
        for row in rows {
            assert_eq!(row.app_id(), app);
            assert!(names.insert(&row.provider().name), "{:?}: {}", app, row.provider().name);
            if let ClaudePreset::Desktop { routes, .. } = row {
                let mut ids = HashSet::new();
                assert!(routes.iter().all(|route| ids.insert(&route.route_id)));
            }
        }
    }
}
