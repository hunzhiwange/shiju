use crate::quick_messages_update::{Effect, QuickMessagesMessage, update};
use crate::system_settings_themes::update::ToastKind;
use config_ui::{DesktopLocale, QuickMessageConfig, QuickMessageGroup, QuickMessageKind};
#[test]
fn quick_message_settings_save_validates_content_and_reports_result() {
    let mut items = vec![QuickMessageConfig {
        text: "  ".into(),
        group: QuickMessageGroup::General,
        kind: QuickMessageKind::Message,
    }];
    let Effect::Toast(ToastKind::Warning, source, arguments) =
        update(&mut items, QuickMessagesMessage::Save)
    else {
        panic!("expected warning")
    };
    let i18n = desktop_i18n::I18n::new(DesktopLocale::SimplifiedChinese);
    assert!(i18n.format(source, &arguments).unwrap().contains("不能为空"));
    assert!(matches!(
        update(&mut items, QuickMessagesMessage::Saved(Ok(()))),
        Effect::Toast(ToastKind::Success, _, _)
    ));
    let Effect::Toast(ToastKind::Error, source, arguments) =
        update(&mut items, QuickMessagesMessage::Saved(Err("gateway unavailable".into())))
    else {
        panic!("expected error")
    };
    assert!(i18n.format(source, &arguments).unwrap().contains("gateway unavailable"));
}
