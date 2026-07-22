use iced::widget::{button, container};
use iced::{Background, Color, Element, Theme};

use crate::app::Message;
use crate::app::assets::Icon;
use crate::app::task::staff::{TaskBoardStaffLevel, TaskBoardStaffRole, TaskBoardStaffStatus};

use super::*;

fn assert_color_eq(actual: Color, expected: Color) {
    assert_eq!(
        (actual.r, actual.g, actual.b, actual.a),
        (expected.r, expected.g, expected.b, expected.a)
    );
}

fn background_color(style: container::Style) -> Color {
    match style.background {
        Some(Background::Color(color)) => color,
        other => panic!("expected solid background, got {other:?}"),
    }
}

#[test]
fn staff_role_labels_cover_every_role() {
    let labels = TaskBoardStaffRole::all().map(role_label);

    assert_eq!(labels, ["指挥调度", "规划员工", "执行员工", "审核员工", "合并员工", "观察员"]);
}

#[test]
fn staff_status_labels_cover_enabled_and_disabled_states() {
    assert_eq!(staff_status_label(TaskBoardStaffStatus::Online, true), "在线");
    assert_eq!(staff_status_label(TaskBoardStaffStatus::Working, true), "工作中");
    assert_eq!(staff_status_label(TaskBoardStaffStatus::Blocked, true), "阻塞");
    assert_eq!(staff_status_label(TaskBoardStaffStatus::Offline, true), "离线");
    assert_eq!(staff_status_label(TaskBoardStaffStatus::Working, false), "已停用");
}

#[test]
fn staff_level_colors_cover_every_level() {
    assert_color_eq(
        staff_level_color(TaskBoardStaffLevel::Principal),
        Color::from_rgb8(59, 130, 246),
    );
    assert_color_eq(
        staff_level_color(TaskBoardStaffLevel::Manager),
        Color::from_rgb8(245, 158, 11),
    );
    assert_color_eq(
        staff_level_color(TaskBoardStaffLevel::Executor),
        Color::from_rgb8(16, 185, 129),
    );
    assert_color_eq(staff_level_color(TaskBoardStaffLevel::Member), display_only_color());
}

#[test]
fn role_colors_cover_every_role() {
    let expected = [
        Color::from_rgb8(59, 130, 246),
        Color::from_rgb8(124, 58, 237),
        Color::from_rgb8(16, 185, 129),
        Color::from_rgb8(245, 158, 11),
        Color::from_rgb8(6, 182, 212),
        Color::from_rgb8(148, 163, 184),
    ];

    for (role, expected) in TaskBoardStaffRole::all().into_iter().zip(expected) {
        assert_color_eq(role_color(role), expected);
    }
}

#[test]
fn staff_status_colors_respect_enabled_flag() {
    assert_color_eq(
        staff_status_color(TaskBoardStaffStatus::Online, true),
        Color::from_rgb8(34, 197, 94),
    );
    assert_color_eq(
        staff_status_color(TaskBoardStaffStatus::Working, true),
        Color::from_rgb8(59, 130, 246),
    );
    assert_color_eq(staff_status_color(TaskBoardStaffStatus::Blocked, true), danger_color());
    assert_color_eq(
        staff_status_color(TaskBoardStaffStatus::Offline, true),
        Color::from_rgb8(100, 116, 139),
    );
    assert_color_eq(
        staff_status_color(TaskBoardStaffStatus::Online, false),
        Color::from_rgb8(100, 116, 139),
    );
}

#[test]
fn mix_color_clamps_amount_and_preserves_source_alpha() {
    let source = Color::from_rgba(0.2, 0.4, 0.6, 0.7);

    assert_color_eq(mix_color(source, Color::WHITE, -1.0), source);
    assert_color_eq(mix_color(source, Color::WHITE, 2.0), Color::from_rgba(1.0, 1.0, 1.0, 0.7));
    assert_color_eq(mix_color(source, Color::BLACK, 0.5), Color::from_rgba(0.1, 0.2, 0.3, 0.7));
}

#[test]
fn theme_detection_and_surface_colors_cover_dark_and_light() {
    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
    assert_color_eq(surface_color(&Theme::Dark), Color::from_rgb8(23, 27, 34));
    assert_color_eq(surface_color(&Theme::Light), Color::from_rgb8(248, 250, 252));
}

#[test]
fn font_scrollbar_and_text_helpers_build_expected_variants() {
    assert_eq!(bold_font().weight, iced::font::Weight::Bold);
    assert_eq!(medium_font().weight, iced::font::Weight::Medium);
    assert!(muted_text_style(&Theme::Dark).color.is_some());
    std::hint::black_box(vertical_scrollbar());
}

#[test]
fn empty_and_warning_panels_build_for_both_themes() {
    for theme in [Theme::Dark, Theme::Light] {
        assert!(empty_panel_style(&theme).background.is_some());
        assert!(warning_panel_style(&theme).background.is_some());
        assert!(warning_panel_style(&theme).text_color.is_some());
    }
}

#[test]
fn panel_styles_use_distinct_requested_alpha_values() {
    let side = background_color(side_panel_style(&Theme::Dark));
    let center = background_color(center_panel_style(&Theme::Dark));
    let root = background_color(root_style(&Theme::Dark));
    let light_side = side_panel_style(&Theme::Light);

    assert_eq!(side.a, 0.74);
    assert_eq!(center.a, 0.82);
    assert_eq!(root.a, 0.58);
    assert_eq!(light_side.shadow.blur_radius, 22.0);
    assert_eq!(light_side.shadow.color.a, 0.08);
}

#[test]
fn toggle_and_card_styles_cover_boolean_branches() {
    let accent = role_color(TaskBoardStaffRole::Executor);

    assert_ne!(
        background_color(autonomous_toggle_style(&Theme::Dark, true)),
        background_color(autonomous_toggle_style(&Theme::Dark, false))
    );
    assert_ne!(
        background_color(lane_card_style(&Theme::Light, accent, true)),
        background_color(lane_card_style(&Theme::Light, accent, false))
    );
    assert_ne!(
        task_card_style(&Theme::Dark, accent, true).border.color,
        task_card_style(&Theme::Dark, accent, false).border.color
    );
    assert_ne!(
        member_card_style(&Theme::Light, accent, true).border.color,
        member_card_style(&Theme::Light, accent, false).border.color
    );
}

#[test]
fn button_styles_cover_all_statuses_and_selection_branches() {
    let statuses = [
        button::Status::Active,
        button::Status::Hovered,
        button::Status::Pressed,
        button::Status::Disabled,
    ];
    let accent = priority_color();

    for status in statuses {
        std::hint::black_box(icon_button_style(&Theme::Dark, status));
        std::hint::black_box(tab_button_style(&Theme::Light, status, false));
        std::hint::black_box(assignment_member_button_style(&Theme::Dark, status, false, accent));
        std::hint::black_box(chip_button_style(&Theme::Light, status, accent));
    }
    std::hint::black_box(tab_button_style(&Theme::Dark, button::Status::Active, true));
    std::hint::black_box(tab_button_style(&Theme::Light, button::Status::Active, true));
    std::hint::black_box(assignment_member_button_style(
        &Theme::Dark,
        button::Status::Active,
        true,
        accent,
    ));
    std::hint::black_box(assignment_member_button_style(
        &Theme::Light,
        button::Status::Active,
        true,
        accent,
    ));
}

#[test]
fn chip_style_covers_strong_weak_and_dark_light_text_colors() {
    let accent = priority_color();
    let strong_dark = chip_style(&Theme::Dark, accent, true);
    let weak_light = chip_style(&Theme::Light, accent, false);

    assert_eq!(background_color(strong_dark).a, 0.20);
    assert_eq!(background_color(weak_light).a, 0.14);
    assert!(strong_dark.text_color.is_some());
    assert!(weak_light.text_color.is_some());
}

#[test]
fn style_elements_build_for_enabled_and_disabled_visuals() {
    let accent = warning_color();
    let elements: Vec<Element<'_, Message>> = vec![
        count_chip("3".to_string(), accent),
        small_chip("候补".to_string(), accent),
        status_dot(accent),
        tooltip(count_chip("1".to_string(), accent), "计数"),
    ];
    for element in elements {
        std::hint::black_box(element);
    }
    std::hint::black_box(count_chip("3".to_string(), accent));
    std::hint::black_box(small_chip("候补".to_string(), accent));
    std::hint::black_box(status_dot(accent));
    std::hint::black_box(tooltip(count_chip("1".to_string(), accent), "计数"));
    std::hint::black_box(tooltip_content("提示"));
    std::hint::black_box(icon(Icon::Gear, 14.0, true));
    std::hint::black_box(icon(Icon::Gear, 14.0, false));
}
