use crate::app::{App, Message, message};
use desktop_chat_tools::selector_input::{SelectorInput, Message as LocalMessage};
fn map_message(local: LocalMessage) -> Message {
 Message::Chat(match local {
 LocalMessage::SessionToolSelectorTabSelected(v) => message::ChatMessage::SessionToolSelectorTabSelected(v),
 LocalMessage::SessionToolSelectorSearchChanged(v) => message::ChatMessage::SessionToolSelectorSearchChanged(v),
 LocalMessage::SessionToolSelectorSkillDirectoryScopeChanged(v) => message::ChatMessage::SessionToolSelectorSkillDirectoryScopeChanged(v),
 LocalMessage::SessionAgentSelected(v) => message::ChatMessage::SessionAgentSelected(v),
 LocalMessage::SessionToolGroupCollapsedToggled(v) => message::ChatMessage::SessionToolGroupCollapsedToggled(v),
 LocalMessage::SessionManualToolSelected(v) => message::ChatMessage::SessionManualToolSelected(v),
 LocalMessage::SessionSkillSelected(v) => message::ChatMessage::SessionSkillSelected(v),
 LocalMessage::OpenAgentSettings => return Message::View(message::ViewMessage::OpenSystemSettingsTab(crate::app::components::system_settings::SystemTab::Agents)),
 })
}
pub(crate) fn session_tool_selector_popover(app: &App) -> iced::Element<'_, Message> {
 let runtime = app.current_session_runtime();
 desktop_chat_tools::tool_selector::session_tool_selector_popover(SelectorInput {
 i18n: crate::app::i18n::I18n::new(app.desktop_locale),
 show_tools: runtime.acp_agent.is_none(),
 active_tab: runtime.visible_session_tool_selector_tab(),
 tool_selector: &runtime.tool_selector,
 base_tools: if runtime.acp_agent.is_none() { app.session_tool_inventory(&runtime).base_tools } else { Vec::new() },
 selected_agent_key: runtime.agent.as_deref().unwrap_or(crate::app::state::MAIN_AGENT_KEY),
 main_agent_key: crate::app::state::MAIN_AGENT_KEY,
 agents: &app.agents_settings.entries,
 skills: &app.skills_settings.catalog,
 skills_loading: app.skills_settings.loading,
 inventory_empty: app.agents_settings.available_tools.is_empty(),
 started: app.current_session_has_started(),
 acp_installed: &app.acp_settings.installed,
 agent_icon: |backend, size| crate::app::components::input_panel::icons::acp_agent_icon(backend, size).map(|_| unreachable!("identity icon emits no messages")),
 }).map(map_message)
}
