use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_18_account_labels_follow_current_locale() {
    let mut state = crate::state::AccountState::default();
    state.operation = crate::state::AccountOperation::StartingBrowser;
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "正在登录…"),
        (DesktopLocale::TraditionalChinese, "正在登入…"),
        (DesktopLocale::English, "Signing in…"),
    ] {
        assert_eq!(crate::view::trigger_label(&state, locale), expected);
    }
}

#[test]
fn desktop_gettext_area_18_account_validation_retains_semantic_feedback() {
    let mut state = crate::state::AccountState::default();
    let services = crate::update::Services {
        gateway: Err("isolated validation".into()),
        store_path: Err("no persistence during validation".into()),
    };
    let mut effects = Vec::new();
    let _ = crate::update::update(
        &mut state,
        crate::message::AccountMessage::SubmitManual,
        &services,
        &mut effects,
    );
    let message = effects
        .into_iter()
        .find_map(|effect| match effect {
            crate::update::Effect::Validation(feedback) => Some(feedback.message()),
            _ => None,
        })
        .expect("validation toast");
    assert_eq!(message.render(I18n::new(DesktopLocale::English)).unwrap(), "skey is required");
    assert_eq!(
        message.render(I18n::new(DesktopLocale::TraditionalChinese)).unwrap(),
        "skey 不能為空"
    );
}
