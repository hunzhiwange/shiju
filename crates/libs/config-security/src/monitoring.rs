use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    config_runtime_paths::estop_state_file_path()
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
