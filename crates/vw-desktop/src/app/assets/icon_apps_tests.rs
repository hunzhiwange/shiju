use super::{Icon, register_icons};
use iced::widget::svg;
use std::collections::HashMap;

const REGISTERED_APP_ICONS: &[Icon] = &[
    Icon::AppTrae,
    Icon::AppGitHubCopilot,
    Icon::AppVSCode,
    Icon::AppCursor,
    Icon::AppAuggie,
    Icon::AppClaudeCode,
    Icon::AppCodex,
    Icon::AppFactoryDroid,
    Icon::AppGeminiCli,
    Icon::AppKiloCode,
    Icon::AppKimiCode,
    Icon::AppOpenClaw,
    Icon::AppOpenCode,
    Icon::AppPi,
    Icon::AppQoder,
    Icon::AppQwenCode,
    Icon::AppZed,
    Icon::AppSublimeText,
    Icon::AppGhostty,
    Icon::AppITerm2,
    Icon::AppPowerShell,
    Icon::AppAndroidStudio,
    Icon::AppAntigravity,
    Icon::AppFileExplorer,
    Icon::AppKiro,
    Icon::DotsThreeVertical,
];

#[test]
fn task_636_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("icon_apps_tests.rs"));
}

#[test]
fn register_icons_inserts_every_app_icon_and_preserves_existing_entries() {
    let mut icons = HashMap::new();
    icons.insert(Icon::AppFinder, svg::Handle::from_memory(Vec::new()));

    register_icons(&mut icons);

    for icon in REGISTERED_APP_ICONS {
        assert!(icons.contains_key(icon), "missing registered icon {icon:?}");
    }
    assert!(icons.contains_key(&Icon::AppFinder));
    assert_eq!(icons.len(), REGISTERED_APP_ICONS.len() + 1);
}

#[test]
fn register_icons_can_be_called_again_without_changing_key_set() {
    let mut icons = HashMap::new();

    register_icons(&mut icons);
    register_icons(&mut icons);

    assert_eq!(icons.len(), REGISTERED_APP_ICONS.len());
}
