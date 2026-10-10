use super::*;

#[derive(Debug, Clone)]
pub(crate) struct MultimodalSettingsState {
    pub(crate) max_images: u32,
    pub(crate) max_image_size_mb: u32,
    pub(crate) allow_remote_fetch: bool,
    pub(crate) ocr_enabled: bool,
    pub(crate) model_supports_images: bool,
    pub(crate) ocr_model: String,
    pub(crate) save_error: Option<String>,
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
    pub(crate) new_agent_expanded: bool,
    pub(crate) new_agent_name_input: String,
    pub(crate) new_agent_command_input: String,
    pub(crate) new_agent_args_input: String,
    pub(crate) loading: bool,
    pub(crate) adding_agent: bool,
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
            new_agent_expanded: false,
            new_agent_name_input: String::new(),
            new_agent_command_input: String::new(),
            new_agent_args_input: String::new(),
            loading: false,
            adding_agent: false,
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
    pub(crate) editing_server_index: Option<usize>,
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
    /// 待删除确认的 MCP 服务器索引。
    pub(crate) delete_pending: Option<usize>,
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
            editing_server_index: None,
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
            delete_pending: None,
        }
    }
}

/// 安全设置面板状态
///
/// 管理系统安全性的配置，包括沙箱、审计、
/// OTP、紧急停止和系统调用异常检测等。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityListKind {
    SandboxFirejailArgs,
    OtpGatedActions,
    OtpGatedDomains,
    OtpGatedDomainCategories,
    SyscallAnomalyBaselineSyscalls,
}

impl SecurityListKind {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 5] = [
        Self::SandboxFirejailArgs,
        Self::OtpGatedActions,
        Self::OtpGatedDomains,
        Self::OtpGatedDomainCategories,
        Self::SyscallAnomalyBaselineSyscalls,
    ];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::SandboxFirejailArgs => 0,
            Self::OtpGatedActions => 1,
            Self::OtpGatedDomains => 2,
            Self::OtpGatedDomainCategories => 3,
            Self::SyscallAnomalyBaselineSyscalls => 4,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SecurityListEditorState {
    drafts: [String; 5],
    pending_remove: Option<(SecurityListKind, usize)>,
}

impl SecurityListEditorState {
    pub(crate) fn draft(&self, kind: SecurityListKind) -> &str {
        &self.drafts[kind.index()]
    }

    pub(crate) fn set_draft(&mut self, kind: SecurityListKind, value: String) {
        self.drafts[kind.index()] = value;
    }

    pub(crate) fn request_remove(&mut self, kind: SecurityListKind, index: usize) {
        self.pending_remove = Some((kind, index));
    }

    pub(crate) fn is_remove_pending(&self, kind: SecurityListKind, index: usize) -> bool {
        self.pending_remove == Some((kind, index))
    }

    pub(crate) fn pending_remove_index(&self, kind: SecurityListKind) -> Option<usize> {
        self.pending_remove
            .and_then(|(pending_kind, index)| (pending_kind == kind).then_some(index))
    }

    pub(crate) fn cancel_remove(&mut self) {
        self.pending_remove = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecuritySettingsTab {
    Bash,
    SandboxAndResources,
    Audit,
    OtpAndEstop,
    Syscall,
    Guard,
}

#[derive(Debug, Clone)]
pub(crate) struct SecuritySettingsState {
    /// 当前显示的安全配置页签
    pub(crate) active_tab: SecuritySettingsTab,
    /// Bash 命令执行前安全检查的完整分层配置
    pub(crate) bash: vw_config_types::security::BashSecurityConfig,
    /// 沙箱启用状态输入
    pub(crate) sandbox_enabled_input: String,
    /// 沙箱后端输入
    pub(crate) sandbox_backend_input: String,
    /// Firejail 参数列表
    pub(crate) sandbox_firejail_args: Vec<String>,
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
    /// OTP 门控操作列表
    pub(crate) otp_gated_actions: Vec<String>,
    /// OTP 门控域列表
    pub(crate) otp_gated_domains: Vec<String>,
    /// OTP 门控域类别列表
    pub(crate) otp_gated_domain_categories: Vec<String>,
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
    /// 基线系统调用列表
    pub(crate) syscall_anomaly_baseline_syscalls: Vec<String>,
    /// 五个逐项列表的草稿输入
    pub(crate) list_editor: SecurityListEditorState,
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
            active_tab: SecuritySettingsTab::Bash,
            bash: vw_config_types::security::BashSecurityConfig::default(),
            sandbox_enabled_input: "false".to_string(),
            sandbox_backend_input: "none".to_string(),
            sandbox_firejail_args: Vec::new(),
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
            otp_gated_actions: vec![
                "shell".to_string(),
                "file_write".to_string(),
                "browser_open".to_string(),
                "browser".to_string(),
                "memory_forget".to_string(),
            ],
            otp_gated_domains: Vec::new(),
            otp_gated_domain_categories: Vec::new(),
            estop_enabled: false,
            estop_state_file: vw_config_types::paths::estop_state_file_path(),
            estop_require_otp_to_resume: true,
            syscall_anomaly_enabled: false,
            syscall_anomaly_strict_mode: false,
            syscall_anomaly_alert_on_unknown_syscall: false,
            syscall_anomaly_max_denied_events_per_minute: 5,
            syscall_anomaly_max_total_events_per_minute: 120,
            syscall_anomaly_max_alerts_per_minute: 30,
            syscall_anomaly_alert_cooldown_secs: 20,
            syscall_anomaly_log_path: "syscall-anomalies.log".to_string(),
            syscall_anomaly_baseline_syscalls: Vec::new(),
            list_editor: SecurityListEditorState::default(),
            canary_tokens: true,
            semantic_guard: false,
            semantic_guard_collection: "semantic_guard".to_string(),
            semantic_guard_threshold: 0.82,
            show_help_modal: false,
            save_error: None,
        }
    }
}

impl SecuritySettingsState {
    pub(crate) fn list(&self, kind: SecurityListKind) -> &[String] {
        match kind {
            SecurityListKind::SandboxFirejailArgs => &self.sandbox_firejail_args,
            SecurityListKind::OtpGatedActions => &self.otp_gated_actions,
            SecurityListKind::OtpGatedDomains => &self.otp_gated_domains,
            SecurityListKind::OtpGatedDomainCategories => &self.otp_gated_domain_categories,
            SecurityListKind::SyscallAnomalyBaselineSyscalls => {
                &self.syscall_anomaly_baseline_syscalls
            }
        }
    }

    pub(crate) fn list_mut(&mut self, kind: SecurityListKind) -> &mut Vec<String> {
        match kind {
            SecurityListKind::SandboxFirejailArgs => &mut self.sandbox_firejail_args,
            SecurityListKind::OtpGatedActions => &mut self.otp_gated_actions,
            SecurityListKind::OtpGatedDomains => &mut self.otp_gated_domains,
            SecurityListKind::OtpGatedDomainCategories => &mut self.otp_gated_domain_categories,
            SecurityListKind::SyscallAnomalyBaselineSyscalls => {
                &mut self.syscall_anomaly_baseline_syscalls
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutonomyListKind {
    AllowedCommands,
    ForbiddenPaths,
    ShellEnvPassthrough,
    AutoApprove,
    AlwaysAsk,
    AllowedRoots,
    NonCliExcludedTools,
}

impl AutonomyListKind {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 7] = [
        Self::AllowedCommands,
        Self::ForbiddenPaths,
        Self::ShellEnvPassthrough,
        Self::AutoApprove,
        Self::AlwaysAsk,
        Self::AllowedRoots,
        Self::NonCliExcludedTools,
    ];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::AllowedCommands => 0,
            Self::ForbiddenPaths => 1,
            Self::ShellEnvPassthrough => 2,
            Self::AutoApprove => 3,
            Self::AlwaysAsk => 4,
            Self::AllowedRoots => 5,
            Self::NonCliExcludedTools => 6,
        }
    }

    pub(crate) const fn is_tool_list(self) -> bool {
        matches!(self, Self::AutoApprove | Self::AlwaysAsk | Self::NonCliExcludedTools)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct AutonomyListEditorState {
    drafts: [String; 7],
    pending_remove: Option<(AutonomyListKind, usize)>,
}

impl AutonomyListEditorState {
    pub(crate) fn draft(&self, kind: AutonomyListKind) -> &str {
        &self.drafts[kind.index()]
    }

    pub(crate) fn set_draft(&mut self, kind: AutonomyListKind, value: String) {
        self.drafts[kind.index()] = value;
    }

    pub(crate) fn request_remove(&mut self, kind: AutonomyListKind, index: usize) {
        self.pending_remove = Some((kind, index));
    }

    pub(crate) fn is_remove_pending(&self, kind: AutonomyListKind, index: usize) -> bool {
        self.pending_remove == Some((kind, index))
    }

    pub(crate) fn pending_remove_index(&self, kind: AutonomyListKind) -> Option<usize> {
        self.pending_remove
            .and_then(|(pending_kind, index)| (pending_kind == kind).then_some(index))
    }

    pub(crate) fn cancel_remove(&mut self) {
        self.pending_remove = None;
    }
}

/// 自主性设置面板状态
///
/// 管理 Agent 自主性级别的配置，包括权限控制、
/// 风险管理和审批策略等。
#[derive(Debug, Clone)]
pub(crate) struct AutonomySettingsState {
    /// 当前显示的自治配置页签
    pub(crate) active_tab: AutonomySettingsTab,
    /// 自主性级别
    pub(crate) level: vw_config_types::security::AutonomyLevel,
    /// 是否限制在工作区
    pub(crate) workspace_only: bool,
    /// 允许的命令列表
    pub(crate) allowed_commands: Vec<String>,
    /// 禁止的路径列表
    pub(crate) forbidden_paths: Vec<String>,
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
    /// Shell 环境变量透传
    pub(crate) shell_env_passthrough: Vec<String>,
    /// 自动审批的操作
    pub(crate) auto_approve: Vec<String>,
    /// 始终询问的操作
    pub(crate) always_ask: Vec<String>,
    /// 允许的根目录
    pub(crate) allowed_roots: Vec<String>,
    /// 非 CLI 排除的工具
    pub(crate) non_cli_excluded_tools: Vec<String>,
    /// 七个逐项列表的草稿输入
    pub(crate) list_editor: AutonomyListEditorState,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutonomySettingsTab {
    Permissions,
    Budget,
    ToolsAndPaths,
    NonCli,
}

impl Default for AutonomySettingsState {
    fn default() -> Self {
        Self::from_config(&vw_config_types::security::AutonomyConfig::default())
    }
}

impl AutonomySettingsState {
    pub(crate) fn from_config(config: &vw_config_types::security::AutonomyConfig) -> Self {
        Self {
            active_tab: AutonomySettingsTab::Permissions,
            level: config.level,
            workspace_only: config.workspace_only,
            allowed_commands: config.allowed_commands.clone(),
            forbidden_paths: config.forbidden_paths.clone(),
            max_actions_per_hour: config.max_actions_per_hour,
            max_cost_per_day_cents: config.max_cost_per_day_cents,
            require_approval_for_medium_risk: config.require_approval_for_medium_risk,
            block_high_risk_commands: config.block_high_risk_commands,
            shell_redirect_policy: config.shell_redirect_policy,
            shell_env_passthrough: config.shell_env_passthrough.clone(),
            auto_approve: config.auto_approve.clone(),
            always_ask: config.always_ask.clone(),
            allowed_roots: config.allowed_roots.clone(),
            non_cli_excluded_tools: config.non_cli_excluded_tools.clone(),
            list_editor: AutonomyListEditorState::default(),
            non_cli_approval_approvers_input: config.non_cli_approval_approvers.join(", "),
            non_cli_natural_language_approval_mode: config.non_cli_natural_language_approval_mode,
            non_cli_natural_language_approval_mode_by_channel_input: config
                .non_cli_natural_language_approval_mode_by_channel
                .iter()
                .map(|(channel, mode)| {
                    let mode = match mode {
                        vw_config_types::security::NonCliNaturalLanguageApprovalMode::Disabled => {
                            "disabled"
                        }
                        vw_config_types::security::NonCliNaturalLanguageApprovalMode::RequestConfirm => {
                            "request_confirm"
                        }
                        vw_config_types::security::NonCliNaturalLanguageApprovalMode::Direct => {
                            "direct"
                        }
                    };
                    format!("{channel}:{mode}")
                })
                .collect::<Vec<_>>()
                .join(", "),
            show_help_modal: false,
            save_error: None,
        }
    }

    pub(crate) fn list(&self, kind: AutonomyListKind) -> &[String] {
        match kind {
            AutonomyListKind::AllowedCommands => &self.allowed_commands,
            AutonomyListKind::ForbiddenPaths => &self.forbidden_paths,
            AutonomyListKind::ShellEnvPassthrough => &self.shell_env_passthrough,
            AutonomyListKind::AutoApprove => &self.auto_approve,
            AutonomyListKind::AlwaysAsk => &self.always_ask,
            AutonomyListKind::AllowedRoots => &self.allowed_roots,
            AutonomyListKind::NonCliExcludedTools => &self.non_cli_excluded_tools,
        }
    }

    pub(crate) fn list_mut(&mut self, kind: AutonomyListKind) -> &mut Vec<String> {
        match kind {
            AutonomyListKind::AllowedCommands => &mut self.allowed_commands,
            AutonomyListKind::ForbiddenPaths => &mut self.forbidden_paths,
            AutonomyListKind::ShellEnvPassthrough => &mut self.shell_env_passthrough,
            AutonomyListKind::AutoApprove => &mut self.auto_approve,
            AutonomyListKind::AlwaysAsk => &mut self.always_ask,
            AutonomyListKind::AllowedRoots => &mut self.allowed_roots,
            AutonomyListKind::NonCliExcludedTools => &mut self.non_cli_excluded_tools,
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

/// 主题设置页视图模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ThemesSettingsMode {
    #[default]
    List,
    Edit,
}

/// 主题编辑器内部分段 Tab（降低单页高度）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeEditorTab {
    #[default]
    Basics,
    Palette,
    Assets,
    Chrome,
    Preview,
}

/// 主题编辑器草稿：命名 + 六色 hex + 资源路径 + chrome。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ThemeEditorDraft {
    pub(crate) id: String,
    /// 编辑已有主题时锁定 id。
    pub(crate) id_readonly: bool,
    /// 非空时表示正在编辑内置主题覆盖层（值为内置显示名）。
    pub(crate) builtin_base: Option<String>,
    pub(crate) name: String,
    pub(crate) tagline: String,
    pub(crate) background: String,
    pub(crate) text: String,
    pub(crate) primary: String,
    pub(crate) success: String,
    pub(crate) warning: String,
    pub(crate) danger: String,
    /// 已入库的封面相对文件名（如 `cover.png`）。
    pub(crate) cover_relative: Option<String>,
    /// 已入库的背景相对文件名（如 `background.jpg`）。
    pub(crate) background_relative: Option<String>,
    /// 已入库的聊头分区相对文件名。
    pub(crate) chat_header_relative: Option<String>,
    /// 已入库的消息区分区相对文件名。
    pub(crate) chat_body_relative: Option<String>,
    /// 已入库的输入区分区相对文件名。
    pub(crate) chat_input_relative: Option<String>,
    /// 待保存时复制的封面外部源路径。
    pub(crate) cover_source: Option<String>,
    /// 待保存时复制的背景外部源路径。
    pub(crate) background_source: Option<String>,
    /// 待保存时复制的聊头分区外部源路径。
    pub(crate) chat_header_source: Option<String>,
    /// 待保存时复制的消息区分区外部源路径。
    pub(crate) chat_body_source: Option<String>,
    /// 待保存时复制的输入区分区外部源路径。
    pub(crate) chat_input_source: Option<String>,
    pub(crate) chrome_input_bg: String,
    pub(crate) chrome_input_border: String,
    pub(crate) chrome_input_text: String,
    pub(crate) chrome_panel_bg: String,
    pub(crate) chrome_panel_border: String,
    pub(crate) chrome_button_bg: String,
    pub(crate) chrome_button_text: String,
    pub(crate) chrome_radius_px: String,
    pub(crate) chrome_panel_alpha: String,
    pub(crate) chrome_chat_header_alpha: String,
    pub(crate) chrome_chat_body_alpha: String,
    pub(crate) chrome_chat_input_alpha: String,
    /// 聊头背景图缩放：cover/contain/fill/scale_down/none；空表示默认 cover。
    pub(crate) chrome_chat_header_image_fit: String,
    /// 消息区背景图缩放。
    pub(crate) chrome_chat_body_image_fit: String,
    /// 输入区背景图缩放。
    pub(crate) chrome_chat_input_image_fit: String,
}

impl Default for ThemeEditorDraft {
    fn default() -> Self {
        Self {
            id: "theme".to_string(),
            id_readonly: false,
            builtin_base: None,
            name: String::new(),
            tagline: String::new(),
            background: "#0f172a".to_string(),
            text: "#e2e8f0".to_string(),
            primary: "#38bdf8".to_string(),
            success: "#22c55e".to_string(),
            warning: "#f59e0b".to_string(),
            danger: "#ef4444".to_string(),
            cover_relative: None,
            background_relative: None,
            chat_header_relative: None,
            chat_body_relative: None,
            chat_input_relative: None,
            cover_source: None,
            background_source: None,
            chat_header_source: None,
            chat_body_source: None,
            chat_input_source: None,
            chrome_input_bg: String::new(),
            chrome_input_border: String::new(),
            chrome_input_text: String::new(),
            chrome_panel_bg: String::new(),
            chrome_panel_border: String::new(),
            chrome_button_bg: String::new(),
            chrome_button_text: String::new(),
            chrome_radius_px: String::new(),
            chrome_panel_alpha: String::new(),
            chrome_chat_header_alpha: String::new(),
            chrome_chat_body_alpha: String::new(),
            chrome_chat_input_alpha: String::new(),
            chrome_chat_header_image_fit: String::new(),
            chrome_chat_body_image_fit: String::new(),
            chrome_chat_input_image_fit: String::new(),
        }
    }
}

/// 主题设置面板状态。
#[derive(Debug, Clone)]
pub(crate) struct ThemesSettingsState {
    /// 画廊条目（内置 + 自定义）。
    pub(crate) entries:
        Vec<crate::app::components::system_settings_themes::gallery::ThemeGalleryEntry>,
    /// 画廊当前页码（从 0 起）。
    pub(crate) page_index: usize,
    /// 最近一次刷新/操作/校验错误。
    pub(crate) error: Option<String>,
    /// 列表或编辑模式。
    pub(crate) mode: ThemesSettingsMode,
    /// 编辑器当前分段 Tab（仅 Edit 模式使用）。
    pub(crate) editor_tab: ThemeEditorTab,
    /// 编辑器草稿（仅 Edit 模式使用）。
    pub(crate) draft: Option<ThemeEditorDraft>,
    /// 当前打开的颜色选择器目标；`None` 表示关闭。
    pub(crate) color_picker_target: Option<crate::app::message::settings::ThemeColorPickerTarget>,
    /// 颜色选择器显示格式。
    pub(crate) color_picker_format: crate::app::components::color_picker::ColorFormat,
}

impl Default for ThemesSettingsState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            page_index: 0,
            error: None,
            mode: ThemesSettingsMode::List,
            editor_tab: ThemeEditorTab::Basics,
            draft: None,
            color_picker_target: None,
            color_picker_format: crate::app::components::color_picker::ColorFormat::Hex,
        }
    }
}

#[cfg(test)]
#[path = "infrastructure_tests.rs"]
mod infrastructure_tests;
