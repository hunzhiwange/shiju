use super::*;

/// Hooks 设置面板状态
///
/// 管理运行时 hooks 总开关与内置钩子的配置。
#[derive(Debug, Clone)]
pub(crate) struct HooksSettingsState {
    /// 是否启用 hooks 运行时
    pub(crate) enabled: bool,
    /// 是否启用 command_logger 内置钩子
    pub(crate) command_logger: bool,
    /// 是否允许加载已启用插件声明的 hooks
    pub(crate) plugins_enabled: bool,
    /// 最近一次成功加载的插件 hook 清单
    pub(crate) catalog: Vec<vw_gateway_client::HookCatalogEntryDto>,
    /// 插件 hook 清单是否正在加载
    pub(crate) catalog_loading: bool,
    /// 最近一次插件 hook 清单加载错误
    pub(crate) catalog_error: Option<String>,
    /// 最近一次成功加载的安全审计记录
    pub(crate) audit: Vec<vw_gateway_client::HookAuditEntryDto>,
    /// 审计记录是否正在加载
    pub(crate) audit_loading: bool,
    /// 最近一次审计记录加载错误
    pub(crate) audit_error: Option<String>,
    /// 信任或撤销请求是否正在执行
    pub(crate) mutation_loading: bool,
    /// 最近一次信任或撤销错误
    pub(crate) mutation_error: Option<String>,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for HooksSettingsState {
    fn default() -> Self {
        Self {
            enabled: true,
            command_logger: false,
            plugins_enabled: true,
            catalog: Vec::new(),
            catalog_loading: false,
            catalog_error: None,
            audit: Vec::new(),
            audit_loading: false,
            audit_error: None,
            mutation_loading: false,
            mutation_error: None,
            save_error: None,
        }
    }
}

/// 运行时设置面板状态
///
/// 管理 runtime 配置，包括 kind 选择、Docker 子配置与推理覆盖选项。
#[derive(Debug, Clone)]
pub(crate) struct RuntimeSettingsState {
    /// 运行时类型
    pub(crate) kind: String,
    /// Docker 镜像
    pub(crate) docker_image: String,
    /// Docker 网络模式
    pub(crate) docker_network: String,
    /// Docker 内存限制输入（MB）
    pub(crate) docker_memory_limit_mb_input: String,
    /// Docker CPU 限制输入
    pub(crate) docker_cpu_limit_input: String,
    /// Docker 只读根文件系统
    pub(crate) docker_read_only_rootfs: bool,
    /// Docker 挂载工作区
    pub(crate) docker_mount_workspace: bool,
    /// Docker 允许挂载的工作区根目录输入
    pub(crate) docker_allowed_workspace_roots_input: String,
    /// 推理覆盖开关输入（自动/启用/禁用，对应 auto/true/false）
    pub(crate) reasoning_enabled_input: String,
    /// 推理级别输入
    pub(crate) reasoning_level_input: String,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for RuntimeSettingsState {
    fn default() -> Self {
        Self {
            kind: "native".to_string(),
            docker_image: "alpine:3.20".to_string(),
            docker_network: "none".to_string(),
            docker_memory_limit_mb_input: "512".to_string(),
            docker_cpu_limit_input: "1".to_string(),
            docker_read_only_rootfs: true,
            docker_mount_workspace: true,
            docker_allowed_workspace_roots_input: String::new(),
            reasoning_enabled_input: "auto".to_string(),
            reasoning_level_input: String::new(),
            save_error: None,
        }
    }
}

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
}

#[derive(Debug, Clone)]
pub(crate) struct SkillsSettingsState {
    /// 是否启用开放技能
    pub(crate) open_skills_enabled: bool,
    /// 是否让委托代理默认加载项目及父级技能
    pub(crate) auto_load_project_skills: bool,
    /// 技能目录提供方
    pub(crate) directory_provider: vw_config_types::skills::SkillsDirectoryProvider,
    /// 开放技能目录路径输入
    pub(crate) open_skills_dir_input: String,
    /// 提示注入模式
    pub(crate) prompt_injection_mode: vw_config_types::skills::SkillsPromptInjectionMode,
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
    /// 插件目录是否正在通过 gateway 加载
    pub(crate) plugin_loading: bool,
    /// 当前选中的插件 ID
    pub(crate) selected_plugin_id: Option<String>,
    /// 插件操作状态提示
    pub(crate) plugin_status_message: Option<String>,
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
            open_skills_enabled: false,
            auto_load_project_skills: true,
            directory_provider: vw_config_types::skills::SkillsDirectoryProvider::Vibewindow,
            open_skills_dir_input: String::new(),
            prompt_injection_mode: vw_config_types::skills::SkillsPromptInjectionMode::Compact,
            active_tab: SkillsSettingsTab::Skills,
            query: String::new(),
            directory_scope: SkillsDirectoryScope::Project,
            loading: false,
            catalog: Vec::new(),
            plugin_catalog: Vec::new(),
            plugin_loading: false,
            selected_plugin_id: None,
            plugin_status_message: None,
            plugin_status_is_error: false,
            selected_skill_id: None,
            selected_skill_detail: None,
            detail_loading: false,
            detail_error: None,
            status_message: None,
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
        Self {
            enabled: false,
            provider: "duckduckgo".to_string(),
            api_key_input: String::new(),
            api_url_input: String::new(),
            brave_api_key_input: String::new(),
            max_results_input: "5".to_string(),
            timeout_secs_input: "15".to_string(),
            user_agent: vw_config_types::tools::DEFAULT_USER_AGENT.to_string(),
            show_help_modal: false,
            save_error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BrowserSettingsState {
    pub(crate) enabled: bool,
    pub(crate) allowed_domains_input: String,
    pub(crate) allowed_domains_editor: text_editor::Content,
    pub(crate) browser_open: String,
    pub(crate) session_name_input: String,
    pub(crate) backend: String,
    pub(crate) native_headless: bool,
    pub(crate) native_webdriver_url: String,
    pub(crate) native_chrome_path_input: String,
    pub(crate) computer_use_endpoint: String,
    pub(crate) computer_use_api_key_input: String,
    pub(crate) computer_use_timeout_ms_input: String,
    pub(crate) computer_use_allow_remote_endpoint: bool,
    pub(crate) computer_use_window_allowlist_input: String,
    pub(crate) computer_use_max_coordinate_x_input: String,
    pub(crate) computer_use_max_coordinate_y_input: String,
    pub(crate) save_error: Option<String>,
}

impl Default for BrowserSettingsState {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_domains_input: String::new(),
            allowed_domains_editor: text_editor::Content::new(),
            browser_open: "new_tab".to_string(),
            session_name_input: String::new(),
            backend: "agent_browser".to_string(),
            native_headless: true,
            native_webdriver_url: "http://127.0.0.1:9515".to_string(),
            native_chrome_path_input: String::new(),
            computer_use_endpoint: "http://127.0.0.1:8787/v1/actions".to_string(),
            computer_use_api_key_input: String::new(),
            computer_use_timeout_ms_input: "15000".to_string(),
            computer_use_allow_remote_endpoint: false,
            computer_use_window_allowlist_input: String::new(),
            computer_use_max_coordinate_x_input: String::new(),
            computer_use_max_coordinate_y_input: String::new(),
            save_error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewaySettingsTab {
    Config,
    Skeys,
}

#[derive(Debug, Clone)]
pub(crate) struct GatewaySettingsState {
    pub(crate) active_tab: GatewaySettingsTab,
    pub(crate) port: u16,
    pub(crate) host_input: String,
    pub(crate) auth_enabled: bool,
    pub(crate) skeys: Vec<vw_config_types::gateway::GatewaySkey>,
    pub(crate) new_skey_name_input: String,
    pub(crate) new_skey_expires_at_input: String,
    pub(crate) new_skey_calendar_month: String,
    pub(crate) new_skey_calendar_open: bool,
    pub(crate) last_created_skey: Option<String>,
    pub(crate) last_created_skey_copied: bool,
    pub(crate) allow_public_bind: bool,
    pub(crate) webhook_rate_limit_per_minute: u32,
    pub(crate) trust_forwarded_headers: bool,
    pub(crate) rate_limit_max_keys: u32,
    pub(crate) idempotency_ttl_secs: u32,
    pub(crate) idempotency_max_keys: u32,
    pub(crate) node_control_enabled: bool,
    pub(crate) node_control_auth_token_input: String,
    pub(crate) node_control_allowed_node_ids_input: String,
    pub(crate) service_action_running: Option<String>,
    pub(crate) service_action_output: Option<String>,
    pub(crate) copied_service_command: Option<String>,
    pub(crate) pending_service_command_confirmation: Option<String>,
    pub(crate) show_help_modal: bool,
    pub(crate) save_error: Option<String>,
}

impl Default for GatewaySettingsState {
    fn default() -> Self {
        Self {
            active_tab: GatewaySettingsTab::Config,
            port: vw_config_types::gateway::DEFAULT_GATEWAY_PORT,
            host_input: "127.0.0.1".to_string(),
            auth_enabled: false,
            skeys: Vec::new(),
            new_skey_name_input: String::new(),
            new_skey_expires_at_input: String::new(),
            new_skey_calendar_month: String::new(),
            new_skey_calendar_open: false,
            last_created_skey: None,
            last_created_skey_copied: false,
            allow_public_bind: false,
            webhook_rate_limit_per_minute: 60,
            trust_forwarded_headers: false,
            rate_limit_max_keys: 10_000,
            idempotency_ttl_secs: 300,
            idempotency_max_keys: 10_000,
            node_control_enabled: false,
            node_control_auth_token_input: String::new(),
            node_control_allowed_node_ids_input: String::new(),
            service_action_running: None,
            service_action_output: None,
            copied_service_command: None,
            pending_service_command_confirmation: None,
            show_help_modal: false,
            save_error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GatewayClientServerDraft {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) platform_id: String,
    pub(crate) company_id: String,
    pub(crate) skey: String,
}

impl GatewayClientServerDraft {
    pub(crate) fn from_config(server: &vw_config_types::ui::GatewayClientServerConfig) -> Self {
        Self {
            id: server.id.clone(),
            name: server.name.clone(),
            host: server.host.clone(),
            port: server.port.clamp(1, u16::MAX),
            platform_id: server.platform_id.clone(),
            company_id: server.company_id.clone(),
            skey: if server.skey.trim().is_empty() {
                server.bearer_token.clone()
            } else {
                server.skey.clone()
            },
        }
    }

    pub(crate) fn to_config(&self) -> vw_config_types::ui::GatewayClientServerConfig {
        vw_config_types::ui::GatewayClientServerConfig {
            id: self.id.clone(),
            name: self.name.clone(),
            host: self.host.clone(),
            port: self.port.clamp(1, u16::MAX),
            platform_id: self.platform_id.clone(),
            company_id: self.company_id.clone(),
            bearer_token: String::new(),
            username: String::new(),
            password: String::new(),
            skey: self.skey.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GatewayClientSettingsState {
    pub(crate) servers: Vec<GatewayClientServerDraft>,
    pub(crate) selected_server_id: String,
    pub(crate) health: HashMap<String, bool>,
    pub(crate) name_input: String,
    pub(crate) host_input: String,
    pub(crate) port: u16,
    pub(crate) platform_id_input: String,
    pub(crate) company_id_input: String,
    pub(crate) skey_input: String,
    pub(crate) pending_remove_server_id: Option<String>,
    pub(crate) show_help_modal: bool,
    pub(crate) save_error: Option<String>,
}

impl Default for GatewayClientSettingsState {
    fn default() -> Self {
        let server = GatewayClientServerDraft::from_config(
            &vw_config_types::ui::GatewayClientServerConfig::default(),
        );
        Self {
            selected_server_id: server.id.clone(),
            name_input: server.name.clone(),
            host_input: "127.0.0.1".to_string(),
            port: vw_config_types::gateway::DEFAULT_GATEWAY_PORT,
            platform_id_input: server.platform_id.clone(),
            company_id_input: server.company_id.clone(),
            servers: vec![server],
            health: HashMap::new(),
            skey_input: String::new(),
            pending_remove_server_id: None,
            show_help_modal: false,
            save_error: None,
        }
    }
}

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

/// 可靠性设置面板状态
///
/// 管理系统可靠性的配置，包括重试策略和退避时间等。
#[derive(Debug, Clone)]
pub(crate) struct ReliabilitySettingsState {
    /// 提供者重试次数
    pub(crate) provider_retries: u32,
    /// 提供者退避时间（毫秒）
    pub(crate) provider_backoff_ms: u64,
    /// 通道初始退避时间（秒）
    pub(crate) channel_initial_backoff_secs: u64,
    /// 通道最大退避时间（秒）
    pub(crate) channel_max_backoff_secs: u64,
    /// 调度器轮询间隔（秒）
    pub(crate) scheduler_poll_secs: u64,
    /// 调度器重试次数
    pub(crate) scheduler_retries: u32,
    /// 是否显示帮助对话框
    pub(crate) show_help_modal: bool,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for ReliabilitySettingsState {
    fn default() -> Self {
        Self {
            provider_retries: 2,
            provider_backoff_ms: 500,
            channel_initial_backoff_secs: 2,
            channel_max_backoff_secs: 60,
            scheduler_poll_secs: 15,
            scheduler_retries: 2,
            show_help_modal: false,
            save_error: None,
        }
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod runtime_tests;
