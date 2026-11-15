//! 桌面壳：借用领域状态并映射局部事件。
use crate::app::{App, Message};
use desktop_settings_cron::system_settings_scheduler as domain;
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
 domain::ViewData { scheduler_settings: &app.scheduler_settings, desktop_locale: app.desktop_locale, app_theme: &app.app_theme, last_copied_code_hash: app.last_copied_code_hash, help_content }
}
fn map(event: Local) -> Message {
 match event { Local::None => Message::None, Local::CopyCode(v) => Message::CopyCode(v), Local::Settings(event) => Message::Settings(match event { Event::SchedulerHelpOpen => Global::SchedulerHelpOpen,
Event::SchedulerHelpClose => Global::SchedulerHelpClose,
Event::SchedulerEnabledToggled(v) => Global::SchedulerEnabledToggled(v),
Event::SchedulerMaxTasksChanged(v) => Global::SchedulerMaxTasksChanged(v),
Event::SchedulerMaxConcurrentChanged(v) => Global::SchedulerMaxConcurrentChanged(v), }) }
}
pub fn view(app: &App) -> Element<'_,Message> { domain::view(&data(app)).map(map) }
pub(crate) fn embedded_view(app: &App) -> Element<'_,Message> { domain::embedded_view(&data(app)).map(map) }
