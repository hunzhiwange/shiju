use iced::Element;
use crate::state::{SessionToolSelectorState, SessionToolSelectorTab, SessionToolGroup};
use desktop_settings_skills::types::{SkillsCatalogItem, SkillsDirectoryScope};
pub struct SelectorInput<'a> {
 pub i18n: desktop_i18n::I18n,
 pub show_tools: bool,
 pub active_tab: SessionToolSelectorTab,
 pub tool_selector: &'a SessionToolSelectorState,
 pub base_tools: Vec<String>,
 pub selected_agent_key: &'a str,
 pub main_agent_key: &'a str,
 pub agents: &'a [desktop_settings_agents::state::DelegateAgentSettingsEntry],
 pub skills: &'a [SkillsCatalogItem],
 pub skills_loading: bool,
 pub inventory_empty: bool,
 pub started: bool,
 pub acp_installed: &'a std::collections::BTreeMap<String, bool>,
 pub agent_icon: fn(&str, f32) -> Element<'static, Message>,
}
#[derive(Debug, Clone)]
pub enum Message {
 SessionToolSelectorTabSelected(SessionToolSelectorTab),
 SessionToolSelectorSearchChanged(String),
 SessionToolSelectorSkillDirectoryScopeChanged(SkillsDirectoryScope),
 SessionAgentSelected(Option<String>),
 SessionToolGroupCollapsedToggled(SessionToolGroup),
 SessionManualToolSelected(String),
 SessionSkillSelected(String),
 OpenAgentSettings,
}
