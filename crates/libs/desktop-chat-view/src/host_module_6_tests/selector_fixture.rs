use desktop_chat_tools::selector_input::{Message, SelectorInput};
use desktop_session::session::agent::SessionRuntimeState;
use desktop_settings_agents::state::DelegateAgentSettingsEntry;
#[derive(Default)]
pub struct AgentCatalog {
    pub entries: Vec<DelegateAgentSettingsEntry>,
    pub available_tools: Vec<String>,
}
#[derive(Default)]
pub struct SkillCatalog {
    pub loading: bool,
    pub catalog: Vec<desktop_settings_skills::types::SkillsCatalogItem>,
}
pub struct SelectorFixture {
    pub agents_settings: AgentCatalog,
    pub skills_settings: SkillCatalog,
    runtime: SessionRuntimeState<()>,
    installed: std::collections::BTreeMap<String, bool>,
}
impl SelectorFixture {
    pub fn new() -> (Self, ()) {
        (
            Self {
                agents_settings: AgentCatalog {
                    entries: vec![DelegateAgentSettingsEntry::from_config("main", None)],
                    ..Default::default()
                },
                skills_settings: Default::default(),
                runtime: Default::default(),
                installed: Default::default(),
            },
            (),
        )
    }
    pub fn current_session_runtime(&self) -> &SessionRuntimeState<()> {
        &self.runtime
    }
    pub fn current_session_runtime_mut(&mut self) -> &mut SessionRuntimeState<()> {
        &mut self.runtime
    }
    pub fn select(&mut self, event: Message) {
        use desktop_session::message::chat::{
            preferences,
            selection::{self, SelectionMessage},
        };
        match event {
            Message::SessionAgentSelected(agent) => {
                preferences::select_agent(&mut self.runtime, agent, None, None, false)
            }
            Message::SessionManualToolSelected(tool) => {
                selection::update(
                    &mut self.runtime.tool_selector,
                    &self.agents_settings.available_tools,
                    SelectionMessage::ManualTool(tool),
                );
            }
            Message::SessionSkillSelected(skill) => {
                selection::update(
                    &mut self.runtime.tool_selector,
                    &self.agents_settings.available_tools,
                    SelectionMessage::Skill(skill),
                );
            }
            _ => panic!("unexpected selection"),
        }
    }
}
pub fn session_tool_selector_popover(app: &SelectorFixture) -> iced::Element<'_, Message> {
    let runtime = &app.runtime;
    desktop_chat_tools::tool_selector::session_tool_selector_popover(SelectorInput {
        i18n: desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        show_tools: runtime.acp_agent.is_none(),
        active_tab: runtime.visible_session_tool_selector_tab(),
        tool_selector: &runtime.tool_selector,
        base_tools: if runtime.acp_agent.is_none() {
            desktop_settings_agents::state::normalize_available_tool_catalog(
                app.agents_settings.available_tools.clone(),
            )
        } else {
            Vec::new()
        },
        selected_agent_key: runtime.agent.as_deref().unwrap_or("main"),
        main_agent_key: "main",
        agents: &app.agents_settings.entries,
        skills: &app.skills_settings.catalog,
        skills_loading: app.skills_settings.loading,
        inventory_empty: app.agents_settings.available_tools.is_empty(),
        started: false,
        acp_installed: &app.installed,
        agent_icon: |backend, size| desktop_ui_controls::icons::acp_agent_icon(backend, size),
    })
}
