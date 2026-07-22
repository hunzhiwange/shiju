use super::*;

#[derive(Debug, Clone)]
pub(crate) struct MultimodalSettingsState {
    pub(crate) max_images: u32,
    pub(crate) max_image_size_mb: u32,
    pub(crate) allow_remote_fetch: bool,
    pub(crate) save_error: Option<String>,
}

impl Default for MultimodalSettingsState {
    fn default() -> Self {
        Self { max_images: 4, max_image_size_mb: 5, allow_remote_fetch: false, save_error: None }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HttpRequestSettingsState {
    pub(crate) enabled: bool,
    pub(crate) allowed_domains: Vec<String>,
    pub(crate) new_allowed_domain_input: String,
    pub(crate) max_response_size: u32,
    pub(crate) timeout_secs: u32,
    pub(crate) user_agent: String,
    pub(crate) save_error: Option<String>,
}

impl Default for HttpRequestSettingsState {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_domains: Vec::new(),
            new_allowed_domain_input: String::new(),
            max_response_size: 1_000_000,
            timeout_secs: 30,
            user_agent: HttpRequestConfig::default().user_agent,
            save_error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AcpSettingsState {
    pub(crate) catalog: std::collections::BTreeMap<String, vw_config_types::config::AcpAgentConfig>,
    pub(crate) enabled: std::collections::BTreeSet<String>,
    pub(crate) loading: bool,
    pub(crate) saving_agent: Option<String>,
    pub(crate) copied_agent: Option<String>,
    pub(crate) save_error: Option<String>,
    pub(crate) status_message: Option<String>,
}

impl Default for AcpSettingsState {
    fn default() -> Self {
        Self {
            catalog: std::collections::BTreeMap::new(),
            enabled: std::collections::BTreeSet::new(),
            loading: false,
            saving_agent: None,
            copied_agent: None,
            save_error: None,
            status_message: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct McpNewServerState {
    pub(crate) name_input: String,
    pub(crate) transport: McpNewServerTransport,
    pub(crate) command_input: String,
    pub(crate) args_input: String,
    pub(crate) url_input: String,
    pub(crate) timeout_input: String,
    pub(crate) enabled: bool,
    pub(crate) required: bool,
}

impl Default for McpNewServerState {
    fn default() -> Self {
        Self {
            name_input: String::new(),
            transport: McpNewServerTransport::Stdio,
            command_input: String::new(),
            args_input: String::new(),
            url_input: String::new(),
            timeout_input: "30".to_string(),
            enabled: true,
            required: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpNewServerTransport {
    Stdio,
    Http,
    Sse,
}

impl McpNewServerTransport {
    pub const ALL: [Self; 3] = [Self::Stdio, Self::Http, Self::Sse];

    pub fn label(self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Http => "http",
            Self::Sse => "sse",
        }
    }
}

impl std::fmt::Display for McpNewServerTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct McpSettingsState {
    pub(crate) active_tab: McpSettingsTab,
    pub(crate) editor: text_editor::Content,
    pub(crate) runtime_overrides: Vec<serde_json::Value>,
    pub(crate) new_server_expanded: bool,
    pub(crate) expanded_previews: std::collections::BTreeSet<String>,
    pub(crate) new_server: McpNewServerState,
    pub(crate) loading: bool,
    pub(crate) inspect_loading: bool,
    pub(crate) saving: bool,
    pub(crate) server_count: usize,
    pub(crate) inspect: Option<vw_gateway_client::vw_api_types::mcp::McpInspectResponse>,
    pub(crate) inspect_error: Option<String>,
    pub(crate) save_error: Option<String>,
    pub(crate) status_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpSettingsTab {
    Servers,
    Runtime,
    Json,
}

impl Default for McpSettingsState {
    fn default() -> Self {
        Self {
            active_tab: McpSettingsTab::Servers,
            editor: text_editor::Content::with_text("[]"),
            runtime_overrides: Vec::new(),
            new_server_expanded: false,
            expanded_previews: std::collections::BTreeSet::new(),
            new_server: McpNewServerState::default(),
            loading: false,
            inspect_loading: false,
            saving: false,
            server_count: 0,
            inspect: None,
            inspect_error: None,
            save_error: None,
            status_message: None,
        }
    }
}

/// 安全设置面板状态
///
/// 管理系统安全性的配置，包括沙箱、审计、
/// OTP、紧急停止和系统调用异常检测等。
#[derive(Debug, Clone)]
pub(crate) struct SecuritySettingsState {
    /// 沙箱启用状态输入
    pub(crate) sandbox_enabled_input: String,
    /// 沙箱后端输入
    pub(crate) sandbox_backend_input: String,
    /// Firejail 参数输入
    pub(crate) sandbox_firejail_args_input: String,
    /// 最大内存限制（MB）
    pub(crate) resources_max_memory_mb: u32,
    /// 最大 CPU 时间（秒）
    pub(crate) resources_max_cpu_time_seconds: u64,
    /// 最大子进程数
    pub(crate) resources_max_subprocesses: u32,
    /// 是否启用内存监控
    pub(crate) resources_memory_monitoring: bool,
    /// 是否启用审计
    pub(crate) audit_enabled: bool,
    /// 审计日志路径
    pub(crate) audit_log_path: String,
    /// 审计日志最大大小（MB）
    pub(crate) audit_max_size_mb: u32,
    /// 是否签名审计事件
    pub(crate) audit_sign_events: bool,
    /// 是否启用 OTP
    pub(crate) otp_enabled: bool,
    /// OTP 方法输入
    pub(crate) otp_method_input: String,
    /// OTP 令牌有效期（秒）
    pub(crate) otp_token_ttl_secs: u64,
    /// OTP 缓存有效期（秒）
    pub(crate) otp_cache_valid_secs: u64,
    /// OTP 门控操作输入
    pub(crate) otp_gated_actions_input: String,
    /// OTP 门控域输入
    pub(crate) otp_gated_domains_input: String,
    /// OTP 门控域类别输入
    pub(crate) otp_gated_domain_categories_input: String,
    /// 是否启用紧急停止
    pub(crate) estop_enabled: bool,
    /// 紧急停止状态文件路径
    pub(crate) estop_state_file: String,
    /// 恢复时是否需要 OTP
    pub(crate) estop_require_otp_to_resume: bool,
    /// 是否启用系统调用异常检测
    pub(crate) syscall_anomaly_enabled: bool,
    /// 系统调用异常严格模式
    pub(crate) syscall_anomaly_strict_mode: bool,
    /// 未知系统调用时是否告警
    pub(crate) syscall_anomaly_alert_on_unknown_syscall: bool,
    /// 每分钟最大拒绝事件数
    pub(crate) syscall_anomaly_max_denied_events_per_minute: u32,
    /// 每分钟最大总事件数
    pub(crate) syscall_anomaly_max_total_events_per_minute: u32,
    /// 每分钟最大告警数
    pub(crate) syscall_anomaly_max_alerts_per_minute: u32,
    /// 告警冷却时间（秒）
    pub(crate) syscall_anomaly_alert_cooldown_secs: u64,
    /// 系统调用异常日志路径
    pub(crate) syscall_anomaly_log_path: String,
    /// 基线系统调用输入
    pub(crate) syscall_anomaly_baseline_syscalls_input: String,
    /// 是否启用金丝雀令牌
    pub(crate) canary_tokens: bool,
    /// 是否启用语义防护
    pub(crate) semantic_guard: bool,
    /// 语义防护集合名称
    pub(crate) semantic_guard_collection: String,
    /// 语义防护阈值
    pub(crate) semantic_guard_threshold: f64,
    /// 是否显示帮助对话框
    pub(crate) show_help_modal: bool,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for SecuritySettingsState {
    fn default() -> Self {
        Self {
            sandbox_enabled_input: "auto".to_string(),
            sandbox_backend_input: "auto".to_string(),
            sandbox_firejail_args_input: String::new(),
            resources_max_memory_mb: 512,
            resources_max_cpu_time_seconds: 60,
            resources_max_subprocesses: 10,
            resources_memory_monitoring: true,
            audit_enabled: true,
            audit_log_path: "audit.log".to_string(),
            audit_max_size_mb: 100,
            audit_sign_events: false,
            otp_enabled: false,
            otp_method_input: "totp".to_string(),
            otp_token_ttl_secs: 30,
            otp_cache_valid_secs: 300,
            otp_gated_actions_input: "shell, file_write, browser_open, browser, memory_forget"
                .to_string(),
            otp_gated_domains_input: String::new(),
            otp_gated_domain_categories_input: String::new(),
            estop_enabled: false,
            estop_state_file: vw_config_types::paths::estop_state_file_path(),
            estop_require_otp_to_resume: true,
            syscall_anomaly_enabled: true,
            syscall_anomaly_strict_mode: false,
            syscall_anomaly_alert_on_unknown_syscall: true,
            syscall_anomaly_max_denied_events_per_minute: 5,
            syscall_anomaly_max_total_events_per_minute: 120,
            syscall_anomaly_max_alerts_per_minute: 30,
            syscall_anomaly_alert_cooldown_secs: 20,
            syscall_anomaly_log_path: "syscall-anomalies.log".to_string(),
            syscall_anomaly_baseline_syscalls_input: String::new(),
            canary_tokens: true,
            semantic_guard: false,
            semantic_guard_collection: "semantic_guard".to_string(),
            semantic_guard_threshold: 0.82,
            show_help_modal: false,
            save_error: None,
        }
    }
}

/// 自主性设置面板状态
///
/// 管理 Agent 自主性级别的配置，包括权限控制、
/// 风险管理和审批策略等。
#[derive(Debug, Clone)]
pub(crate) struct AutonomySettingsState {
    /// 自主性级别
    pub(crate) level: vw_config_types::security::AutonomyLevel,
    /// 是否限制在工作区
    pub(crate) workspace_only: bool,
    /// 允许的命令列表输入
    pub(crate) allowed_commands_input: String,
    /// 禁止的路径列表输入
    pub(crate) forbidden_paths_input: String,
    /// 每小时最大操作数
    pub(crate) max_actions_per_hour: u32,
    /// 每天最大成本（美分）
    pub(crate) max_cost_per_day_cents: u32,
    /// 中等风险操作是否需要审批
    pub(crate) require_approval_for_medium_risk: bool,
    /// 是否阻止高风险命令
    pub(crate) block_high_risk_commands: bool,
    /// Shell 重定向策略
    pub(crate) shell_redirect_policy: vw_config_types::security::ShellRedirectPolicy,
    /// Shell 环境变量透传输入
    pub(crate) shell_env_passthrough_input: String,
    /// 自动审批的操作输入
    pub(crate) auto_approve_input: String,
    /// 始终询问的操作输入
    pub(crate) always_ask_input: String,
    /// 允许的根目录输入
    pub(crate) allowed_roots_input: String,
    /// 非 CLI 排除的工具输入
    pub(crate) non_cli_excluded_tools_input: String,
    /// 非 CLI 审批人输入
    pub(crate) non_cli_approval_approvers_input: String,
    /// 非 CLI 自然语言审批模式
    pub(crate) non_cli_natural_language_approval_mode:
        vw_config_types::security::NonCliNaturalLanguageApprovalMode,
    /// 按 channel 的非 CLI 自然语言审批模式输入
    pub(crate) non_cli_natural_language_approval_mode_by_channel_input: String,
    /// 是否显示帮助对话框
    pub(crate) show_help_modal: bool,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for AutonomySettingsState {
    fn default() -> Self {
        Self {
            level: vw_config_types::security::AutonomyLevel::Supervised,
            workspace_only: true,
            allowed_commands_input:
                "git, npm, cargo, ls, cat, grep, find, echo, pwd, wc, head, tail, date"
                    .to_string(),
            forbidden_paths_input: "/etc, /root, /home, /usr, /bin, /sbin, /lib, /opt, /boot, /dev, /proc, /sys, /var, /tmp, ~/.ssh, ~/.gnupg, ~/.aws, ~/.config".to_string(),
            max_actions_per_hour: 20,
            max_cost_per_day_cents: 500,
            require_approval_for_medium_risk: true,
            block_high_risk_commands: true,
            shell_redirect_policy: vw_config_types::security::ShellRedirectPolicy::Block,
            shell_env_passthrough_input: String::new(),
            auto_approve_input: "file_read, memory_recall".to_string(),
            always_ask_input: String::new(),
            allowed_roots_input: String::new(),
            non_cli_excluded_tools_input: "shell, file_write, git_operations, browser, browser_open, http_request, schedule, cron_add, cron_remove, cron_update, cron_run, memory_store, memory_forget, proxy_config, model_routing_config, pushover, composio, AgentTool, screenshot, image_info".to_string(),
            non_cli_approval_approvers_input: String::new(),
            non_cli_natural_language_approval_mode:
                vw_config_types::security::NonCliNaturalLanguageApprovalMode::Direct,
            non_cli_natural_language_approval_mode_by_channel_input: String::new(),
            show_help_modal: false,
            save_error: None,
        }
    }
}

/// 可观测性设置面板状态
///
/// 管理系统可观测性的配置，包括 OpenTelemetry 和运行时追踪等。
#[derive(Debug, Clone)]
pub(crate) struct ObservabilitySettingsState {
    /// 后端类型
    pub(crate) backend: String,
    /// OTEL 端点输入
    pub(crate) otel_endpoint_input: String,
    /// OTEL 服务名称输入
    pub(crate) otel_service_name_input: String,
    /// 运行时追踪模式
    pub(crate) runtime_trace_mode: String,
    /// 运行时追踪路径输入
    pub(crate) runtime_trace_path_input: String,
    /// 运行时追踪最大条目数
    pub(crate) runtime_trace_max_entries: u32,
    /// 是否显示帮助对话框
    pub(crate) show_help_modal: bool,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
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

/// 存储设置面板状态
///
/// 管理持久化存储 provider、连接地址、schema、table 与 TLS 配置。
#[derive(Debug, Clone)]
pub(crate) struct StorageSettingsState {
    /// 存储 provider 标识
    pub(crate) provider: String,
    /// 数据库连接 URL 输入
    pub(crate) db_url_input: String,
    /// schema 输入
    pub(crate) schema: String,
    /// table 输入
    pub(crate) table: String,
    /// 连接超时输入（秒）
    pub(crate) connect_timeout_secs_input: String,
    /// 是否启用 TLS
    pub(crate) tls: bool,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for StorageSettingsState {
    fn default() -> Self {
        Self {
            provider: String::new(),
            db_url_input: String::new(),
            schema: "public".to_string(),
            table: "memories".to_string(),
            connect_timeout_secs_input: String::new(),
            tls: false,
            save_error: None,
        }
    }
}

/// 代理设置面板状态
///
/// 管理网络代理的配置，包括 HTTP/HTTPS 代理和作用域等。
#[derive(Debug, Clone)]
pub(crate) struct ProxySettingsState {
    /// 是否启用代理
    pub(crate) enabled: bool,
    /// HTTP 代理地址
    pub(crate) http_proxy: String,
    /// HTTPS 代理地址
    pub(crate) https_proxy: String,
    /// 全局代理地址
    pub(crate) all_proxy: String,
    /// 不使用代理的地址输入
    pub(crate) no_proxy_input: String,
    /// 代理作用域
    pub(crate) scope: vw_config_types::proxy::ProxyScope,
    /// 服务列表输入
    pub(crate) services_input: String,
    /// 是否显示帮助对话框
    pub(crate) show_help_modal: bool,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for ProxySettingsState {
    fn default() -> Self {
        Self {
            enabled: false,
            http_proxy: String::new(),
            https_proxy: String::new(),
            all_proxy: String::new(),
            no_proxy_input: String::new(),
            scope: vw_config_types::proxy::ProxyScope::Vibewindow,
            services_input: String::new(),
            show_help_modal: false,
            save_error: None,
        }
    }
}

/// 隧道设置面板状态
///
/// 管理网关公网暴露的隧道 provider 及其参数。
#[derive(Debug, Clone)]
pub(crate) struct TunnelSettingsState {
    /// 当前选择的隧道提供商
    pub(crate) provider: String,
    /// Cloudflare Tunnel token
    pub(crate) cloudflare_token: String,
    /// Tailscale 是否启用 funnel
    pub(crate) tailscale_funnel: bool,
    /// Tailscale 主机名
    pub(crate) tailscale_hostname: String,
    /// ngrok auth token
    pub(crate) ngrok_auth_token: String,
    /// ngrok 自定义域名
    pub(crate) ngrok_domain: String,
    /// 自定义隧道启动命令
    pub(crate) custom_start_command: String,
    /// 自定义隧道健康检查 URL
    pub(crate) custom_health_url: String,
    /// 自定义隧道公网 URL 匹配模式
    pub(crate) custom_url_pattern: String,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for TunnelSettingsState {
    fn default() -> Self {
        Self {
            provider: "none".to_string(),
            cloudflare_token: String::new(),
            tailscale_funnel: false,
            tailscale_hostname: String::new(),
            ngrok_auth_token: String::new(),
            ngrok_domain: String::new(),
            custom_start_command: String::new(),
            custom_health_url: String::new(),
            custom_url_pattern: String::new(),
            save_error: None,
        }
    }
}

/// Composio 设置面板状态
///
/// 管理 Composio 工具集成的启用状态、API 密钥和实体标识。
#[derive(Debug, Clone)]
pub(crate) struct ComposioSettingsState {
    /// 是否启用 Composio 集成
    pub(crate) enabled: bool,
    /// API 密钥输入
    pub(crate) api_key_input: String,
    /// 默认实体 ID 输入
    pub(crate) entity_id_input: String,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for ComposioSettingsState {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key_input: String::new(),
            entity_id_input: "default".to_string(),
            save_error: None,
        }
    }
}

/// 转录设置面板状态
///
/// 管理音频转录功能的配置，包括 API 地址、模型和语言等。
#[derive(Debug, Clone)]
pub(crate) struct TranscriptionSettingsState {
    /// 是否启用转录
    pub(crate) enabled: bool,
    /// API URL
    pub(crate) api_url: String,
    /// 转录模型
    pub(crate) model: String,
    /// 语言代码
    pub(crate) language: String,
    /// 最大录音时长（秒）
    pub(crate) max_duration_secs: u64,
    /// 是否显示帮助对话框
    pub(crate) show_help_modal: bool,
    /// 保存错误信息
    pub(crate) save_error: Option<String>,
}

impl Default for TranscriptionSettingsState {
    fn default() -> Self {
        Self {
            enabled: false,
            api_url: "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
            model: "whisper-large-v3-turbo".to_string(),
            language: String::new(),
            max_duration_secs: 120,
            show_help_modal: false,
            save_error: None,
        }
    }
}

#[cfg(test)]
#[path = "infrastructure_tests.rs"]
mod infrastructure_tests;
