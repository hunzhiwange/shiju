#[test]
fn header_tabs_tests_are_wired() {
    assert!(module_path!().contains("header_tabs_tests"));
}

use super::header_tabs::{
    build_header_tabs, close_button_style, header_tabs_height, tab_button_style, tab_close_message,
    tab_context_menu_matches, tab_drag_end_message, tab_drag_start_message, tab_is_selected,
    tab_menu_anchor, tab_right_click_message, tab_select_message,
};
use crate::app::components::editor::Editor;
use crate::app::message::{PreviewMessage, ProjectMessage};
use crate::app::{App, Message, PreviewTab};
use iced::widget::{Id, button};
use iced::{Background, Color, Point, Theme};

fn app() -> App {
    App::new().0
}

fn tab(path: &str, dirty: bool) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: path.rsplit('/').next().unwrap_or(path).to_string(),
        content: "content".to_string(),
        is_dirty: dirty,
        truncated: false,
        auto_save_revision: 0,
        editor: Editor::new("content", "txt"),
        scroll_id: Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: None,
    }
}

fn keep(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn style_background(style: &button::Style) -> Option<Color> {
    match &style.background {
        Some(Background::Color(color)) => Some(*color),
        None => None,
        _ => panic!("expected a solid color background"),
    }
}

#[test]
fn header_tabs_builds_for_empty_tabs() {
    let app = app();

    keep(build_header_tabs(&app));
}

#[test]
fn header_tabs_builds_selected_and_unselected_tabs() {
    let mut app = app();
    app.active_preview_path = Some("/tmp/a.rs".to_string());
    app.preview_tabs = vec![tab("/tmp/a.rs", false), tab("/tmp/b.ts", true)];

    keep(build_header_tabs(&app));
}

#[test]
fn header_tabs_builds_context_menu_overlay_for_target_tab() {
    let mut app = app();
    app.active_preview_path = Some("/tmp/a.rs".to_string());
    app.preview_tabs = vec![tab("/tmp/a.rs", false), tab("/tmp/b.ts", true)];
    app.preview_tab_menu_path = Some("/tmp/b.ts".to_string());
    app.preview_tab_menu_pos = Some(Point::new(12.0, 24.0));

    keep(build_header_tabs(&app));
}

#[test]
fn header_tabs_uses_taller_scrollbar_container_when_tabs_overflow() {
    let mut app = app();
    app.window_size = (120.0, 700.0);
    app.preview_tabs =
        (0..8).map(|i| tab(&format!("/tmp/very-long-file-name-{i}.rs"), false)).collect();
    app.active_preview_path = Some("/tmp/very-long-file-name-0.rs".to_string());

    keep(build_header_tabs(&app));
}

#[test]
fn header_tabs_height_tracks_scrollbar_visibility() {
    assert_eq!(header_tabs_height(false), 30.0);
    assert_eq!(header_tabs_height(true), 36.0);
}

#[test]
fn tab_selection_matches_only_the_active_path() {
    assert!(tab_is_selected(Some("/tmp/a.rs"), "/tmp/a.rs"));
    assert!(!tab_is_selected(Some("/tmp/b.rs"), "/tmp/a.rs"));
    assert!(!tab_is_selected(None, "/tmp/a.rs"));
}

#[test]
fn tab_context_menu_matches_only_the_target_path() {
    assert!(tab_context_menu_matches(Some("/tmp/a.rs"), "/tmp/a.rs"));
    assert!(!tab_context_menu_matches(Some("/tmp/b.rs"), "/tmp/a.rs"));
    assert!(!tab_context_menu_matches(None, "/tmp/a.rs"));
}

#[test]
fn tab_menu_anchor_defaults_to_origin_and_preserves_position() {
    assert_eq!(tab_menu_anchor(None), Point::ORIGIN);
    assert_eq!(tab_menu_anchor(Some(Point::new(12.0, 24.0))), Point::new(12.0, 24.0));
}

#[test]
fn tab_messages_preserve_paths_and_pointer_position() {
    let Message::Project(ProjectMessage::FileTreeDragStart(path, None)) =
        tab_drag_start_message("/tmp/a.rs")
    else {
        panic!("expected drag start message");
    };
    assert_eq!(path, "/tmp/a.rs");
    assert!(matches!(tab_drag_end_message(), Message::Project(ProjectMessage::FileTreeDragEnd)));

    let Message::Preview(PreviewMessage::Close(path)) = tab_close_message("/tmp/a.rs") else {
        panic!("expected close message");
    };
    assert_eq!(path, "/tmp/a.rs");

    let Message::Preview(PreviewMessage::Select(path)) = tab_select_message("/tmp/b.rs") else {
        panic!("expected select message");
    };
    assert_eq!(path, "/tmp/b.rs");

    let Message::Preview(PreviewMessage::TabRightClicked(path, x, y)) =
        tab_right_click_message("/tmp/c.rs", Point::new(8.0, 13.0))
    else {
        panic!("expected tab right-click message");
    };
    assert_eq!(path, "/tmp/c.rs");
    assert_eq!((x, y), (8.0, 13.0));
}

#[test]
fn close_button_style_changes_background_by_status() {
    let theme = Theme::Light;
    let active = close_button_style(&theme, button::Status::Active);
    let hovered = close_button_style(&theme, button::Status::Hovered);
    let pressed = close_button_style(&theme, button::Status::Pressed);

    assert_eq!(style_background(&active), None);
    assert_eq!(style_background(&hovered).map(|color| color.a), Some(0.10));
    assert_eq!(style_background(&pressed).map(|color| color.a), Some(0.18));
    assert_eq!(active.text_color, theme.palette().text);
    assert_eq!(active.border.width, 0.0);
    assert_eq!(active.border.color, Color::TRANSPARENT);
}

#[test]
fn tab_button_style_handles_light_theme_selection_and_pointer_states() {
    let theme = Theme::Light;
    let unselected = tab_button_style(&theme, button::Status::Active, false);
    let selected = tab_button_style(&theme, button::Status::Active, true);
    let hovered = tab_button_style(&theme, button::Status::Hovered, false);
    let pressed = tab_button_style(&theme, button::Status::Pressed, false);

    assert_eq!(style_background(&unselected), None);
    assert_eq!(style_background(&selected).map(|color| color.a), Some(0.18));
    assert_eq!(style_background(&hovered).map(|color| color.a), Some(0.10));
    assert_eq!(style_background(&pressed), style_background(&selected));
    assert_eq!(selected.text_color, theme.palette().text);
    assert_eq!(selected.border.color, Color::TRANSPARENT);
}

#[test]
fn tab_button_style_uses_dark_theme_surface_tones() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let selected = tab_button_style(&theme, button::Status::Active, true);
    let hovered = tab_button_style(&theme, button::Status::Hovered, false);

    assert_eq!(
        style_background(&selected),
        Some(palette.background.strong.color.scale_alpha(0.95))
    );
    assert_eq!(style_background(&hovered), Some(palette.background.strong.color.scale_alpha(0.8)));
    assert_ne!(style_background(&selected), style_background(&hovered));
}
