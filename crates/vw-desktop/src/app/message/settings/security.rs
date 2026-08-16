//! 安全设置消息处理模块
//!
//! 该模块负责处理用户界面中所有安全相关设置的消息和状态更新。
//! 它是设置界面的安全配置子系统的核心处理逻辑。
//!
//! # 主要功能
//!
//! - **沙箱配置**：管理沙箱的启用状态、后端选择和自定义参数
//! - **资源限制**：设置内存、CPU时间和子进程数量等资源使用限制
//! - **审计日志**：配置审计日志的启用状态、路径和大小限制
//! - **OTP 认证**：管理一次性密码认证的方法、有效期和受保护操作
//! - **紧急停止**：配置紧急停止机制及其恢复策略
//! - **系统调用监控**：设置异常系统调用的检测和告警规则
//! - **安全令牌**：管理 Canary tokens 和语义防护功能
//!
//! # 架构说明
//!
//! 该模块遵循 VibeWindow 的安全设计原则：
//! - 所有安全配置变更都会立即持久化到配置文件
//! - 输入值会经过规范化和边界检查
//! - 默认采用安全优先策略

use crate::app::config::update_security_config_async;
use crate::app::state::{SecurityListKind, SecuritySettingsState};
use crate::app::{App, Message};
use iced::Task;
use vw_config_types::security::SecurityConfig;

use super::messages::SettingsMessage;

/// 规范化沙箱启用状态输入
///
/// 将用户输入的沙箱启用状态转换为标准格式。支持以下值：
/// - `"auto"`：自动检测是否启用沙箱
/// - `"true"`：强制启用沙箱
/// - `"false"`：强制禁用沙箱
///
/// 任何无效输入都会被转换为 `"false"`，确保默认禁用沙箱。
///
/// # 参数
///
/// - `raw`：用户输入的原始字符串
///
/// # 返回值
///
/// 规范化后的字符串，值为 `"auto"`、`"true"` 或 `"false"`
///
/// # 示例
///
/// ```ignore
/// assert_eq!(normalize_security_sandbox_enabled("  AUTO  "), "auto");
/// assert_eq!(normalize_security_sandbox_enabled("True"), "true");
/// assert_eq!(normalize_security_sandbox_enabled("invalid"), "false");
/// ```
fn normalize_security_sandbox_enabled(raw: &str) -> String {
    let v = raw.trim().to_ascii_lowercase();
    match v.as_str() {
        "auto" | "true" | "false" => v,
        _ => "false".to_string(),
    }
}

/// 规范化沙箱后端类型输入
///
/// 将用户输入的沙箱后端类型转换为标准格式。支持以下后端：
/// - `"auto"`：自动选择最适合的后端
/// - `"landlock"`：Linux Landlock LSM
/// - `"firejail"`：Firejail 沙箱
/// - `"bubblewrap"`：Bubblewrap 容器
/// - `"docker"`：Docker 容器
/// - `"none"`：不使用沙箱
///
/// 任何无效输入都会被转换为 `"none"`。
///
/// # 参数
///
/// - `raw`：用户输入的原始字符串
///
/// # 返回值
///
/// 规范化后的字符串，表示沙箱后端类型
///
/// # 示例
///
/// ```ignore
/// assert_eq!(normalize_security_sandbox_backend("  LANDLOCK  "), "landlock");
/// assert_eq!(normalize_security_sandbox_backend("Firejail"), "firejail");
/// assert_eq!(normalize_security_sandbox_backend("unknown"), "none");
/// ```
fn normalize_security_sandbox_backend(raw: &str) -> String {
    let v = raw.trim().to_ascii_lowercase();
    match v.as_str() {
        "auto" | "landlock" | "firejail" | "bubblewrap" | "docker" | "none" => v,
        _ => "none".to_string(),
    }
}

/// 根据沙箱启用状态约束后端取值。
///
/// - 当启用状态为 `false` 时，只允许 `none`。
/// - 当启用状态为 `true` 时，不允许 `none`，无效值回退到 `auto`。
/// - 当启用状态为 `auto` 时，允许所有已知后端。
fn normalize_security_sandbox_backend_for_enabled(raw: &str, sandbox_enabled: &str) -> String {
    let normalized = normalize_security_sandbox_backend(raw);
    match sandbox_enabled {
        "false" => "none".to_string(),
        "true" if normalized == "none" => "auto".to_string(),
        _ => normalized,
    }
}

/// 规范化 OTP 认证方法输入
///
/// 将用户输入的一次性密码（OTP）认证方法转换为标准格式。支持以下方法：
/// - `"totp"`：基于时间的一次性密码（默认）
/// - `"pairing"`：设备配对认证
/// - `"cli-prompt"`：命令行交互式提示
///
/// 任何无效输入都会被转换为 `"totp"`，确保有默认的认证机制。
///
/// # 参数
///
/// - `raw`：用户输入的原始字符串
///
/// # 返回值
///
/// 规范化后的字符串，表示 OTP 认证方法
///
/// # 示例
///
/// ```ignore
/// assert_eq!(normalize_security_otp_method("  TOTP  "), "totp");
/// assert_eq!(normalize_security_otp_method("Pairing"), "pairing");
/// assert_eq!(normalize_security_otp_method("invalid"), "totp");
/// ```
fn normalize_security_otp_method(raw: &str) -> String {
    let v = raw.trim().to_ascii_lowercase();
    match v.as_str() {
        "totp" | "pairing" | "cli-prompt" => v,
        _ => "totp".to_string(),
    }
}

/// 持久化安全设置到配置文件
///
/// 该函数从应用程序的 UI 状态读取所有安全相关的设置，
/// 对输入值进行规范化和边界检查，然后通过配置系统持久化到磁盘。
///
/// # 处理的配置项
///
/// ## 沙箱配置
/// - `enabled`：沙箱启用状态（true/false/auto）
/// - `backend`：沙箱后端类型（landlock/firejail/bubblewrap/docker/none/auto）
/// - `firejail_args`：Firejail 自定义参数列表
///
/// ## 资源限制
/// - `max_memory_mb`：最大内存使用量（32-65536 MB）
/// - `max_cpu_time_seconds`：最大 CPU 时间（1-86400 秒）
/// - `max_subprocesses`：最大子进程数（1-10000）
/// - `memory_monitoring`：是否启用内存监控
///
/// ## 审计日志
/// - `enabled`：是否启用审计
/// - `log_path`：日志文件路径（默认 "audit.log"）
/// - `max_size_mb`：日志最大大小（1-10000 MB）
/// - `sign_events`：是否对审计事件进行签名
///
/// ## OTP 认证
/// - `enabled`：是否启用 OTP
/// - `method`：认证方法（totp/pairing/cli-prompt）
/// - `token_ttl_secs`：令牌有效期（1-600 秒）
/// - `cache_valid_secs`：缓存有效期（1-86400 秒）
/// - `gated_actions`：需要 OTP 的操作列表
/// - `gated_domains`：需要 OTP 的域名列表
/// - `gated_domain_categories`：需要 OTP 的域名分类列表
///
/// ## 紧急停止
/// - `enabled`：是否启用紧急停止
/// - `state_file`：状态文件路径（默认 "~/.vibewindow/estop-state.json"）
/// - `require_otp_to_resume`：恢复时是否需要 OTP
///
/// ## 系统调用异常检测
/// - `enabled`：是否启用异常检测
/// - `strict_mode`：是否使用严格模式
/// - `alert_on_unknown_syscall`：未知系统调用是否告警
/// - `max_denied_events_per_minute`：每分钟最大拒绝事件数（1-10000）
/// - `max_total_events_per_minute`：每分钟最大总事件数（1-100000）
/// - `max_alerts_per_minute`：每分钟最大告警数（1-10000）
/// - `alert_cooldown_secs`：告警冷却时间（1-3600 秒）
/// - `log_path`：异常日志路径（默认 "syscall-anomalies.log"）
/// - `baseline_syscalls`：基线系统调用列表
///
/// ## 安全令牌和语义防护
/// - `canary_tokens`：是否启用 Canary tokens
/// - `semantic_guard`：是否启用语义防护
/// - `semantic_guard_collection`：语义防护集合名称（默认 "semantic_guard"）
/// - `semantic_guard_threshold`：语义防护阈值（0.0-1.0）
///
/// # 参数
///
/// - `app`：可变引用应用程序状态，用于读取 UI 输入和更新配置
///
/// # 副作用
///
/// 该函数会调用 `crate::app::update_security_config_async` 将配置异步写入磁盘
fn apply_security_settings_to_config(security: &mut SecurityConfig, s: &SecuritySettingsState) {
    let bash = s.bash.clone();
    let resources_max_memory_mb = s.resources_max_memory_mb.clamp(32, 65_536);
    let resources_max_cpu_time_seconds = s.resources_max_cpu_time_seconds.clamp(1, 86_400);
    let resources_max_subprocesses = s.resources_max_subprocesses.clamp(1, 10_000);
    let resources_memory_monitoring = s.resources_memory_monitoring;
    let audit_enabled = s.audit_enabled;
    let audit_max_size_mb = s.audit_max_size_mb.clamp(1, 10_000);
    let audit_sign_events = s.audit_sign_events;
    let otp_enabled = s.otp_enabled;
    let otp_token_ttl_secs = s.otp_token_ttl_secs.clamp(1, 600);
    let otp_cache_valid_secs = s.otp_cache_valid_secs.clamp(1, 86_400);
    let otp_gated_actions = s.otp_gated_actions.clone();
    let otp_gated_domains = s.otp_gated_domains.clone();
    let otp_gated_domain_categories = s.otp_gated_domain_categories.clone();
    let estop_enabled = s.estop_enabled;
    let estop_require_otp_to_resume = s.estop_require_otp_to_resume;
    let syscall_anomaly_enabled = s.syscall_anomaly_enabled;
    let syscall_anomaly_strict_mode = s.syscall_anomaly_strict_mode;
    let syscall_anomaly_alert_on_unknown_syscall = s.syscall_anomaly_alert_on_unknown_syscall;
    let syscall_anomaly_max_denied_events_per_minute =
        s.syscall_anomaly_max_denied_events_per_minute.clamp(1, 10_000);
    let syscall_anomaly_max_total_events_per_minute =
        s.syscall_anomaly_max_total_events_per_minute.clamp(1, 100_000);
    let syscall_anomaly_max_alerts_per_minute =
        s.syscall_anomaly_max_alerts_per_minute.clamp(1, 10_000);
    let syscall_anomaly_alert_cooldown_secs = s.syscall_anomaly_alert_cooldown_secs.clamp(1, 3600);
    let syscall_anomaly_baseline_syscalls = s.syscall_anomaly_baseline_syscalls.clone();
    let canary_tokens = s.canary_tokens;
    let semantic_guard = s.semantic_guard;
    let semantic_guard_threshold = s.semantic_guard_threshold.clamp(0.0, 1.0);

    // 规范化和提取沙箱配置
    let sandbox_enabled = normalize_security_sandbox_enabled(&s.sandbox_enabled_input);
    let sandbox_backend =
        normalize_security_sandbox_backend_for_enabled(&s.sandbox_backend_input, &sandbox_enabled);
    let sandbox_firejail_args = s.sandbox_firejail_args.clone();

    // 提取并清理路径配置
    let audit_log_path = s.audit_log_path.trim().to_string();
    let otp_method = normalize_security_otp_method(&s.otp_method_input);
    let estop_state_file = s.estop_state_file.trim().to_string();
    let syscall_anomaly_log_path = s.syscall_anomaly_log_path.trim().to_string();
    let semantic_guard_collection = s.semantic_guard_collection.trim().to_string();

    // 配置 Bash 命令执行前安全检查
    security.bash = bash;

    // 配置沙箱启用状态
    // 将字符串转换为 Option<bool>，"auto" 对应 None
    security.sandbox.enabled = match sandbox_enabled.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    };

    // 配置沙箱后端类型
    security.sandbox.backend = match sandbox_backend.as_str() {
        "landlock" => vw_config_types::security::SandboxBackend::Landlock,
        "firejail" => vw_config_types::security::SandboxBackend::Firejail,
        "bubblewrap" => vw_config_types::security::SandboxBackend::Bubblewrap,
        "docker" => vw_config_types::security::SandboxBackend::Docker,
        "none" => vw_config_types::security::SandboxBackend::None,
        _ => vw_config_types::security::SandboxBackend::Auto,
    };

    // 配置 Firejail 自定义参数
    security.sandbox.firejail_args = sandbox_firejail_args;

    // 配置资源限制（带边界检查）
    // 内存限制：32 MB - 64 GB
    security.resources.max_memory_mb = resources_max_memory_mb;
    // CPU 时间限制：1 秒 - 24 小时
    security.resources.max_cpu_time_seconds = resources_max_cpu_time_seconds;
    // 子进程数限制：1 - 10000
    security.resources.max_subprocesses = resources_max_subprocesses;
    // 内存监控开关
    security.resources.memory_monitoring = resources_memory_monitoring;

    // 配置审计日志
    security.audit.enabled = audit_enabled;
    // 如果路径为空，使用默认值 "audit.log"
    security.audit.log_path =
        if audit_log_path.is_empty() { "audit.log".to_string() } else { audit_log_path };
    // 日志大小限制：1 MB - 10 GB
    security.audit.max_size_mb = audit_max_size_mb;
    // 审计事件签名开关
    security.audit.sign_events = audit_sign_events;

    // 配置 OTP 认证
    security.otp.enabled = otp_enabled;
    security.otp.method = match otp_method.as_str() {
        "pairing" => vw_config_types::security::OtpMethod::Pairing,
        "cli-prompt" => vw_config_types::security::OtpMethod::CliPrompt,
        _ => vw_config_types::security::OtpMethod::Totp,
    };
    // OTP 令牌有效期：1 秒 - 10 分钟
    security.otp.token_ttl_secs = otp_token_ttl_secs;
    // OTP 缓存有效期：1 秒 - 24 小时
    security.otp.cache_valid_secs = otp_cache_valid_secs;
    // 需要 OTP 保护的配置项
    security.otp.gated_actions = otp_gated_actions;
    security.otp.gated_domains = otp_gated_domains;
    security.otp.gated_domain_categories = otp_gated_domain_categories;

    // 配置紧急停止机制
    security.estop.enabled = estop_enabled;
    // 如果状态文件路径为空，使用默认值
    security.estop.state_file = if estop_state_file.is_empty() {
        vw_config_types::paths::estop_state_file_path()
    } else {
        estop_state_file
    };
    // 恢复时是否需要 OTP 验证
    security.estop.require_otp_to_resume = estop_require_otp_to_resume;

    // 配置系统调用异常检测
    security.syscall_anomaly.enabled = syscall_anomaly_enabled;
    security.syscall_anomaly.strict_mode = syscall_anomaly_strict_mode;
    security.syscall_anomaly.alert_on_unknown_syscall = syscall_anomaly_alert_on_unknown_syscall;

    // 异常检测限流配置（防止告警风暴）
    // 每分钟最大拒绝事件数：1 - 10000
    security.syscall_anomaly.max_denied_events_per_minute =
        syscall_anomaly_max_denied_events_per_minute;
    // 每分钟最大总事件数：1 - 100000
    security.syscall_anomaly.max_total_events_per_minute =
        syscall_anomaly_max_total_events_per_minute;
    // 每分钟最大告警数：1 - 10000
    security.syscall_anomaly.max_alerts_per_minute = syscall_anomaly_max_alerts_per_minute;
    // 告警冷却时间：1 秒 - 1 小时
    security.syscall_anomaly.alert_cooldown_secs = syscall_anomaly_alert_cooldown_secs;

    // 异常日志路径，如果为空使用默认值
    security.syscall_anomaly.log_path = if syscall_anomaly_log_path.is_empty() {
        "syscall-anomalies.log".to_string()
    } else {
        syscall_anomaly_log_path
    };
    // 基线系统调用列表（白名单）
    security.syscall_anomaly.baseline_syscalls = syscall_anomaly_baseline_syscalls;

    // 配置安全令牌和语义防护
    security.canary_tokens = canary_tokens;
    security.semantic_guard = semantic_guard;
    // 语义防护集合名称，如果为空使用默认值
    security.semantic_guard_collection = if semantic_guard_collection.is_empty() {
        "semantic_guard".to_string()
    } else {
        semantic_guard_collection
    };
    // 语义防护阈值：0.0 - 1.0
    security.semantic_guard_threshold = semantic_guard_threshold;
}

fn persist_security_settings(app: &mut App) -> Task<Message> {
    let security_settings = app.security_settings.clone();
    update_security_config_async(move |security| {
        apply_security_settings_to_config(security, &security_settings);
    })
}

fn save_security_settings(app: &mut App) -> Task<Message> {
    app.security_settings.save_error = None;
    persist_security_settings(app)
}

fn add_list_item(app: &mut App, kind: SecurityListKind) -> Task<Message> {
    let value = app.security_settings.list_editor.draft(kind).trim().to_string();
    if value.is_empty() {
        app.security_settings.save_error = Some("列表项不能为空".to_string());
        return Task::none();
    }
    if app.security_settings.list(kind).iter().any(|item| item == &value) {
        app.security_settings.save_error = Some(format!("列表项已存在：{value}"));
        return Task::none();
    }

    app.security_settings.list_mut(kind).push(value);
    app.security_settings.list_editor.set_draft(kind, String::new());
    save_security_settings(app)
}

fn remove_list_item(app: &mut App, kind: SecurityListKind, index: usize) -> Task<Message> {
    if !app.security_settings.list_editor.is_remove_pending(kind, index) {
        return Task::none();
    }
    app.security_settings.list_editor.cancel_remove();
    if index >= app.security_settings.list(kind).len() {
        app.security_settings.save_error = Some("要删除的列表项不存在".to_string());
        return Task::none();
    }
    app.security_settings.list_mut(kind).remove(index);
    save_security_settings(app)
}

/// 处理安全设置相关的消息
///
/// 该函数是安全设置 UI 交互的主要入口点，负责处理所有安全配置相关的用户操作。
/// 每次配置变更都会立即规范化输入并持久化到配置文件。
///
/// # 参数
///
/// - `app`：可变引用应用程序状态
/// - `message`：要处理的安全设置消息
///
/// # 返回值
///
/// 返回一个 `Task<Message>`，通常为 `Task::none()`，因为配置变更不需要额外的异步操作
///
/// # 消息类型
///
/// 该函数处理以下消息类别：
///
/// ## 沙箱配置
/// - `SecuritySandboxEnabledChanged`：沙箱启用状态变更
/// - `SecuritySandboxBackendChanged`：沙箱后端类型变更
/// - `SecurityListDraftChanged/Add/Remove*`：Firejail、OTP 与系统调用列表变更
///
/// ## 资源限制
/// - `SecurityResourcesMaxMemoryMbChanged`：内存限制变更
/// - `SecurityResourcesMaxCpuTimeSecondsChanged`：CPU 时间限制变更
/// - `SecurityResourcesMaxSubprocessesChanged`：子进程数限制变更
/// - `SecurityResourcesMemoryMonitoringToggled`：内存监控开关变更
///
/// ## 审计日志
/// - `SecurityAuditEnabledToggled`：审计启用开关变更
/// - `SecurityAuditLogPathChanged`：审计日志路径变更
/// - `SecurityAuditMaxSizeMbChanged`：审计日志大小限制变更
/// - `SecurityAuditSignEventsToggled`：事件签名开关变更
///
/// ## OTP 认证
/// - `SecurityOtpEnabledToggled`：OTP 启用开关变更
/// - `SecurityOtpMethodChanged`：OTP 方法变更
/// - `SecurityOtpTokenTtlSecsChanged`：令牌有效期变更
/// - `SecurityOtpCacheValidSecsChanged`：缓存有效期变更
///
/// ## 紧急停止
/// - `SecurityEstopEnabledToggled`：紧急停止启用开关变更
/// - `SecurityEstopStateFileChanged`：状态文件路径变更
/// - `SecurityEstopRequireOtpToResumeToggled`：恢复需要 OTP 开关变更
///
/// ## 系统调用异常检测
/// - `SecuritySyscallAnomalyEnabledToggled`：异常检测启用开关变更
/// - `SecuritySyscallAnomalyStrictModeToggled`：严格模式开关变更
/// - `SecuritySyscallAnomalyAlertOnUnknownSyscallToggled`：未知系统调用告警开关变更
/// - `SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged`：拒绝事件限流变更
/// - `SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged`：总事件限流变更
/// - `SecuritySyscallAnomalyMaxAlertsPerMinuteChanged`：告警限流变更
/// - `SecuritySyscallAnomalyAlertCooldownSecsChanged`：告警冷却时间变更
/// - `SecuritySyscallAnomalyLogPathChanged`：异常日志路径变更
///
/// ## 安全令牌和语义防护
/// - `SecurityCanaryTokensToggled`：Canary tokens 开关变更
/// - `SecuritySemanticGuardToggled`：语义防护开关变更
/// - `SecuritySemanticGuardCollectionChanged`：语义防护集合变更
/// - `SecuritySemanticGuardThresholdChanged`：语义防护阈值变更
///
/// ## 其他操作
/// - `SecuritySave`：手动保存所有安全设置
/// - `SecurityHelpOpen`：打开帮助模态框
/// - `SecurityHelpClose`：关闭帮助模态框
///
/// # 示例
///
/// ```ignore
/// // 处理沙箱启用状态变更
/// let task = update(&mut app, SettingsMessage::SecuritySandboxEnabledChanged("true".to_string()));
///
/// // 处理内存限制变更
/// let task = update(&mut app, SettingsMessage::SecurityResourcesMaxMemoryMbChanged(1024));
///
/// // 打开帮助模态框
/// let task = update(&mut app, SettingsMessage::SecurityHelpOpen);
/// ```
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        // Bash 命令执行前安全检查开关变更
        SettingsMessage::SecurityBashSecurityEnabledToggled(v) => {
            app.security_settings.bash.enabled = v;
            save_security_settings(app)
        }
        SettingsMessage::SecurityBashPolicyCheckToggled(check, v) => {
            use super::messages::BashPolicyCheck;
            let policy = &mut app.security_settings.bash.policy;
            match check {
                BashPolicyCheck::Enabled => policy.enabled = v,
                BashPolicyCheck::ReadOnly => policy.read_only = v,
                BashPolicyCheck::UnsafeShellPatterns => policy.unsafe_shell_patterns = v,
                BashPolicyCheck::CommandRules => policy.command_rules = v,
                BashPolicyCheck::WorkspacePaths => policy.workspace_paths = v,
                BashPolicyCheck::ForbiddenPaths => policy.forbidden_paths = v,
                BashPolicyCheck::Risk => policy.risk = v,
                BashPolicyCheck::Redirects => policy.redirects = v,
            }
            save_security_settings(app)
        }
        SettingsMessage::SecurityBashValidatorCheckToggled(check, v) => {
            use super::messages::BashValidatorCheck;
            let validators = &mut app.security_settings.bash.validators;
            match check {
                BashValidatorCheck::Enabled => validators.enabled = v,
                BashValidatorCheck::EmptyCommand => validators.empty_command = v,
                BashValidatorCheck::Heredoc => validators.heredoc = v,
                BashValidatorCheck::GitCommit => validators.git_commit = v,
                BashValidatorCheck::Jq => validators.jq = v,
                BashValidatorCheck::Metacharacters => validators.metacharacters = v,
                BashValidatorCheck::DangerousVars => validators.dangerous_vars = v,
                BashValidatorCheck::Substitution => validators.substitution = v,
                BashValidatorCheck::Redirection => validators.redirection = v,
                BashValidatorCheck::NewlineInjection => validators.newline_injection = v,
                BashValidatorCheck::IfsInjection => validators.ifs_injection = v,
                BashValidatorCheck::ProcEnviron => validators.proc_environ = v,
                BashValidatorCheck::Malformed => validators.malformed = v,
                BashValidatorCheck::ObfuscatedFlags => validators.obfuscated_flags = v,
                BashValidatorCheck::Backslash => validators.backslash = v,
                BashValidatorCheck::BraceExpansion => validators.brace_expansion = v,
                BashValidatorCheck::UnicodeWhitespace => validators.unicode_whitespace = v,
                BashValidatorCheck::HashComment => validators.hash_comment = v,
                BashValidatorCheck::CommentQuoteDesync => validators.comment_quote_desync = v,
                BashValidatorCheck::QuotedNewline => validators.quoted_newline = v,
                BashValidatorCheck::ZshDangerous => validators.zsh_dangerous = v,
                BashValidatorCheck::ControlChars => validators.control_chars = v,
            }
            save_security_settings(app)
        }

        SettingsMessage::SecurityTabSelected(tab) => {
            app.security_settings.active_tab = tab;
            Task::none()
        }

        SettingsMessage::SecurityListDraftChanged(kind, value) => {
            app.security_settings.list_editor.set_draft(kind, value);
            app.security_settings.save_error = None;
            Task::none()
        }

        SettingsMessage::SecurityListAdd(kind) => add_list_item(app, kind),

        SettingsMessage::SecurityListRemoveRequested(kind, index) => {
            if index < app.security_settings.list(kind).len() {
                app.security_settings.list_editor.request_remove(kind, index);
            }
            Task::none()
        }

        SettingsMessage::SecurityListRemoveConfirmed(kind, index) => {
            remove_list_item(app, kind, index)
        }

        SettingsMessage::SecurityListRemoveCanceled => {
            app.security_settings.list_editor.cancel_remove();
            Task::none()
        }

        // 沙箱启用状态变更
        SettingsMessage::SecuritySandboxEnabledChanged(v) => {
            app.security_settings.sandbox_enabled_input = normalize_security_sandbox_enabled(&v);
            app.security_settings.sandbox_backend_input =
                normalize_security_sandbox_backend_for_enabled(
                    &app.security_settings.sandbox_backend_input,
                    &app.security_settings.sandbox_enabled_input,
                );
            save_security_settings(app)
        }

        // 沙箱后端类型变更
        SettingsMessage::SecuritySandboxBackendChanged(v) => {
            app.security_settings.sandbox_backend_input =
                normalize_security_sandbox_backend_for_enabled(
                    &v,
                    &app.security_settings.sandbox_enabled_input,
                );
            save_security_settings(app)
        }

        // 内存限制变更（32 MB - 64 GB）
        SettingsMessage::SecurityResourcesMaxMemoryMbChanged(v) => {
            app.security_settings.resources_max_memory_mb = v.clamp(32, 65_536);
            save_security_settings(app)
        }

        // CPU 时间限制变更（1 秒 - 24 小时）
        SettingsMessage::SecurityResourcesMaxCpuTimeSecondsChanged(v) => {
            app.security_settings.resources_max_cpu_time_seconds = v.clamp(1, 86_400);
            save_security_settings(app)
        }

        // 子进程数限制变更（1 - 10000）
        SettingsMessage::SecurityResourcesMaxSubprocessesChanged(v) => {
            app.security_settings.resources_max_subprocesses = v.clamp(1, 10_000);
            save_security_settings(app)
        }

        // 内存监控开关变更
        SettingsMessage::SecurityResourcesMemoryMonitoringToggled(v) => {
            app.security_settings.resources_memory_monitoring = v;
            save_security_settings(app)
        }

        // 审计启用开关变更
        SettingsMessage::SecurityAuditEnabledToggled(v) => {
            app.security_settings.audit_enabled = v;
            save_security_settings(app)
        }

        // 审计日志路径变更
        SettingsMessage::SecurityAuditLogPathChanged(v) => {
            app.security_settings.audit_log_path = v;
            save_security_settings(app)
        }

        // 审计日志大小限制变更（1 MB - 10 GB）
        SettingsMessage::SecurityAuditMaxSizeMbChanged(v) => {
            app.security_settings.audit_max_size_mb = v.clamp(1, 10_000);
            save_security_settings(app)
        }

        // 审计事件签名开关变更
        SettingsMessage::SecurityAuditSignEventsToggled(v) => {
            app.security_settings.audit_sign_events = v;
            save_security_settings(app)
        }

        // OTP 启用开关变更
        SettingsMessage::SecurityOtpEnabledToggled(v) => {
            app.security_settings.otp_enabled = v;
            save_security_settings(app)
        }

        // OTP 认证方法变更
        SettingsMessage::SecurityOtpMethodChanged(v) => {
            app.security_settings.otp_method_input = normalize_security_otp_method(&v);
            save_security_settings(app)
        }

        // OTP 令牌有效期变更（1 秒 - 10 分钟）
        SettingsMessage::SecurityOtpTokenTtlSecsChanged(v) => {
            app.security_settings.otp_token_ttl_secs = v.clamp(1, 600);
            save_security_settings(app)
        }

        // OTP 缓存有效期变更（1 秒 - 24 小时）
        SettingsMessage::SecurityOtpCacheValidSecsChanged(v) => {
            app.security_settings.otp_cache_valid_secs = v.clamp(1, 86_400);
            save_security_settings(app)
        }

        // 紧急停止启用开关变更
        SettingsMessage::SecurityEstopEnabledToggled(v) => {
            app.security_settings.estop_enabled = v;
            save_security_settings(app)
        }

        // 紧急停止状态文件路径变更
        SettingsMessage::SecurityEstopStateFileChanged(v) => {
            app.security_settings.estop_state_file = v;
            save_security_settings(app)
        }

        // 恢复时需要 OTP 验证开关变更
        SettingsMessage::SecurityEstopRequireOtpToResumeToggled(v) => {
            app.security_settings.estop_require_otp_to_resume = v;
            save_security_settings(app)
        }

        // 系统调用异常检测启用开关变更
        SettingsMessage::SecuritySyscallAnomalyEnabledToggled(v) => {
            app.security_settings.syscall_anomaly_enabled = v;
            save_security_settings(app)
        }

        // 系统调用异常检测严格模式开关变更
        SettingsMessage::SecuritySyscallAnomalyStrictModeToggled(v) => {
            app.security_settings.syscall_anomaly_strict_mode = v;
            save_security_settings(app)
        }

        // 未知系统调用告警开关变更
        SettingsMessage::SecuritySyscallAnomalyAlertOnUnknownSyscallToggled(v) => {
            app.security_settings.syscall_anomaly_alert_on_unknown_syscall = v;
            save_security_settings(app)
        }

        // 每分钟最大拒绝事件数变更（1 - 10000）
        SettingsMessage::SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged(v) => {
            app.security_settings.syscall_anomaly_max_denied_events_per_minute = v.clamp(1, 10_000);
            save_security_settings(app)
        }

        // 每分钟最大总事件数变更（1 - 100000）
        SettingsMessage::SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged(v) => {
            app.security_settings.syscall_anomaly_max_total_events_per_minute = v.clamp(1, 100_000);
            save_security_settings(app)
        }

        // 每分钟最大告警数变更（1 - 10000）
        SettingsMessage::SecuritySyscallAnomalyMaxAlertsPerMinuteChanged(v) => {
            app.security_settings.syscall_anomaly_max_alerts_per_minute = v.clamp(1, 10_000);
            save_security_settings(app)
        }

        // 告警冷却时间变更（1 秒 - 1 小时）
        SettingsMessage::SecuritySyscallAnomalyAlertCooldownSecsChanged(v) => {
            app.security_settings.syscall_anomaly_alert_cooldown_secs = v.clamp(1, 3600);
            save_security_settings(app)
        }

        // 系统调用异常日志路径变更
        SettingsMessage::SecuritySyscallAnomalyLogPathChanged(v) => {
            app.security_settings.syscall_anomaly_log_path = v;
            save_security_settings(app)
        }

        // Canary tokens 开关变更
        SettingsMessage::SecurityCanaryTokensToggled(v) => {
            app.security_settings.canary_tokens = v;
            save_security_settings(app)
        }

        // 语义防护开关变更
        SettingsMessage::SecuritySemanticGuardToggled(v) => {
            app.security_settings.semantic_guard = v;
            save_security_settings(app)
        }

        // 语义防护集合名称变更
        SettingsMessage::SecuritySemanticGuardCollectionChanged(v) => {
            app.security_settings.semantic_guard_collection = v;
            save_security_settings(app)
        }

        // 语义防护阈值变更（0.0 - 1.0）
        SettingsMessage::SecuritySemanticGuardThresholdChanged(v) => {
            app.security_settings.semantic_guard_threshold = v.clamp(0.0, 1.0);
            save_security_settings(app)
        }

        // 手动保存所有安全设置
        // 规范化所有输入并持久化配置
        SettingsMessage::SecuritySave => {
            app.security_settings.sandbox_enabled_input =
                normalize_security_sandbox_enabled(&app.security_settings.sandbox_enabled_input);
            app.security_settings.sandbox_backend_input =
                normalize_security_sandbox_backend_for_enabled(
                    &app.security_settings.sandbox_backend_input,
                    &app.security_settings.sandbox_enabled_input,
                );
            app.security_settings.otp_method_input =
                normalize_security_otp_method(&app.security_settings.otp_method_input);
            save_security_settings(app)
        }

        // 打开帮助模态框
        SettingsMessage::SecurityHelpOpen => {
            app.security_settings.show_help_modal = true;
            Task::none()
        }

        // 关闭帮助模态框
        SettingsMessage::SecurityHelpClose => {
            app.security_settings.show_help_modal = false;
            Task::none()
        }

        // 忽略其他消息
        _ => Task::none(),
    }
}
#[cfg(test)]
#[path = "security_tests.rs"]
mod security_tests;
