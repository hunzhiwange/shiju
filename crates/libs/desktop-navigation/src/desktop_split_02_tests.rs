// Tests migrated from desktop host state; production types retain their original owners.
mod behavior_0 {
use crate::components::top_bar::types::{TopBarGatewayTab};
#[test]
fn top_bar_gateway_tab_defaults_to_gateway() {
    assert_eq!(TopBarGatewayTab::default(), TopBarGatewayTab::Gateway);
}
}

mod behavior_1 {
use crate::components::top_bar::types::{WorkspaceMode};
#[test]
fn workspace_mode_defaults_and_round_trips_config_values() {
    assert_eq!(WorkspaceMode::default(), WorkspaceMode::Work);
    assert_eq!(WorkspaceMode::Code.as_str(), "code");
    assert_eq!(WorkspaceMode::Work.as_str(), "work");
    assert_eq!(WorkspaceMode::from_config(Some("code")), WorkspaceMode::Code);
    assert_eq!(WorkspaceMode::from_config(Some("work")), WorkspaceMode::Work);
    assert_eq!(WorkspaceMode::from_config(Some(" WORK ")), WorkspaceMode::Work);
    assert_eq!(WorkspaceMode::from_config(Some("unknown")), WorkspaceMode::Work);
    assert_eq!(WorkspaceMode::from_config(None), WorkspaceMode::Work);
}
}

mod behavior_2 {
use crate::components::top_bar::types::{RuntimePlatform};
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
}

mod behavior_3 {
use crate::components::top_bar::types::{RuntimePlatform};
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
}

mod behavior_4 {
use crate::components::top_bar::types::{RuntimePlatform};
#[test]
fn runtime_platform_file_manager_labels_match_platform() {
    assert_eq!(RuntimePlatform::MacOs.file_manager_label(), "Finder");
    assert_eq!(RuntimePlatform::Windows.file_manager_label(), "File Explorer");
    assert_eq!(RuntimePlatform::Linux.file_manager_label(), "File Manager");
}
}

mod behavior_5 {
use crate::components::top_bar::types::{ExternalOpenApp};
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
}

mod behavior_6 {
use crate::components::model_hover_tooltip::{ModelPopoverHover, HoverAnchor};
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
}

mod behavior_7 {
use crate::components::notification::Notification;
#[test]
fn notification_clone_preserves_identity_message_and_timestamp() {
    let notification = Notification {
        localized_message: None,
        id: 41,
        message: "saved".to_string(),
        created_at: web_time::SystemTime::UNIX_EPOCH,
    };

    let cloned = notification.clone();

    assert_eq!(cloned.id, 41);
    assert_eq!(cloned.message, "saved");
    assert_eq!(cloned.created_at, web_time::SystemTime::UNIX_EPOCH);
}
}

mod app_tab {
use crate::components::top_bar::types::AppTab;
use crate::ui_types::Screen;
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
}
