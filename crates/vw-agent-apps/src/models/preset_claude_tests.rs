use super::{
    preset_catalog::{claude_preset, claude_presets, universal_preset},
    preset_claude::{ClaudePreset, ClaudePresetApiFormat, ClaudePresetCategory},
};
use std::collections::HashSet;
use vw_api_types::{agent_apps::AppId, app_providers::desktop_mapping::DesktopMappingMode};

#[test]
fn catalog_presets_claude_separates_claude_apps() {
    let code = claude_preset(AppId::Claude, "OpenCode Go");
    let desktop = claude_preset(AppId::ClaudeDesktop, "OpenCode Go");
    assert_eq!(
        (code.map(ClaudePreset::app_id), desktop.map(ClaudePreset::app_id)),
        (Some(AppId::Claude), Some(AppId::ClaudeDesktop)),
        "same-name presets must remain queryable under their own application",
    );

    let Some(ClaudePreset::Code { provider, models, .. }) = code else {
        panic!("expected Claude Code model roles");
    };
    assert_eq!(provider.base_url.as_deref(), Some("https://opencode.ai/zen/go"));
    assert_eq!(provider.api_format, ClaudePresetApiFormat::Anthropic);
    assert!(provider.api_key_header);
    assert_eq!(models.model.as_deref(), Some("deepseek-v4.1-flash[1M]"));
    assert_eq!(models.haiku_model, models.model);

    let Some(ClaudePreset::Desktop { provider, routes, .. }) = desktop else {
        panic!("expected Claude Desktop routes");
    };
    assert_eq!(provider.base_url.as_deref(), Some("https://opencode.ai/inference/go/anthropic"),);
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].route_id, "claude-sonnet-5");
    assert_eq!(routes[0].target.model, "deepseek-v4.1-flash");
    assert_eq!(routes[0].target.label_override.as_deref(), Some("deepseek-v4.1-flash"));
    assert!(routes[0].target.supports_1m);
}

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
        claude_preset(AppId::ClaudeDesktop, "PackyCode")
    else {
        panic!("missing Claude Desktop PackyCode preset");
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
    let code = claude_preset(AppId::Claude, "OpenCode Zen").expect("missing Code-only deployment");
    assert_eq!(code.provider().base_url.as_deref(), Some("https://opencode.ai/zen"));
    assert!(claude_preset(AppId::ClaudeDesktop, "OpenCode Zen").is_none());
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
    for (app, expected_count) in [(AppId::Claude, 139), (AppId::ClaudeDesktop, 129)] {
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
