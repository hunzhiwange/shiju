//! 验证通知面板的空态、编辑器内容与复制反馈渲染。
//! 有无详情编辑器分别构造，确保通知条目不会依赖可选正文。

use crate::app::components::notification::{
    accent_badge_style, compact_action_btn_style, copy_action_btn_style, delete_action_btn_style,
    is_dark_theme, item_style, notification_copy_label, notification_text_editor_style,
    panel_style, view,
};
use crate::app::{App, Message};
use iced::widget::button;
use iced::{Background, Color, Theme};

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        _ => panic!("expected color background"),
    }
}

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    let _ = element;
}

#[test]
fn theme_detection_splits_light_and_dark_palettes() {
    assert!(!is_dark_theme(&Theme::Light));
    assert!(is_dark_theme(&Theme::Dark));
}

#[test]
fn panel_style_uses_rounded_panel_and_theme_specific_background() {
    let light = panel_style(&Theme::Light);
    let dark = panel_style(&Theme::Dark);

    assert_eq!(light.border.radius.top_left, 22.0);
    assert_eq!(dark.border.radius.top_left, 22.0);
    assert_eq!(light.shadow.offset.y, 16.0);
    assert_eq!(dark.shadow.blur_radius, 28.0);
    assert!(background_color(dark.background).a > background_color(light.background).a);
}

#[test]
fn accent_badge_style_changes_alpha_for_dark_theme() {
    let accent = Color::from_rgb8(0x4D, 0x7C, 0xD6);
    let light = accent_badge_style(&Theme::Light, accent);
    let dark = accent_badge_style(&Theme::Dark, accent);

    assert_eq!(light.border.width, 1.0);
    assert_eq!(dark.border.radius.top_left, 999.0);
    assert!(background_color(dark.background).a > background_color(light.background).a);
    assert!(dark.border.color.a > light.border.color.a);
}

#[test]
fn item_style_sets_theme_appropriate_border_and_shadow() {
    let accent = Color::from_rgb8(0x4D, 0x7C, 0xD6);
    let light = item_style(&Theme::Light, accent);
    let dark = item_style(&Theme::Dark, accent);

    assert_eq!(light.border.radius.top_left, 16.0);
    assert_eq!(dark.border.width, 1.0);
    assert_eq!(light.shadow.offset.y, 8.0);
    assert!(dark.shadow.color.a > light.shadow.color.a);
    assert_eq!(background_color(light.background).a, 0.82);
}

#[test]
fn compact_copy_and_delete_button_styles_keep_expected_semantics() {
    let compact = compact_action_btn_style(&Theme::Light, button::Status::Active);
    let not_copied = copy_action_btn_style(&Theme::Light, button::Status::Active, false);
    let copied = copy_action_btn_style(&Theme::Light, button::Status::Active, true);
    let delete = delete_action_btn_style(&Theme::Dark, button::Status::Active);

    assert_eq!(compact.border.radius.top_left, 999.0);
    assert_eq!(not_copied.text_color, compact.text_color);
    assert_ne!(copied.text_color, compact.text_color);
    assert_ne!(delete.text_color, compact.text_color);
    assert!(delete.border.color.a > 0.0);
}

#[test]
fn copied_and_delete_styles_apply_semantic_colors_in_both_themes() {
    for theme in [Theme::Light, Theme::Dark] {
        let compact = compact_action_btn_style(&theme, button::Status::Hovered);
        let copied = copy_action_btn_style(&theme, button::Status::Hovered, true);
        let delete = delete_action_btn_style(&theme, button::Status::Hovered);

        assert_ne!(copied.text_color, compact.text_color);
        assert_ne!(delete.text_color, compact.text_color);
        assert!(copied.border.color.a > 0.0);
        assert!(delete.border.color.a > 0.0);
    }
}

#[test]
fn notification_copy_label_and_text_editor_style_reflect_copy_and_theme_state() {
    assert_eq!(notification_copy_label(false), "复制");
    assert_eq!(notification_copy_label(true), "✓");

    let light = notification_text_editor_style(&Theme::Light);
    let dark = notification_text_editor_style(&Theme::Dark);

    assert_eq!(light.border.width, 0.0);
    assert_eq!(light.background, Background::Color(Color::TRANSPARENT));
    assert_ne!(light.selection, dark.selection);
    assert!(light.selection.a > dark.selection.a);
}

#[test]
fn view_builds_empty_notification_panel() {
    let app = test_app();

    keep_element(view(&app));
}

#[test]
fn view_builds_notification_items_with_editors_and_copy_feedback() {
    let mut app = test_app();
    let _ = crate::app::message::notification::update(
        &mut app,
        crate::app::message::NotificationMessage::Add("first".to_string()),
    );
    let _ = crate::app::message::notification::update(
        &mut app,
        crate::app::message::NotificationMessage::Add("second".to_string()),
    );
    app.copied_notification_id = Some(app.notifications[1].id);

    keep_element(view(&app));
}

#[test]
fn view_builds_notification_items_without_editor_content() {
    let mut app = test_app();
    let _ = crate::app::message::notification::update(
        &mut app,
        crate::app::message::NotificationMessage::Add("plain".to_string()),
    );
    app.notification_editors.clear();

    keep_element(view(&app));
}
