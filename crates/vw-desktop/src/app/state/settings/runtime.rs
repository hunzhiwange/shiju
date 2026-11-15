use super::*;

/// 技能设置面板状态
///
/// 管理技能系统的配置，包括开放技能目录和提示注入模式。
pub use desktop_settings_skills::types::{SkillsSettingsTab, SkillsDirectoryScope, SkillsCatalogKind, SkillsSelectedDetail, SkillsCatalogItem};

pub use desktop_settings_skills::types::PluginCatalogItem;

#[derive(Debug, Clone)]
pub(crate) struct SkillsSettingsState {
    pub(crate) bindings: crate::app::message::settings::skills::bindings_view::State,
    pub(crate) backups: crate::app::message::settings::skills::backup_modal::State,
    pub(crate) storage: crate::app::message::settings::skills::storage_modal::State,
    pub(crate) claude_config: crate::app::message::settings::skills::claude_view::State,
    pub(crate) jobs: crate::app::message::settings::skills::jobs_view::State,
    pub(crate) updates: crate::app::message::settings::skills::updates_view::State,
    pub(crate) install: crate::app::message::settings::skills::install_view::State,
    pub(crate) market: crate::app::message::settings::skills::market_view::State,
    pub(crate) import: crate::app::message::settings::skills::import_modal::State,
    pub(crate) repositories: crate::app::message::settings::skills::repo_modal::State,
    /// 是否让委托代理默认加载项目及父级技能
    pub(crate) auto_load_project_skills: bool,
    /// 技能目录提供方
    pub(crate) directory_provider: config_skills::SkillsDirectoryProvider,
    /// 提示注入模式
    pub(crate) prompt_injection_mode: config_skills::SkillsPromptInjectionMode,
    /// 当前激活的技能设置页签
    pub(crate) active_tab: SkillsSettingsTab,
    /// 技能搜索关键词
    pub(crate) query: String,
    /// 当前选中的目录范围
    pub(crate) directory_scope: SkillsDirectoryScope,
    /// 技能目录是否正在通过 gateway 加载
    pub(crate) loading: bool,
    /// 当前展示的技能目录项
    pub(crate) catalog: Vec<SkillsCatalogItem>,
    /// 当前展示的插件目录项
    pub(crate) plugin_catalog: Vec<PluginCatalogItem>,
    pub(crate) plugin_install: Option<crate::app::state::PluginInstallForm>,
    pub(crate) plugin_operation_id: u64,
    /// 插件目录是否正在通过 gateway 加载
    pub(crate) plugin_loading: bool,
    /// 当前选中的插件 ID
    pub(crate) selected_plugin_id: Option<String>,
    /// 插件操作状态提示
    pub(crate) plugin_status_message: Option<String>,
    pub(crate) plugin_status_message_translation: Option<crate::app::i18n::format::Message>,
    /// 当前插件状态提示是否为错误
    pub(crate) plugin_status_is_error: bool,
    /// 当前选中的技能 ID
    pub(crate) selected_skill_id: Option<String>,
    /// 当前选中的技能详情
    pub(crate) selected_skill_detail: Option<SkillsSelectedDetail>,
    /// 技能详情是否正在加载
    pub(crate) detail_loading: bool,
    /// 技能详情错误
    pub(crate) detail_error: Option<String>,
    /// 技能操作状态提示
    pub(crate) status_message: Option<String>,
    pub(crate) status_message_translation: Option<crate::app::i18n::format::Message>,
    /// 当前状态提示是否为错误
    pub(crate) status_is_error: bool,
    /// 是否显示帮助对话框
    pub(crate) show_help_modal: bool,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for SkillsSettingsState {
    fn default() -> Self {
        Self {
            updates: Default::default(),
            jobs: Default::default(),
            install: Default::default(),
            market: Default::default(),
            import: Default::default(),
            repositories: Default::default(),
            bindings: Default::default(),
            backups: Default::default(),
            storage: Default::default(),
            claude_config: Default::default(),
            auto_load_project_skills: true,
            directory_provider: config_skills::SkillsDirectoryProvider::Vibewindow,
            prompt_injection_mode: config_skills::SkillsPromptInjectionMode::Compact,
            active_tab: SkillsSettingsTab::Skills,
            query: String::new(),
            directory_scope: SkillsDirectoryScope::Project,
            loading: false,
            catalog: Vec::new(),
            plugin_catalog: Vec::new(),
            plugin_install: None,
            plugin_operation_id: 0,
            plugin_loading: false,
            selected_plugin_id: None,
            plugin_status_message: None,
            plugin_status_message_translation: None,
            plugin_status_is_error: false,
            selected_skill_id: None,
            selected_skill_detail: None,
            detail_loading: false,
            detail_error: None,
            status_message: None,
            status_message_translation: None,
            status_is_error: false,
            show_help_modal: false,
            save_error: None,
        }
    }
}

pub(crate) use desktop_settings_network::app_init::state::{WebSearchSettingsState};

pub(crate) use desktop_settings_network::state::{BrowserSettingsState, GatewaySettingsTab, GatewaySettingsState, GatewayClientServerDraft, GatewayClientSettingsState};

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub(crate) struct CostPriceInput {
    pub(crate) model: String,
    pub(crate) input_price: String,
    pub(crate) output_price: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct CostSettingsState {
    pub(crate) enabled: bool,
    pub(crate) daily_limit_usd_input: String,
    pub(crate) monthly_limit_usd_input: String,
    pub(crate) warn_at_percent_input: String,
    pub(crate) allow_override: bool,
    pub(crate) prices: Vec<CostPriceInput>,
    pub(crate) show_help_modal: bool,
    pub(crate) save_error: Option<String>,
}

impl Default for CostSettingsState {
    fn default() -> Self {
        Self {
            enabled: false,
            daily_limit_usd_input: "10".to_string(),
            monthly_limit_usd_input: "100".to_string(),
            warn_at_percent_input: "80".to_string(),
            allow_override: false,
            prices: Vec::new(),
            show_help_modal: false,
            save_error: None,
        }
    }
}

pub use desktop_settings_memory::state::MemorySettingsTab;

pub(crate) use desktop_settings_memory::state::MemorySettingsState;



#[cfg(test)]
#[path = "runtime_tests.rs"]
mod runtime_tests;

pub(crate) use desktop_settings_runtime::state::{HooksSettingsState, RuntimeSettingsState, ReliabilitySettingsState};
