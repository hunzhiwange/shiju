//! 系统自治配置设置界面组件
//!
//! 本模块提供自治配置（Autonomy Settings）的可视化编辑界面，
//! 用于配置代理运行时的权限边界、预算限制和审批策略。
//!
//! # 主要功能
//!
//! - **权限级别设置**：配置代理的自治级别（只读/监督/完全）
//! - **预算管理**：设置每小时动作上限和每日成本上限
//! - **安全策略**：配置中风险审批、高风险阻断、重定向策略等
//! - **路径与命令控制**：管理允许的命令、禁止的路径、允许的根目录等
//! - **审批流程**：配置自动审批工具、始终询问工具、审批人列表
//! - **通道模式**：按通道配置自然语言审批模式
//!
//! # 配置持久化
//!
//! 所有配置项会通过 Gateway 保存到共享数据库的 `autonomy` 字段中。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_divider, settings_help_button,
    settings_muted_text_style, settings_page_intro, settings_panel, settings_section_card,
    settings_segment_button_style, settings_text_input_style, settings_value_badge,
};
use crate::app::{App, Message, message};
use iced::widget::{button, checkbox, column, container, row, slider, text, text_input};
use iced::{Alignment, Element, Length};
use vw_config_types::security::NonCliNaturalLanguageApprovalMode;
use vw_config_types::security::{AutonomyLevel, ShellRedirectPolicy};

/// 构建与可观测性配置一致的 Tab 风格按钮。
///
/// 当前选中项使用主题主色高亮，悬停项显示弱背景，未选中项保持透明背景。
fn tab_button<'a>(
    label: &'static str,
    is_active: bool,
    message: Message,
) -> iced::widget::Button<'a, Message> {
    button(text(label)).on_press(message).padding([8, 14]).style(
        move |theme: &iced::Theme, status| settings_segment_button_style(theme, status, is_active),
    )
}

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

fn text_row<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        text_input(placeholder, value)
            .on_input(on_input)
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    )
}

fn bool_row<'a>(
    label: &'static str,
    description: &'static str,
    checked: bool,
    checkbox_label: &'static str,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        checkbox(checked).label(checkbox_label).on_toggle(on_toggle).style(settings_checkbox_style),
    )
}

/// 构建自治配置设置界面的视图
///
/// 该函数创建一个完整的自治配置编辑界面，包含所有配置项的输入控件和帮助信息。
/// 界面采用垂直布局，每个配置项占据一行，并提供帮助按钮和模态框说明。
///
/// # 参数
///
/// - `app`: 应用状态引用，包含当前的自治配置数据
///
/// # 返回值
///
/// 返回构建好的 Iced UI 元素，包含所有配置控件和可选的帮助模态框
///
/// # 界面布局
///
/// 界面从上到下依次包含：
/// 1. 标题栏（标题 + 帮助按钮）
/// 2. 配置文件路径说明
/// 3. 权限级别选择器
/// 4. 工作区限制开关
/// 5. 每小时动作上限滑块
/// 6. 每日成本上限滑块
/// 7. 中风险审批开关
/// 8. 高风险阻断开关
/// 9. 重定向策略选择器
/// 10. 自然语言审批模式选择器
/// 11. 允许命令输入框
/// 12. 禁止路径输入框
/// 13. 环境变量透传输入框
/// 14. 自动审批工具输入框
/// 15. 始终询问工具输入框
/// 16. 允许根目录输入框
/// 17. 非 CLI 排除工具输入框
/// 18. 审批人列表输入框
/// 19. 按通道模式输入框
/// 20. 输入格式提示
/// 21. 错误信息（如有）
///
/// # 示例
///
/// ```ignore
/// let element = view(&app);
/// // 将 element 添加到 Iced 应用中显示
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.autonomy_settings;
    let help_btn =
        settings_help_button(Message::Settings(message::SettingsMessage::AutonomyHelpOpen));

    let level_row = field_row(
        "级别",
        "决定代理默认可执行的动作范围。",
        row![
            tab_button(
                "只读",
                s.level == AutonomyLevel::ReadOnly,
                Message::Settings(message::SettingsMessage::AutonomyLevelChanged(
                    AutonomyLevel::ReadOnly,
                )),
            ),
            tab_button(
                "监督",
                s.level == AutonomyLevel::Supervised,
                Message::Settings(message::SettingsMessage::AutonomyLevelChanged(
                    AutonomyLevel::Supervised,
                )),
            ),
            tab_button(
                "完全",
                s.level == AutonomyLevel::Full,
                Message::Settings(message::SettingsMessage::AutonomyLevelChanged(
                    AutonomyLevel::Full,
                )),
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    let workspace_only_row = bool_row(
        "工作区限制",
        "仅允许在工作区路径内操作，除非显式加入 allowed_roots。",
        s.workspace_only,
        "仅限工作区",
        |v| Message::Settings(message::SettingsMessage::AutonomyWorkspaceOnlyToggled(v)),
    );

    let max_actions_row = field_row(
        "每小时动作上限",
        "限制自治过程中每小时可执行的动作数量。",
        row![
            slider(1.0..=10_000.0, s.max_actions_per_hour as f32, |v| Message::Settings(
                message::SettingsMessage::AutonomyMaxActionsPerHourChanged(v.round() as u32)
            ))
            .width(Length::Fill),
            settings_value_badge(s.max_actions_per_hour),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    );

    let max_cost_row = field_row(
        "每日成本上限(分)",
        "限制自治行为每天可消耗的成本预算。",
        row![
            slider(1.0..=1_000_000.0, s.max_cost_per_day_cents as f32, |v| Message::Settings(
                message::SettingsMessage::AutonomyMaxCostPerDayCentsChanged(v.round() as u32)
            ))
            .width(Length::Fill),
            settings_value_badge(s.max_cost_per_day_cents),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    );

    let approve_medium_row = bool_row(
        "中风险审批",
        "对中风险动作触发审批，而不是直接执行。",
        s.require_approval_for_medium_risk,
        "中风险需审批",
        |v| {
            Message::Settings(
                message::SettingsMessage::AutonomyRequireApprovalForMediumRiskToggled(v),
            )
        },
    );

    let block_high_row = bool_row(
        "高风险阻断",
        "对高风险命令直接阻断，不进入执行链路。",
        s.block_high_risk_commands,
        "阻断高风险",
        |v| Message::Settings(message::SettingsMessage::AutonomyBlockHighRiskCommandsToggled(v)),
    );

    let redirect_row = field_row(
        "重定向策略",
        "控制 shell 命令中的输出重定向如何处理。",
        row![
            tab_button(
                "阻断",
                s.shell_redirect_policy == ShellRedirectPolicy::Block,
                Message::Settings(message::SettingsMessage::AutonomyShellRedirectPolicyChanged(
                    ShellRedirectPolicy::Block,
                ),),
            ),
            tab_button(
                "移除",
                s.shell_redirect_policy == ShellRedirectPolicy::Strip,
                Message::Settings(message::SettingsMessage::AutonomyShellRedirectPolicyChanged(
                    ShellRedirectPolicy::Strip,
                ),),
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    let mode_row = field_row(
        "自然语言审批",
        "控制非 CLI 场景下自然语言请求的执行方式。",
        row![
            tab_button(
                "禁用",
                s.non_cli_natural_language_approval_mode
                    == NonCliNaturalLanguageApprovalMode::Disabled,
                Message::Settings(
                    message::SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeChanged(
                        NonCliNaturalLanguageApprovalMode::Disabled,
                    ),
                ),
            ),
            tab_button(
                "请求确认",
                s.non_cli_natural_language_approval_mode
                    == NonCliNaturalLanguageApprovalMode::RequestConfirm,
                Message::Settings(
                    message::SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeChanged(
                        NonCliNaturalLanguageApprovalMode::RequestConfirm,
                    ),
                ),
            ),
            tab_button(
                "直接执行",
                s.non_cli_natural_language_approval_mode
                    == NonCliNaturalLanguageApprovalMode::Direct,
                Message::Settings(
                    message::SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeChanged(
                        NonCliNaturalLanguageApprovalMode::Direct,
                    ),
                ),
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    let allowed_commands_row = text_row(
        "允许命令",
        "Shell 白名单，支持逗号或换行分隔。",
        "git, npm, cargo",
        &s.allowed_commands_input,
        |v| Message::Settings(message::SettingsMessage::AutonomyAllowedCommandsChanged(v)),
    );

    let forbidden_paths_row = text_row(
        "禁止路径",
        "限制代理访问这些路径，即使在工作区外也保持拒绝。",
        "/etc, /root, ...",
        &s.forbidden_paths_input,
        |v| Message::Settings(message::SettingsMessage::AutonomyForbiddenPathsChanged(v)),
    );

    let shell_env_row = text_row(
        "环境变量透传",
        "允许透传到 shell 的环境变量名列表。",
        "FOO, BAR",
        &s.shell_env_passthrough_input,
        |v| Message::Settings(message::SettingsMessage::AutonomyShellEnvPassthroughChanged(v)),
    );

    let auto_approve_row = text_row(
        "自动审批",
        "无需额外审批即可执行的工具列表。",
        "file_read, memory_recall",
        &s.auto_approve_input,
        |v| Message::Settings(message::SettingsMessage::AutonomyAutoApproveChanged(v)),
    );

    let always_ask_row = text_row(
        "始终询问",
        "无论风险等级如何，都要求确认的工具列表。",
        "tool_a, tool_b",
        &s.always_ask_input,
        |v| Message::Settings(message::SettingsMessage::AutonomyAlwaysAskChanged(v)),
    );

    let roots_row = text_row(
        "允许根目录",
        "额外允许访问的目录列表。",
        "/path/a, ~/path/b",
        &s.allowed_roots_input,
        |v| Message::Settings(message::SettingsMessage::AutonomyAllowedRootsChanged(v)),
    );

    let excluded_tools_row = text_row(
        "非 CLI 排除工具",
        "在非 CLI 通道里默认禁用的工具列表。",
        "shell, file_write, ...",
        &s.non_cli_excluded_tools_input,
        |v| Message::Settings(message::SettingsMessage::AutonomyNonCliExcludedToolsChanged(v)),
    );

    let approvers_row = text_row(
        "审批人列表",
        "格式为 channel:user，支持逗号或换行分隔。",
        "telegram:alice, *:bob",
        &s.non_cli_approval_approvers_input,
        |v| Message::Settings(message::SettingsMessage::AutonomyNonCliApprovalApproversChanged(v)),
    );

    let mode_by_channel_row =
        text_row(
            "按通道模式",
            "格式为 channel:mode；列表字段支持逗号或换行分隔。",
            "telegram:direct, discord:request_confirm",
            &s.non_cli_natural_language_approval_mode_by_channel_input,
            |v| {
                Message::Settings(
            message::SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(v),
        )
            },
        );

    let col = column![
        row![
            settings_page_intro("自治配置", "配置代理的权限边界、预算限制与审批规则。"),
            container(text(" ")).width(Length::Fill),
            help_btn,
        ]
        .align_y(Alignment::Start),
        settings_section_card("权限与审批", "核心执行级别、审批阈值与重定向策略。"),
        settings_panel(
            column![
                level_row,
                settings_divider(),
                workspace_only_row,
                settings_divider(),
                approve_medium_row,
                settings_divider(),
                block_high_row,
                settings_divider(),
                redirect_row,
                settings_divider(),
                mode_row,
            ]
            .spacing(0)
        ),
        settings_section_card("预算", "限制单位时间的动作量与成本。"),
        settings_panel(column![max_actions_row, settings_divider(), max_cost_row].spacing(0)),
        settings_section_card("工具与路径", "为 shell 命令、路径访问和自动审批设置边界。"),
        settings_panel(
            column![
                allowed_commands_row,
                settings_divider(),
                forbidden_paths_row,
                settings_divider(),
                shell_env_row,
                settings_divider(),
                auto_approve_row,
                settings_divider(),
                always_ask_row,
                settings_divider(),
                roots_row,
            ]
            .spacing(0)
        ),
        settings_section_card("非 CLI 暴露", "控制外部通道中的工具暴露与审批人。"),
        settings_panel(
            column![
                excluded_tools_row,
                settings_divider(),
                approvers_row,
                settings_divider(),
                mode_by_channel_row,
            ]
            .spacing(0)
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    col.into()
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.autonomy_settings;
    if !s.show_help_modal {
        return dialog;
    }

    let help_text = r#"自治配置说明

一、作用
- autonomy 控制代理执行权限边界、预算与审批策略。
- 该配置影响 shell、路径访问、非 CLI 工具暴露与审批流程。

二、字段说明（常用）
1) level
- read_only | supervised | full。

2) workspace_only
- true 时仅允许工作区路径（除 allowed_roots）。

3) allowed_commands / forbidden_paths
- shell 命令白名单与路径黑名单。

4) max_actions_per_hour / max_cost_per_day_cents
- 每小时动作与每日成本预算上限。

5) require_approval_for_medium_risk / block_high_risk_commands
- 中风险需要审批；高风险命令可直接阻断。

6) shell_redirect_policy
- block（默认）或 strip。

7) non_cli_*
- 控制非 CLI 工具暴露与审批人、自然语言审批模式。

三、示例
{
  "autonomy": {
    "level": "supervised",
    "workspace_only": true,
    "allowed_commands": [
      "git",
      "npm",
      "cargo",
      "ls",
      "cat",
      "grep",
      "find",
      "echo",
      "pwd",
      "wc",
      "head",
      "tail",
      "date"
    ],
    "forbidden_paths": [
      "/etc",
      "/root",
      "/home",
      "/usr",
      "/bin",
      "/sbin",
      "/lib",
      "/opt",
      "/boot",
      "/dev",
      "/proc",
      "/sys",
      "/var",
      "/tmp",
      "~/.ssh",
      "~/.gnupg",
      "~/.aws",
      "~/.config"
    ],
    "max_actions_per_hour": 20,
    "max_cost_per_day_cents": 500,
    "require_approval_for_medium_risk": true,
    "block_high_risk_commands": true,
    "shell_redirect_policy": "block",
    "shell_env_passthrough": [],
    "auto_approve": [
      "file_read",
      "memory_recall"
    ],
    "always_ask": [],
    "allowed_roots": [],
    "non_cli_excluded_tools": [
      "shell",
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
    "AgentTool",
      "screenshot",
      "image_info"
    ],
    "non_cli_approval_approvers": [],
    "non_cli_natural_language_approval_mode": "direct",
    "non_cli_natural_language_approval_mode_by_channel": {}
  }
}
"#;

    crate::app::components::system_settings_common::with_settings_help_modal(
        app,
        dialog,
        "自治配置帮助",
        help_text,
        Message::Settings(message::SettingsMessage::AutonomyHelpClose),
    )
}
#[cfg(test)]
#[path = "system_settings_autonomy_tests.rs"]
mod system_settings_autonomy_tests;
