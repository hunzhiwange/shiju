pub use desktop_settings_apps::system_settings_apps::drag::{DragState, DragMessage, reordered};
use crate::app::{App, Message, state::AppsCatalogState};
pub(crate) fn event(value: DragMessage) -> Message { super::message(crate::app::message::settings::AppsCatalogMessage::Drag(value)) }
pub(super) fn can_start(state: &AppsCatalogState, id: vw_api_app_identity::AppId) -> bool { desktop_settings_apps::system_settings_apps::drag::can_start(&super::view_state(state), id) }
pub(crate) fn update(app: &mut App, value: DragMessage) -> iced::Task<Message> {
    let result = desktop_settings_apps::system_settings_apps::drag::update(&super::view_state(&app.apps_catalog), value);
    app.apps_catalog.drag = result.drag;
    app.apps_catalog.expanded = result.expanded;
    app.apps_catalog.hovered = result.hovered;
    if let Some(order) = result.order { return super::preferences::update(app, super::preferences::PreferencesMessage::Reorder(order)); }
    iced::Task::none()
}
pub(crate) fn subscription(state: &DragState, main: Option<iced::window::Id>) -> iced::Subscription<Message> { desktop_settings_apps::system_settings_apps::drag::subscription(state, main).map(event) }
