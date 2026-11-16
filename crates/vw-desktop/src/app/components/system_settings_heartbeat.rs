//! 桌面壳：借用领域状态并映射局部事件。
use crate::app::{App, Message};
use desktop_settings_cron::system_settings_heartbeat as domain;
use domain::{Message as Local, SettingsMessage as Event};
use crate::app::message::SettingsMessage as Global;
use iced::Element;
fn help_content<'a>(title: &'a str, body: std::borrow::Cow<'a,str>, color: Option<iced::Color>, i18n: desktop_i18n::I18n) -> Element<'a,Local> {
 crate::app::components::chat_panel::message_view::selectable_titled_text(title,body,color,i18n).map(|event| match event {
 Message::None => Local::None, Message::CopyCode(value) => Local::CopyCode(value),
 _ => unreachable!("selectable help only emits copy or no-op events"),
 })
}
fn data(app: &App) -> domain::ViewData<'_> {
 domain::ViewData { heartbeat_settings: &app.heartbeat_settings, desktop_locale: app.general_settings.desktop_locale, app_theme: &app.appearance.app_theme, last_copied_code_hash: app.clipboard_state.last_copied_code_hash, help_content }
}
fn map(event: Local) -> Message {
 match event { Local::None => Message::None, Local::CopyCode(v) => Message::CopyCode(v), Local::Settings(event) => Message::Settings(match event { Event::HeartbeatHelpOpen => Global::HeartbeatHelpOpen,
Event::HeartbeatHelpClose => Global::HeartbeatHelpClose,
Event::HeartbeatEnabledToggled(v) => Global::HeartbeatEnabledToggled(v),
Event::HeartbeatIntervalChanged(v) => Global::HeartbeatIntervalChanged(v),
Event::HeartbeatMessageChanged(v) => Global::HeartbeatMessageChanged(v),
Event::HeartbeatTargetChanged(v) => Global::HeartbeatTargetChanged(v),
Event::HeartbeatToChanged(v) => Global::HeartbeatToChanged(v), }) }
}
pub fn view(app: &App) -> Element<'_,Message> { domain::view(&data(app)).map(map) }
pub fn view_overlays<'a>(app: &'a App, base: Element<'a,Message>) -> Element<'a,Message> { domain::view_overlays(&data(app),base,map) }
