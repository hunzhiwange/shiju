pub use desktop_apps_ui::app_routing::config::*;
use crate::app::{Message, state::AppsCatalogState};
pub(crate) fn overlay<'a>(catalog: &'a AppsCatalogState, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    iced::widget::stack![base, desktop_apps_ui::app_routing::config::overlay(&catalog.routing, iced::widget::Space::new().into()).map(super::lifecycle::message)].into()
}
pub(crate) fn timeout_button(id: vw_api_app_identity::AppId) -> iced::Element<'static, Message> { desktop_apps_ui::app_routing::config::timeout_button(id).map(super::lifecycle::message) }
