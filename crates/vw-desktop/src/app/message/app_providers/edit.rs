use crate::app::{App, Message, message::SettingsMessage};
pub use desktop_provider_apps::message::edit::EditMessage;
use iced::Task;
pub(crate) fn message(value: EditMessage) -> Message {
    Message::Settings(SettingsMessage::AppProviderEdit(value))
}
pub(crate) fn update(app: &mut App, event: EditMessage) -> Task<Message> {
    if matches!(
        &event,
        EditMessage::Claude(_)
            | EditMessage::Codex(_)
            | EditMessage::Gemini(_)
            | EditMessage::Grok(_)
            | EditMessage::Hermes(_)
            | EditMessage::Pi(_)
            | EditMessage::Kimi(_)
            | EditMessage::Mcode(_)
            | EditMessage::Agy(_)
            | EditMessage::Dsh(_)
            | EditMessage::Omp(_)
            | EditMessage::OpenClaw(_)
            | EditMessage::OpenCode(_)
    ) {
        return super::provider_update::effect(
            app,
            desktop_provider_apps::message::provider_update::Effect::Field(event),
        );
    }
    let current =
        app.app_providers.scope.as_ref().and_then(|s| super::list::current(app, s.app).ok());
    super::provider_update::edit(app, current, event)
}
pub(crate) fn load(
    client: vw_gateway_client::GatewayClient,
    scope: desktop_provider_apps::Scope,
    generation: u64,
    id: String,
    copy: bool,
    delete: bool,
    claude: bool,
    token: String,
) -> Task<Message> {
    desktop_provider_apps::message::edit_dispatch::load(
        client, scope, generation, id, copy, delete, claude, token,
    )
    .map(message)
}
pub(crate) fn dispatch_save(
    client: vw_gateway_client::GatewayClient,
    scope: desktop_provider_apps::Scope,
    generation: u64,
    form: desktop_provider_apps::state::edit::EditForm,
    token: String,
) -> Task<Message> {
    desktop_provider_apps::message::edit_dispatch::dispatch_save(
        client, scope, generation, form, token,
    )
    .map(message)
}
