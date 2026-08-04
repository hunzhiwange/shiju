use super::{
    fullscreen_icon_style, fullscreen_tooltip_style, overlay_icon_button_style,
    preview_breadcrumb_segments, show_fullscreen_controls, view,
};
use crate::app::components::editor::Editor;
use crate::app::state::WorkspaceMode;
use crate::app::{App, Message, PreviewTab, Screen};
use iced::widget::button;
use iced::{Background, Color, Theme, widget::Id};

fn app() -> App {
    App::new().0
}

fn tab(path: &str, server_key: Option<&'static str>) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: path.rsplit('/').next().unwrap_or(path).to_string(),
        content: "fn main() {}".to_string(),
        is_dirty: false,
        truncated: false,
        auto_save_revision: 0,
        editor: Editor::new("fn main() {}", "rust"),
        scroll_id: Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: server_key,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: Some(format!("file://{path}")),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: Some("rust".to_string()),
    }
}

fn keep(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn view_builds_project_preview_with_tabs_and_content() {
    let mut app = app();
    app.screen = Screen::Project;
    app.active_preview_path = Some("/tmp/main.rs".to_string());
    app.preview_tabs = vec![tab("/tmp/main.rs", None)];

    keep(view(&app));
}

#[test]
fn view_builds_settings_overlay_when_enabled() {
    let mut app = app();
    app.screen = Screen::Preview;
    app.show_preview_settings = true;

    keep(view(&app));
}

#[test]
fn view_builds_changes_layout_without_preview_header() {
    let mut app = app();
    app.screen = Screen::Project;
    app.file_manager_show_changes = true;

    keep(view(&app));
}

#[test]
fn view_builds_preview_without_an_active_file() {
    let mut app = app();
    app.screen = Screen::Preview;
    app.active_preview_path = None;
    app.file_manager_show_changes = false;

    keep(view(&app));
}

#[test]
fn view_builds_breadcrumb_when_active_file_has_no_matching_tab() {
    let mut app = app();
    app.screen = Screen::Project;
    app.project_path = Some("/tmp/project".to_string());
    app.active_preview_path = Some("/tmp/project/src/missing.rs".to_string());

    keep(view(&app));
}

#[test]
fn view_builds_context_menu_with_and_without_anchor() {
    let mut app = app();
    app.show_preview_context_menu = true;

    keep(view(&app));

    app.preview_context_menu_pos = Some((18.0, 27.0));
    keep(view(&app));
}

#[test]
fn view_builds_fullscreen_exit_controls() {
    let mut app = app();
    app.screen = Screen::Project;
    app.git_diff_fullscreen = true;

    keep(view(&app));
}

#[test]
fn preview_fullscreen_controls_only_show_in_code_mode() {
    let mut app = app();
    app.screen = Screen::Project;

    app.workspace_mode = WorkspaceMode::Code;
    assert!(show_fullscreen_controls(&app));

    app.workspace_mode = WorkspaceMode::Work;
    assert!(!show_fullscreen_controls(&app));
}

#[test]
fn overlay_icon_button_style_distinguishes_statuses() {
    let theme = Theme::Light;
    let active = overlay_icon_button_style(&theme, button::Status::Active);
    let hovered = overlay_icon_button_style(&theme, button::Status::Hovered);
    let pressed = overlay_icon_button_style(&theme, button::Status::Pressed);

    assert!(active.background.is_none());
    assert_eq!(active.border.color, Color::TRANSPARENT);
    assert!(matches!(hovered.background, Some(Background::Color(_))));
    assert!(matches!(pressed.background, Some(Background::Color(_))));
    assert_ne!(hovered.border.color, pressed.border.color);
}

#[test]
fn fullscreen_tooltip_style_switches_between_light_and_dark_palettes() {
    let light = fullscreen_tooltip_style(&Theme::Light);
    let dark = fullscreen_tooltip_style(&Theme::Dark);

    assert_eq!(light.text_color, Some(Color::WHITE));
    assert_ne!(light.background, dark.background);
    assert!(light.shadow.blur_radius > dark.shadow.blur_radius);
}

#[test]
fn fullscreen_icon_style_uses_visible_theme_color() {
    let light = fullscreen_icon_style(&Theme::Light);
    let dark = fullscreen_icon_style(&Theme::Dark);

    assert!(light.color.is_some());
    assert!(dark.color.is_some());
    assert_ne!(light.color, dark.color);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_requires_matching_active_tab() {
    let mut app = app();
    assert!(super::active_preview_lsp_badge(&app).is_none());

    app.active_preview_path = Some("/tmp/main.rs".to_string());
    assert!(super::active_preview_lsp_badge(&app).is_none());

    app.preview_tabs = vec![tab("/tmp/main.rs", Some("rust-analyzer"))];
    assert!(super::active_preview_lsp_badge(&app).is_some());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_builds_working_state_from_progress() {
    let mut app = app();
    app.active_preview_path = Some("/tmp/main.rs".to_string());
    app.preview_tabs = vec![tab("/tmp/main.rs", Some("rust-analyzer"))];
    app.lsp_progress.insert(
        "rust-analyzer".to_string(),
        std::collections::HashMap::from([(
            "index".to_string(),
            crate::app::preview::LspProgress {
                title: "indexing".to_string(),
                message: None,
                percentage: None,
            },
        )]),
    );

    assert!(super::active_preview_lsp_badge(&app).is_some());
}

#[test]
fn preview_breadcrumb_segments_prefers_project_relative_segments_and_falls_back_to_path() {
    let relative = preview_breadcrumb_segments(
        std::path::Path::new("/tmp/project/src/main.rs"),
        Some("/tmp/project"),
    );
    assert_eq!(relative, vec!["src".to_string(), "main.rs".to_string()]);

    let root =
        preview_breadcrumb_segments(std::path::Path::new("/tmp/project"), Some("/tmp/project"));
    assert_eq!(root, vec!["/tmp/project".to_string()]);

    let unrelated = preview_breadcrumb_segments(
        std::path::Path::new("/tmp/other/main.rs"),
        Some("/tmp/project"),
    );
    assert_eq!(unrelated, vec!["/tmp/other/main.rs".to_string()]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_state_and_colors_cover_all_tones() {
    assert_eq!(
        super::active_preview_lsp_badge_state(Some("rust-analyzer"), false, true),
        ("LSP", super::BadgeTone::Ready)
    );
    assert_eq!(
        super::active_preview_lsp_badge_state(Some("rust-analyzer"), true, true),
        ("LSP", super::BadgeTone::Working)
    );
    assert_eq!(
        super::active_preview_lsp_badge_state(None, false, true),
        ("LSP", super::BadgeTone::Error)
    );
    assert_eq!(
        super::active_preview_lsp_badge_state(None, false, false),
        ("Plain Text", super::BadgeTone::Neutral)
    );

    for tone in [
        super::BadgeTone::Ready,
        super::BadgeTone::Working,
        super::BadgeTone::Error,
        super::BadgeTone::Neutral,
    ] {
        let dot = super::active_preview_lsp_badge_dot_color(&Theme::Dark, tone);
        let (background, text, border) = super::active_preview_lsp_badge_colors(&Theme::Dark, tone);
        assert_ne!(dot, Color::TRANSPARENT);
        assert_ne!(background, Color::TRANSPARENT);
        assert_ne!(text, Color::TRANSPARENT);
        assert_ne!(border, Color::TRANSPARENT);
    }
}
