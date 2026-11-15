pub use desktop_apps_ui::agent_apps_auth::actions::{ActionEvent, Outcome};
use super::message;
use crate::app::{App, Message, state::agent_apps_auth::Scope};
use gateway_endpoint::GatewayEndpoint;
use iced::Task;
pub(super) fn update(app: &mut App, event: ActionEvent, endpoint: GatewayEndpoint, scope: Scope, visible: bool) -> Task<Message> {
    let mut notice = None;
    let task = desktop_apps_ui::agent_apps_auth::actions::update(&mut app.agent_apps_auth, &mut notice, event, endpoint, scope, visible).map(message);
    let toast = match notice {
        Some(desktop_apps_ui::Notice::Error(text)) => app.show_error_toast(text),
        Some(desktop_apps_ui::Notice::Success(text)) => app.show_success_toast(text),
        None => Task::none(),
    };
    Task::batch([task, toast])
}
