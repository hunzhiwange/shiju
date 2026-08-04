use super::{ViewMessage, take_last_persisted_app_theme, update};
use crate::app::components::editor::Editor;
use crate::app::theme_catalog::{
    ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex, ThemeStore, config_value_for_theme,
    set_test_themes_root, to_iced_theme,
};
use crate::app::{App, PreviewTab};
use iced::{Theme, widget::Id};

fn test_app() -> App {
    App::new().0
}

fn preview_tab(path: &str, content: &str) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: path.rsplit('/').next().unwrap_or(path).to_string(),
        content: content.to_string(),
        is_dirty: false,
        truncated: false,
        auto_save_revision: 0,
        editor: Editor::new(content, "text"),
        scroll_id: Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: None,
    }
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("theme_tests"));
}

#[test]
fn app_theme_selected_updates_theme_when_editor_is_independent() {
    let mut app = test_app();
    app.app_theme = Theme::Light;
    app.editor_theme = Theme::Light;
    app.editor_follow_system_theme = false;

    let _ = update(&mut app, ViewMessage::AppThemeSelected(Theme::Dark));

    assert_eq!(app.app_theme.to_string(), Theme::Dark.to_string());
    assert_eq!(app.editor_theme.to_string(), Theme::Light.to_string());
    assert_eq!(app.effective_editor_theme().to_string(), Theme::Light.to_string());
}

#[test]
fn app_theme_selected_changes_effective_editor_theme_when_following_system() {
    let mut app = test_app();
    app.app_theme = Theme::Light;
    app.editor_theme = Theme::Light;
    app.editor_follow_system_theme = true;

    let _ = update(&mut app, ViewMessage::AppThemeSelected(Theme::Dark));

    assert_eq!(app.app_theme.to_string(), Theme::Dark.to_string());
    assert_eq!(app.effective_editor_theme().to_string(), Theme::Dark.to_string());
}

#[test]
fn app_theme_selected_with_preview_tabs_preserves_editor_data() {
    let mut app = test_app();
    app.app_theme = Theme::Light;
    app.editor_follow_system_theme = true;
    app.preview_tabs = vec![
        preview_tab("/tmp/first.txt", "first content"),
        preview_tab("/tmp/second.txt", "second content"),
    ];

    let _ = update(&mut app, ViewMessage::AppThemeSelected(Theme::Dark));

    assert_eq!(app.app_theme.to_string(), Theme::Dark.to_string());
    assert_eq!(app.preview_tabs[0].path, "/tmp/first.txt");
    assert_eq!(app.preview_tabs[0].editor.content(), "first content");
    assert_eq!(app.preview_tabs[1].path, "/tmp/second.txt");
    assert_eq!(app.preview_tabs[1].editor.content(), "second content");
}

#[test]
fn unrelated_view_message_leaves_theme_state_unchanged() {
    let mut app = test_app();
    app.app_theme = Theme::Dark;
    app.editor_theme = Theme::Light;
    app.editor_follow_system_theme = false;

    let _ = update(&mut app, ViewMessage::ToggleSettingsPanel);

    assert_eq!(app.app_theme.to_string(), Theme::Dark.to_string());
    assert_eq!(app.editor_theme.to_string(), Theme::Light.to_string());
    assert!(!app.editor_follow_system_theme);
}

#[test]
fn app_theme_selected_persists_custom_prefix_for_custom_theme() {
    let definition = ThemeDefinition {
        id: "mint-night".to_string(),
        name: "Mint Night".to_string(),
        tagline: None,
        palette: ThemePaletteHex {
            background: "#0b1220".to_string(),
            text: "#e2e8f0".to_string(),
            primary: "#34d399".to_string(),
            success: "#22c55e".to_string(),
            warning: "#f59e0b".to_string(),
            danger: "#ef4444".to_string(),
        },
        assets: ThemeAssets::default(),
        chrome: ThemeChrome::default(),
    };
    let theme = to_iced_theme(&definition).expect("convert");
    assert_eq!(config_value_for_theme(&theme, Some("mint-night")), "custom:mint-night");
    assert_eq!(config_value_for_theme(&Theme::Dark, None), "Dark");

    let temp = tempfile::tempdir().expect("tempdir");
    let store = ThemeStore::new(temp.path());
    store.save(&definition).expect("save theme for persist lookup");
    set_test_themes_root(Some(temp.path().to_path_buf()));

    let mut app = test_app();
    app.app_theme = Theme::Light;
    app.active_custom_theme_id = None;
    let _ = take_last_persisted_app_theme();

    let _ = update(&mut app, ViewMessage::AppThemeSelected(theme.clone()));

    assert!(matches!(app.app_theme, Theme::Custom(_)));
    assert_eq!(app.app_theme.to_string(), "Mint Night");
    assert_eq!(app.app_theme.palette(), theme.palette());
    assert_eq!(app.active_custom_theme_id.as_deref(), Some("mint-night"));
    assert_eq!(take_last_persisted_app_theme().as_deref(), Some("custom:mint-night"));

    let _ = update(&mut app, ViewMessage::AppThemeSelected(Theme::Dark));
    assert!(app.active_custom_theme_id.is_none());
    assert_eq!(take_last_persisted_app_theme().as_deref(), Some("Dark"));

    set_test_themes_root(None);
}
