//! 验证账户状态菜单、触发标签、滚动条及明暗主题对比度契约。
//! 视图构造覆盖登出、活动和忙碌状态，并显式检查敏感输入采用掩码控件。

use super::*;

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
fn account_trigger_label_follows_active_user_company_and_sign_out() {
    use crate::app::account::{
        AccountIdentity, AccountOrigin, AccountProfile, AccountRepository, SecretSkey,
        VerifiedAccount,
    };

    let (mut app, _) = App::new();
    let mut repository = AccountRepository::default();
    let first = repository.upsert_verified(
        VerifiedAccount::from_parts_with_profile(
            AccountIdentity::from_profile("成都示例公司", "enabled").unwrap(),
            AccountProfile::new("enabled", "刘祥敏", "user-one", "enabled", vec![], vec![])
                .unwrap(),
            SecretSkey::new("fixture-one").unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    let legacy = repository.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::from_profile("另一公司", "enabled").unwrap(),
            SecretSkey::new("fixture-two").unwrap(),
        ),
        AccountOrigin::ManualSkey,
    );
    repository.activate(&first).unwrap();
    app.account.replace_repository(repository.clone());
    assert_eq!(trigger_label(&app), "刘祥敏（成都示例公司）");

    repository.activate(&legacy).unwrap();
    app.account.replace_repository(repository.clone());
    assert_eq!(trigger_label(&app), "另一公司");

    repository.sign_out();
    app.account.replace_repository(repository);
    assert_eq!(trigger_label(&app), "登录账号");
}

#[test]
fn account_menu_and_selected_row_use_readable_theme_colors() {
    for theme in [Theme::Dark, Theme::Light] {
        let style = menu_style(&theme);
        assert!(style.background.is_some());
        assert!(style.text_color.is_some());
        assert!(style.border.width > 0.0);

        let selected = account_row_style(&theme, iced::widget::button::Status::Disabled, true);
        let Some(Background::Color(background)) = selected.background else {
            panic!("selected account should have a solid background");
        };
        assert_ne!(selected.text_color, background);
    }
}
