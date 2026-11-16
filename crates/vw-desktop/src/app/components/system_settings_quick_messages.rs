//! 快捷消息设置的宿主借用与动作映射。
use crate::app::{App, Message, message::{SettingsMessage, settings::QuickMessagesMessage}};
use desktop_settings_appearance::quick_messages as domain;
use iced::Element;
pub fn view(app: &App) -> Element<'_, Message> { domain::view(&domain::View { quick_messages: &app.general_settings.quick_messages, desktop_locale: app.general_settings.desktop_locale, app_theme: &app.appearance.app_theme }, render_help).map(map_message) }
fn map_message(action: domain::Message) -> Message {
 use domain::Message as A;
 let action = match action {
 A::None => return Message::None, A::CopyCode(v) => return Message::CopyCode(v),
 A::Add => QuickMessagesMessage::Add, A::Save => QuickMessagesMessage::Save,
 A::Remove(v) => QuickMessagesMessage::Remove(v), A::MoveUp(v) => QuickMessagesMessage::MoveUp(v), A::MoveDown(v) => QuickMessagesMessage::MoveDown(v),
 A::TextChanged(i,v) => QuickMessagesMessage::TextChanged(i,v), A::GroupChanged(i,v) => QuickMessagesMessage::GroupChanged(i,v), A::KindChanged(i,v) => QuickMessagesMessage::KindChanged(i,v),
 };
 Message::Settings(SettingsMessage::QuickMessages(action))
}
fn render_help<'a>(title: &'a str, body: std::borrow::Cow<'a,str>, color: Option<iced::Color>, i18n: desktop_i18n::I18n) -> Element<'a, domain::Message> {
 super::chat_panel::message_view::selectable_titled_text(title, body, color, i18n).map(|m| match m { Message::None => domain::Message::None, Message::CopyCode(v) => domain::Message::CopyCode(v), _ => unreachable!("plain help emitted chat action") })
}
