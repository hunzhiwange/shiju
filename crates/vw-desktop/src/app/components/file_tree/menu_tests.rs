//! 验证文件树菜单按查找与剪贴板状态组合条目并适配主题样式。
//! 复制和剪切共享粘贴入口，但无剪贴板内容时不得出现不可执行菜单项。

use super::menu::{
    build_file_tree_menu, file_tree_menu_button_style, file_tree_menu_container_style,
    file_tree_menu_separator_style, file_tree_menu_shortcut,
};
use crate::app::App;
use crate::app::Message;
use crate::app::message::project::{FileTreeAction, ProjectMessage};
use crate::app::state::{FileTreeClipboard, FileTreeClipboardMode};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::{Layout, layout, mouse, widget::Tree};
use iced::keyboard::{Key, Modifiers, key::Named};
use iced::widget::button;
use iced::{Background, Color, Element, Font, Pixels, Rectangle, Size, Theme, Vector};

fn app_with_clipboard(mode: FileTreeClipboardMode) -> App {
    let mut app = App::new().0;
    app.file_tree_clipboard =
        Some(FileTreeClipboard { mode, src_paths: vec!["fixture/path".to_string()] });
    app
}

fn menu_item_count(element: &Element<'_, crate::app::Message>) -> usize {
    let children = element.as_widget().children();
    children.len()
}

#[test]
fn file_tree_menu_shortcuts_require_the_bound_key_and_modifiers() {
    let command = Modifiers::COMMAND;
    let none = Modifiers::NONE;
    let platform_keys = if cfg!(target_os = "macos") {
        [
            (Named::ArrowDown, command, FileTreeAction::Open),
            (Named::Enter, none, FileTreeAction::Rename),
            (Named::Backspace, command, FileTreeAction::Delete),
        ]
    } else {
        [
            (Named::Enter, none, FileTreeAction::Open),
            (Named::F2, none, FileTreeAction::Rename),
            (Named::Delete, none, FileTreeAction::Delete),
        ]
    };
    let clipboard_keys = [
        ("x", FileTreeAction::Cut),
        ("C", FileTreeAction::Copy),
        ("d", FileTreeAction::Duplicate),
        ("v", FileTreeAction::Paste),
    ];

    let bindings = clipboard_keys
        .into_iter()
        .map(|(key, action)| (Key::Character(key.into()), command, action))
        .chain(
            platform_keys
                .into_iter()
                .map(|(key, modifiers, action)| (Key::Named(key), modifiers, action)),
        );
    for (key, modifiers, expected) in bindings {
        assert!(matches!(
            file_tree_menu_shortcut(&key, modifiers, true),
            Some(Message::Project(ProjectMessage::FileTreeAction(action))) if action == expected
        ));
        assert!(file_tree_menu_shortcut(&key, modifiers | Modifiers::SHIFT, true).is_none());
    }
    assert!(file_tree_menu_shortcut(&Key::Character("d".into()), none, true).is_none());
    assert!(file_tree_menu_shortcut(&Key::Character("v".into()), command, false).is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn file_tree_menu_renders_shortcuts_in_light_and_dark_themes() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let app = app_with_clipboard(FileTreeClipboardMode::Copy);
    let mut menu = build_file_tree_menu(&app, true);
    let size = Size::new(220.0, 600.0);
    let mut tree = Tree::new(menu.as_widget());
    let node =
        menu.as_widget_mut().layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, size));
    let viewport = Rectangle::with_size(size);
    let menu_column = Layout::new(&node).children().next().unwrap();
    let shortcut_bounds: Vec<_> = menu_column
        .children()
        .filter_map(|item| item.children().next()?.children().next()?.children().nth(1))
        .map(|shortcut| shortcut.bounds())
        .collect();

    assert_eq!(shortcut_bounds.len(), 7);
    let right_edge = shortcut_bounds[0].x + shortcut_bounds[0].width;
    assert!(
        shortcut_bounds.iter().all(|bounds| (bounds.x + bounds.width - right_edge).abs() < 0.01)
    );

    for theme in [Theme::Light, Theme::Dark] {
        menu.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &Style { text_color: theme.palette().text },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &viewport,
        );
        let pixels = renderer.screenshot(Size::new(220, 600), 1.0, Color::TRANSPARENT);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] == 255));
    }
}

#[test]
fn file_tree_menu_without_optional_items_has_base_structure() {
    let app = App::new().0;

    let menu = build_file_tree_menu(&app, false);

    assert_eq!(menu_item_count(&menu), 16);
}

#[test]
fn file_tree_menu_find_option_adds_one_item() {
    let app = App::new().0;

    let without_find = build_file_tree_menu(&app, false);
    let with_find = build_file_tree_menu(&app, true);

    assert_eq!(menu_item_count(&without_find), 16);
    assert_eq!(menu_item_count(&with_find), 17);
}

#[test]
fn file_tree_menu_clipboard_adds_paste_for_copy_and_cut_modes() {
    let empty_app = App::new().0;
    let copy_app = app_with_clipboard(FileTreeClipboardMode::Copy);
    let cut_app = app_with_clipboard(FileTreeClipboardMode::Cut);

    let without_clipboard = build_file_tree_menu(&empty_app, false);
    let with_copy = build_file_tree_menu(&copy_app, false);
    let with_cut = build_file_tree_menu(&cut_app, false);

    assert_eq!(menu_item_count(&without_clipboard), 16);
    assert_eq!(menu_item_count(&with_copy), 17);
    assert_eq!(menu_item_count(&with_cut), 17);
}

#[test]
fn file_tree_menu_combines_find_and_paste_options() {
    let app = app_with_clipboard(FileTreeClipboardMode::Copy);

    let menu = build_file_tree_menu(&app, true);

    assert_eq!(menu_item_count(&menu), 18);
}

#[test]
fn file_tree_menu_button_style_maps_interaction_states_for_each_theme() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let active = file_tree_menu_button_style(&theme, button::Status::Active);
        let hovered = file_tree_menu_button_style(&theme, button::Status::Hovered);
        let pressed = file_tree_menu_button_style(&theme, button::Status::Pressed);

        assert_eq!(active.background, Some(Background::Color(Color::TRANSPARENT)));
        assert_eq!(hovered.background, Some(Background::Color(palette.background.weak.color)));
        assert_eq!(pressed.background, Some(Background::Color(palette.background.strong.color)));
        assert_eq!(active.text_color, theme.palette().text);
        assert_eq!(active.border.radius, 4.0.into());
    }
}

#[test]
fn file_tree_menu_separator_style_uses_strong_theme_background() {
    for theme in [Theme::Light, Theme::Dark] {
        let style = file_tree_menu_separator_style(&theme);

        assert_eq!(
            style.background,
            Some(Background::Color(theme.extended_palette().background.strong.color))
        );
    }
}

#[test]
fn file_tree_menu_container_style_keeps_theme_surface_and_geometry() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let style = file_tree_menu_container_style(&theme);

        assert_eq!(style.background, Some(Background::Color(palette.background.base.color)));
        assert_eq!(style.border.color, palette.background.strong.color);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.radius, 8.0.into());
        assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(0.1));
        assert_eq!(style.shadow.offset, Vector::new(0.0, 4.0));
        assert_eq!(style.shadow.blur_radius, 12.0);
    }
}
