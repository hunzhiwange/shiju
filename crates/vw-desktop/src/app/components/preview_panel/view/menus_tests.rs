#[test]
fn menus_tests_are_wired() {
    assert!(module_path!().contains("menus_tests"));
}

use super::menus::{
    build_menu_ui, context_menu_container_style, nav_popup_container_style,
    nav_popup_item_button_style, nav_popup_item_icon, nav_popup_item_message,
};
use crate::app::assets::Icon;
use crate::app::components::editor::Editor;
use crate::app::message::{PreviewMessage, ProjectMessage};
use crate::app::{App, Message, PreviewTab};
use iced::widget::Id;
use iced::widget::button;
use iced::{Background, Color, Theme};

fn app() -> App {
    App::new().0
}

fn tab(path: &str) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: path.rsplit('/').next().unwrap_or(path).to_string(),
        content: "content".to_string(),
        is_dirty: false,
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

fn assert_close_nav_then(message: Message, expected_path: &str, expected_dir: bool) {
    let Message::Batch(messages) = message else {
        panic!("expected batch message");
    };

    assert_eq!(messages.len(), 2);
    assert!(matches!(messages[0], Message::Preview(PreviewMessage::CloseNavPopup)));

    match (&messages[1], expected_dir) {
        (Message::Project(ProjectMessage::ToggleTreeDir(path)), true) => {
            assert_eq!(path, expected_path);
        }
        (Message::Preview(PreviewMessage::Open(path)), false) => {
            assert_eq!(path, expected_path);
        }
        _ => panic!("unexpected nav popup action"),
    }
}

#[test]
fn menu_ui_builds_empty_layers_when_hidden() {
    let app = app();

    keep(build_menu_ui(&app));
}

#[test]
fn menu_ui_handles_context_menu_without_target_or_position() {
    let mut app = app();
    app.show_preview_context_menu = true;

    keep(build_menu_ui(&app));

    app.preview_context_target = Some(("/tmp/a.rs".to_string(), 1, 1, 1, 1));
    keep(build_menu_ui(&app));
}

#[test]
fn menu_ui_builds_context_menu_for_matching_tab() {
    let mut app = app();
    app.show_preview_context_menu = true;
    app.preview_context_target = Some(("/tmp/a.rs".to_string(), 1, 1, 2, 4));
    app.preview_context_menu_pos = Some((20.0, 30.0));
    app.preview_tabs = vec![tab("/tmp/a.rs")];

    keep(build_menu_ui(&app));
}

#[test]
fn menu_ui_uses_empty_context_items_for_missing_tab() {
    let mut app = app();
    app.show_preview_context_menu = true;
    app.preview_context_target = Some(("/tmp/missing.rs".to_string(), 1, 1, 2, 4));
    app.preview_context_menu_pos = Some((20.0, 30.0));
    app.preview_tabs = vec![tab("/tmp/a.rs")];

    keep(build_menu_ui(&app));
}

#[test]
fn menu_ui_builds_nav_popup_for_directory_and_file_items() {
    let mut app = app();
    app.preview_nav_popup = Some((
        "/tmp/project".to_string(),
        10.0,
        20.0,
        vec![("src".to_string(), true), ("main.rs".to_string(), false)],
    ));

    keep(build_menu_ui(&app));
}

#[test]
fn context_menu_container_style_uses_theme_background_and_border() {
    let style = context_menu_container_style(&Theme::Dark);

    let Some(Background::Color(background)) = style.background else {
        panic!("expected context menu background");
    };

    assert_eq!(background.a, 0.98);
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.color, Color::from_rgb8(0xCC, 0xCC, 0xCC));
}

#[test]
fn nav_popup_item_icon_distinguishes_directory_and_file() {
    assert_eq!(nav_popup_item_icon("src", true), Icon::FolderOpen);
    assert_eq!(nav_popup_item_icon("main.rs", false), Icon::Rust);
    assert_eq!(nav_popup_item_icon("archive.bin", false), Icon::Document);
}

#[test]
fn nav_popup_item_message_closes_popup_then_opens_target() {
    assert_close_nav_then(
        nav_popup_item_message("/workspace", "src", true),
        "/workspace/src",
        true,
    );
    assert_close_nav_then(
        nav_popup_item_message("/workspace", "main.rs", false),
        "/workspace/main.rs",
        false,
    );
}

#[test]
fn nav_popup_item_button_style_changes_background_by_status() {
    let theme = Theme::Light;
    let active = nav_popup_item_button_style(&theme, button::Status::Active);
    let hovered = nav_popup_item_button_style(&theme, button::Status::Hovered);
    let pressed = nav_popup_item_button_style(&theme, button::Status::Pressed);

    assert!(active.background.is_none());
    assert_eq!(active.text_color, theme.palette().text);
    assert_eq!(active.border.color, Color::TRANSPARENT);
    assert!(matches!(hovered.background, Some(Background::Color(_))));
    assert!(matches!(pressed.background, Some(Background::Color(_))));
    assert_ne!(hovered.background, pressed.background);
}

#[test]
fn nav_popup_container_style_uses_popup_surface_border_and_shadow() {
    let light = nav_popup_container_style(&Theme::Light);
    let dark = nav_popup_container_style(&Theme::Dark);

    assert!(matches!(light.background, Some(Background::Color(_))));
    assert_eq!(light.border.width, 1.0);
    assert_eq!(light.shadow.color, Color::BLACK);
    assert_eq!(light.shadow.offset, iced::Vector::new(0.0, 4.0));
    assert_eq!(light.shadow.blur_radius, 12.0);
    assert_ne!(light.background, dark.background);
}
