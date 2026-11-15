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
    pub const ALL: [Self; 5] = [
        Self::SandboxFirejailArgs,
        Self::OtpGatedActions,
        Self::OtpGatedDomains,
        Self::OtpGatedDomainCategories,
        Self::SyscallAnomalyBaselineSyscalls,
    ];

    pub const fn index(self) -> usize {
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
pub struct SecurityListEditorState {
    drafts: [String; 5],
    pending_remove: Option<(SecurityListKind, usize)>,
}

impl SecurityListEditorState {
    pub fn draft(&self, kind: SecurityListKind) -> &str {
        &self.drafts[kind.index()]
    }

    pub fn set_draft(&mut self, kind: SecurityListKind, value: String) {
        self.drafts[kind.index()] = value;
    }

    pub fn request_remove(&mut self, kind: SecurityListKind, index: usize) {
        self.pending_remove = Some((kind, index));
    }

    pub fn is_remove_pending(&self, kind: SecurityListKind, index: usize) -> bool {
        self.pending_remove == Some((kind, index))
    }

    pub fn pending_remove_index(&self, kind: SecurityListKind) -> Option<usize> {
        self.pending_remove
            .and_then(|(pending_kind, index)| (pending_kind == kind).then_some(index))
    }

    pub fn cancel_remove(&mut self) {
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
pub struct SecuritySettingsState {
    /// 当前显示的安全配置页签
    pub active_tab: SecuritySettingsTab,
    /// Bash 命令执行前安全检查的完整分层配置
    pub bash: config_security::BashSecurityConfig,
    /// 沙箱启用状态输入
    pub sandbox_enabled_input: String,
    /// 沙箱后端输入
    pub sandbox_backend_input: String,
    /// Firejail 参数列表
    pub sandbox_firejail_args: Vec<String>,
    /// 最大内存限制（MB）
    pub resources_max_memory_mb: u32,
    /// 最大 CPU 时间（秒）
    pub resources_max_cpu_time_seconds: u64,
    /// 最大子进程数
    pub resources_max_subprocesses: u32,
    /// 是否启用内存监控
    pub resources_memory_monitoring: bool,
    /// 是否启用审计
    pub audit_enabled: bool,
    /// 审计日志路径
    pub audit_log_path: String,
    /// 审计日志最大大小（MB）
    pub audit_max_size_mb: u32,
    /// 是否签名审计事件
    pub audit_sign_events: bool,
    /// 是否启用 OTP
    pub otp_enabled: bool,
    /// OTP 方法输入
    pub otp_method_input: String,
    /// OTP 令牌有效期（秒）
    pub otp_token_ttl_secs: u64,
    /// OTP 缓存有效期（秒）
    pub otp_cache_valid_secs: u64,
    /// OTP 门控操作列表
    pub otp_gated_actions: Vec<String>,
    /// OTP 门控域列表
    pub otp_gated_domains: Vec<String>,
    /// OTP 门控域类别列表
    pub otp_gated_domain_categories: Vec<String>,
    /// 是否启用紧急停止
    pub estop_enabled: bool,
    /// 紧急停止状态文件路径
    pub estop_state_file: String,
    /// 恢复时是否需要 OTP
    pub estop_require_otp_to_resume: bool,
    /// 是否启用系统调用异常检测
    pub syscall_anomaly_enabled: bool,
    /// 系统调用异常严格模式
    pub syscall_anomaly_strict_mode: bool,
    /// 未知系统调用时是否告警
    pub syscall_anomaly_alert_on_unknown_syscall: bool,
    /// 每分钟最大拒绝事件数
    pub syscall_anomaly_max_denied_events_per_minute: u32,
    /// 每分钟最大总事件数
    pub syscall_anomaly_max_total_events_per_minute: u32,
    /// 每分钟最大告警数
    pub syscall_anomaly_max_alerts_per_minute: u32,
    /// 告警冷却时间（秒）
    pub syscall_anomaly_alert_cooldown_secs: u64,
    /// 系统调用异常日志路径
    pub syscall_anomaly_log_path: String,
    /// 基线系统调用列表
    pub syscall_anomaly_baseline_syscalls: Vec<String>,
    /// 五个逐项列表的草稿输入
    pub list_editor: SecurityListEditorState,
    /// 是否启用金丝雀令牌
    pub canary_tokens: bool,
    /// 是否启用语义防护
    pub semantic_guard: bool,
    /// 语义防护集合名称
    pub semantic_guard_collection: String,
    /// 语义防护阈值
    pub semantic_guard_threshold: f64,
    /// 是否显示帮助对话框
    pub show_help_modal: bool,
    /// 保存错误信息
    pub save_error: Option<String>,
}

impl Default for SecuritySettingsState {
    fn default() -> Self {
        Self {
            active_tab: SecuritySettingsTab::Bash,
            bash: config_security::BashSecurityConfig::default(),
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
            estop_state_file: config_runtime_paths::estop_state_file_path(),
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
    pub fn list(&self, kind: SecurityListKind) -> &[String] {
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

    pub fn list_mut(&mut self, kind: SecurityListKind) -> &mut Vec<String> {
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
