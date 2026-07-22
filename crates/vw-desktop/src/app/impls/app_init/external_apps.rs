//! 组织桌面应用初始化阶段的 external_apps.rs 逻辑。
//! 本模块把启动输入、配置加载和初始状态装配拆开，便于定位启动失败路径。

use super::*;

/// 模块内可见函数，执行 resolve_external_apps 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn resolve_external_apps(
    cfg: &serde_json::Value,
) -> (Option<crate::app::state::RuntimePlatform>, HashMap<ExternalOpenApp, bool>, ExternalOpenApp) {
    let mut open_external_exists = HashMap::new();
    open_external_exists.insert(ExternalOpenApp::Finder, true);

    #[cfg(target_os = "macos")]
    {
        let app_exists = |bundle_path: &str| std::path::Path::new(bundle_path).exists();
        open_external_exists
            .insert(ExternalOpenApp::VSCode, app_exists("/Applications/Visual Studio Code.app"));
        open_external_exists
            .insert(ExternalOpenApp::Cursor, app_exists("/Applications/Cursor.app"));
        open_external_exists.insert(
            ExternalOpenApp::Trae,
            app_exists("/Applications/Trae.app") || app_exists("/Applications/Trae IDE.app"),
        );
        open_external_exists
            .insert(ExternalOpenApp::Windsurf, app_exists("/Applications/Windsurf.app"));
        open_external_exists.insert(ExternalOpenApp::Kiro, app_exists("/Applications/Kiro.app"));
        open_external_exists.insert(ExternalOpenApp::Zed, app_exists("/Applications/Zed.app"));
        open_external_exists
            .insert(ExternalOpenApp::TextMate, app_exists("/Applications/TextMate.app"));
        open_external_exists
            .insert(ExternalOpenApp::Antigravity, app_exists("/Applications/Antigravity.app"));
        open_external_exists.insert(
            ExternalOpenApp::Terminal,
            app_exists("/System/Applications/Utilities/Terminal.app")
                || app_exists("/Applications/Utilities/Terminal.app"),
        );
        open_external_exists.insert(ExternalOpenApp::ITerm2, app_exists("/Applications/iTerm.app"));
        open_external_exists
            .insert(ExternalOpenApp::Ghostty, app_exists("/Applications/Ghostty.app"));
        open_external_exists.insert(ExternalOpenApp::Xcode, app_exists("/Applications/Xcode.app"));
        open_external_exists
            .insert(ExternalOpenApp::AndroidStudio, app_exists("/Applications/Android Studio.app"));
        open_external_exists
            .insert(ExternalOpenApp::SublimeText, app_exists("/Applications/Sublime Text.app"));
    }

    #[cfg(windows)]
    {
        let cmd_exists = |cmd: &str| {
            std::process::Command::new("cmd")
                .args(["/C", "where", cmd])
                .status()
                .is_ok_and(|status| status.success())
        };
        open_external_exists.insert(ExternalOpenApp::VSCode, cmd_exists("code"));
        open_external_exists.insert(ExternalOpenApp::Cursor, cmd_exists("cursor"));
        open_external_exists.insert(ExternalOpenApp::Zed, cmd_exists("zed"));
        open_external_exists.insert(ExternalOpenApp::Trae, cmd_exists("trae"));
        open_external_exists.insert(ExternalOpenApp::Windsurf, cmd_exists("windsurf"));
        open_external_exists.insert(ExternalOpenApp::Kiro, cmd_exists("kiro"));
        open_external_exists.insert(ExternalOpenApp::SublimeText, cmd_exists("subl"));
        open_external_exists.insert(ExternalOpenApp::PowerShell, true);
    }

    #[cfg(target_os = "linux")]
    {
        let cmd_exists = |cmd: &str| {
            std::process::Command::new("sh")
                .args(["-lc", &format!("command -v {cmd} >/dev/null 2>&1")])
                .status()
                .is_ok_and(|status| status.success())
        };
        open_external_exists.insert(ExternalOpenApp::VSCode, cmd_exists("code"));
        open_external_exists.insert(ExternalOpenApp::Cursor, cmd_exists("cursor"));
        open_external_exists.insert(ExternalOpenApp::Zed, cmd_exists("zed"));
        open_external_exists.insert(ExternalOpenApp::Trae, cmd_exists("trae"));
        open_external_exists.insert(ExternalOpenApp::Windsurf, cmd_exists("windsurf"));
        open_external_exists.insert(ExternalOpenApp::Kiro, cmd_exists("kiro"));
        open_external_exists.insert(ExternalOpenApp::SublimeText, cmd_exists("subl"));
    }

    let open_external_platform = crate::app::state::RuntimePlatform::current_target();
    let open_external_app = select_external_app(cfg, &open_external_exists, open_external_platform);

    (open_external_platform, open_external_exists, open_external_app)
}

fn select_external_app(
    cfg: &serde_json::Value,
    open_external_exists: &HashMap<ExternalOpenApp, bool>,
    platform: Option<crate::app::state::RuntimePlatform>,
) -> ExternalOpenApp {
    configured_external_app(cfg)
        .filter(|app| open_external_exists.get(app).copied().unwrap_or(false))
        .or_else(|| {
            priority_external_apps(platform)
                .iter()
                .copied()
                .find(|app| open_external_exists.get(app).copied().unwrap_or(false))
        })
        .unwrap_or(ExternalOpenApp::Finder)
}

fn configured_external_app(cfg: &serde_json::Value) -> Option<ExternalOpenApp> {
    cfg.get("open_external_app").and_then(|value: &serde_json::Value| value.as_str()).and_then(
        |value| match value {
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
        },
    )
}

fn priority_external_apps(
    platform: Option<crate::app::state::RuntimePlatform>,
) -> &'static [ExternalOpenApp] {
    if matches!(platform, Some(crate::app::state::RuntimePlatform::MacOs)) {
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
    } else if matches!(platform, Some(crate::app::state::RuntimePlatform::Windows)) {
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

#[cfg(test)]
#[path = "external_apps_tests.rs"]
mod external_apps_tests;
