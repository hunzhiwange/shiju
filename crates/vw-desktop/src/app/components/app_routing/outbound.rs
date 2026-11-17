use crate::app::Message;
#[cfg(test)]
pub(crate) fn overlay<'a>(catalog: &'a AppsCatalogState, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    iced::widget::stack![base, desktop_apps_ui::app_routing::outbound::overlay(&catalog.routing, iced::widget::Space::new().into()).map(super::lifecycle::message)].into()
}
pub(crate) fn entry(id: vw_api_app_identity::AppId) -> iced::Element<'static, Message> { desktop_apps_ui::app_routing::outbound::entry(id).map(super::lifecycle::message) }

#[cfg(test)]
use crate::app::state::AppsCatalogState;
#[cfg(test)]
pub use desktop_apps_ui::app_routing::outbound::*;
