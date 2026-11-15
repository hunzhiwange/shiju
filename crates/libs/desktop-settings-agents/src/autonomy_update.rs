//! 自主性设置局部更新；返回值表示是否需要持久化。
use desktop_settings_security::app_init::state::{AutonomyListKind, AutonomySettingsState};
fn parse_comma_or_newline_list(input: &str) -> Vec<String> { input.split([',', '\n']).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string).collect() }
/// 解析非 CLI 模式下按通道的自然语言审批模式配置
///
/// 该函数将用户输入的字符串解析为通道到审批模式的映射表。
/// 输入格式为：`channel1:mode1, channel2:mode2`，支持逗号或换行符分隔。
///
/// # 参数
///
/// * `input` - 包含通道和模式配置的字符串，格式为 `channel:mode`
///
/// # 返回值
///
/// 返回一个 HashMap，键为通道名称（小写），值为对应的审批模式。
/// 无效的条目会被跳过。
///
/// # 支持的模式
///
/// - `direct`：直接执行模式，无需确认
/// - `request_confirm` / `request-confirm` / `requestconfirm`：请求确认模式
/// - `disabled`：禁用模式
///
/// # 示例
///
/// ```ignore
/// let input = "telegram:direct, slack:request_confirm, discord:disabled";
/// let modes = parse_non_cli_mode_by_channel(input);
/// // 结果：{"telegram" => Direct, "slack" => RequestConfirm, "discord" => Disabled}
/// ```
fn parse_non_cli_mode_by_channel(
    input: &str,
) -> std::collections::HashMap<String, config_security::NonCliNaturalLanguageApprovalMode>
{
    let mut out = std::collections::HashMap::new();

    // 遍历所有条目（支持逗号或换行符分隔）
    for entry in parse_comma_or_newline_list(input) {
        // 尝试分割通道名称和模式
        let Some((channel, mode_raw)) = entry.split_once(':') else {
            continue;
        };

        // 规范化通道名称：去除首尾空格并转为小写
        let channel = channel.trim().to_ascii_lowercase();
        if channel.is_empty() {
            continue;
        }

        // 解析模式（支持多种格式）
        let mode = match mode_raw.trim().to_ascii_lowercase().as_str() {
            // 直接执行模式
            "direct" => config_security::NonCliNaturalLanguageApprovalMode::Direct,
            // 请求确认模式（支持多种格式变体）
            "request_confirm" | "request-confirm" | "requestconfirm" => {
                config_security::NonCliNaturalLanguageApprovalMode::RequestConfirm
            }
            // 禁用模式
            "disabled" => config_security::NonCliNaturalLanguageApprovalMode::Disabled,
            // 无效模式，跳过该条目
            _ => continue,
        };

        out.insert(channel, mode);
    }
    out
}

#[cfg(test)]
#[path = "autonomy_tests.rs"]
mod autonomy_tests;

pub fn apply_autonomy_settings_to_config(
    settings: &AutonomySettingsState,
    autonomy: &mut config_security::AutonomyConfig,
) {
    autonomy.level = settings.level;
    autonomy.workspace_only = settings.workspace_only;
    autonomy.allowed_commands = settings.allowed_commands.clone();
    autonomy.forbidden_paths = settings.forbidden_paths.clone();
    autonomy.max_actions_per_hour = settings.max_actions_per_hour.clamp(1, 1_000_000);
    autonomy.max_cost_per_day_cents = settings.max_cost_per_day_cents.clamp(1, 100_000_000);
    autonomy.require_approval_for_medium_risk = settings.require_approval_for_medium_risk;
    autonomy.block_high_risk_commands = settings.block_high_risk_commands;
    autonomy.shell_redirect_policy = settings.shell_redirect_policy;
    autonomy.shell_env_passthrough = settings.shell_env_passthrough.clone();
    autonomy.auto_approve = settings.auto_approve.clone();
    autonomy.always_ask = settings.always_ask.clone();
    autonomy.allowed_roots = settings.allowed_roots.clone();
    autonomy.non_cli_excluded_tools = settings.non_cli_excluded_tools.clone();
    autonomy.non_cli_approval_approvers =
        parse_comma_or_newline_list(&settings.non_cli_approval_approvers_input);
    autonomy.non_cli_natural_language_approval_mode =
        settings.non_cli_natural_language_approval_mode;
    autonomy.non_cli_natural_language_approval_mode_by_channel = parse_non_cli_mode_by_channel(
        &settings.non_cli_natural_language_approval_mode_by_channel_input,
    );
}

#[cfg(test)]
fn config_list(
    config: &config_security::AutonomyConfig,
    kind: AutonomyListKind,
) -> &[String] {
    match kind {
        AutonomyListKind::AllowedCommands => &config.allowed_commands,
        AutonomyListKind::ForbiddenPaths => &config.forbidden_paths,
        AutonomyListKind::ShellEnvPassthrough => &config.shell_env_passthrough,
        AutonomyListKind::AutoApprove => &config.auto_approve,
        AutonomyListKind::AlwaysAsk => &config.always_ask,
        AutonomyListKind::AllowedRoots => &config.allowed_roots,
        AutonomyListKind::NonCliExcludedTools => &config.non_cli_excluded_tools,
    }
}

fn is_valid_environment_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(b'A'..=b'Z' | b'a'..=b'z' | b'_'))
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn add_list_item(settings: &mut AutonomySettingsState, kind: AutonomyListKind) -> bool {
    let value = settings.list_editor.draft(kind).trim().to_string();
    let error = if value.is_empty() {
        Some("列表项不能为空".to_string())
    } else if kind == AutonomyListKind::ShellEnvPassthrough && !is_valid_environment_name(&value) {
        Some("环境变量名必须匹配 [A-Za-z_][A-Za-z0-9_]*".to_string())
    } else if settings.list(kind).iter().any(|item| item == &value) {
        Some("该列表项已存在".to_string())
    } else {
        None
    };

    if let Some(error) = error {
        settings.save_error_translation =
            Some(desktop_i18n::format::Message::new(error.clone(), vec![]));
        settings.save_error = Some(error);
        return false;
    }

    settings.list_mut(kind).push(value);
    settings.list_editor.set_draft(kind, String::new());
    settings.save_error = None;
    settings.save_error_translation = None;
    true
}

fn remove_list_item(settings: &mut AutonomySettingsState, kind: AutonomyListKind, index: usize) -> bool {
    if !settings.list_editor.is_remove_pending(kind, index) {
        return false;
    }
    settings.list_editor.cancel_remove();
    if index >= settings.list(kind).len() {
        settings.save_error_translation =
            Some(desktop_i18n::format::Message::new("要删除的列表项不存在", vec![]));
        settings.save_error = Some("要删除的列表项不存在".to_string());
        return false;
    }

    settings.list_mut(kind).remove(index);
    settings.save_error = None;
    settings.save_error_translation = None;
    true
}

/// 更新本域状态；返回 true 时由壳按既有配置入口持久化。
pub fn update(settings: &mut AutonomySettingsState, message: crate::autonomy_message::Message) -> bool {
    match message {
        crate::autonomy_message::Message::AutonomyTabSelected(tab) => {
            settings.active_tab = tab;
            false
        }

        // 自主性级别变更
        crate::autonomy_message::Message::AutonomyLevelChanged(v) => {
            settings.level = v;
            settings.save_error = None;
            settings.save_error_translation = None;
            true
        }

        // 工作空间限制开关
        crate::autonomy_message::Message::AutonomyWorkspaceOnlyToggled(v) => {
            settings.workspace_only = v;
            settings.save_error = None;
            settings.save_error_translation = None;
            true
        }

        crate::autonomy_message::Message::AutonomyListDraftChanged(kind, value) => {
            settings.list_editor.set_draft(kind, value);
            settings.save_error = None;
            settings.save_error_translation = None;
            false
        }

        crate::autonomy_message::Message::AutonomyListAdd(kind) => add_list_item(settings, kind),

        crate::autonomy_message::Message::AutonomyListRemoveRequested(kind, index) => {
            if index < settings.list(kind).len() {
                settings.list_editor.request_remove(kind, index);
            }
            false
        }

        crate::autonomy_message::Message::AutonomyListRemoveConfirmed(kind, index) => {
            remove_list_item(settings, kind, index)
        }

        crate::autonomy_message::Message::AutonomyListRemoveCanceled => {
            settings.list_editor.cancel_remove();
            false
        }

        // 每小时最大操作数变更
        crate::autonomy_message::Message::AutonomyMaxActionsPerHourChanged(v) => {
            settings.max_actions_per_hour = v.clamp(1, 1_000_000);
            settings.save_error = None;
            settings.save_error_translation = None;
            true
        }

        // 每日最大成本变更
        crate::autonomy_message::Message::AutonomyMaxCostPerDayCentsChanged(v) => {
            settings.max_cost_per_day_cents = v.clamp(1, 100_000_000);
            settings.save_error = None;
            settings.save_error_translation = None;
            true
        }

        // 中等风险审批开关
        crate::autonomy_message::Message::AutonomyRequireApprovalForMediumRiskToggled(v) => {
            settings.require_approval_for_medium_risk = v;
            settings.save_error = None;
            settings.save_error_translation = None;
            true
        }

        // 高风险命令阻止开关
        crate::autonomy_message::Message::AutonomyBlockHighRiskCommandsToggled(v) => {
            settings.block_high_risk_commands = v;
            settings.save_error = None;
            settings.save_error_translation = None;
            true
        }

        // Shell 重定向策略变更
        crate::autonomy_message::Message::AutonomyShellRedirectPolicyChanged(v) => {
            settings.shell_redirect_policy = v;
            settings.save_error = None;
            settings.save_error_translation = None;
            true
        }

        // 非 CLI 模式审批者列表变更
        crate::autonomy_message::Message::AutonomyNonCliApprovalApproversChanged(v) => {
            settings.non_cli_approval_approvers_input = v;
            settings.save_error = None;
            settings.save_error_translation = None;
            true
        }

        // 非 CLI 模式默认审批模式变更
        crate::autonomy_message::Message::AutonomyNonCliNaturalLanguageApprovalModeChanged(v) => {
            settings.non_cli_natural_language_approval_mode = v;
            settings.save_error = None;
            settings.save_error_translation = None;
            true
        }

        // 按 CLI 模式通道的审批模式变更
        crate::autonomy_message::Message::AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(v) => {
            settings.non_cli_natural_language_approval_mode_by_channel_input = v;
            settings.save_error = None;
            settings.save_error_translation = None;
            true
        }

        // 手动保存设置
        crate::autonomy_message::Message::AutonomySave => {
            settings.save_error = None;
            settings.save_error_translation = None;
            true
        }

        // 打开帮助模态框
        crate::autonomy_message::Message::AutonomyHelpOpen => {
            settings.show_help_modal = true;
            false
        }

        // 关闭帮助模态框
        crate::autonomy_message::Message::AutonomyHelpClose => {
            settings.show_help_modal = false;
            false
        }

        // 其他非自主性设置相关的消息，不做处理
        _ => false,
    }
}
