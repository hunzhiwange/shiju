pub use desktop_apps_ui::app_routing::lifecycle::*;
use crate::app::{App, Message, state::AppsCatalogState};
pub(super) fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::SettingsMessage::AppsCatalog(crate::app::message::settings::AppsCatalogMessage::Routing(event)))
}
fn context(catalog: &AppsCatalogState) -> Option<Context> {
    desktop_apps_ui::app_routing::lifecycle::context(catalog.context.as_ref(), &catalog.host, catalog.generation, catalog.model_diagnostics.access_generation, catalog.loading, &catalog.token)
}
pub(super) fn current(app: &App) -> Option<Context> {
    let current = context(&app.apps_catalog)?;
    let client = crate::app::config::gateway_client().ok()?;
    let expected = crate::app::state::AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.navigation_tabs.project_path.clone() };
    (app.apps_catalog.context.as_ref() == Some(&expected)).then_some(current)
}
pub(crate) fn update(app: &mut App, event: Event) -> iced::Task<Message> {
    let context = current(app);
    let client = crate::app::config::gateway_client().ok();
    let mut notice = None;
    let task = desktop_apps_ui::app_routing::lifecycle::update(&mut app.apps_catalog.routing, context, app.project_id.clone(), &app.apps_catalog.token, client, &mut notice, event).map(message);
    let feedback = match notice {
        Some(desktop_apps_ui::Notice::Error(error)) => app.show_error_toast(error),
        Some(desktop_apps_ui::Notice::Success(text)) => app.show_success_toast(text),
        None => iced::Task::none(),
    };
    iced::Task::batch([task, feedback])
}
pub(crate) fn view(catalog: &AppsCatalogState) -> iced::Element<'_, Message> {
    desktop_apps_ui::app_routing::lifecycle::view(&catalog.routing, context(catalog)).map(message)
}
