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
 LocalMessage::OpenAgentSettings => return Message::View(message::ViewMessage::OpenAgentManagement(None)),
 })
}
pub(crate) fn session_tool_selector_popover(app: &App) -> iced::Element<'_, Message> {
 selector(app, None)
}

pub(crate) fn session_tool_selector_picker(
 app: &App,
 tab: desktop_chat_tools::state::SessionToolSelectorTab,
 placeholder: &'static str,
 experts: bool,
) -> iced::Element<'static, Message> {
 selector(app, Some((tab, placeholder, experts)))
}

fn selector(
 app: &App,
 picker: Option<(desktop_chat_tools::state::SessionToolSelectorTab, &'static str, bool)>,
) -> iced::Element<'static, Message> {
 let runtime = app.current_session_runtime();
 let expert_skills = picker.filter(|(_, _, experts)| *experts).map(|_| {
     app.skills_settings.catalog.iter()
         .filter(|skill| skill.installed && skill.enabled && app.skills_settings.expert_skill_ids.contains(&skill.id))
         .cloned().collect::<Vec<_>>()
 });
 let input = SelectorInput {
 i18n: crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
 show_tools: runtime.acp_agent.is_none(),
 active_tab: picker.map_or_else(|| runtime.visible_session_tool_selector_tab(), |(tab, _, _)| tab),
 tool_selector: &runtime.tool_selector,
 base_tools: if runtime.acp_agent.is_none() { app.session_tool_inventory(&runtime).base_tools } else { Vec::new() },
 selected_agent_key: runtime.agent.as_deref().unwrap_or(crate::app::state::MAIN_AGENT_KEY),
 main_agent_key: crate::app::state::MAIN_AGENT_KEY,
 agents: &app.agents_settings.entries,
 skills: expert_skills.as_deref().unwrap_or(&app.skills_settings.catalog),
 skills_loading: app.skills_settings.loading,
 inventory_empty: app.agents_settings.available_tools.is_empty(),
 started: app.current_session_has_started(),
 acp_installed: &app.acp_settings.installed,
 agent_icon: |backend, size| crate::app::components::input_panel::icons::acp_agent_icon::<Message>(backend, size).map(|_| unreachable!("identity icon emits no messages")),
 };
 let view = if let Some((_, placeholder, experts)) = picker {
     desktop_chat_tools::tool_selector::session_tool_selector_picker(input, placeholder, experts)
 } else {
     desktop_chat_tools::tool_selector::session_tool_selector_popover(input)
 };
 view.map(map_message)
}
