//! 安全页面输入到持久配置的映射。
use crate::security_state::SecuritySettingsState;
use config_security::SecurityConfig;
use super::security_update::{normalize_security_sandbox_enabled, normalize_security_sandbox_backend_for_enabled, normalize_security_otp_method};

/// 将安全页面输入规范化并应用到配置；不执行持久化。
pub fn apply_security_settings_to_config(security: &mut SecurityConfig, s: &SecuritySettingsState) {
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
        "landlock" => config_security::SandboxBackend::Landlock,
        "firejail" => config_security::SandboxBackend::Firejail,
        "bubblewrap" => config_security::SandboxBackend::Bubblewrap,
        "docker" => config_security::SandboxBackend::Docker,
        "none" => config_security::SandboxBackend::None,
        _ => config_security::SandboxBackend::Auto,
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
        "pairing" => config_security::OtpMethod::Pairing,
        "cli-prompt" => config_security::OtpMethod::CliPrompt,
        _ => config_security::OtpMethod::Totp,
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
        config_runtime_paths::estop_state_file_path()
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
