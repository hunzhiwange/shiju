//! 自主性设置消息处理模块
//!
//! 该模块负责处理所有与代理自主性相关的设置变更消息，包括：
//! - 自主性级别控制
//! - 工作空间限制
//! - 命令和路径的访问控制
//! - 风险管理和审批策略
//! - Shell 环境和重定向策略
//! - 非 CLI 模式的自然语言审批配置
//!
//! 所有设置变更都会立即持久化到配置文件中，确保代理行为的一致性。

use crate::app::state::{AutonomyListKind, AutonomySettingsState};
use crate::app::{App, Message};
use iced::Task;

use super::messages::SettingsMessage;
use super::util::parse_comma_or_newline_list;

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
) -> std::collections::HashMap<String, vw_config_types::security::NonCliNaturalLanguageApprovalMode>
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
            "direct" => vw_config_types::security::NonCliNaturalLanguageApprovalMode::Direct,
            // 请求确认模式（支持多种格式变体）
            "request_confirm" | "request-confirm" | "requestconfirm" => {
                vw_config_types::security::NonCliNaturalLanguageApprovalMode::RequestConfirm
            }
            // 禁用模式
            "disabled" => vw_config_types::security::NonCliNaturalLanguageApprovalMode::Disabled,
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

/// 持久化自主性设置到配置文件
///
/// 该函数将应用中的自主性设置状态同步到持久化配置中。
/// 它会将 UI 输入字段中的字符串值解析为结构化数据，并确保数值在合理范围内。
///
/// # 参数
///
/// * `app` - 可变引用的应用实例，包含当前的自主性设置状态
///
/// # 处理的设置项
///
/// - `level`：自主性级别
/// - `workspace_only`：是否仅在工作空间内操作
/// - `allowed_commands`：允许执行的命令列表
/// - `forbidden_paths`：禁止访问的路径列表
/// - `max_actions_per_hour`：每小时最大操作数（限制在 1-10,000 之间）
/// - `max_cost_per_day_cents`：每日最大成本（美分，限制在 1-1,000,000 之间）
/// - `require_approval_for_medium_risk`：中等风险操作是否需要审批
/// - `block_high_risk_commands`：是否阻止高风险命令
/// - `shell_redirect_policy`：Shell 重定向策略
/// - `shell_env_passthrough`：允许传递的环境变量列表
/// - `auto_approve`：自动批准的工具/操作列表
/// - `always_ask`：总是询问的工具/操作列表
/// - `allowed_roots`：允许访问的根目录列表
/// - `non_cli_excluded_tools`：非 CLI 模式下排除的工具列表
/// - `non_cli_approval_approvers`：非 CLI 模式审批者列表
/// - `non_cli_natural_language_approval_mode`：非 CLI 模式默认审批模式
/// - `non_cli_natural_language_approval_mode_by_channel`：按通道的自定义审批模式
fn persist_autonomy_settings(app: &mut App) -> Task<Message> {
    let settings = app.autonomy_settings.clone();
    crate::app::update_autonomy_config_async(move |autonomy| {
        apply_autonomy_settings_to_config(&settings, autonomy);
    })
}

fn apply_autonomy_settings_to_config(
    settings: &AutonomySettingsState,
    autonomy: &mut vw_config_types::security::AutonomyConfig,
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
    config: &vw_config_types::security::AutonomyConfig,
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

fn add_list_item(app: &mut App, kind: AutonomyListKind) -> Task<Message> {
    let value = app.autonomy_settings.list_editor.draft(kind).trim().to_string();
    let error = if value.is_empty() {
        Some("列表项不能为空".to_string())
    } else if kind == AutonomyListKind::ShellEnvPassthrough
        && !is_valid_environment_name(&value)
    {
        Some("环境变量名必须匹配 [A-Za-z_][A-Za-z0-9_]*".to_string())
    } else if app.autonomy_settings.list(kind).iter().any(|item| item == &value) {
        Some("该列表项已存在".to_string())
    } else {
        None
    };

    if let Some(error) = error {
        app.autonomy_settings.save_error = Some(error);
        return Task::none();
    }

    app.autonomy_settings.list_mut(kind).push(value);
    app.autonomy_settings.list_editor.set_draft(kind, String::new());
    app.autonomy_settings.save_error = None;
    persist_autonomy_settings(app)
}

fn remove_list_item(app: &mut App, kind: AutonomyListKind, index: usize) -> Task<Message> {
    if index >= app.autonomy_settings.list(kind).len() {
        app.autonomy_settings.save_error = Some("要删除的列表项不存在".to_string());
        return Task::none();
    }

    app.autonomy_settings.list_mut(kind).remove(index);
    app.autonomy_settings.save_error = None;
    persist_autonomy_settings(app)
}

/// 处理自主性设置相关的消息更新
///
/// 该函数是自主性设置模块的主要消息处理器，负责响应各种 UI 交互事件，
/// 更新应用状态，并将设置持久化到配置文件中。
///
/// # 参数
///
/// * `app` - 可变引用的应用实例
/// * `message` - 设置消息枚举，包含具体的设置变更信息
///
/// # 返回值
///
/// 返回一个 `Task<Message>`，用于执行异步操作。大多数情况下返回 `Task::none()`，
/// 因为设置变更会立即同步处理。
///
/// # 支持的消息类型
///
/// ## 基础自主性设置
/// - `AutonomyLevelChanged`：自主性级别变更
/// - `AutonomyWorkspaceOnlyToggled`：工作空间限制开关
///
/// ## 逐项列表管理
/// - `AutonomyListDraftChanged`：更新指定列表的单项草稿
/// - `AutonomyListAdd`：校验并添加指定列表项
/// - `AutonomyListRemove`：按索引删除指定列表项
///
/// ## 资源限制
/// - `AutonomyMaxActionsPerHourChanged`：每小时最大操作数变更
/// - `AutonomyMaxCostPerDayCentsChanged`：每日最大成本变更
///
/// ## 风险管理
/// - `AutonomyRequireApprovalForMediumRiskToggled`：中等风险审批开关
/// - `AutonomyBlockHighRiskCommandsToggled`：高风险命令阻止开关
///
/// ## Shell 行为
/// - `AutonomyShellRedirectPolicyChanged`：Shell 重定向策略变更
///
/// ## 非 CLI 模式设置
/// - `AutonomyNonCliApprovalApproversChanged`：非 CLI 审批者列表变更
/// - `AutonomyNonCliNaturalLanguageApprovalModeChanged`：非 CLI 默认审批模式变更
/// - `AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged`：按通道的审批模式变更
///
/// ## 其他操作
/// - `AutonomySave`：手动保存设置
/// - `AutonomyHelpOpen`：打开帮助模态框
/// - `AutonomyHelpClose`：关闭帮助模态框
///
/// # 行为说明
///
/// 所有设置变更都会：
/// 1. 更新应用内存中的状态
/// 2. 立即持久化到配置文件
/// 3. 清除任何之前的保存错误信息
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::AutonomyTabSelected(tab) => {
            app.autonomy_settings.active_tab = tab;
            Task::none()
        }

        // 自主性级别变更
        SettingsMessage::AutonomyLevelChanged(v) => {
            app.autonomy_settings.level = v;
            app.autonomy_settings.save_error = None;
            persist_autonomy_settings(app)
        }

        // 工作空间限制开关
        SettingsMessage::AutonomyWorkspaceOnlyToggled(v) => {
            app.autonomy_settings.workspace_only = v;
            app.autonomy_settings.save_error = None;
            persist_autonomy_settings(app)
        }

        SettingsMessage::AutonomyListDraftChanged(kind, value) => {
            app.autonomy_settings.list_editor.set_draft(kind, value);
            app.autonomy_settings.save_error = None;
            Task::none()
        }

        SettingsMessage::AutonomyListAdd(kind) => add_list_item(app, kind),

        SettingsMessage::AutonomyListRemove(kind, index) => remove_list_item(app, kind, index),

        // 每小时最大操作数变更
        SettingsMessage::AutonomyMaxActionsPerHourChanged(v) => {
            app.autonomy_settings.max_actions_per_hour = v.clamp(1, 1_000_000);
            app.autonomy_settings.save_error = None;
            persist_autonomy_settings(app)
        }

        // 每日最大成本变更
        SettingsMessage::AutonomyMaxCostPerDayCentsChanged(v) => {
            app.autonomy_settings.max_cost_per_day_cents = v.clamp(1, 100_000_000);
            app.autonomy_settings.save_error = None;
            persist_autonomy_settings(app)
        }

        // 中等风险审批开关
        SettingsMessage::AutonomyRequireApprovalForMediumRiskToggled(v) => {
            app.autonomy_settings.require_approval_for_medium_risk = v;
            app.autonomy_settings.save_error = None;
            persist_autonomy_settings(app)
        }

        // 高风险命令阻止开关
        SettingsMessage::AutonomyBlockHighRiskCommandsToggled(v) => {
            app.autonomy_settings.block_high_risk_commands = v;
            app.autonomy_settings.save_error = None;
            persist_autonomy_settings(app)
        }

        // Shell 重定向策略变更
        SettingsMessage::AutonomyShellRedirectPolicyChanged(v) => {
            app.autonomy_settings.shell_redirect_policy = v;
            app.autonomy_settings.save_error = None;
            persist_autonomy_settings(app)
        }

        // 非 CLI 模式审批者列表变更
        SettingsMessage::AutonomyNonCliApprovalApproversChanged(v) => {
            app.autonomy_settings.non_cli_approval_approvers_input = v;
            app.autonomy_settings.save_error = None;
            persist_autonomy_settings(app)
        }

        // 非 CLI 模式默认审批模式变更
        SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeChanged(v) => {
            app.autonomy_settings.non_cli_natural_language_approval_mode = v;
            app.autonomy_settings.save_error = None;
            persist_autonomy_settings(app)
        }

        // 按 CLI 模式通道的审批模式变更
        SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(v) => {
            app.autonomy_settings.non_cli_natural_language_approval_mode_by_channel_input = v;
            app.autonomy_settings.save_error = None;
            persist_autonomy_settings(app)
        }

        // 手动保存设置
        SettingsMessage::AutonomySave => {
            app.autonomy_settings.save_error = None;
            persist_autonomy_settings(app)
        }

        // 打开帮助模态框
        SettingsMessage::AutonomyHelpOpen => {
            app.autonomy_settings.show_help_modal = true;
            Task::none()
        }

        // 关闭帮助模态框
        SettingsMessage::AutonomyHelpClose => {
            app.autonomy_settings.show_help_modal = false;
            Task::none()
        }

        // 其他非自主性设置相关的消息，不做处理
        _ => Task::none(),
    }
}
