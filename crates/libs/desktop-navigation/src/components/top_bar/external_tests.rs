use super::external::*;
use super::types::*;
use desktop_assets::Icon;
#[test]
fn file_manager_label_falls_back_and_uses_platform_label() {
    assert_eq!(file_manager_label(None), "File Manager");
    assert_eq!(file_manager_label(Some(RuntimePlatform::MacOs)), "Finder");
    assert_eq!(file_manager_label(Some(RuntimePlatform::Windows)), "File Explorer");
    assert_eq!(file_manager_label(Some(RuntimePlatform::Linux)), "File Manager");
}

#[test]
fn external_app_label_uses_platform_only_for_finder() {
    assert_eq!(
        external_app_label(ExternalOpenApp::Finder, Some(RuntimePlatform::Windows)),
        "File Explorer"
    );
    assert_eq!(
        external_app_label(ExternalOpenApp::VSCode, Some(RuntimePlatform::Windows)),
        "VS Code"
    );
    assert_eq!(external_app_label(ExternalOpenApp::Cursor, None), "Cursor");
}

#[test]
fn external_app_logo_covers_every_supported_target() {
    let expected = [
        (ExternalOpenApp::Finder, ExternalAppLogo::Svg(Icon::AppFileExplorer)),
        (ExternalOpenApp::Trae, ExternalAppLogo::Svg(Icon::AppTrae)),
        (ExternalOpenApp::Windsurf, ExternalAppLogo::Image(Icon::AppWindsurf)),
        (ExternalOpenApp::Kiro, ExternalAppLogo::Svg(Icon::AppKiro)),
        (ExternalOpenApp::Cursor, ExternalAppLogo::Svg(Icon::AppCursor)),
        (ExternalOpenApp::VSCode, ExternalAppLogo::Svg(Icon::AppVSCode)),
        (ExternalOpenApp::Zed, ExternalAppLogo::Svg(Icon::AppZed)),
        (ExternalOpenApp::TextMate, ExternalAppLogo::Image(Icon::AppTextMate)),
        (ExternalOpenApp::Antigravity, ExternalAppLogo::Svg(Icon::AppAntigravity)),
        (ExternalOpenApp::Terminal, ExternalAppLogo::Image(Icon::AppTerminal)),
        (ExternalOpenApp::ITerm2, ExternalAppLogo::Svg(Icon::AppITerm2)),
        (ExternalOpenApp::Ghostty, ExternalAppLogo::Svg(Icon::AppGhostty)),
        (ExternalOpenApp::Xcode, ExternalAppLogo::Image(Icon::AppXcode)),
        (ExternalOpenApp::AndroidStudio, ExternalAppLogo::Svg(Icon::AppAndroidStudio)),
        (ExternalOpenApp::PowerShell, ExternalAppLogo::Svg(Icon::AppPowerShell)),
        (ExternalOpenApp::SublimeText, ExternalAppLogo::Svg(Icon::AppSublimeText)),
    ];

    assert_eq!(OPEN_EXTERNAL_TARGETS.len(), expected.len());
    for (target, logo) in expected {
        assert_eq!(external_app_logo(target, None), logo);
    }
    assert_eq!(
        external_app_logo(ExternalOpenApp::Finder, Some(RuntimePlatform::MacOs)),
        ExternalAppLogo::Image(Icon::AppFinder)
    );
}

#[test]
fn finder_logo_uses_file_explorer_svg_on_non_macos_platforms() {
    for platform in [RuntimePlatform::Windows, RuntimePlatform::Linux] {
        assert_eq!(
            external_app_logo(ExternalOpenApp::Finder, Some(platform)),
            ExternalAppLogo::Svg(Icon::AppFileExplorer)
        );
    }
}
