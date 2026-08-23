//! 验证设置页按钮、输入控件、选择器与弹窗的完整主题状态。
//! 测试逐项比较交互状态，尤其锁定禁用、聚焦和菜单展开时的差异。

use super::{
    danger_action_btn_style, primary_action_btn_style, round_icon_btn_style,
    rounded_action_btn_style, settings_checkbox_style, settings_modal_backdrop_style,
    settings_modal_card_style, settings_muted_text_style, settings_panel_style,
    settings_pick_list_menu_style, settings_pick_list_style, settings_segment_button_style,
    settings_text_editor_style, settings_text_input_style,
};
use iced::widget::{button, checkbox, pick_list, text_editor, text_input};
use iced::{Background, Color, Theme};

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected solid color background, got {other:?}"),
    }
}

fn solid_background(background: Background) -> Color {
    match background {
        Background::Color(color) => color,
        other => panic!("expected solid color background, got {other:?}"),
    }
}

#[test]
fn action_button_styles_produce_text_colors_for_dark_theme() {
    let theme = Theme::Dark;

    assert_ne!(
        rounded_action_btn_style(&theme, button::Status::Active).text_color,
        iced::Color::TRANSPARENT
    );
    assert_ne!(
        primary_action_btn_style(&theme, button::Status::Active).text_color,
        iced::Color::TRANSPARENT
    );
    assert_ne!(
        danger_action_btn_style(&theme, button::Status::Active).text_color,
        iced::Color::TRANSPARENT
    );
}

#[test]
fn action_button_styles_cover_light_hovered_pressed_and_disabled_statuses() {
    let theme = Theme::Light;
    for status in [
        button::Status::Active,
        button::Status::Hovered,
        button::Status::Pressed,
        button::Status::Disabled,
    ] {
        assert_eq!(rounded_action_btn_style(&theme, status).border.width, 1.0);
        assert_eq!(round_icon_btn_style(&theme, status).border.width, 1.0);
        assert_eq!(primary_action_btn_style(&theme, status).border.radius.top_left, 8.0);
        assert_eq!(danger_action_btn_style(&theme, status).border.radius.top_left, 8.0);
    }
}

#[test]
fn rounded_and_round_icon_button_styles_change_background_by_status() {
    let theme = Theme::Dark;

    assert_ne!(
        background_color(rounded_action_btn_style(&theme, button::Status::Active).background),
        background_color(rounded_action_btn_style(&theme, button::Status::Hovered).background)
    );
    assert_ne!(
        background_color(round_icon_btn_style(&theme, button::Status::Active).background),
        background_color(round_icon_btn_style(&theme, button::Status::Pressed).background)
    );
}

#[test]
fn primary_and_danger_action_button_styles_adjust_for_pressed_state() {
    let theme = Theme::Light;

    assert_ne!(
        background_color(primary_action_btn_style(&theme, button::Status::Active).background),
        background_color(primary_action_btn_style(&theme, button::Status::Pressed).background)
    );
    assert_ne!(
        background_color(danger_action_btn_style(&theme, button::Status::Active).background),
        background_color(danger_action_btn_style(&theme, button::Status::Pressed).background)
    );
}

#[test]
fn muted_text_style_uses_different_alpha_between_light_and_dark_themes() {
    let dark = settings_muted_text_style(&Theme::Dark).color.expect("dark muted text color");
    let light = settings_muted_text_style(&Theme::Light).color.expect("light muted text color");

    assert!(dark.a > light.a);
}

#[test]
fn panel_and_modal_styles_keep_expected_border_and_shadow_shape() {
    let panel = settings_panel_style(&Theme::Dark);
    let modal = settings_modal_card_style(&Theme::Light);

    assert_eq!(panel.border.width, 1.0);
    assert_eq!(panel.shadow.offset.y, 3.0);
    assert_eq!(modal.border.width, 1.0);
    assert_eq!(modal.shadow.offset.y, 24.0);
}

#[test]
fn modal_backdrop_style_uses_stronger_opacity_for_dark_theme() {
    let dark = solid_background(
        settings_modal_backdrop_style(&Theme::Dark).background.expect("dark backdrop background"),
    );
    let light = solid_background(
        settings_modal_backdrop_style(&Theme::Light).background.expect("light backdrop background"),
    );

    assert!(dark.a > light.a);
}

#[test]
fn text_input_style_covers_active_hovered_focused_and_disabled_states() {
    let theme = Theme::Dark;
    let active = settings_text_input_style(&theme, text_input::Status::Active);
    let hovered = settings_text_input_style(&theme, text_input::Status::Hovered);
    let focused =
        settings_text_input_style(&theme, text_input::Status::Focused { is_hovered: true });
    let disabled = settings_text_input_style(&theme, text_input::Status::Disabled);

    assert_ne!(active.border.color, hovered.border.color);
    assert_ne!(hovered.border.color, focused.border.color);
    assert_ne!(disabled.value, focused.value);
}

#[test]
fn text_input_style_covers_light_theme_and_focused_without_hover() {
    let theme = Theme::Light;
    let active = settings_text_input_style(&theme, text_input::Status::Active);
    let focused =
        settings_text_input_style(&theme, text_input::Status::Focused { is_hovered: false });
    let focused_hovered =
        settings_text_input_style(&theme, text_input::Status::Focused { is_hovered: true });
    let disabled = settings_text_input_style(&theme, text_input::Status::Disabled);

    assert_ne!(solid_background(active.background), solid_background(focused.background));
    assert_eq!(focused.border.color, focused_hovered.border.color);
    assert_ne!(disabled.value, active.value);
}

#[test]
fn text_editor_style_tracks_theme_palette() {
    let dark =
        settings_text_editor_style(&Theme::Dark, text_editor::Status::Focused { is_hovered: true });
    let light = settings_text_editor_style(
        &Theme::Light,
        text_editor::Status::Focused { is_hovered: true },
    );

    assert_ne!(solid_background(dark.background), solid_background(light.background));
    assert_ne!(dark.border.color, light.border.color);
}

#[test]
fn pick_list_styles_change_for_hover_and_open_states() {
    let theme = Theme::Light;
    let active = settings_pick_list_style(&theme, pick_list::Status::Active);
    let hovered = settings_pick_list_style(&theme, pick_list::Status::Hovered);
    let opened = settings_pick_list_style(&theme, pick_list::Status::Opened { is_hovered: true });

    assert_ne!(solid_background(active.background), solid_background(hovered.background));
    assert_ne!(hovered.border.color, opened.border.color);
    assert_ne!(hovered.handle_color, opened.handle_color);
}

#[test]
fn pick_list_style_covers_dark_open_without_hover() {
    let theme = Theme::Dark;
    let active = settings_pick_list_style(&theme, pick_list::Status::Active);
    let opened = settings_pick_list_style(&theme, pick_list::Status::Opened { is_hovered: false });

    assert_ne!(solid_background(active.background), solid_background(opened.background));
    assert_ne!(active.handle_color, opened.handle_color);
}

#[test]
fn pick_list_menu_style_uses_selected_background_and_shadow() {
    let style = settings_pick_list_menu_style(&Theme::Dark);

    assert_ne!(solid_background(style.background), solid_background(style.selected_background));
    assert_eq!(style.shadow.offset.y, 10.0);
}

#[test]
fn checkbox_style_distinguishes_checked_hovered_and_disabled_states() {
    let theme = Theme::Dark;
    let active = settings_checkbox_style(&theme, checkbox::Status::Active { is_checked: false });
    let hovered = settings_checkbox_style(&theme, checkbox::Status::Hovered { is_checked: false });
    let checked = settings_checkbox_style(&theme, checkbox::Status::Active { is_checked: true });
    let disabled = settings_checkbox_style(&theme, checkbox::Status::Disabled { is_checked: true });

    assert_ne!(solid_background(active.background), solid_background(hovered.background));
    assert_eq!(checked.icon_color, Color::WHITE);
    assert_ne!(checked.border.color, disabled.border.color);
}

#[test]
fn checkbox_style_covers_light_disabled_unchecked_and_hovered_checked() {
    let theme = Theme::Light;
    let disabled_unchecked =
        settings_checkbox_style(&theme, checkbox::Status::Disabled { is_checked: false });
    let hovered_checked =
        settings_checkbox_style(&theme, checkbox::Status::Hovered { is_checked: true });
    let active_checked =
        settings_checkbox_style(&theme, checkbox::Status::Active { is_checked: true });

    assert_ne!(
        solid_background(disabled_unchecked.background),
        solid_background(hovered_checked.background)
    );
    assert_eq!(hovered_checked.icon_color, Color::WHITE);
    assert_eq!(hovered_checked.border.color, active_checked.border.color);
}

#[test]
fn segment_button_style_distinguishes_active_and_inactive_states() {
    let theme = Theme::Light;
    let inactive = settings_segment_button_style(&theme, button::Status::Active, false);
    let hovered = settings_segment_button_style(&theme, button::Status::Hovered, false);
    let active = settings_segment_button_style(&theme, button::Status::Active, true);

    assert_ne!(background_color(inactive.background), background_color(hovered.background));
    assert_ne!(inactive.text_color, active.text_color);
    assert_ne!(inactive.border.color, active.border.color);
}

#[test]
fn segment_button_style_covers_dark_pressed_and_active() {
    let theme = Theme::Dark;
    let inactive = settings_segment_button_style(&theme, button::Status::Active, false);
    let pressed = settings_segment_button_style(&theme, button::Status::Pressed, false);
    let active = settings_segment_button_style(&theme, button::Status::Disabled, true);

    assert_ne!(background_color(inactive.background), background_color(pressed.background));
    assert_ne!(pressed.text_color, active.text_color);
}

#[test]
fn rounded_and_round_icon_button_styles_cover_remaining_dark_statuses() {
    let theme = Theme::Dark;
    let rounded_active =
        background_color(rounded_action_btn_style(&theme, button::Status::Active).background);
    let rounded_pressed =
        background_color(rounded_action_btn_style(&theme, button::Status::Pressed).background);
    let rounded_disabled =
        background_color(rounded_action_btn_style(&theme, button::Status::Disabled).background);
    let round_hovered =
        background_color(round_icon_btn_style(&theme, button::Status::Hovered).background);
    let round_disabled =
        background_color(round_icon_btn_style(&theme, button::Status::Disabled).background);

    assert_ne!(rounded_active, rounded_pressed);
    assert_eq!(rounded_active, rounded_disabled);
    assert_ne!(round_hovered, round_disabled);
}

#[test]
fn panel_and_modal_styles_cover_light_and_dark_branches() {
    let dark_panel = settings_panel_style(&Theme::Dark);
    let light_panel = settings_panel_style(&Theme::Light);
    let dark_modal = settings_modal_card_style(&Theme::Dark);
    let light_modal = settings_modal_card_style(&Theme::Light);

    assert_ne!(background_color(dark_panel.background), background_color(light_panel.background));
    assert_ne!(dark_panel.border.color, light_panel.border.color);
    assert_ne!(background_color(dark_modal.background), background_color(light_modal.background));
    assert_ne!(dark_modal.shadow.color, light_modal.shadow.color);
}

#[test]
fn text_input_style_covers_light_hovered_background() {
    let theme = Theme::Light;
    let active = settings_text_input_style(&theme, text_input::Status::Active);
    let hovered = settings_text_input_style(&theme, text_input::Status::Hovered);

    assert_ne!(solid_background(active.background), solid_background(hovered.background));
    assert_ne!(active.border.color, hovered.border.color);
}

#[test]
fn text_editor_style_accepts_all_statuses_without_changing_output() {
    let theme = Theme::Dark;
    let active = settings_text_editor_style(&theme, text_editor::Status::Active);
    let hovered = settings_text_editor_style(&theme, text_editor::Status::Hovered);
    let disabled = settings_text_editor_style(&theme, text_editor::Status::Disabled);

    assert_eq!(active, hovered);
    assert_eq!(hovered, disabled);
}

#[test]
fn pick_list_styles_cover_dark_hovered_and_light_opened_without_hover() {
    let dark_active = settings_pick_list_style(&Theme::Dark, pick_list::Status::Active);
    let dark_hovered = settings_pick_list_style(&Theme::Dark, pick_list::Status::Hovered);
    let light_active = settings_pick_list_style(&Theme::Light, pick_list::Status::Active);
    let light_opened =
        settings_pick_list_style(&Theme::Light, pick_list::Status::Opened { is_hovered: false });

    assert_ne!(solid_background(dark_active.background), solid_background(dark_hovered.background));
    assert_ne!(dark_active.border.color, dark_hovered.border.color);
    assert_ne!(
        solid_background(light_active.background),
        solid_background(light_opened.background)
    );
    assert_ne!(light_active.handle_color, light_opened.handle_color);
}

#[test]
fn pick_list_menu_style_covers_light_theme_branch() {
    let dark = settings_pick_list_menu_style(&Theme::Dark);
    let light = settings_pick_list_menu_style(&Theme::Light);

    assert_ne!(solid_background(dark.background), solid_background(light.background));
    assert_ne!(dark.border.color, light.border.color);
    assert_ne!(dark.selected_background, light.selected_background);
    assert_ne!(dark.shadow.color, light.shadow.color);
}

#[test]
fn checkbox_style_covers_unchecked_light_active_and_dark_disabled() {
    let light_active =
        settings_checkbox_style(&Theme::Light, checkbox::Status::Active { is_checked: false });
    let light_hovered =
        settings_checkbox_style(&Theme::Light, checkbox::Status::Hovered { is_checked: false });
    let dark_disabled =
        settings_checkbox_style(&Theme::Dark, checkbox::Status::Disabled { is_checked: false });

    assert_ne!(
        solid_background(light_active.background),
        solid_background(light_hovered.background)
    );
    assert_ne!(
        solid_background(light_active.background),
        solid_background(dark_disabled.background)
    );
    assert_eq!(light_active.icon_color, Color::TRANSPARENT);
}

#[test]
fn checkbox_style_dims_text_only_when_disabled() {
    let theme = Theme::Dark;
    let active = settings_checkbox_style(&theme, checkbox::Status::Active { is_checked: false });
    let disabled =
        settings_checkbox_style(&theme, checkbox::Status::Disabled { is_checked: false });

    assert_eq!(active.text_color, Some(theme.palette().text.scale_alpha(0.94)));
    assert_eq!(disabled.text_color, Some(theme.palette().text.scale_alpha(0.48)));
}

#[test]
fn segment_button_style_covers_light_pressed_and_dark_hovered_branches() {
    let light_active = settings_segment_button_style(&Theme::Light, button::Status::Active, false);
    let light_pressed =
        settings_segment_button_style(&Theme::Light, button::Status::Pressed, false);
    let dark_active = settings_segment_button_style(&Theme::Dark, button::Status::Active, false);
    let dark_hovered = settings_segment_button_style(&Theme::Dark, button::Status::Hovered, false);

    assert_ne!(
        background_color(light_active.background),
        background_color(light_pressed.background)
    );
    assert_ne!(background_color(dark_active.background), background_color(dark_hovered.background));
}
