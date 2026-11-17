//! Shell navigation and feedback mapping for tool discovery.
use super::{McpMessage, SettingsMessage};
use crate::app::{App, Message, message::agent_apps_diagnostics as diagnostics};
pub use desktop_settings_mcp::configuration_update::tools::{ToolsMessage, ToolsState};
use iced::Task;
pub(crate) fn message(event: ToolsMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::Tools(event)))
}
pub(crate) fn visible(app: &App) -> Option<&ToolsState> {
    desktop_settings_mcp::configuration_update::tools::visible(
        &app.mcp_settings.tools,
        diagnostics::context(app).as_ref(),
    )
}

fn run(app: &mut App, event: Option<ToolsMessage>) -> Task<Message> {
    use crate::app::{components::system_settings::SystemTab, state::AgentConfigTab};
    let current = diagnostics::context(app);
    let on_page = app.navigation_panels.show_system_settings
        && app.navigation_panels.system_settings_tab == SystemTab::Agents
        && app.agents_settings.active_config_tab == AgentConfigTab::Mcp;
    let mutating = app.mcp_settings.mutating();
    let mut input = desktop_settings_mcp::configuration_update::tools::Update {
        state: &mut app.mcp_settings.tools,
        current,
        on_page,
        mutating,
        client: crate::app::config::gateway_client(),
        token: app.apps_catalog.token.clone(),
        effects: Vec::new(),
    };
    let task = match event {
        Some(event) => desktop_settings_mcp::configuration_update::tools::update(&mut input, event),
        None => desktop_settings_mcp::configuration_update::tools::invalidate(&mut input),
    }
    .map(message);
    let effects = std::mem::take(&mut input.effects);
    apply_effects(app, task, effects)
}
pub(crate) fn invalidate(app: &mut App) -> Task<Message> {
    run(app, None)
}
pub(crate) fn update(app: &mut App, event: ToolsMessage) -> Task<Message> {
    run(app, Some(event))
}
pub(crate) fn apply_effects(
    app: &mut App,
    task: Task<Message>,
    effects: Vec<desktop_settings_mcp::management_effect::Effect>,
) -> Task<Message> {
    use desktop_settings_mcp::management_effect::Effect;
    let mut tasks = vec![task];
    for effect in effects {
        tasks.push(match effect {
            Effect::Info(value) => app.show_info_toast(value),
            Effect::Error(value) => app.show_error_toast(value),
            Effect::Success(value) => app.show_success_toast(value),
            Effect::Refresh => super::mcp_management_list::update(
                app,
                super::mcp_management_list::CatalogMessage::Refresh,
            ),
        });
    }
    Task::batch(tasks)
}

#[cfg(test)]
pub use desktop_settings_mcp::configuration_update::tools::Context;
