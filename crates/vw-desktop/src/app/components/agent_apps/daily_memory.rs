//! Desktop composition for the application editor.
use crate::app::{App, Message, state::AgentAppsContext};
use desktop_apps_ui::{Connection, Notice, agent_apps::daily_memory as domain};
use iced::{Element, Task};
pub(crate) use domain::State;
pub use domain::Event;
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::DailyMemory(event)))
}
pub(crate) fn entry(i18n: desktop_i18n::I18n, ) -> Element<'static, Message> { domain::entry(i18n, ).map(message) }
pub(crate) fn overlay<'a>(i18n: desktop_i18n::I18n, state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if !domain::is_open(state) { return base; }
    iced::widget::stack![base, domain::overlay(i18n, state, iced::widget::Space::new().into()).map(message)].into()
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) { domain::close(&mut app.apps_catalog.daily_memory); return Task::none(); }
    let Ok(client) = crate::app::config::gateway_client() else { return app.show_localized_toast(crate::app::state::ToastKind::Error, "网关不可用，草稿已保留", vec![]); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.navigation_tabs.project_path.clone() };
    let project = vw_api_transport::local_settings_project(app.workspace_state.project_id.as_deref()).to_owned();
    let connection = Connection { client, context: context.clone(), catalog_context: Some(context.clone()), project,
        host: app.apps_catalog.host.clone(), generation: app.apps_catalog.generation, token: app.apps_catalog.token.clone() };
    let mut notice = None;
    let task = domain::update(&mut app.apps_catalog.daily_memory, connection, event, &mut notice).map(message);
    let feedback = match notice {
        Some(Notice::Error(text)) => app.show_message_toast(crate::app::state::ToastKind::Error, text),
        Some(Notice::Success(text)) => app.show_message_toast(crate::app::state::ToastKind::Success, text),
        None => Task::none(),
    };
    Task::batch([task, feedback])
}
pub(crate) fn subscription(state: &State) -> iced::Subscription<Message> { domain::subscription(state).map(message) }
