//! Maps provider selection and toast effects to the usage settings owner.
pub use desktop_usage::apps_usage_settings::Event;
use desktop_usage::apps_usage_settings::{self as domain, Message as LocalMessage};
use crate::app::{App, Message, message::app_providers::list::{ListMessage, message}};
use super::apps_usage_cards;
pub(super) fn send(event: Event) -> Message { message(ListMessage::UsageSettings(event)) }
fn map(event: LocalMessage) -> Message {
    match event {
        LocalMessage::Event(event) => send(event),
        LocalMessage::Error(message) => send(Event::Feedback(false, message)),
        LocalMessage::Success(message) => send(Event::Feedback(true, message)),
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> iced::Task<Message> {
    if let Event::Feedback(success, message) = event { return if success { app.show_success_toast(message) } else { app.show_error_toast(message) }; }
    let endpoint = crate::app::gateway_client_endpoint();
    let scope = apps_usage_cards::scope(app, &endpoint, false);
    let visible = crate::app::components::third_party_settings::is_visible(app);
    let selected = match &event {
        Event::Open(app_id, provider_id) => app.app_providers.snapshot.as_ref().is_some_and(|snapshot| snapshot.catalog.rows.iter().any(|row| &row.provider.app_id == app_id && &row.provider.id == provider_id)),
        _ => false,
    };
    domain::update(&mut app.app_providers.usage.settings, scope, visible, selected, endpoint, &app.apps_catalog.token, event).map(map)
}
pub(crate) fn overlay<'a>(app: &'a App, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    let scope = (crate::app::components::third_party_settings::is_visible(app)).then(|| apps_usage_cards::current_scope(app, false)).flatten();
    domain::overlay(&app.app_providers.usage.settings, scope, base, map)
}

#[cfg(test)]
pub use desktop_usage::apps_usage_settings::Editor;
