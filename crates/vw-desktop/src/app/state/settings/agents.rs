//! 维护系统设置状态及其按领域拆分的派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;
use config_agent::{builtin_agent_config, builtin_agent_keys};
pub(crate) use desktop_settings_agents::state::{
    DelegateAgentSettingsEntry, main_agent_model_fields, ordered_agent_keys,
    filter_mcp_from_available_tools, normalize_available_tool_catalog,
    merge_builtin_required_tools, apply_default_allowed_tools,
    retain_available_allowed_tools, agent_settings_entries_from_config,
};

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
pub(crate) struct AgentsSettingsState {
    pub(crate) catalog_official: bool,
    pub(crate) catalog_query: String,
    pub(crate) catalog_busy: bool,
    pub(crate) profile_entering_key: bool,
    pub(crate) profile_modal: Option<DelegateAgentSettingsEntry>,
    pub(crate) profile_saving: bool,
    pub(crate) profile_deleting: bool,
    pub(crate) loading: bool,
    /// 只有从 Gateway 成功加载全局配置后，界面状态才可作为保存来源。
    pub(crate) config_ready: bool,
    /// 兼容旧刷新消息；智能体定义的读写始终使用租户全局端点。
    pub(crate) project_path: Option<String>,
    pub(crate) skill_directory: Option<String>,
    pub(crate) skill_catalog_generation: u64,
    pub(crate) skill_catalog: Vec<SkillsCatalogItem>,
    pub(crate) skill_catalog_loading: bool,
    pub(crate) skill_catalog_error: Option<String>,
    pub(crate) skill_directory_scope: SkillsDirectoryScope,
    pub(crate) providers: Vec<ProviderSummary>,
    pub(crate) provider_models: Vec<ProviderModelsSummary>,
    pub(crate) entries: Vec<DelegateAgentSettingsEntry>,
    pub(crate) new_agent_key_input: String,
    pub(crate) selected_agent: String,
    pub(crate) active_config_tab: AgentConfigTab,
    pub(crate) active_detail_tab: String,
    pub(crate) active_tools_list_tab: String,
    pub(crate) active_prompt_tab: String,
    pub(crate) workspace_identity_files: Vec<WorkspaceIdentityFileState>,
    pub(crate) workspace_identity_root_path: Option<String>,
    /// 网关成功返回的完整工具目录；`None` 表示本轮结果不可用于清理。
    pub(crate) registered_tools: Option<Vec<String>>,
    pub(crate) available_tools: Vec<String>,
    pub(crate) save_error: Option<String>,
    pub(crate) save_error_translation: Option<crate::app::i18n::format::Message>,
    pub(crate) save_in_flight: bool,
    pub(crate) dirty: bool,
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
