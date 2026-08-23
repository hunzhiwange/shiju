//! 验证 Git 过滤控件的帮助文案、消息映射、计数与主题样式。
//! 空查询、大计数和所有开关组合用于确认输入值被原样传递。

use crate::app::{Message, message};
use iced::widget::{button, svg};
use iced::{Background, Theme};

#[test]
fn task_717_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("filter_options_tests.rs"));
}

fn app() -> crate::app::App {
    crate::app::App::new().0
}

#[test]
fn help_copy_is_stable_and_mentions_all_filter_controls() {
    assert_eq!(super::filter_options::FILTER_HELP_TITLE, "筛选选项");

    for expected in
        ["路径筛选", "已包含到提交", "已排除出提交", "新增文件", "修改文件", "删除文件", "清除筛选"]
    {
        assert!(super::filter_options::FILTER_HELP_TEXT.contains(expected), "{expected}");
    }
}

#[test]
fn view_builds_with_empty_filters_and_zero_counts() {
    let app = app();

    let _element = super::filter_options::view(&app, 0, 0, 0, 0, 0);
}

#[test]
fn view_builds_with_active_query_and_all_toggles() {
    let mut app = app();
    app.git_filter_query = "src/lib".to_string();
    app.git_filter_included = true;
    app.git_filter_excluded = true;
    app.git_filter_new = true;
    app.git_filter_modified = true;
    app.git_filter_deleted = true;

    let _element = super::filter_options::view(&app, 3, 4, 1, 2, 3);
}

#[test]
fn view_builds_with_large_counts() {
    let app = app();

    let _element = super::filter_options::view(&app, usize::MAX, usize::MAX - 1, 999, 1000, 1001);
}

#[test]
fn event_helpers_emit_git_filter_messages() {
    match super::filter_options::filter_query_changed("src".to_string()) {
        Message::Git(message::GitMessage::FilterQueryChanged(query)) => assert_eq!(query, "src"),
        other => panic!("unexpected message: {other:?}"),
    }

    for (message, expected) in [
        (super::filter_options::filter_toggle_included(true), "included"),
        (super::filter_options::filter_toggle_excluded(false), "excluded"),
        (super::filter_options::filter_toggle_new(true), "new"),
        (super::filter_options::filter_toggle_modified(false), "modified"),
        (super::filter_options::filter_toggle_deleted(true), "deleted"),
    ] {
        match (message, expected) {
            (Message::Git(message::GitMessage::FilterToggleIncluded(true)), "included") => {}
            (Message::Git(message::GitMessage::FilterToggleExcluded(false)), "excluded") => {}
            (Message::Git(message::GitMessage::FilterToggleNew(true)), "new") => {}
            (Message::Git(message::GitMessage::FilterToggleModified(false)), "modified") => {}
            (Message::Git(message::GitMessage::FilterToggleDeleted(true)), "deleted") => {}
            (other, label) => panic!("unexpected {label} message: {other:?}"),
        }
    }
}

#[test]
fn event_helpers_preserve_empty_queries_and_toggle_boolean_values() {
    match super::filter_options::filter_query_changed(String::new()) {
        Message::Git(message::GitMessage::FilterQueryChanged(query)) => assert!(query.is_empty()),
        other => panic!("unexpected empty query message: {other:?}"),
    }

    for enabled in [false, true] {
        assert_toggle_value(
            super::filter_options::filter_toggle_included(enabled),
            "included",
            enabled,
        );
        assert_toggle_value(
            super::filter_options::filter_toggle_excluded(enabled),
            "excluded",
            enabled,
        );
        assert_toggle_value(super::filter_options::filter_toggle_new(enabled), "new", enabled);
        assert_toggle_value(
            super::filter_options::filter_toggle_modified(enabled),
            "modified",
            enabled,
        );
        assert_toggle_value(
            super::filter_options::filter_toggle_deleted(enabled),
            "deleted",
            enabled,
        );
    }
}

#[test]
fn styles_cover_light_dark_and_interaction_states() {
    let light = Theme::Light;
    let dark = Theme::Dark;

    assert!(super::filter_options::help_icon_style(&light, svg::Status::Idle).color.is_some());
    assert!(super::filter_options::help_icon_style(&dark, svg::Status::Hovered).color.is_some());

    assert!(
        super::filter_options::help_button_style(&light, button::Status::Active)
            .background
            .is_none()
    );
    assert!(
        super::filter_options::help_button_style(&light, button::Status::Disabled)
            .background
            .is_none()
    );
    assert!(
        super::filter_options::help_button_style(&light, button::Status::Hovered)
            .background
            .is_some()
    );
    assert!(
        super::filter_options::help_button_style(&dark, button::Status::Pressed)
            .background
            .is_some()
    );

    let light_row = super::filter_options::filter_toggle_row_style(&light);
    let dark_row = super::filter_options::filter_toggle_row_style(&dark);

    assert_eq!(light_row.border.width, 1.0);
    assert_eq!(dark_row.border.width, 1.0);
    assert_ne!(background_alpha(light_row.background), background_alpha(dark_row.background));
}

#[test]
fn row_style_uses_light_and_dark_theme_specific_surfaces() {
    let light_row = super::filter_options::filter_toggle_row_style(&Theme::Light);
    let dark_row = super::filter_options::filter_toggle_row_style(&Theme::Dark);

    let light_background = background_color(light_row.background);
    let dark_background = background_color(dark_row.background);

    assert_eq!(light_background.a, 0.96);
    assert_eq!(light_row.border.color.a, 0.06);
    assert_eq!(light_row.border.radius.top_left, 14.0);
    assert!(dark_background.a < light_background.a);
    assert!(dark_row.border.color.a > light_row.border.color.a);
}

#[test]
fn help_button_style_keeps_geometry_stable_across_statuses() {
    for status in [
        button::Status::Active,
        button::Status::Hovered,
        button::Status::Pressed,
        button::Status::Disabled,
    ] {
        let style = super::filter_options::help_button_style(&Theme::Dark, status);

        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.radius.top_left, 999.0);
        assert_eq!(style.text_color, Theme::Dark.palette().text);
    }
}

fn assert_toggle_value(message: Message, label: &str, expected: bool) {
    match (message, label) {
        (Message::Git(message::GitMessage::FilterToggleIncluded(value)), "included") => {
            assert_eq!(value, expected);
        }
        (Message::Git(message::GitMessage::FilterToggleExcluded(value)), "excluded") => {
            assert_eq!(value, expected);
        }
        (Message::Git(message::GitMessage::FilterToggleNew(value)), "new") => {
            assert_eq!(value, expected);
        }
        (Message::Git(message::GitMessage::FilterToggleModified(value)), "modified") => {
            assert_eq!(value, expected);
        }
        (Message::Git(message::GitMessage::FilterToggleDeleted(value)), "deleted") => {
            assert_eq!(value, expected);
        }
        (other, label) => panic!("unexpected {label} message: {other:?}"),
    }
}

fn background_alpha(background: Option<Background>) -> f32 {
    background_color(background).a
}

fn background_color(background: Option<Background>) -> iced::Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}
