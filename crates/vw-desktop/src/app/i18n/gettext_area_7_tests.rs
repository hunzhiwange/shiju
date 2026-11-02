use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_7_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "应用外观与预览行为。"),
        (DesktopLocale::TraditionalChinese, "應用程式外觀與預覽行為。"),
        (DesktopLocale::English, "Application appearance and preview behavior."),
    ] {
        assert_eq!(I18n::new(locale).__("应用外观与预览行为。"), expected);
    }
}

#[test]
fn desktop_gettext_area_7_general_editor_sources_have_catalog_entries() {
    let en = super::catalog::parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    for source in [
        include_str!("../components/system_settings_general.rs"),
        include_str!("../components/system_settings_editor.rs"),
        include_str!("../components/system_settings_quick_messages.rs"),
        include_str!("../components/system_settings_dialogue_flow.rs"),
        include_str!("../components/system_settings_reliability.rs"),
    ] {
        for tail in source.split(".__(\"").skip(1) {
            let key = tail.split('"').next().unwrap();
            assert!(en.contains_key(key), "missing English: {key}");
            assert!(tw.contains_key(key), "missing Traditional Chinese: {key}");
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_7_quick_message_error_switches_locale_without_changing_raw_error() {
    use crate::app::message::settings::{QuickMessagesMessage, SettingsMessage};
    let (mut app, _) = crate::app::App::new();
    let raw = "使用者/%n/%s/原始错误";
    let _ = crate::app::message::settings::update(
        &mut app,
        SettingsMessage::QuickMessages(QuickMessagesMessage::Saved(Err(raw.into()))),
    );
    let message = app.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "保存快捷消息失败："),
        (DesktopLocale::TraditionalChinese, "儲存快捷訊息失敗："),
        (DesktopLocale::English, "Failed to save quick messages: "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_7_dialogue_feedback_switches_locale() {
    use crate::app::message::settings::SettingsMessage;
    let (mut app, _) = crate::app::App::new();
    let raw = "原始 %n/%s 错误";
    let _ = crate::app::message::settings::update(
        &mut app,
        SettingsMessage::DialogueFlowUiSettingsSaved(Err(raw.into())),
    );
    let status = app.dialogue_flow_settings_translation.as_ref().unwrap();
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        assert!(status.render(I18n::new(locale)).unwrap().ends_with(raw));
    }
    let _ = crate::app::message::settings::update(
        &mut app,
        SettingsMessage::DialogueFlowUiSettingsSaved(Ok(())),
    );
    assert_eq!(
        app.dialogue_flow_settings_translation
            .as_ref()
            .unwrap()
            .render(I18n::new(DesktopLocale::English))
            .unwrap(),
        "Conversation flow configuration saved"
    );
}
