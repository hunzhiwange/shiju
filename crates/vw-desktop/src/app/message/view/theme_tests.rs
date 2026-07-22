use super::{ViewMessage, update};
use crate::app::components::editor::Editor;
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
