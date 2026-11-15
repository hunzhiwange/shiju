//! 桌面只注入领域状态、主题和语言。
use crate::app::{App, Message};
use iced::Element;
use desktop_expert_hub::views::{ViewInput, installed_skills};
fn input(app: &App) -> ViewInput<'_> { ViewInput { expert_hub: &app.expert_hub, desktop_locale: app.desktop_locale, app_theme: &app.app_theme } }
fn map(message: desktop_expert_hub::views::Message) -> Message {
 match message {
  desktop_expert_hub::views::Message::SkillHub(value) => Message::SkillHub(crate::app::message::skill_hub::map_domain(value)),
  desktop_expert_hub::views::Message::Connectors(value) => Message::SkillHub(crate::app::message::SkillHubMessage::Connectors(value)),
  desktop_expert_hub::views::Message::None => Message::None,
 }
}
pub(super) fn view(app: &App) -> Element<'_, Message> { {
 let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
 let title = i18n.__("我安装的技能");
 let header = installed_skills::header(i18n, move || crate::app::components::system_settings_common::settings_hover_help_label(i18n, &app.app_theme, title, iced::widget::text(title).size(25).font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }), i18n.__("管理当前目录范围内可启用、关闭或卸载的本地技能")), Message::SkillHub(crate::app::message::SkillHubMessage::Public(crate::app::message::skill_hub::PublicSkillMessage::CloseInstalled)));
 installed_skills::view(&input(app), header, map)
} }
pub(super) fn with_delete_confirmation<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
 match installed_skills::delete_confirmation(&input(app)) {
 Some((close, card)) => desktop_ui_controls::system_settings_common::settings_modal_overlay(Some(base), map(close), card.map(map)),
 None => base,
 }
}
