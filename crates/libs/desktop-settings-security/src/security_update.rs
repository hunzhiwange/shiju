//! 安全设置的局部更新与配置映射。
use crate::security_state::{SecurityListKind, SecuritySettingsState};
use crate::security_message::Message;
pub use crate::security_config::apply_security_settings_to_config;

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
pub(crate) fn normalize_security_sandbox_enabled(raw: &str) -> String {
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
pub(crate) fn normalize_security_sandbox_backend_for_enabled(raw: &str, sandbox_enabled: &str) -> String {
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
pub(crate) fn normalize_security_otp_method(raw: &str) -> String {
    let v = raw.trim().to_ascii_lowercase();
    match v.as_str() {
        "totp" | "pairing" | "cli-prompt" => v,
        _ => "totp".to_string(),
    }
}

fn save_security_settings(state: &mut SecuritySettingsState) -> bool {
    state.save_error = None;
    true
}

fn add_list_item(state: &mut SecuritySettingsState, kind: SecurityListKind) -> bool {
    let value = state.list_editor.draft(kind).trim().to_string();
    if value.is_empty() {
        state.save_error = Some("列表项不能为空".to_string());
        return false;
    }
    if state.list(kind).iter().any(|item| item == &value) {
        state.save_error = Some(format!("列表项已存在：{value}"));
        return false;
    }

    state.list_mut(kind).push(value);
    state.list_editor.set_draft(kind, String::new());
    save_security_settings(state)
}

fn remove_list_item(state: &mut SecuritySettingsState, kind: SecurityListKind, index: usize) -> bool {
    if !state.list_editor.is_remove_pending(kind, index) {
        return false;
    }
    state.list_editor.cancel_remove();
    if index >= state.list(kind).len() {
        state.save_error = Some("要删除的列表项不存在".to_string());
        return false;
    }
    state.list_mut(kind).remove(index);
    save_security_settings(state)
}

/// 更新页面状态，返回是否需要保存配置。
pub fn update(state: &mut SecuritySettingsState, message: Message) -> bool {
    match message {
        // Bash 命令执行前安全检查开关变更
        Message::SecurityBashSecurityEnabledToggled(v) => {
            state.bash.enabled = v;
            save_security_settings(state)
        }
        Message::SecurityBashPolicyCheckToggled(check, v) => {
            use crate::security_message::BashPolicyCheck;
            let policy = &mut state.bash.policy;
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
            save_security_settings(state)
        }
        Message::SecurityBashValidatorCheckToggled(check, v) => {
            use crate::security_message::BashValidatorCheck;
            let validators = &mut state.bash.validators;
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
            save_security_settings(state)
        }

        Message::SecurityTabSelected(tab) => {
            state.active_tab = tab;
            false
        }

        Message::SecurityListDraftChanged(kind, value) => {
            state.list_editor.set_draft(kind, value);
            state.save_error = None;
            false
        }

        Message::SecurityListAdd(kind) => add_list_item(state, kind),

        Message::SecurityListRemoveRequested(kind, index) => {
            if index < state.list(kind).len() {
                state.list_editor.request_remove(kind, index);
            }
            false
        }

        Message::SecurityListRemoveConfirmed(kind, index) => {
            remove_list_item(state, kind, index)
        }

        Message::SecurityListRemoveCanceled => {
            state.list_editor.cancel_remove();
            false
        }

        // 沙箱启用状态变更
        Message::SecuritySandboxEnabledChanged(v) => {
            state.sandbox_enabled_input = normalize_security_sandbox_enabled(&v);
            state.sandbox_backend_input =
                normalize_security_sandbox_backend_for_enabled(
                    &state.sandbox_backend_input,
                    &state.sandbox_enabled_input,
                );
            save_security_settings(state)
        }

        // 沙箱后端类型变更
        Message::SecuritySandboxBackendChanged(v) => {
            state.sandbox_backend_input =
                normalize_security_sandbox_backend_for_enabled(
                    &v,
                    &state.sandbox_enabled_input,
                );
            save_security_settings(state)
        }

        // 内存限制变更（32 MB - 64 GB）
        Message::SecurityResourcesMaxMemoryMbChanged(v) => {
            state.resources_max_memory_mb = v.clamp(32, 65_536);
            save_security_settings(state)
        }

        // CPU 时间限制变更（1 秒 - 24 小时）
        Message::SecurityResourcesMaxCpuTimeSecondsChanged(v) => {
            state.resources_max_cpu_time_seconds = v.clamp(1, 86_400);
            save_security_settings(state)
        }

        // 子进程数限制变更（1 - 10000）
        Message::SecurityResourcesMaxSubprocessesChanged(v) => {
            state.resources_max_subprocesses = v.clamp(1, 10_000);
            save_security_settings(state)
        }

        // 内存监控开关变更
        Message::SecurityResourcesMemoryMonitoringToggled(v) => {
            state.resources_memory_monitoring = v;
            save_security_settings(state)
        }

        // 审计启用开关变更
        Message::SecurityAuditEnabledToggled(v) => {
            state.audit_enabled = v;
            save_security_settings(state)
        }

        // 审计日志路径变更
        Message::SecurityAuditLogPathChanged(v) => {
            state.audit_log_path = v;
            save_security_settings(state)
        }

        // 审计日志大小限制变更（1 MB - 10 GB）
        Message::SecurityAuditMaxSizeMbChanged(v) => {
            state.audit_max_size_mb = v.clamp(1, 10_000);
            save_security_settings(state)
        }

        // 审计事件签名开关变更
        Message::SecurityAuditSignEventsToggled(v) => {
            state.audit_sign_events = v;
            save_security_settings(state)
        }

        // OTP 启用开关变更
        Message::SecurityOtpEnabledToggled(v) => {
            state.otp_enabled = v;
            save_security_settings(state)
        }

        // OTP 认证方法变更
        Message::SecurityOtpMethodChanged(v) => {
            state.otp_method_input = normalize_security_otp_method(&v);
            save_security_settings(state)
        }

        // OTP 令牌有效期变更（1 秒 - 10 分钟）
        Message::SecurityOtpTokenTtlSecsChanged(v) => {
            state.otp_token_ttl_secs = v.clamp(1, 600);
            save_security_settings(state)
        }

        // OTP 缓存有效期变更（1 秒 - 24 小时）
        Message::SecurityOtpCacheValidSecsChanged(v) => {
            state.otp_cache_valid_secs = v.clamp(1, 86_400);
            save_security_settings(state)
        }

        // 紧急停止启用开关变更
        Message::SecurityEstopEnabledToggled(v) => {
            state.estop_enabled = v;
            save_security_settings(state)
        }

        // 紧急停止状态文件路径变更
        Message::SecurityEstopStateFileChanged(v) => {
            state.estop_state_file = v;
            save_security_settings(state)
        }

        // 恢复时需要 OTP 验证开关变更
        Message::SecurityEstopRequireOtpToResumeToggled(v) => {
            state.estop_require_otp_to_resume = v;
            save_security_settings(state)
        }

        // 系统调用异常检测启用开关变更
        Message::SecuritySyscallAnomalyEnabledToggled(v) => {
            state.syscall_anomaly_enabled = v;
            save_security_settings(state)
        }

        // 系统调用异常检测严格模式开关变更
        Message::SecuritySyscallAnomalyStrictModeToggled(v) => {
            state.syscall_anomaly_strict_mode = v;
            save_security_settings(state)
        }

        // 未知系统调用告警开关变更
        Message::SecuritySyscallAnomalyAlertOnUnknownSyscallToggled(v) => {
            state.syscall_anomaly_alert_on_unknown_syscall = v;
            save_security_settings(state)
        }

        // 每分钟最大拒绝事件数变更（1 - 10000）
        Message::SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged(v) => {
            state.syscall_anomaly_max_denied_events_per_minute = v.clamp(1, 10_000);
            save_security_settings(state)
        }

        // 每分钟最大总事件数变更（1 - 100000）
        Message::SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged(v) => {
            state.syscall_anomaly_max_total_events_per_minute = v.clamp(1, 100_000);
            save_security_settings(state)
        }

        // 每分钟最大告警数变更（1 - 10000）
        Message::SecuritySyscallAnomalyMaxAlertsPerMinuteChanged(v) => {
            state.syscall_anomaly_max_alerts_per_minute = v.clamp(1, 10_000);
            save_security_settings(state)
        }

        // 告警冷却时间变更（1 秒 - 1 小时）
        Message::SecuritySyscallAnomalyAlertCooldownSecsChanged(v) => {
            state.syscall_anomaly_alert_cooldown_secs = v.clamp(1, 3600);
            save_security_settings(state)
        }

        // 系统调用异常日志路径变更
        Message::SecuritySyscallAnomalyLogPathChanged(v) => {
            state.syscall_anomaly_log_path = v;
            save_security_settings(state)
        }

        // Canary tokens 开关变更
        Message::SecurityCanaryTokensToggled(v) => {
            state.canary_tokens = v;
            save_security_settings(state)
        }

        // 语义防护开关变更
        Message::SecuritySemanticGuardToggled(v) => {
            state.semantic_guard = v;
            save_security_settings(state)
        }

        // 语义防护集合名称变更
        Message::SecuritySemanticGuardCollectionChanged(v) => {
            state.semantic_guard_collection = v;
            save_security_settings(state)
        }

        // 语义防护阈值变更（0.0 - 1.0）
        Message::SecuritySemanticGuardThresholdChanged(v) => {
            state.semantic_guard_threshold = v.clamp(0.0, 1.0);
            save_security_settings(state)
        }

        // 手动保存所有安全设置
        // 规范化所有输入并持久化配置
        Message::SecuritySave => {
            state.sandbox_enabled_input =
                normalize_security_sandbox_enabled(&state.sandbox_enabled_input);
            state.sandbox_backend_input =
                normalize_security_sandbox_backend_for_enabled(
                    &state.sandbox_backend_input,
                    &state.sandbox_enabled_input,
                );
            state.otp_method_input =
                normalize_security_otp_method(&state.otp_method_input);
            save_security_settings(state)
        }

        // 打开帮助模态框
        Message::SecurityHelpOpen => {
            state.show_help_modal = true;
            false
        }

        // 关闭帮助模态框
        Message::SecurityHelpClose => {
            state.show_help_modal = false;
            false
        }

        Message::CopyCode(_) | Message::OpenUrlExternal(_) | Message::AppendText(_) | Message::None => false,
    }
}
#[cfg(test)]
#[path = "security_update_tests.rs"]
mod security_tests;
