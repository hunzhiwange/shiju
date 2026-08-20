use iced::widget::{button, text_editor, text_input};
use iced::{Background, Color, Element, Length, Theme};

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected solid color background, got {other:?}"),
    }
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("ui_tests"));
}

#[test]
fn icon_svg_uses_requested_square_size() {
    let element: Element<'_, crate::app::Message> =
        super::icon_svg(crate::app::assets::Icon::X, 14.0).into();
    let size = element.as_widget().size();

    assert_eq!(size.width, Length::Fixed(14.0));
    assert_eq!(size.height, Length::Fixed(14.0));
}

#[test]
fn action_icon_maps_every_supported_label_and_falls_back_for_other_text() {
    use crate::app::assets::Icon;

    let cases = [
        ("打开", Icon::ChevronRight),
        ("打开最近", Icon::ArrowClockwise),
        ("打开文件夹", Icon::FolderOpen),
        ("项目管理", Icon::Gear),
        ("添加", Icon::Plus),
        ("独立窗口", Icon::Box),
        ("浏览器", Icon::ArrowUp),
        ("编辑", Icon::Pencil),
        ("取消", Icon::X),
        ("删除", Icon::Trash),
        ("保存", Icon::Save),
    ];

    for (label, expected) in cases {
        assert_eq!(super::action_icon(label), expected);
    }
    assert_eq!(super::action_icon(""), Icon::ChevronRight);
    assert_eq!(super::action_icon("未知操作"), Icon::ChevronRight);
}

#[test]
fn dark_theme_detection_matches_builtin_theme_brightness() {
    assert!(super::is_dark_theme(&Theme::Dark));
    assert!(!super::is_dark_theme(&Theme::Light));
}

#[test]
fn cool_icon_button_style_maps_each_status_to_surface_and_elevation() {
    let cases = [
        (button::Status::Active, Color::from_rgba8(24, 24, 24, 0.92), 0.30),
        (button::Status::Hovered, Color::from_rgba8(34, 34, 34, 0.95), 0.30),
        (button::Status::Pressed, Color::from_rgba8(44, 44, 44, 0.95), 0.30),
        (button::Status::Disabled, Color::from_rgba8(24, 24, 24, 0.55), 0.0),
    ];

    for (status, expected_background, shadow_alpha) in cases {
        let style = super::cool_icon_button_style(&Theme::Light, status);

        assert_eq!(background_color(style.background), expected_background);
        assert_eq!(style.text_color, Color::WHITE);
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.color, Color::TRANSPARENT);
        assert_eq!(style.border.radius.top_left, 999.0);
        assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(shadow_alpha));
        assert_eq!(style.shadow.offset, iced::Vector::new(0.0, 4.0));
        assert_eq!(style.shadow.blur_radius, 12.0);
    }
}

#[test]
fn tile_button_style_covers_dark_theme_status_surfaces() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let primary = theme.palette().primary;
    let cases = [
        (
            button::Status::Active,
            palette.background.base.color.scale_alpha(0.92),
            palette.background.strong.color.scale_alpha(0.9),
        ),
        (
            button::Status::Hovered,
            palette.background.weak.color.scale_alpha(0.28),
            primary.scale_alpha(0.34),
        ),
        (
            button::Status::Pressed,
            palette.background.strong.color.scale_alpha(0.32),
            primary.scale_alpha(0.42),
        ),
        (
            button::Status::Disabled,
            palette.background.base.color.scale_alpha(0.92),
            palette.background.strong.color.scale_alpha(0.9),
        ),
    ];

    for (status, expected_background, expected_border) in cases {
        let style = super::tile_button_style(&theme, status);

        assert_eq!(background_color(style.background), expected_background);
        assert_eq!(style.border.color, expected_border);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.radius.top_left, 20.0);
        assert_eq!(style.text_color, theme.palette().text);
        assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(0.18));
        assert_eq!(style.shadow.offset, iced::Vector::new(0.0, 12.0));
        assert_eq!(style.shadow.blur_radius, 24.0);
    }
}

#[test]
fn tile_button_style_covers_light_theme_status_surfaces() {
    let theme = Theme::Light;
    let palette = theme.extended_palette();
    let primary = theme.palette().primary;
    let cases = [
        (
            button::Status::Active,
            Color::from_rgba8(255, 255, 255, 0.88),
            Color::from_rgba8(15, 23, 42, 0.08),
        ),
        (button::Status::Hovered, Color::WHITE.scale_alpha(0.96), primary.scale_alpha(0.20)),
        (
            button::Status::Pressed,
            palette.background.weak.color.scale_alpha(0.92),
            primary.scale_alpha(0.26),
        ),
        (
            button::Status::Disabled,
            Color::from_rgba8(255, 255, 255, 0.88),
            Color::from_rgba8(15, 23, 42, 0.08),
        ),
    ];

    for (status, expected_background, expected_border) in cases {
        let style = super::tile_button_style(&theme, status);

        assert_eq!(background_color(style.background), expected_background);
        assert_eq!(style.border.color, expected_border);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.radius.top_left, 20.0);
        assert_eq!(style.text_color, theme.palette().text);
        assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(0.08));
        assert_eq!(style.shadow.offset, iced::Vector::new(0.0, 12.0));
        assert_eq!(style.shadow.blur_radius, 24.0);
    }
}

#[test]
fn primary_and_icon_button_styles_preserve_shared_styles_except_radius() {
    let themes = [Theme::Dark, Theme::Light];
    let statuses = [
        button::Status::Active,
        button::Status::Hovered,
        button::Status::Pressed,
        button::Status::Disabled,
    ];

    for theme in &themes {
        for status in statuses {
            let mut expected_primary = super::settings_primary_button_style(theme, status);
            expected_primary.border.radius = 12.0.into();
            assert_eq!(super::primary_button_style(theme, status), expected_primary);

            let mut expected_icon = super::settings_rounded_button_style(theme, status);
            expected_icon.border.radius = 12.0.into();
            assert_eq!(super::icon_button_style(theme, status), expected_icon);
        }
    }
}

#[test]
fn input_and_editor_styles_delegate_all_statuses_to_shared_settings_styles() {
    let themes = [Theme::Dark, Theme::Light];
    let input_statuses = [
        text_input::Status::Active,
        text_input::Status::Hovered,
        text_input::Status::Focused { is_hovered: false },
        text_input::Status::Focused { is_hovered: true },
        text_input::Status::Disabled,
    ];
    let editor_statuses = [
        text_editor::Status::Active,
        text_editor::Status::Hovered,
        text_editor::Status::Focused { is_hovered: false },
        text_editor::Status::Focused { is_hovered: true },
        text_editor::Status::Disabled,
    ];

    for theme in &themes {
        for status in input_statuses {
            assert_eq!(
                super::figma_text_input_style(theme, status),
                super::settings_input_style(theme, status)
            );
        }
        for status in editor_statuses {
            assert_eq!(
                super::figma_text_editor_style(theme, status),
                super::settings_editor_style(theme, status)
            );
        }
    }
}

#[test]
fn tooltip_helpers_wrap_exactly_one_content_widget() {
    let text_tooltip: Element<'_, crate::app::Message> = super::tooltip_bubble("tip").into();
    let rich_tooltip: Element<'_, crate::app::Message> =
        super::tooltip_bubble_el(iced::widget::text("rich").into()).into();

    assert!(text_tooltip.as_widget().children().is_empty());
    assert!(rich_tooltip.as_widget().children().is_empty());
}

#[test]
fn tile_keeps_empty_and_multiple_action_rows_in_its_component_tree() {
    let empty = super::tile(
        crate::app::assets::Icon::LayoutTextWindow,
        "Empty actions".to_string(),
        Color::from_rgb8(1, 2, 3),
        vec![],
        crate::app::Message::None,
    );
    let populated = super::tile(
        crate::app::assets::Icon::LayoutTextWindow,
        "Two actions".to_string(),
        Color::from_rgb8(3, 2, 1),
        vec![("打开", crate::app::Message::None), ("未知操作", crate::app::Message::None)],
        crate::app::Message::None,
    );

    let empty_button_children = empty.as_widget().children();
    let empty_content_children = &empty_button_children[0].children;
    assert_eq!(empty_content_children.len(), 3);
    assert_eq!(empty_content_children[2].children.len(), 0);

    let populated_button_children = populated.as_widget().children();
    let populated_content_children = &populated_button_children[0].children;
    assert_eq!(populated_content_children.len(), 3);
    assert_eq!(populated_content_children[2].children.len(), 2);
}
