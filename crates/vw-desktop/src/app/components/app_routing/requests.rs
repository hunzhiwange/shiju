use crate::app::{Message, state::AppsCatalogState};
pub(crate) fn overlay<'a>(i18n: desktop_i18n::I18n, catalog: &'a AppsCatalogState, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    iced::widget::stack![base, desktop_apps_ui::app_routing::requests::overlay(i18n, &catalog.routing, iced::widget::Space::new().into()).map(super::lifecycle::message)].into()
}
pub(crate) fn entry(i18n: desktop_i18n::I18n, id: vw_api_app_identity::AppId) -> iced::Element<'static, Message> { desktop_apps_ui::app_routing::requests::entry(i18n, id).map(super::lifecycle::message) }
