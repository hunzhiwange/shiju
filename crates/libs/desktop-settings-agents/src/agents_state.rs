use crate::state::*;
use iced::widget::text_editor;
use config_agent::{builtin_agent_config, builtin_agent_keys};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AgentConfigTab {
    #[default]
    Agents,
    Channels,
    Runtime,
    Acp,
    Mcp,
    Skills,
    Plugins,
    Reliability,
    DialogueFlow,
    QuickMessages,
    Heartbeat,
}

#[derive(Debug, Clone)]
/// 表示 AgentsSettingsState 相关的应用状态或派生数据。
pub struct AgentsSettingsState {
    pub catalog_official: bool,
    pub catalog_query: String,
    pub catalog_busy: bool,
    pub profile_entering_key: bool,
    pub profile_modal: Option<DelegateAgentSettingsEntry>,
    pub profile_saving: bool,
    pub profile_deleting: bool,
    pub loading: bool,
    /// 只有从 Gateway 成功加载全局配置后，界面状态才可作为保存来源。
    pub config_ready: bool,
    /// 兼容旧刷新消息；智能体定义的读写始终使用租户全局端点。
    pub project_path: Option<String>,
    pub skill_directory: Option<String>,
    pub skill_catalog_generation: u64,
    pub skill_catalog: Vec<SkillsCatalogItem>,
    pub skill_catalog_loading: bool,
    pub skill_catalog_error: Option<String>,
    pub skill_directory_scope: SkillsDirectoryScope,
    pub providers: Vec<ProviderSummary>,
    pub provider_models: Vec<ProviderModelsSummary>,
    pub entries: Vec<DelegateAgentSettingsEntry>,
    pub new_agent_key_input: String,
    pub selected_agent: String,
    pub active_config_tab: AgentConfigTab,
    pub active_detail_tab: String,
    pub active_tools_list_tab: String,
    pub active_prompt_tab: String,
    pub workspace_identity_files: Vec<WorkspaceIdentityFileState>,
    pub workspace_identity_root_path: Option<String>,
    /// 网关成功返回的完整工具目录；`None` 表示本轮结果不可用于清理。
    pub registered_tools: Option<Vec<String>>,
    pub available_tools: Vec<String>,
    pub save_error: Option<String>,
    pub save_error_translation: Option<desktop_i18n::format::Message>,
    pub save_in_flight: bool,
    pub dirty: bool,
}

impl Default for AgentsSettingsState {
    fn default() -> Self {
        Self {
            catalog_official: false,
            catalog_query: String::new(),
            catalog_busy: false,
            profile_entering_key: false,
            profile_modal: None,
            profile_saving: false,
            profile_deleting: false,
            loading: true,
            config_ready: false,
            project_path: None,
            skill_directory: None,
            skill_catalog_generation: 0,
            skill_catalog: Vec::new(),
            skill_catalog_loading: false,
            skill_catalog_error: None,
            skill_directory_scope: SkillsDirectoryScope::All,
            providers: Vec::new(),
            provider_models: Vec::new(),
            entries: builtin_agent_keys()
                .iter()
                .map(|key| {
                    DelegateAgentSettingsEntry::from_config(
                        key,
                        Some(builtin_agent_config(key).unwrap_or_default()),
                    )
                })
                .collect(),
            new_agent_key_input: String::new(),
            selected_agent: MAIN_AGENT_KEY.to_string(),
            active_config_tab: AgentConfigTab::Agents,
            active_detail_tab: AGENT_DETAIL_BASIC_TAB.to_string(),
            active_tools_list_tab: AGENT_TOOLS_OPTIONAL_TAB.to_string(),
            active_prompt_tab: AGENT_PROMPT_SYSTEM_TAB.to_string(),
            workspace_identity_files: WORKSPACE_IDENTITY_FILES
                .iter()
                .map(|(file_name, label)| WorkspaceIdentityFileState {
                    file_name: (*file_name).to_string(),
                    label: (*label).to_string(),
                    editor: text_editor::Content::with_text(""),
                    size_bytes: None,
                    modified_at_ms: None,
                })
                .collect(),
            workspace_identity_root_path: None,
            registered_tools: None,
            available_tools: Vec::new(),
            save_error: None,
            save_error_translation: None,
            save_in_flight: false,
            dirty: false,
        }
    }
}

#[cfg(test)]
#[path = "agents_tests.rs"]
mod agents_tests;
