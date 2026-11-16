use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_7_quick_message_error_switches_locale_without_changing_raw_error() {
    let raw = "使用者/%n/%s/原始错误";
    let effect = crate::quick_messages_update::update(
        &mut Vec::new(),
        crate::quick_messages_update::QuickMessagesMessage::Saved(Err(raw.into())),
    );
    let crate::quick_messages_update::Effect::Toast(_, source, arguments) = effect else {
        panic!("expected feedback")
    };
    let message = desktop_i18n::format::Message::new(source, arguments);
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "保存快捷消息失败："),
        (DesktopLocale::TraditionalChinese, "儲存快捷訊息失敗："),
        (DesktopLocale::English, "Failed to save quick messages: "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
}

#[test]
fn desktop_gettext_area_7_theme_delete_guard_retains_localized_feedback() {
    let name = iced::Theme::Light.to_string();
    let mut state = crate::system_settings_themes::state::ThemesSettingsState::default();
    let mut theme = iced::Theme::Light;
    let mut id = None;
    let mut skin = None;
    let mut input = crate::system_settings_themes::update::Update {
        themes_settings: &mut state,
        active_custom_theme_id: &mut id,
        active_theme_skin: &mut skin,
        app_theme: &mut theme,
        effects: Vec::new(),
    };
    let _ = crate::system_settings_themes::update::update(
        &mut input,
        crate::system_settings_themes::messages::ThemesMessage::DeleteCustom(name.clone()),
    );
    let feedback = state.error_translation.as_ref().unwrap();
    assert_eq!(
        feedback.render(I18n::new(DesktopLocale::English)).unwrap(),
        format!("Built-in theme `{name}` cannot be deleted")
    );
    assert_eq!(
        feedback.render(I18n::new(DesktopLocale::TraditionalChinese)).unwrap(),
        format!("內建主題 `{name}` 不可刪除")
    );
}
