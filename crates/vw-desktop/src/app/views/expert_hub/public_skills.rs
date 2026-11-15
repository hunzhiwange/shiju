use crate::app::{App, Message};
use iced::Element;
use desktop_expert_hub::views::{public_skills as domain, ViewInput};
pub(super) fn view(app: &App) -> Element<'_, Message> {
 domain::view(&ViewInput { expert_hub: &app.expert_hub, desktop_locale: app.desktop_locale, app_theme: &app.app_theme }).map(map)
}
fn map(message: desktop_expert_hub::views::Message) -> crate::app::Message {
 match message {
 desktop_expert_hub::views::Message::SkillHub(m) => crate::app::Message::SkillHub(m),
 desktop_expert_hub::views::Message::Connectors(m) => crate::app::Message::Connectors(m),
 desktop_expert_hub::views::Message::StartBrowserLogin => crate::app::Message::Account(crate::app::message::AccountMessage::StartBrowserLogin),
 desktop_expert_hub::views::Message::None => crate::app::Message::None,
 }
}
pub(super) use domain::{grid_column_count, public_tab_specs, source_selector_visible};
pub(super) fn loading_view() -> Element<'static, Message> { domain::loading_view().map(map) }
pub(super) fn error_view(i18n: crate::app::i18n::I18n, error: &str) -> Element<'static, Message> { domain::error_view(i18n, error).map(map) }
pub(super) fn suites_content(i18n: crate::app::i18n::I18n, state: &crate::app::skill_hub::PublicSkillMarketState) -> Element<'static, Message> { domain::suites_content(i18n, state).map(map) }
pub(super) fn source_selector(i18n: crate::app::i18n::I18n, state: &crate::app::skill_hub::PublicSkillMarketState) -> Element<'static, Message> { domain::source_selector(i18n, state).map(map) }
