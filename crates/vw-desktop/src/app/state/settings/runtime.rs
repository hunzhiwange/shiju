use super::*;

/// 技能设置面板状态
///
/// 管理技能系统的配置，包括开放技能目录和提示注入模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillsSettingsTab {
    Skills,
    DiscoveryOrder,
    SystemConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillsDirectoryScope {
    Project,
    Ancestor,
    Global,
    Bundled,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillsCatalogKind {
    Recommended,
    System,
    Personal,
}

#[derive(Debug, Clone)]
pub(crate) struct SkillsSelectedDetail {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) kind: SkillsCatalogKind,
    pub(crate) installed: bool,
    pub(crate) enabled: bool,
    pub(crate) source: String,
    pub(crate) source_path: Option<String>,
    pub(crate) document_name: String,
    pub(crate) document_content: String,
    pub(crate) can_install: bool,
    pub(crate) can_toggle: bool,
    pub(crate) can_delete: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct SkillsCatalogItem {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) kind: SkillsCatalogKind,
    pub(crate) resource_count: usize,
    pub(crate) installed: bool,
    pub(crate) enabled: bool,
    pub(crate) source: String,
    pub(crate) source_path: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PluginCatalogItem {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) version: String,
    pub(crate) description: String,
    pub(crate) enabled: bool,
    pub(crate) component_count: usize,
    pub(crate) source_path: Option<String>,
    pub(crate) dependency_checks: Vec<gateway_desktop_settings::DesktopPluginDependencyDto>,
    pub(crate) origin: gateway_desktop_settings::DesktopPluginSourceDto,
    pub(crate) hook_review: String,
}

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

#[derive(Debug, Clone)]
pub(crate) struct WebSearchSettingsState {
    pub(crate) enabled: bool,
    pub(crate) provider: String,
    pub(crate) api_key_input: String,
    pub(crate) api_url_input: String,
    pub(crate) brave_api_key_input: String,
    pub(crate) max_results_input: String,
    pub(crate) timeout_secs_input: String,
    pub(crate) user_agent: String,
    pub(crate) show_help_modal: bool,
    pub(crate) save_error: Option<String>,
}

impl Default for WebSearchSettingsState {
    fn default() -> Self {
        let config = config_tools::WebSearchConfig::default();
        Self {
            enabled: config.enabled,
            provider: config.provider,
            api_key_input: String::new(),
            api_url_input: String::new(),
            brave_api_key_input: String::new(),
            max_results_input: config.max_results.to_string(),
            timeout_secs_input: config.timeout_secs.to_string(),
            user_agent: config.user_agent,
            show_help_modal: false,
            save_error: None,
        }
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemorySettingsTab {
    General,
    RetentionAndCache,
    Embedding,
    EmbeddingRoutes,
    Storage,
    Qdrant,
}

#[derive(Debug, Clone)]
pub(crate) struct MemorySettingsState {
    /// 当前显示的记忆配置页签
    pub(crate) active_tab: MemorySettingsTab,
    pub(crate) backend: String,
    pub(crate) auto_save: bool,
    pub(crate) hygiene_enabled: bool,
    pub(crate) archive_after_days: u32,
    pub(crate) purge_after_days: u32,
    pub(crate) conversation_retention_days: u32,
    pub(crate) embedding_provider: String,
    pub(crate) embedding_model: String,
    pub(crate) embedding_dimensions: u32,
    pub(crate) vector_weight: f32,
    pub(crate) keyword_weight: f32,
    pub(crate) min_relevance_score: f32,
    pub(crate) embedding_cache_size: u32,
    pub(crate) chunk_max_tokens: u32,
    pub(crate) response_cache_enabled: bool,
    pub(crate) response_cache_ttl_minutes: u32,
    pub(crate) response_cache_max_entries: u32,
    pub(crate) snapshot_enabled: bool,
    pub(crate) snapshot_on_hygiene: bool,
    pub(crate) auto_hydrate: bool,
    pub(crate) sqlite_open_timeout_secs: u32,
    pub(crate) qdrant_url_input: String,
    pub(crate) qdrant_collection: String,
    pub(crate) qdrant_api_key_input: String,
    pub(crate) save_error: Option<String>,
}

impl Default for MemorySettingsState {
    fn default() -> Self {
        Self {
            active_tab: MemorySettingsTab::General,
            backend: "sqlite".to_string(),
            auto_save: true,
            hygiene_enabled: true,
            archive_after_days: 7,
            purge_after_days: 30,
            conversation_retention_days: 30,
            embedding_provider: "none".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
            embedding_dimensions: 1536,
            vector_weight: 0.7,
            keyword_weight: 0.3,
            min_relevance_score: 0.4,
            embedding_cache_size: 10_000,
            chunk_max_tokens: 512,
            response_cache_enabled: false,
            response_cache_ttl_minutes: 60,
            response_cache_max_entries: 5_000,
            snapshot_enabled: false,
            snapshot_on_hygiene: false,
            auto_hydrate: true,
            sqlite_open_timeout_secs: 0,
            qdrant_url_input: String::new(),
            qdrant_collection: "vibewindow_memories".to_string(),
            qdrant_api_key_input: String::new(),
            save_error: None,
        }
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod runtime_tests;

pub(crate) use desktop_settings_runtime::state::{HooksSettingsState, RuntimeSettingsState, ReliabilitySettingsState};
