use super::Icon;
use iced::widget::svg;
use std::collections::HashMap;

pub(super) fn register_icons(m: &mut HashMap<Icon, svg::Handle>) {
    // Trae 应用图标
    m.insert(
        Icon::AppTrae,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/trae.svg")),
    );

    // === 应用程序图标（SVG 格式）===

    // GitHub Copilot 应用图标
    m.insert(
        Icon::AppGitHubCopilot,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/app/githubcopilot.svg"
        )),
    );

    // VS Code 应用图标
    m.insert(
        Icon::AppVSCode,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/vscode.svg")),
    );

    // Cursor 应用图标
    m.insert(
        Icon::AppCursor,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/cursor.svg")),
    );

    // Auggie 应用图标
    m.insert(
        Icon::AppAuggie,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/auggie.svg")),
    );

    // Claude Code 应用图标
    m.insert(
        Icon::AppClaudeCode,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/claude-code.svg")),
    );

    // Codex 应用图标
    m.insert(
        Icon::AppCodex,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/codex.svg")),
    );

    // Factory Droid 应用图标
    m.insert(
        Icon::AppFactoryDroid,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/app/factory-droid.svg"
        )),
    );

    // Gemini CLI 应用图标
    m.insert(
        Icon::AppGeminiCli,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/gemini-cli.svg")),
    );

    // KiloCode 应用图标
    m.insert(
        Icon::AppKiloCode,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/kilocode.svg")),
    );

    // Kimi Code 应用图标
    m.insert(
        Icon::AppKimiCode,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/kimi-color.svg")),
    );

    // OpenClaw 应用图标
    m.insert(
        Icon::AppOpenClaw,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/openclaw.svg")),
    );

    // OpenCode 应用图标
    m.insert(
        Icon::AppOpenCode,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/opencode.svg")),
    );

    // pi-acp 应用图标
    m.insert(
        Icon::AppPi,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/pi.svg")),
    );

    // Qoder 应用图标
    m.insert(
        Icon::AppQoder,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/qoder.svg")),
    );

    // Qwen Code 应用图标
    m.insert(
        Icon::AppQwenCode,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/qwen-code.svg")),
    );

    // Zed 应用图标
    m.insert(
        Icon::AppZed,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/zed.svg")),
    );

    // Sublime Text 应用图标
    m.insert(
        Icon::AppSublimeText,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/sublimetext.svg")),
    );

    // Ghostty 终端应用图标
    m.insert(
        Icon::AppGhostty,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/ghostty.svg")),
    );

    // iTerm2 终端应用图标
    m.insert(
        Icon::AppITerm2,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/iterm2.svg")),
    );

    // PowerShell 应用图标
    m.insert(
        Icon::AppPowerShell,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/powershell.svg")),
    );

    // Android Studio 应用图标
    m.insert(
        Icon::AppAndroidStudio,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/app/android-studio.svg"
        )),
    );

    // Antigravity 应用图标
    m.insert(
        Icon::AppAntigravity,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/antigravity.svg")),
    );

    // 文件资源管理器应用图标
    m.insert(
        Icon::AppFileExplorer,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/app/file-explorer.svg"
        )),
    );

    // Kiro 应用图标
    m.insert(
        Icon::AppKiro,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/app/kiro.svg")),
    );

    // 三个垂直点/更多选项图标
    m.insert(
        Icon::DotsThreeVertical,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/three-dots-vertical.svg"
        )),
    );
}
#[cfg(test)]
#[path = "icon_apps_tests.rs"]
mod icon_apps_tests;
