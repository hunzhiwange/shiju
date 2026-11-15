use crate::app::{App, Message};
use crate::app::message::app_providers::list::{self, ListMessage};
use iced::Task;
pub use desktop_settings_apps::aggregation::list::*;
fn current(app: &App) -> Option<(GatewayClient, Selection)> {
    let scope = app.app_providers.scope.as_ref()?;
    let (client, now) = list::current(app, scope.app).ok()?;
    if &now != scope {
        return None;
    }
    let project = app.project_id.as_deref().filter(|s| !s.trim().is_empty())?.into();
    Some((
        client,
        Selection { scope: now, project, catalog_generation: app.apps_catalog.generation },
    ))
}
use vw_gateway_client::GatewayClient;
#[path = "aggregation_publish.rs"]
pub(crate) mod publish;
#[path = "aggregation_clients.rs"]
pub(crate) mod clients;
pub(crate) fn message(event: Event) -> Message { list::message(ListMessage::Aggregation(event)) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let current = current(app);
    let mut notice = None;
    let task = desktop_settings_apps::aggregation::list::update(&mut app.app_providers.aggregation, &mut app.app_providers.generation, app.app_providers.loading, &app.apps_catalog.token, current, &mut notice, event).map(message);
    let feedback = match notice {
        Some(desktop_settings_apps::aggregation::Notice::Error(s)) => app.show_error_toast(s),
        Some(desktop_settings_apps::aggregation::Notice::Info(s)) => app.show_info_toast(s),
        Some(desktop_settings_apps::aggregation::Notice::Success(s)) => app.show_success_toast(s),
        Some(desktop_settings_apps::aggregation::Notice::Warning(s)) => app.show_warning_toast(s),
        None => Task::none(),
    };
    Task::batch([task, feedback])
}
