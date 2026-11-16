#[derive(Debug, Clone)]
pub struct HooksSettingsState {
    /// 是否启用 hooks 运行时
    pub enabled: bool,
    /// 是否启用 command_logger 内置钩子
    pub command_logger: bool,
    /// 是否允许加载已启用插件声明的 hooks
    pub plugins_enabled: bool,
    /// 最近一次成功加载的插件 hook 清单
    pub catalog: Vec<gateway_desktop_settings::HookCatalogEntryDto>,
    /// 插件 hook 清单是否正在加载
    pub catalog_loading: bool,
    /// 最近一次插件 hook 清单加载错误
    pub catalog_error: Option<String>,
    /// 最近一次成功加载的安全审计记录
    pub audit: Vec<gateway_desktop_settings::HookAuditEntryDto>,
    /// 审计记录是否正在加载
    pub audit_loading: bool,
    /// 最近一次审计记录加载错误
    pub audit_error: Option<String>,
    /// 信任或撤销请求是否正在执行
    pub mutation_loading: bool,
    /// 最近一次信任或撤销错误
    pub mutation_error: Option<String>,
    /// 保存错误信息
    pub save_error: Option<String>,
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

#[derive(Debug, Clone)]
pub struct RuntimeSettingsState {
    /// 运行时类型
    pub kind: String,
    /// Docker 镜像
    pub docker_image: String,
    /// Docker 网络模式
    pub docker_network: String,
    /// Docker 内存限制输入（MB）
    pub docker_memory_limit_mb_input: String,
    /// Docker CPU 限制输入
    pub docker_cpu_limit_input: String,
    /// Docker 只读根文件系统
    pub docker_read_only_rootfs: bool,
    /// Docker 挂载工作区
    pub docker_mount_workspace: bool,
    /// Docker 允许挂载的工作区根目录输入
    pub docker_allowed_workspace_roots_input: String,
    /// 推理覆盖开关输入（自动/启用/禁用，对应 auto/true/false）
    pub reasoning_enabled_input: String,
    /// 推理级别输入
    pub reasoning_level_input: String,
    /// 保存错误信息
    pub save_error: Option<String>,
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

#[derive(Debug, Clone)]
pub struct ReliabilitySettingsState {
    /// 提供者重试次数
    pub provider_retries: u32,
    /// 提供者退避时间（毫秒）
    pub provider_backoff_ms: u64,
    /// 通道初始退避时间（秒）
    pub channel_initial_backoff_secs: u64,
    /// 通道最大退避时间（秒）
    pub channel_max_backoff_secs: u64,
    /// 调度器轮询间隔（秒）
    pub scheduler_poll_secs: u64,
    /// 调度器重试次数
    pub scheduler_retries: u32,
    /// 是否显示帮助对话框
    pub show_help_modal: bool,
    /// 保存错误信息
    pub save_error: Option<String>,
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

#[derive(Debug, Clone)]
pub struct MultimodalSettingsState {
    pub max_images: u32,
    pub max_image_size_mb: u32,
    pub allow_remote_fetch: bool,
    pub ocr_enabled: bool,
    pub model_supports_images: bool,
    pub ocr_model: String,
    pub save_error: Option<String>,
}
impl Default for MultimodalSettingsState {
    fn default() -> Self {
        Self {
            max_images: 10,
            max_image_size_mb: 5,
            allow_remote_fetch: true,
            ocr_enabled: false,
            model_supports_images: true,
            ocr_model: String::new(),
            save_error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObservabilitySettingsState {
    /// 后端类型
    pub backend: String,
    /// OTEL 端点输入
    pub otel_endpoint_input: String,
    /// OTEL 服务名称输入
    pub otel_service_name_input: String,
    /// 运行时追踪模式
    pub runtime_trace_mode: String,
    /// 运行时追踪路径输入
    pub runtime_trace_path_input: String,
    /// 运行时追踪最大条目数
    pub runtime_trace_max_entries: u32,
    /// 是否显示帮助对话框
    pub show_help_modal: bool,
    /// 保存错误信息
    pub save_error: Option<String>,
}
impl Default for ObservabilitySettingsState {
    fn default() -> Self {
        Self {
            backend: "none".to_string(),
            otel_endpoint_input: String::new(),
            otel_service_name_input: String::new(),
            runtime_trace_mode: "none".to_string(),
            runtime_trace_path_input: "state/runtime-trace.jsonl".to_string(),
            runtime_trace_max_entries: 200,
            show_help_modal: false,
            save_error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryClassificationRuleInput {
    /// 匹配模式输入。
    pub pattern: String,
    /// 分类类别输入。
    pub category: String,
    /// 优先级输入。
    pub priority_input: String,
}
impl Default for QueryClassificationRuleInput {
    fn default() -> Self {
        Self { pattern: String::new(), category: String::new(), priority_input: "0".to_string() }
    }
}

#[derive(Debug, Clone, Default)]
pub struct QueryClassificationSettingsState {
    /// 是否启用查询分类。
    pub enabled: bool,
    /// 查询分类规则列表。
    pub rules: Vec<QueryClassificationRuleInput>,
    /// 保存错误信息。
    pub save_error: Option<String>,
    pub save_error_translation: Option<desktop_i18n::format::Message>,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct CostPriceInput {
    pub model: String,
    pub input_price: String,
    pub output_price: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CostSettingsState {
    pub enabled: bool,
    pub daily_limit_usd_input: String,
    pub monthly_limit_usd_input: String,
    pub warn_at_percent_input: String,
    pub allow_override: bool,
    pub prices: Vec<CostPriceInput>,
    pub show_help_modal: bool,
    pub save_error: Option<String>,
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
