use super::*;

#[test]
fn parse_external_open_app_accepts_known_stable_ids() {
    assert_eq!(parse_external_open_app("finder"), Some(ExternalOpenApp::Finder));
    assert_eq!(parse_external_open_app("vscode"), Some(ExternalOpenApp::VSCode));
    assert_eq!(
        parse_external_open_app("android-studio"),
        Some(ExternalOpenApp::AndroidStudio)
    );
    assert_eq!(parse_external_open_app("powershell"), Some(ExternalOpenApp::PowerShell));
    assert_eq!(parse_external_open_app("unknown"), None);
}

#[test]
fn parse_external_open_app_maps_remaining_stable_ids() {
    let cases = [
        ("cursor", ExternalOpenApp::Cursor),
        ("trae", ExternalOpenApp::Trae),
        ("windsurf", ExternalOpenApp::Windsurf),
        ("kiro", ExternalOpenApp::Kiro),
        ("zed", ExternalOpenApp::Zed),
        ("textmate", ExternalOpenApp::TextMate),
        ("antigravity", ExternalOpenApp::Antigravity),
        ("terminal", ExternalOpenApp::Terminal),
        ("iterm2", ExternalOpenApp::ITerm2),
        ("ghostty", ExternalOpenApp::Ghostty),
        ("xcode", ExternalOpenApp::Xcode),
        ("sublime-text", ExternalOpenApp::SublimeText),
    ];

    for (id, expected) in cases {
        assert_eq!(parse_external_open_app(id), Some(expected));
    }
}

#[test]
fn external_open_priority_is_platform_specific() {
    assert_eq!(external_open_priority(Some(RuntimePlatform::MacOs))[0], ExternalOpenApp::Trae);
    assert!(
        external_open_priority(Some(RuntimePlatform::MacOs))
            .contains(&ExternalOpenApp::Terminal)
    );
    assert!(
        external_open_priority(Some(RuntimePlatform::Windows))
            .contains(&ExternalOpenApp::PowerShell)
    );
    assert!(
        !external_open_priority(Some(RuntimePlatform::Linux))
            .contains(&ExternalOpenApp::PowerShell)
    );
}
