//! 展示层状态类型测试，覆盖稳定标签、配置值解析、平台差异和克隆语义。
//! 枚举全集及默认值是界面与配置间的契约，未知输入应明确拒绝而非猜测。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("presentation_tests"));
}

use super::{
    AppTab, ConventionalCommitType, ExternalOpenApp, ModelPopoverHover, RuntimePlatform,
    TopBarGatewayTab, WorkResourceMode, WorkspaceMode,
};
use crate::app::Screen;
use crate::app::components::model_hover_tooltip::HoverAnchor;

#[test]
fn top_bar_gateway_tab_defaults_to_gateway() {
    assert_eq!(TopBarGatewayTab::default(), TopBarGatewayTab::Gateway);
}

#[test]
fn workspace_mode_defaults_and_round_trips_config_values() {
    assert_eq!(WorkspaceMode::default(), WorkspaceMode::Code);
    assert_eq!(WorkspaceMode::Code.as_str(), "code");
    assert_eq!(WorkspaceMode::Work.as_str(), "work");
    assert_eq!(WorkspaceMode::from_config(Some("code")), WorkspaceMode::Code);
    assert_eq!(WorkspaceMode::from_config(Some("work")), WorkspaceMode::Work);
    assert_eq!(WorkspaceMode::from_config(Some(" WORK ")), WorkspaceMode::Work);
    assert_eq!(WorkspaceMode::from_config(Some("unknown")), WorkspaceMode::Code);
    assert_eq!(WorkspaceMode::from_config(None), WorkspaceMode::Code);
}

#[test]
fn work_workspace_resource_mode_defaults_to_overview() {
    assert_eq!(WorkResourceMode::default(), WorkResourceMode::Overview);
}

#[test]
fn runtime_platform_parses_gateway_aliases_and_rejects_unknown_values() {
    let cases = [
        ("macos", Some(RuntimePlatform::MacOs)),
        (" Darwin ", Some(RuntimePlatform::MacOs)),
        ("MAC", Some(RuntimePlatform::MacOs)),
        ("windows", Some(RuntimePlatform::Windows)),
        (" Win32 ", Some(RuntimePlatform::Windows)),
        ("LINUX", Some(RuntimePlatform::Linux)),
        ("", None),
        ("freebsd", None),
    ];

    for (value, expected) in cases {
        assert_eq!(RuntimePlatform::from_gateway_str(value), expected);
    }
}

#[test]
fn runtime_platform_current_target_matches_compilation_target() {
    #[cfg(target_os = "macos")]
    assert_eq!(RuntimePlatform::current_target(), Some(RuntimePlatform::MacOs));

    #[cfg(windows)]
    assert_eq!(RuntimePlatform::current_target(), Some(RuntimePlatform::Windows));

    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "macos"), not(windows)))]
    assert_eq!(RuntimePlatform::current_target(), Some(RuntimePlatform::Linux));

    #[cfg(target_arch = "wasm32")]
    assert_eq!(RuntimePlatform::current_target(), None);
}

#[test]
fn runtime_platform_file_manager_labels_match_platform() {
    assert_eq!(RuntimePlatform::MacOs.file_manager_label(), "Finder");
    assert_eq!(RuntimePlatform::Windows.file_manager_label(), "File Explorer");
    assert_eq!(RuntimePlatform::Linux.file_manager_label(), "File Manager");
}

#[test]
fn external_open_app_identifiers_and_labels_cover_all_variants() {
    let cases = [
        (ExternalOpenApp::Finder, "finder", "文件管理器"),
        (ExternalOpenApp::VSCode, "vscode", "VS Code"),
        (ExternalOpenApp::Cursor, "cursor", "Cursor"),
        (ExternalOpenApp::Trae, "trae", "Trae"),
        (ExternalOpenApp::Windsurf, "windsurf", "Windsurf"),
        (ExternalOpenApp::Kiro, "kiro", "Kiro"),
        (ExternalOpenApp::Zed, "zed", "Zed"),
        (ExternalOpenApp::TextMate, "textmate", "TextMate"),
        (ExternalOpenApp::Antigravity, "antigravity", "Antigravity"),
        (ExternalOpenApp::Terminal, "terminal", "Terminal"),
        (ExternalOpenApp::ITerm2, "iterm2", "iTerm2"),
        (ExternalOpenApp::Ghostty, "ghostty", "Ghostty"),
        (ExternalOpenApp::Xcode, "xcode", "Xcode"),
        (ExternalOpenApp::AndroidStudio, "android-studio", "Android Studio"),
        (ExternalOpenApp::PowerShell, "powershell", "PowerShell"),
        (ExternalOpenApp::SublimeText, "sublime-text", "Sublime Text"),
    ];

    for (app, identifier, label) in cases {
        assert_eq!(app.as_str(), identifier);
        assert_eq!(app.label(), label);
    }
}

#[test]
fn app_tab_clone_preserves_identity_navigation_and_optional_project() {
    let tab = AppTab {
        id: "project-1".to_string(),
        title: "Project One".to_string(),
        screen: Screen::Project,
        project_path: Some("/workspace/project-one".to_string()),
    };

    assert_eq!(tab.clone(), tab);
    assert_ne!(
        tab,
        AppTab {
            id: "home".to_string(),
            title: "Home".to_string(),
            screen: Screen::Home,
            project_path: None,
        }
    );
}

#[test]
fn model_popover_hover_clone_preserves_each_variant_payload() {
    let model = ModelPopoverHover::Model {
        provider_id: "openai".to_string(),
        model_id: "gpt-test".to_string(),
        anchor: Some(HoverAnchor { x: 12.5, y: 24.0 }),
    };
    let ModelPopoverHover::Model { provider_id, model_id, anchor } = model.clone() else {
        panic!("cloned model hover should preserve its variant");
    };
    assert_eq!(provider_id, "openai");
    assert_eq!(model_id, "gpt-test");
    let anchor = anchor.expect("model hover anchor");
    assert_eq!((anchor.x, anchor.y), (12.5, 24.0));

    let text = ModelPopoverHover::Text { text: "Model details".to_string(), anchor: None };
    let ModelPopoverHover::Text { text, anchor } = text.clone() else {
        panic!("cloned text hover should preserve its variant");
    };
    assert_eq!(text, "Model details");
    assert!(anchor.is_none());
}

#[test]
fn conventional_commit_types_round_trip_every_prefix_and_display_in_declared_order() {
    let expected = [
        (ConventionalCommitType::Feat, "feat"),
        (ConventionalCommitType::Fix, "fix"),
        (ConventionalCommitType::Docs, "docs"),
        (ConventionalCommitType::Style, "style"),
        (ConventionalCommitType::Refactor, "refactor"),
        (ConventionalCommitType::Perf, "perf"),
        (ConventionalCommitType::Test, "test"),
        (ConventionalCommitType::Build, "build"),
        (ConventionalCommitType::Ci, "ci"),
        (ConventionalCommitType::Chore, "chore"),
        (ConventionalCommitType::Revert, "revert"),
        (ConventionalCommitType::Init, "init"),
        (ConventionalCommitType::Config, "config"),
        (ConventionalCommitType::Release, "release"),
        (ConventionalCommitType::Deploy, "deploy"),
        (ConventionalCommitType::Merge, "merge"),
        (ConventionalCommitType::Wip, "wip"),
        (ConventionalCommitType::Typo, "typo"),
        (ConventionalCommitType::Locale, "locale"),
    ];

    assert_eq!(ConventionalCommitType::all(), expected.map(|(kind, _)| kind));
    for (kind, prefix) in expected {
        assert_eq!(kind.as_str(), prefix);
        assert_eq!(kind.to_string(), prefix);
        assert_eq!(ConventionalCommitType::from_prefix(prefix), Some(kind));
    }
    assert_eq!(ConventionalCommitType::from_prefix("unknown"), None);
    assert_eq!(ConventionalCommitType::from_prefix("FEAT"), None);
    assert_eq!(ConventionalCommitType::from_prefix(" feat"), None);
}
