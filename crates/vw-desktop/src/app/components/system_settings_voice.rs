//! Desktop composition of the voice settings page and recordings domain.
use crate::app::{App, Message};
use crate::app::i18n::I18n;
use crate::app::message::{SettingsMessage, settings::voice::Event};
use iced::{Element, widget::{column, text}};
fn message(event: Event) -> Message { Message::Settings(SettingsMessage::Voice(event)) }
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    column![
        desktop_settings_voice::view::view(&app.voice.settings, i18n, app.voice.hotkeys.status()).map(message),
        super::voice_recordings::view(app),
        text(i18n.__("屏幕分享默认关闭，每次需确认窗口；本机语音使用当前普通聊天图像模型。"))
            .style(super::system_settings_common::settings_muted_text_style),
    ].spacing(14).into()
}
pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    if app.voice.settings.edit.is_none() { return base; }
    iced::widget::stack![base, desktop_settings_voice::view::modal(&app.voice.settings,
        I18n::new(app.general_settings.desktop_locale), iced::widget::Space::new().into()).map(message)].into()
}
