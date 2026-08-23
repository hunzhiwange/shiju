//! 验证账户详情弹窗的资料状态、编辑流程和敏感字段显示边界。
//!
//! 账户夹具区分已登录、加载、错误和编辑草稿，避免敏感值进入普通展示路径。

use super::*;

#[test]
fn status_labels_cover_gateway_values() {
    assert_eq!(status_label("enabled"), "启用");
    assert_eq!(status_label("ACTIVE"), "启用");
    assert_eq!(status_label("disabled"), "停用");
    assert_eq!(status_label("custom"), "custom");
}

#[test]
fn empty_and_multiple_values_have_readable_labels() {
    assert_eq!(values_label(&[]), "—");
    assert_eq!(values_label(&["AI创新组".to_string(), "平台组".to_string()]), "AI创新组、平台组");
}

#[test]
fn account_details_uses_four_pixel_scrollbar() {
    assert_eq!(DETAILS_SCROLLBAR_WIDTH, 4);
}

#[test]
fn account_profile_editor_exposes_reference_form_and_safe_theme_styles() {
    assert_eq!(PROFILE_EDITOR_SCROLLBAR_WIDTH, 4);
    assert_eq!(
        PROFILE_EDITOR_LABELS,
        ["编辑我的资料", "返回我的", "显示名称", "登录账号", "保存资料"]
    );

    let ready = crate::app::account::AccountProfileDraft::new("刘祥敏", "liuxiangmin").unwrap();
    assert!(profile_editor_can_submit(&crate::app::state::AccountProfileEditorState::Editing(
        ready
    )));
    assert!(!profile_editor_can_submit(&crate::app::state::AccountProfileEditorState::Loading));

    for theme in [Theme::Light, Theme::Dark] {
        let input = profile_input_style(&theme, iced::widget::text_input::Status::Active);
        let Background::Color(background) = input.background else {
            panic!("profile input must use a solid background");
        };
        assert_ne!(input.value, background);
        let action = profile_save_button_style(&theme, iced::widget::button::Status::Active);
        let Some(Background::Color(action_background)) = action.background else {
            panic!("profile action must use a solid background");
        };
        assert_ne!(action.text_color, action_background);
    }
}

#[test]
fn account_password_editor_exposes_secure_reference_form_and_theme_styles() {
    assert_eq!(PASSWORD_EDITOR_SCROLLBAR_WIDTH, 4);
    assert!(PASSWORD_INPUTS_SECURE);
    assert_eq!(
        PASSWORD_EDITOR_LABELS,
        ["修改我的密码", "返回我的", "当前密码", "新密码", "确认新密码", "修改密码"]
    );

    let ready = crate::app::account::AccountPasswordDraft::new(
        "current-password-123",
        "new-password-456",
        "new-password-456",
    );
    assert!(password_editor_can_submit(&crate::app::state::AccountPasswordEditorState::Editing(
        ready
    )));
    assert!(password_editor_can_submit(&crate::app::state::AccountPasswordEditorState::Editing(
        Default::default()
    )));
    assert!(!password_editor_can_submit(&crate::app::state::AccountPasswordEditorState::Saving(
        Default::default()
    )));
    assert!(!password_editor_can_submit(&crate::app::state::AccountPasswordEditorState::Closed));

    for theme in [Theme::Light, Theme::Dark] {
        let input = password_input_style(&theme, iced::widget::text_input::Status::Active);
        let Background::Color(background) = input.background else {
            panic!("password input must use a solid background");
        };
        assert_ne!(input.value, background);
        let action = password_submit_button_style(&theme, iced::widget::button::Status::Active);
        let Some(Background::Color(action_background)) = action.background else {
            panic!("password action must use a solid background");
        };
        assert_ne!(action.text_color, action_background);
        let error = password_error_style(&theme);
        let Some(Background::Color(error_background)) = error.background else {
            panic!("password error must use a solid background");
        };
        assert_ne!(error.text_color.expect("error text"), error_background);
    }
}
