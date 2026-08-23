//! 验证账户状态菜单、触发标签、滚动条及明暗主题对比度契约。
//! 视图构造覆盖登出、活动和忙碌状态，并显式检查敏感输入采用掩码控件。

use super::*;

fn background_color(style: iced::widget::button::Style) -> Color {
    match style.background {
        Some(Background::Color(color)) => color,
        _ => panic!("expected button background color"),
    }
}

fn luminance(color: Color) -> f32 {
    0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b
}

#[test]
fn account_menu_uses_masked_input_and_four_pixel_scrollbar() {
    assert_eq!(ACCOUNT_SCROLLBAR_WIDTH, 4);
    let (mut app, _) = App::new();
    app.account.manual_input = crate::app::account::ManualSkeyInput::new("never-render-plain");
    let _ = view(&app);
    assert_eq!(app.account.manual_input.expose(), "never-render-plain");
}

#[test]
fn account_menu_install_entry_opens_the_install_page() {
    assert!(matches!(
        install_guide_message(),
        Message::SkillHub(SkillHubMessage::OpenInstallGuide)
    ));
}

#[test]
fn account_trigger_labels_cover_signed_out_active_and_busy_states() {
    let (mut app, _) = App::new();
    assert_eq!(trigger_label(&app), "登录账号");
    app.account.operation = crate::app::state::AccountOperation::Validating;
    assert_eq!(trigger_label(&app), "正在验证…");
    app.account.operation = crate::app::state::AccountOperation::StartingBrowser;
    assert_eq!(trigger_label(&app), "正在登录…");
}

#[test]
fn account_styles_keep_dark_and_light_contrast() {
    for theme in [Theme::Dark, Theme::Light] {
        let style = menu_style(&theme);
        assert!(style.background.is_some());
        assert!(style.text_color.is_some());
        assert!(style.border.width > 0.0);

        let input = login_input_style(&theme, iced::widget::text_input::Status::Active);
        assert_eq!(input.border.radius, 10.0.into());

        for status in [
            iced::widget::button::Status::Active,
            iced::widget::button::Status::Hovered,
            iced::widget::button::Status::Pressed,
            iced::widget::button::Status::Disabled,
        ] {
            let button = login_button_style(&theme, status);
            assert_eq!(button.border.radius, 10.0.into());
        }
    }

    let light_active = login_button_style(&Theme::Light, iced::widget::button::Status::Active);
    let light_hovered = login_button_style(&Theme::Light, iced::widget::button::Status::Hovered);
    let dark_active = login_button_style(&Theme::Dark, iced::widget::button::Status::Active);

    assert!(
        luminance(background_color(light_active)) > 0.90,
        "light account actions should use a pale surface"
    );
    assert_ne!(background_color(light_active), background_color(light_hovered));
    assert!(
        luminance(background_color(dark_active)) < 0.50,
        "dark account actions should stay on a dark surface"
    );
}
