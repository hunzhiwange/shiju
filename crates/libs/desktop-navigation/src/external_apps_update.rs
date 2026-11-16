use std::collections::HashMap;
use crate::components::top_bar::types::{ExternalOpenApp, RuntimePlatform};

pub fn parse_external_open_app(value: &str) -> Option<ExternalOpenApp> {
    match value {
        "finder" => Some(ExternalOpenApp::Finder),
        "vscode" => Some(ExternalOpenApp::VSCode),
        "cursor" => Some(ExternalOpenApp::Cursor),
        "trae" => Some(ExternalOpenApp::Trae),
        "windsurf" => Some(ExternalOpenApp::Windsurf),
        "kiro" => Some(ExternalOpenApp::Kiro),
        "zed" => Some(ExternalOpenApp::Zed),
        "textmate" => Some(ExternalOpenApp::TextMate),
        "antigravity" => Some(ExternalOpenApp::Antigravity),
        "terminal" => Some(ExternalOpenApp::Terminal),
        "iterm2" => Some(ExternalOpenApp::ITerm2),
        "ghostty" => Some(ExternalOpenApp::Ghostty),
        "xcode" => Some(ExternalOpenApp::Xcode),
        "android-studio" => Some(ExternalOpenApp::AndroidStudio),
        "powershell" => Some(ExternalOpenApp::PowerShell),
        "sublime-text" => Some(ExternalOpenApp::SublimeText),
        _ => None,
    }
}

pub fn external_open_priority(
    platform: Option<RuntimePlatform>,
) -> &'static [ExternalOpenApp] {
    if matches!(platform, Some(RuntimePlatform::MacOs)) {
        &[
            ExternalOpenApp::Trae,
            ExternalOpenApp::Windsurf,
            ExternalOpenApp::Kiro,
            ExternalOpenApp::Cursor,
            ExternalOpenApp::VSCode,
            ExternalOpenApp::Zed,
            ExternalOpenApp::Xcode,
            ExternalOpenApp::AndroidStudio,
            ExternalOpenApp::SublimeText,
            ExternalOpenApp::TextMate,
            ExternalOpenApp::Antigravity,
            ExternalOpenApp::Ghostty,
            ExternalOpenApp::ITerm2,
            ExternalOpenApp::Terminal,
            ExternalOpenApp::Finder,
        ]
    } else if matches!(platform, Some(RuntimePlatform::Windows)) {
        &[
            ExternalOpenApp::Trae,
            ExternalOpenApp::Windsurf,
            ExternalOpenApp::Kiro,
            ExternalOpenApp::Cursor,
            ExternalOpenApp::VSCode,
            ExternalOpenApp::Zed,
            ExternalOpenApp::SublimeText,
            ExternalOpenApp::PowerShell,
            ExternalOpenApp::Finder,
        ]
    } else {
        &[
            ExternalOpenApp::Trae,
            ExternalOpenApp::Windsurf,
            ExternalOpenApp::Kiro,
            ExternalOpenApp::Cursor,
            ExternalOpenApp::VSCode,
            ExternalOpenApp::Zed,
            ExternalOpenApp::SublimeText,
            ExternalOpenApp::Finder,
        ]
    }
}

pub fn apply_external_apps_state(
    open_external_platform: &mut Option<RuntimePlatform>,
    open_external_exists: &mut HashMap<ExternalOpenApp, bool>,
    open_external_app: &mut ExternalOpenApp,
    platform: Option<RuntimePlatform>,
    apps: Vec<(String, bool)>,
) {
    let mut available_apps: HashMap<ExternalOpenApp, bool> =
        HashMap::new();
    for (id, available) in apps {
        if let Some(target) = parse_external_open_app(&id) {
            available_apps.insert(target, available);
        }
    }
    available_apps.insert(ExternalOpenApp::Finder, true);
    *open_external_platform = platform.or(*open_external_platform);
    *open_external_exists = available_apps;

    if open_external_exists.get(&*open_external_app).copied().unwrap_or(false) {
        return;
    }
    if let Some(target) = external_open_priority(*open_external_platform)
        .iter()
        .copied()
        .find(|candidate| open_external_exists.get(candidate).copied().unwrap_or(false))
    {
        *open_external_app = target;
    } else {
        *open_external_app = ExternalOpenApp::Finder;
    }
}


#[cfg(test)]
#[path = "external_apps_update_tests.rs"]
mod tests;

pub fn can_open_external(exists: &HashMap<ExternalOpenApp, bool>, target: ExternalOpenApp) -> bool {
    exists.get(&target).copied().unwrap_or(false)
}
