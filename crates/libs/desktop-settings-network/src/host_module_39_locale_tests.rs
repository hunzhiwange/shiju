use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_7_local_validation_feedback_switches_language() {
    let mut state = crate::state::BrowserSettingsState::default();
    let _ = crate::message::browser::update(
        &mut state,
        crate::messages::BrowserMessage::ComputerUseTimeoutMsChanged("0".into()),
    );
    assert_eq!(
        state
            .save_error_translation
            .as_ref()
            .unwrap()
            .render(I18n::new(DesktopLocale::English))
            .unwrap(),
        "computer_use.timeout_ms must be greater than 0"
    );
}
