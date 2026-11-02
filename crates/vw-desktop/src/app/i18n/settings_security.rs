use vw_config_types::ui::DesktopLocale;
/// Translate a Chinese source key in the caller's locale.
pub fn text(locale: DesktopLocale, source: &'static str) -> &'static str {
    super::I18n::new(locale).__(source)
}

pub fn help(locale: DesktopLocale) -> String {
    let content = super::I18n::new(locale).__("安全配置说明\n\n一、作用\n- security 用于配置沙箱、资源限制、审计、OTP 门控、紧急停止与系统调用异常检测。\n\n二、关键字段\n- bash.enabled: 总开关，默认 false；完全访问始终绕过。\n- bash.policy.enabled / bash.validators.enabled: 分组开关。\n- bash.policy.* / bash.validators.*: 单项开关；有效状态为总开关 && 分组开关 && 单项开关。\n- sandbox.enabled: auto/true/false。\n- sandbox.backend: auto | landlock | firejail | bubblewrap | docker | none。\n- resources.*: 进程资源上限。\n- audit.*: 审计日志行为。\n- otp.*: 敏感动作二次验证。\n- estop.*: 紧急停止状态机。\n- syscall_anomaly.*: 系统调用异常检测。\n- canary_tokens / semantic_guard*: 注入与外泄防护。\n\n三、示例\n{\n  \"security\": {\n    \"bash\": {\n      \"enabled\": true,\n      \"policy\": {\"enabled\": true, \"read_only\": true, \"redirects\": true},\n      \"validators\": {\"enabled\": true, \"heredoc\": true, \"quoted_newline\": false}\n    },\n    \"sandbox\": {\"enabled\": null, \"backend\": \"auto\", \"firejail_args\": []},\n    \"resources\": {\n      \"max_memory_mb\": 512,\n      \"max_cpu_time_seconds\": 60,\n      \"max_subprocesses\": 10,\n      \"memory_monitoring\": true\n    },\n    \"audit\": {\"enabled\": true, \"log_path\": \"audit.log\", \"max_size_mb\": 100, \"sign_events\": false}\n  }\n}\n");
    if locale == DesktopLocale::SimplifiedChinese {
        content.to_owned()
    } else {
        format!(
            "{content}\n\n{}",
            "{\n  \"security\": {\n    \"bash\": {\n      \"enabled\": true,\n      \"policy\": {\"enabled\": true, \"read_only\": true, \"redirects\": true},\n      \"validators\": {\"enabled\": true, \"heredoc\": true, \"quoted_newline\": false}\n    },\n    \"sandbox\": {\"enabled\": null, \"backend\": \"auto\", \"firejail_args\": []},\n    \"resources\": {\n      \"max_memory_mb\": 512,\n      \"max_cpu_time_seconds\": 60,\n      \"max_subprocesses\": 10,\n      \"memory_monitoring\": true\n    },\n    \"audit\": {\"enabled\": true, \"log_path\": \"audit.log\", \"max_size_mb\": 100, \"sign_events\": false}\n  }\n}"
        )
    }
}
