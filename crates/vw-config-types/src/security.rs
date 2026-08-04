//! 安全与自治策略配置模块。
//!
//! 本模块定义代理在执行工具、访问文件系统、使用沙箱、记录审计日志、触发 OTP
//! 校验等场景下的安全相关配置。
//!
//! # 主要关注点
//!
//! - 代理自治等级与审批策略
//! - 工具调用的命令、路径与风险边界
//! - 沙箱与资源限制
//! - 审计、急停、OTP 与异常系统调用监控

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 代理自治等级。
///
/// 用于决定代理在没有人工参与的情况下可以执行到什么程度。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AutonomyLevel {
    /// 仅允许只读操作。
    ReadOnly,
    /// 需要监督的模式。
    Supervised,
    /// 允许完全自主执行的默认模式。
    #[default]
    Full,
}

impl std::str::FromStr for AutonomyLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "read_only" | "readonly" => Ok(Self::ReadOnly),
            "supervised" => Ok(Self::Supervised),
            "full" => Ok(Self::Full),
            _ => Err(format!(
                "invalid autonomy level '{s}': expected read_only, supervised, or full"
            )),
        }
    }
}

/// Shell 重定向处理策略。
///
/// 用于控制 shell 命令中出现的 `>`、`>>` 等重定向语法如何被处理。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShellRedirectPolicy {
    /// 直接阻止包含重定向的命令。
    Block,
    /// 剥离重定向片段后再执行。
    Strip,
    /// 保留重定向并直接执行。
    #[default]
    Allow,
}

/// 密钥与敏感信息存储配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecretsConfig {
    /// 是否对持久化的敏感信息进行加密。
    #[serde(default = "default_true")]
    pub encrypt: bool,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self { encrypt: true }
    }
}

/// 代理身份文档配置。
///
/// 用于指定身份文档格式，以及文档来自文件还是内联内容。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IdentityConfig {
    /// 身份文档格式，例如 `openclaw`。
    #[serde(default = "default_identity_format")]
    pub format: String,
    /// 外部身份文档文件路径。
    #[serde(default)]
    pub aieos_path: Option<String>,
    /// 内联身份文档内容。
    #[serde(default)]
    pub aieos_inline: Option<String>,
}

fn default_identity_format() -> String {
    "openclaw".into()
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self { format: default_identity_format(), aieos_path: None, aieos_inline: None }
    }
}

/// 非 CLI 通道的自然语言审批模式。
///
/// 用于控制如 IM、协作平台等非命令行入口中，代理如何处理自然语言批准请求。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NonCliNaturalLanguageApprovalMode {
    /// 禁用自然语言审批。
    Disabled,
    /// 先请求确认，再继续执行。
    RequestConfirm,
    /// 直接把自然语言批准当作有效批准。
    #[default]
    Direct,
}

/// 代理自治配置。
///
/// 该结构集中定义代理在执行命令与工具调用时的权限边界、审批阈值以及资源预算。
/// 它是代理风险控制的第一层配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AutonomyConfig {
    /// 代理自治等级。
    pub level: AutonomyLevel,
    /// 是否仅允许在工作区范围内操作。
    pub workspace_only: bool,
    /// 允许执行的命令白名单。
    pub allowed_commands: Vec<String>,
    /// 禁止访问的路径列表。
    pub forbidden_paths: Vec<String>,
    /// 每小时允许执行的最大动作数。
    pub max_actions_per_hour: u32,
    /// 每日允许消耗的最大成本，单位为美分。
    pub max_cost_per_day_cents: u32,

    /// 中风险操作是否需要审批。
    pub require_approval_for_medium_risk: bool,

    /// 是否阻止高风险命令。
    pub block_high_risk_commands: bool,

    /// Shell 重定向处理策略。
    #[serde(default)]
    pub shell_redirect_policy: ShellRedirectPolicy,

    /// 允许透传到 shell 环境中的环境变量名列表。
    #[serde(default)]
    pub shell_env_passthrough: Vec<String>,

    /// 默认可自动批准的工具名列表。
    #[serde(default = "default_auto_approve")]
    pub auto_approve: Vec<String>,

    /// 无论如何都要询问用户的工具名列表。
    #[serde(default = "default_always_ask")]
    pub always_ask: Vec<String>,

    /// 额外允许访问的根目录列表。
    #[serde(default)]
    pub allowed_roots: Vec<String>,

    /// 非 CLI 场景中禁用的工具列表。
    #[serde(default = "default_non_cli_excluded_tools")]
    pub non_cli_excluded_tools: Vec<String>,

    /// 非 CLI 场景中可充当审批人的主体列表。
    #[serde(default)]
    pub non_cli_approval_approvers: Vec<String>,

    /// 非 CLI 场景的默认自然语言审批模式。
    #[serde(default)]
    pub non_cli_natural_language_approval_mode: NonCliNaturalLanguageApprovalMode,

    /// 是否允许潜在危险的 shell 模式。
    #[serde(default)]
    pub allow_unsafe_shell_patterns: bool,

    /// 按通道覆盖的自然语言审批模式映射。
    #[serde(default)]
    pub non_cli_natural_language_approval_mode_by_channel:
        HashMap<String, NonCliNaturalLanguageApprovalMode>,
}

fn default_auto_approve() -> Vec<String> {
    vec!["file_read".into(), "memory_recall".into()]
}

fn default_always_ask() -> Vec<String> {
    vec![]
}

fn default_non_cli_excluded_tools() -> Vec<String> {
    [
        "bash",
        "file_write",
        "git_operations",
        "browser",
        "browser_open",
        "http_request",
        "schedule",
        "cron_add",
        "cron_remove",
        "cron_update",
        "cron_run",
        "memory_store",
        "memory_forget",
        "proxy_config",
        "model_routing_config",
        "pushover",
        "composio",
        "delegate",
        "screenshot",
        "image_info",
    ]
    .into_iter()
    .map(std::string::ToString::to_string)
    .collect()
}

impl Default for AutonomyConfig {
    fn default() -> Self {
        Self {
            level: AutonomyLevel::Full,
            workspace_only: false,
            allowed_commands: vec![
                "git".into(),
                "npm".into(),
                "cargo".into(),
                "ls".into(),
                "cat".into(),
                "grep".into(),
                "find".into(),
                "echo".into(),
                "pwd".into(),
                "wc".into(),
                "head".into(),
                "tail".into(),
                "date".into(),
            ],
            forbidden_paths: vec![
                "/etc".into(),
                "/root".into(),
                "/home".into(),
                "/usr".into(),
                "/bin".into(),
                "/sbin".into(),
                "/lib".into(),
                "/opt".into(),
                "/boot".into(),
                "/dev".into(),
                "/proc".into(),
                "/sys".into(),
                "/var".into(),
                "/tmp".into(),
                "~/.ssh".into(),
                "~/.gnupg".into(),
                "~/.aws".into(),
                "~/.config".into(),
            ],
            max_actions_per_hour: 2_000,
            max_cost_per_day_cents: 50_000,
            require_approval_for_medium_risk: false,
            block_high_risk_commands: false,
            shell_redirect_policy: ShellRedirectPolicy::Allow,
            shell_env_passthrough: vec![],
            auto_approve: default_auto_approve(),
            always_ask: default_always_ask(),
            allowed_roots: Vec::new(),
            non_cli_excluded_tools: default_non_cli_excluded_tools(),
            non_cli_approval_approvers: Vec::new(),
            non_cli_natural_language_approval_mode: NonCliNaturalLanguageApprovalMode::default(),
            non_cli_natural_language_approval_mode_by_channel: HashMap::new(),
            allow_unsafe_shell_patterns: false,
        }
    }
}

/// Bash 策略检查配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct BashPolicyChecksConfig {
    /// 是否启用策略检查。
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 是否检查只读约束。
    #[serde(default = "default_true")]
    pub read_only: bool,
    /// 是否检查危险 Shell 结构。
    #[serde(default = "default_true")]
    pub unsafe_shell_patterns: bool,
    /// 是否检查命令规则。
    #[serde(default = "default_true")]
    pub command_rules: bool,
    /// 是否检查工作区路径约束。
    #[serde(default = "default_true")]
    pub workspace_paths: bool,
    /// 是否检查禁止路径。
    #[serde(default = "default_true")]
    pub forbidden_paths: bool,
    /// 是否检查风险等级。
    #[serde(default = "default_true")]
    pub risk: bool,
    /// 是否检查重定向。
    #[serde(default = "default_true")]
    pub redirects: bool,
}

impl Default for BashPolicyChecksConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            read_only: true,
            unsafe_shell_patterns: true,
            command_rules: true,
            workspace_paths: true,
            forbidden_paths: true,
            risk: true,
            redirects: true,
        }
    }
}

/// Bash Shell validator 检查配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct BashValidatorChecksConfig {
    /// 是否启用 Shell validator 检查。
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 是否检查空命令。
    #[serde(default = "default_true")]
    pub empty_command: bool,
    /// 是否检查 heredoc。
    #[serde(default = "default_true")]
    pub heredoc: bool,
    /// 是否检查 Git commit 命令。
    #[serde(default = "default_true")]
    pub git_commit: bool,
    /// 是否检查 jq 命令。
    #[serde(default = "default_true")]
    pub jq: bool,
    /// 是否检查 Shell 元字符。
    #[serde(default = "default_true")]
    pub metacharacters: bool,
    /// 是否检查危险变量。
    #[serde(default = "default_true")]
    pub dangerous_vars: bool,
    /// 是否检查命令替换。
    #[serde(default = "default_true")]
    pub substitution: bool,
    /// 是否检查重定向。
    #[serde(default = "default_true")]
    pub redirection: bool,
    /// 是否检查换行注入。
    #[serde(default = "default_true")]
    pub newline_injection: bool,
    /// 是否检查 IFS 注入。
    #[serde(default = "default_true")]
    pub ifs_injection: bool,
    /// 是否检查进程环境访问。
    #[serde(default = "default_true")]
    pub proc_environ: bool,
    /// 是否检查格式异常的命令。
    #[serde(default = "default_true")]
    pub malformed: bool,
    /// 是否检查混淆参数。
    #[serde(default = "default_true")]
    pub obfuscated_flags: bool,
    /// 是否检查反斜杠。
    #[serde(default = "default_true")]
    pub backslash: bool,
    /// 是否检查花括号扩展。
    #[serde(default = "default_true")]
    pub brace_expansion: bool,
    /// 是否检查 Unicode 空白字符。
    #[serde(default = "default_true")]
    pub unicode_whitespace: bool,
    /// 是否检查哈希注释。
    #[serde(default = "default_true")]
    pub hash_comment: bool,
    /// 是否检查注释与引号状态不同步。
    #[serde(default = "default_true")]
    pub comment_quote_desync: bool,
    /// 是否检查引号内换行符。
    #[serde(default)]
    pub quoted_newline: bool,
    /// 是否检查 zsh 危险结构。
    #[serde(default = "default_true")]
    pub zsh_dangerous: bool,
    /// 是否检查控制字符。
    #[serde(default = "default_true")]
    pub control_chars: bool,
}

impl Default for BashValidatorChecksConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            empty_command: true,
            heredoc: true,
            git_commit: true,
            jq: true,
            metacharacters: true,
            dangerous_vars: true,
            substitution: true,
            redirection: true,
            newline_injection: true,
            ifs_injection: true,
            proc_environ: true,
            malformed: true,
            obfuscated_flags: true,
            backslash: true,
            brace_expansion: true,
            unicode_whitespace: true,
            hash_comment: true,
            comment_quote_desync: true,
            quoted_newline: false,
            zsh_dangerous: true,
            control_chars: true,
        }
    }
}

/// Bash 命令执行前安全检查配置。
///
/// 关闭后跳过命令白名单、路径、风险、重定向与 Shell validator 检查。沙箱、超时、
/// 输出限制和系统调用异常检测仍由各自独立配置控制。
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct BashSecurityConfig {
    /// 是否启用 Bash 命令执行前安全检查。
    #[serde(default)]
    pub enabled: bool,
    /// 策略检查配置。
    #[serde(default)]
    pub policy: BashPolicyChecksConfig,
    /// Shell validator 检查配置。
    #[serde(default)]
    pub validators: BashValidatorChecksConfig,
}

/// 顶层安全配置。
///
/// 聚合沙箱、资源限制、审计、OTP、急停与异常调用检测等子系统配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecurityConfig {
    /// Bash 命令执行前安全检查配置。
    #[serde(default)]
    pub bash: BashSecurityConfig,

    /// 沙箱配置。
    #[serde(default)]
    pub sandbox: SandboxConfig,

    /// 资源限制配置。
    #[serde(default)]
    pub resources: ResourceLimitsConfig,

    /// 审计配置。
    #[serde(default)]
    pub audit: AuditConfig,

    /// OTP 配置。
    #[serde(default)]
    pub otp: OtpConfig,

    /// 急停配置。
    #[serde(default)]
    pub estop: EstopConfig,

    /// 系统调用异常检测配置。
    #[serde(default)]
    pub syscall_anomaly: SyscallAnomalyConfig,

    /// 是否启用蜜罐令牌保护。
    #[serde(default = "default_true")]
    pub canary_tokens: bool,

    /// 是否启用语义防护。
    #[serde(default)]
    pub semantic_guard: bool,

    /// 语义防护使用的集合名称。
    #[serde(default = "default_semantic_guard_collection")]
    pub semantic_guard_collection: String,

    /// 语义防护判定阈值。
    #[serde(default = "default_semantic_guard_threshold")]
    pub semantic_guard_threshold: f64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            bash: BashSecurityConfig::default(),
            sandbox: SandboxConfig::default(),
            resources: ResourceLimitsConfig::default(),
            audit: AuditConfig::default(),
            otp: OtpConfig::default(),
            estop: EstopConfig::default(),
            syscall_anomaly: SyscallAnomalyConfig::default(),
            canary_tokens: default_true(),
            semantic_guard: false,
            semantic_guard_collection: default_semantic_guard_collection(),
            semantic_guard_threshold: default_semantic_guard_threshold(),
        }
    }
}

fn default_semantic_guard_collection() -> String {
    "semantic_guard".to_string()
}

fn default_semantic_guard_threshold() -> f64 {
    0.82
}

/// OTP 校验方式。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OtpMethod {
    /// 使用 TOTP 动态口令。
    #[default]
    Totp,
    /// 使用配对流程确认。
    Pairing,
    /// 使用 CLI 提示确认。
    CliPrompt,
}

/// OTP 保护配置。
///
/// 用于要求部分敏感动作在执行前经过一次性口令验证。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OtpConfig {
    /// 是否启用 OTP。
    #[serde(default)]
    pub enabled: bool,

    /// OTP 校验方式。
    #[serde(default)]
    pub method: OtpMethod,

    /// OTP token 的有效期，单位为秒。
    #[serde(default = "default_otp_token_ttl_secs")]
    pub token_ttl_secs: u64,

    /// 已验证 token 的缓存有效期，单位为秒。
    #[serde(default = "default_otp_cache_valid_secs")]
    pub cache_valid_secs: u64,

    /// 需要 OTP 保护的动作列表。
    #[serde(default = "default_otp_gated_actions")]
    pub gated_actions: Vec<String>,

    /// 需要 OTP 保护的域名列表。
    #[serde(default)]
    pub gated_domains: Vec<String>,

    /// 需要 OTP 保护的域名分类列表。
    #[serde(default)]
    pub gated_domain_categories: Vec<String>,
}

fn default_otp_token_ttl_secs() -> u64 {
    30
}

fn default_otp_cache_valid_secs() -> u64 {
    300
}

fn default_otp_gated_actions() -> Vec<String> {
    vec![
        "bash".to_string(),
        "file_write".to_string(),
        "browser_open".to_string(),
        "browser".to_string(),
        "memory_forget".to_string(),
    ]
}

impl Default for OtpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            method: OtpMethod::Totp,
            token_ttl_secs: default_otp_token_ttl_secs(),
            cache_valid_secs: default_otp_cache_valid_secs(),
            gated_actions: default_otp_gated_actions(),
            gated_domains: Vec::new(),
            gated_domain_categories: Vec::new(),
        }
    }
}

/// 急停配置。
///
/// 急停启用后，运行时可以进入一种全局暂停状态，阻止敏感动作继续执行。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstopConfig {
    /// 是否启用急停功能。
    #[serde(default)]
    pub enabled: bool,

    /// 急停状态文件路径。
    #[serde(default = "default_estop_state_file")]
    pub state_file: String,

    /// 从急停恢复时是否必须经过 OTP。
    #[serde(default = "default_true")]
    pub require_otp_to_resume: bool,
}

fn default_estop_state_file() -> String {
    crate::paths::estop_state_file_path()
}

impl Default for EstopConfig {
    fn default() -> Self {
        Self { enabled: false, state_file: default_estop_state_file(), require_otp_to_resume: true }
    }
}

/// 系统调用异常检测配置。
///
/// 用于对异常系统调用模式进行限流、告警与日志记录，帮助发现潜在越权或逃逸行为。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SyscallAnomalyConfig {
    /// 是否启用系统调用异常检测。
    #[serde(default)]
    pub enabled: bool,

    /// 是否启用严格模式。
    #[serde(default)]
    pub strict_mode: bool,

    #[serde(default)]
    pub alert_on_unknown_syscall: bool,

    #[serde(default = "default_syscall_anomaly_max_denied_events_per_minute")]
    pub max_denied_events_per_minute: u32,

    #[serde(default = "default_syscall_anomaly_max_total_events_per_minute")]
    pub max_total_events_per_minute: u32,

    #[serde(default = "default_syscall_anomaly_max_alerts_per_minute")]
    pub max_alerts_per_minute: u32,

    #[serde(default = "default_syscall_anomaly_alert_cooldown_secs")]
    pub alert_cooldown_secs: u64,

    #[serde(default = "default_syscall_anomaly_log_path")]
    pub log_path: String,

    #[serde(default = "default_syscall_anomaly_baseline_syscalls")]
    pub baseline_syscalls: Vec<String>,
}

fn default_syscall_anomaly_max_denied_events_per_minute() -> u32 {
    5
}

fn default_syscall_anomaly_max_total_events_per_minute() -> u32 {
    120
}

fn default_syscall_anomaly_max_alerts_per_minute() -> u32 {
    30
}

fn default_syscall_anomaly_alert_cooldown_secs() -> u64 {
    20
}

fn default_syscall_anomaly_log_path() -> String {
    "syscall-anomalies.log".to_string()
}

fn default_syscall_anomaly_baseline_syscalls() -> Vec<String> {
    vec![
        "read".to_string(),
        "write".to_string(),
        "open".to_string(),
        "openat".to_string(),
        "close".to_string(),
        "stat".to_string(),
        "fstat".to_string(),
        "newfstatat".to_string(),
        "lseek".to_string(),
        "mmap".to_string(),
        "mprotect".to_string(),
        "munmap".to_string(),
        "brk".to_string(),
        "rt_sigaction".to_string(),
        "rt_sigprocmask".to_string(),
        "ioctl".to_string(),
        "fcntl".to_string(),
        "access".to_string(),
        "pipe2".to_string(),
        "dup".to_string(),
        "dup2".to_string(),
        "dup3".to_string(),
        "epoll_create1".to_string(),
        "epoll_ctl".to_string(),
        "epoll_wait".to_string(),
        "poll".to_string(),
        "ppoll".to_string(),
        "select".to_string(),
        "futex".to_string(),
        "clock_gettime".to_string(),
        "nanosleep".to_string(),
        "getpid".to_string(),
        "gettid".to_string(),
        "set_tid_address".to_string(),
        "set_robust_list".to_string(),
        "clone".to_string(),
        "clone3".to_string(),
        "fork".to_string(),
        "execve".to_string(),
        "wait4".to_string(),
        "exit".to_string(),
        "exit_group".to_string(),
        "socket".to_string(),
        "connect".to_string(),
        "accept".to_string(),
        "accept4".to_string(),
        "listen".to_string(),
        "sendto".to_string(),
        "recvfrom".to_string(),
        "sendmsg".to_string(),
        "recvmsg".to_string(),
        "getsockname".to_string(),
        "getpeername".to_string(),
        "setsockopt".to_string(),
        "getsockopt".to_string(),
        "getrandom".to_string(),
        "statx".to_string(),
    ]
}

impl Default for SyscallAnomalyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            strict_mode: false,
            alert_on_unknown_syscall: false,
            max_denied_events_per_minute: default_syscall_anomaly_max_denied_events_per_minute(),
            max_total_events_per_minute: default_syscall_anomaly_max_total_events_per_minute(),
            max_alerts_per_minute: default_syscall_anomaly_max_alerts_per_minute(),
            alert_cooldown_secs: default_syscall_anomaly_alert_cooldown_secs(),
            log_path: default_syscall_anomaly_log_path(),
            baseline_syscalls: default_syscall_anomaly_baseline_syscalls(),
        }
    }
}

/// 沙箱配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SandboxConfig {
    /// 是否启用沙箱；`None` 表示由运行时自动决定。
    #[serde(default = "default_sandbox_enabled")]
    pub enabled: Option<bool>,

    /// 沙箱后端类型。
    #[serde(default)]
    pub backend: SandboxBackend,

    /// 传递给 firejail 的额外参数。
    #[serde(default)]
    pub firejail_args: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: default_sandbox_enabled(),
            backend: SandboxBackend::default(),
            firejail_args: Vec::new(),
        }
    }
}

fn default_sandbox_enabled() -> Option<bool> {
    Some(false)
}

/// 沙箱后端类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SandboxBackend {
    /// 自动选择最合适的后端。
    Auto,
    /// 使用 Landlock。
    Landlock,
    /// 使用 Firejail。
    Firejail,
    /// 使用 Bubblewrap。
    Bubblewrap,
    /// 使用 Docker。
    Docker,
    /// 不使用沙箱。
    #[default]
    None,
}

/// 资源限制配置。
///
/// 用于限制单次执行可用的内存、CPU 时间和子进程数量。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceLimitsConfig {
    /// 最大内存限制，单位为 MB。
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: u32,

    /// 最大 CPU 时间限制，单位为秒。
    #[serde(default = "default_max_cpu_time_seconds")]
    pub max_cpu_time_seconds: u64,

    /// 最多允许创建的子进程数。
    #[serde(default = "default_max_subprocesses")]
    pub max_subprocesses: u32,

    /// 是否启用内存监控。
    #[serde(default = "default_memory_monitoring_enabled")]
    pub memory_monitoring: bool,
}

fn default_max_memory_mb() -> u32 {
    512
}

fn default_max_cpu_time_seconds() -> u64 {
    60
}

fn default_max_subprocesses() -> u32 {
    10
}

fn default_memory_monitoring_enabled() -> bool {
    true
}

impl Default for ResourceLimitsConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: default_max_memory_mb(),
            max_cpu_time_seconds: default_max_cpu_time_seconds(),
            max_subprocesses: default_max_subprocesses(),
            memory_monitoring: default_memory_monitoring_enabled(),
        }
    }
}

/// 审计日志配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuditConfig {
    /// 是否启用审计日志。
    #[serde(default = "default_audit_enabled")]
    pub enabled: bool,

    /// 审计日志文件路径。
    #[serde(default = "default_audit_log_path")]
    pub log_path: String,

    /// 审计日志文件大小上限，单位为 MB。
    #[serde(default = "default_audit_max_size_mb")]
    pub max_size_mb: u32,

    /// 是否对审计事件签名。
    #[serde(default)]
    pub sign_events: bool,
}

fn default_audit_enabled() -> bool {
    true
}

fn default_audit_log_path() -> String {
    "audit.log".to_string()
}

fn default_audit_max_size_mb() -> u32 {
    100
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: default_audit_enabled(),
            log_path: default_audit_log_path(),
            max_size_mb: default_audit_max_size_mb(),
            sign_events: false,
        }
    }
}

fn default_true() -> bool {
    true
}
#[cfg(test)]
#[path = "security_tests.rs"]
mod security_tests;
