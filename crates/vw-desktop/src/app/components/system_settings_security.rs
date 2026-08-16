//! 系统安全设置界面组件
//!
//! 本模块提供安全配置的可视化界面，允许用户通过图形界面配置 VibeWindow 的各项安全参数。
//! 所有配置项都对应共享数据库中的 `security` 字段，并统一通过 Gateway 读写。
//!
//! # 主要功能
//!
//! - **沙箱配置**：启用/禁用沙箱、选择沙箱后端（landlock/firejail/bubblewrap/docker）
//! - **资源限制**：设置进程的最大内存、CPU 时长和子进程数量
//! - **审计日志**：配置审计日志的开启、路径、大小限制和签名
//! - **OTP 二次验证**：配置一次性密码验证机制，保护敏感操作
//! - **紧急停止**：配置紧急停止功能和恢复机制
//! - **系统调用异常检测**：监控系统调用行为，检测异常活动
//! - **安全防护**：配置 Canary Token 和语义注入防护
//!
//! # 使用方式
//!
//! 通过 [`view`] 函数创建安全设置界面的 UI 元素，该函数返回一个 Iced 框架的 `Element`。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, danger_action_btn_style, settings_checkbox_style, settings_divider,
    settings_help_button, settings_muted_text_style, settings_page_intro, settings_panel,
    settings_pick_list_menu_style, settings_pick_list_style, settings_section_card,
    settings_segment_button_style, settings_text_input_style, settings_value_badge,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::state::{SecurityListKind, SecuritySettingsTab};
use crate::app::{App, Message, message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, slider, text, text_input,
};
use iced::{Alignment, Element, Length, Padding};

const LIST_EDITOR_MAX_HEIGHT: f32 = 176.0;

pub(super) const SECURITY_TABS: &[SettingsTabDescriptor<SecuritySettingsTab>] = &[
    SettingsTabDescriptor::new(SecuritySettingsTab::Bash, "Bash", &["bash", "命令安全", "policy"]),
    SettingsTabDescriptor::new(
        SecuritySettingsTab::SandboxAndResources,
        "沙箱与资源",
        &["sandbox", "沙箱后端", "资源限制", "内存", "CPU", "子进程"],
    ),
    SettingsTabDescriptor::new(SecuritySettingsTab::Audit, "审计", &["audit", "审计日志", "签名"]),
    SettingsTabDescriptor::new(
        SecuritySettingsTab::OtpAndEstop,
        "OTP 与急停",
        &["OTP", "紧急停止", "E-Stop", "一次性密码"],
    ),
    SettingsTabDescriptor::new(
        SecuritySettingsTab::Syscall,
        "系统调用",
        &["syscall", "异常检测", "基线系统调用"],
    ),
    SettingsTabDescriptor::new(
        SecuritySettingsTab::Guard,
        "防护",
        &["Canary Token", "语义注入防护", "semantic guard"],
    ),
];

#[derive(Clone, PartialEq)]
struct LabeledOption {
    value: &'static str,
    label: &'static str,
}

impl std::fmt::Display for LabeledOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

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

fn list_editor<'a>(
    kind: SecurityListKind,
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    items: &'a [String],
    draft: &'a str,
    pending_remove_index: Option<usize>,
) -> Element<'a, Message> {
    let draft_changed = move |value| {
        Message::Settings(message::SettingsMessage::SecurityListDraftChanged(kind, value))
    };
    let add_message = Message::Settings(message::SettingsMessage::SecurityListAdd(kind));
    let add_button = {
        let button = button(text("添加"))
            .padding([9, 14])
            .style(|theme, status| settings_segment_button_style(theme, status, false));
        if draft.trim().is_empty() { button } else { button.on_press(add_message.clone()) }
    };

    let mut item_rows = column![].spacing(6);
    for (index, item) in items.iter().enumerate() {
        let mut item_content = column![
            row![
                text(item).size(13).width(Length::Fill),
                button(text("删除").size(12))
                    .on_press(Message::Settings(
                        message::SettingsMessage::SecurityListRemoveRequested(kind, index),
                    ))
                    .padding([6, 10])
                    .style(|theme, status| { settings_segment_button_style(theme, status, false) }),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(6);
        if pending_remove_index == Some(index) {
            item_content = item_content.push(
                row![
                    text("确认删除该列表项？").size(12).width(Length::Fill),
                    button(text("取消").size(12))
                        .on_press(Message::Settings(
                            message::SettingsMessage::SecurityListRemoveCanceled,
                        ))
                        .padding([6, 10])
                        .style(|theme, status| {
                            settings_segment_button_style(theme, status, false)
                        }),
                    button(text("确认删除").size(12))
                        .on_press(Message::Settings(
                            message::SettingsMessage::SecurityListRemoveConfirmed(kind, index),
                        ))
                        .padding([6, 10])
                        .style(danger_action_btn_style),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        item_rows = item_rows.push(container(item_content).padding([6, 8]).width(Length::Fill));
    }

    let item_list = container(
        scrollable(container(item_rows).padding(Padding { right: 12.0, ..Padding::default() }))
            .height(Length::Shrink)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
    )
    .max_height(LIST_EDITOR_MAX_HEIGHT)
    .width(Length::Fill);

    column![
        column![text(label).size(13), text(description).size(11).style(settings_muted_text_style),]
            .spacing(4),
        row![
            text_input(placeholder, draft)
                .on_input(draft_changed)
                .on_submit(add_message)
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill),
            add_button,
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        item_list,
    ]
    .spacing(10)
    .padding([14, 0])
    .width(Length::Fill)
    .into()
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

fn bash_bool_row<'a>(
    label: &'static str,
    description: &'static str,
    checked: bool,
    active: bool,
    on_toggle: impl Fn(bool) -> message::SettingsMessage + 'a,
) -> Element<'a, Message> {
    let checkbox =
        checkbox(checked).label(if active { "开启" } else { "已保存，当前不生效" });
    let checkbox = if active {
        checkbox.on_toggle(move |value| Message::Settings(on_toggle(value)))
    } else {
        checkbox
    };
    field_row(label, description, checkbox.style(settings_checkbox_style))
}

fn slider_row<'a>(
    label: &'static str,
    description: &'static str,
    slider: impl Into<Element<'a, Message>>,
    value: impl ToString,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        row![slider.into(), settings_value_badge(value)].spacing(12).align_y(Alignment::Center),
    )
}

fn sandbox_enabled_options() -> [LabeledOption; 3] {
    [
        LabeledOption { value: "auto", label: "自动检测" },
        LabeledOption { value: "true", label: "强制启用" },
        LabeledOption { value: "false", label: "强制禁用" },
    ]
}

fn sandbox_backend_options(enabled: &str) -> Vec<LabeledOption> {
    match enabled {
        "false" => vec![LabeledOption { value: "none", label: "禁止沙箱" }],
        "true" => vec![
            LabeledOption { value: "auto", label: "自动选择" },
            LabeledOption { value: "landlock", label: "Landlock" },
            LabeledOption { value: "firejail", label: "Firejail" },
            LabeledOption { value: "bubblewrap", label: "Bubblewrap" },
            LabeledOption { value: "docker", label: "Docker" },
        ],
        _ => vec![
            LabeledOption { value: "auto", label: "自动选择" },
            LabeledOption { value: "landlock", label: "Landlock" },
            LabeledOption { value: "firejail", label: "Firejail" },
            LabeledOption { value: "bubblewrap", label: "Bubblewrap" },
            LabeledOption { value: "docker", label: "Docker" },
            LabeledOption { value: "none", label: "禁止沙箱" },
        ],
    }
}

fn sandbox_backend_description(backend: &str) -> &'static str {
    match backend {
        "landlock" => {
            "使用 Linux Landlock 约束文件系统与进程权限，适合支持 Landlock 的 Linux 环境。"
        }
        "firejail" => "使用 Firejail 进行沙箱隔离，可额外传入命令行参数定制策略。",
        "bubblewrap" => "使用 Bubblewrap 构造轻量级容器隔离，适合更细粒度的命名空间控制。",
        "docker" => "使用 Docker 容器作为执行边界，适合已有容器环境的部署方式。",
        "none" => "不使用任何沙箱。仅在你明确接受更低隔离级别时使用。",
        _ => "由运行时自动选择可用的最优沙箱实现；无法确定时会回退到安全允许的路径。",
    }
}

/// 创建安全设置界面的视图元素
///
/// 该函数构建一个包含所有安全配置选项的表单界面，包括文本输入框、滑块和复选框等控件。
/// 每个配置项都会触发相应的消息以更新应用状态。
///
/// # 参数
///
/// - `app`: 应用程序状态的不可变引用，从中读取当前的安全配置值
///
/// # 返回值
///
/// 返回一个 Iced `Element`，包含完整的安全设置界面。如果启用了帮助模态框，
/// 则返回包含模态框的堆叠布局。
///
/// # 界面布局
///
/// 界面从上到下依次包含：
/// 1. 标题栏（带帮助按钮）
/// 2. 沙箱配置区域
/// 3. 资源限制配置区域
/// 4. 审计日志配置区域
/// 5. OTP 二次验证配置区域
/// 6. 紧急停止配置区域
/// 7. 系统调用异常检测配置区域
/// 8. 安全防护配置区域
///
/// # 示例
///
/// ```ignore
/// let settings_view = system_settings_security::view(&app);
/// // 将 settings_view 添加到主界面中
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.security_settings;
    let bash_security_row = bool_row(
        "Bash 安全检查",
        "统一控制命令、路径、风险、重定向与 Shell 语法审计；完全访问始终绕过。",
        s.bash.enabled,
        "开启",
        |v| Message::Settings(message::SettingsMessage::SecurityBashSecurityEnabledToggled(v)),
    );
    use message::settings::{BashPolicyCheck as P, BashValidatorCheck as V};
    let policy_active = s.bash.enabled;
    let policy_items_active = policy_active && s.bash.policy.enabled;
    let validator_active = s.bash.enabled;
    let validator_items_active = validator_active && s.bash.validators.enabled;
    let policy_group_row = bash_bool_row(
        "策略检查",
        "关闭后保留各单项值，但命令、路径、风险和重定向策略均不生效。",
        s.bash.policy.enabled,
        policy_active,
        |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::Enabled, v),
    );
    let policy_rows = column![
        policy_group_row,
        settings_divider(),
        bash_bool_row(
            "只读约束",
            "阻止只读会话执行写操作。",
            s.bash.policy.read_only,
            policy_items_active,
            |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::ReadOnly, v)
        ),
        settings_divider(),
        bash_bool_row(
            "危险 Shell 结构",
            "检查危险的 Shell 组合结构。",
            s.bash.policy.unsafe_shell_patterns,
            policy_items_active,
            |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::UnsafeShellPatterns, v)
        ),
        settings_divider(),
        bash_bool_row(
            "命令规则",
            "应用允许命令与规则引擎约束。",
            s.bash.policy.command_rules,
            policy_items_active,
            |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::CommandRules, v)
        ),
        settings_divider(),
        bash_bool_row(
            "工作区路径",
            "限制命令访问工作区外路径。",
            s.bash.policy.workspace_paths,
            policy_items_active,
            |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::WorkspacePaths, v)
        ),
        settings_divider(),
        bash_bool_row(
            "禁止路径",
            "检查明确禁止访问的路径。",
            s.bash.policy.forbidden_paths,
            policy_items_active,
            |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::ForbiddenPaths, v)
        ),
        settings_divider(),
        bash_bool_row(
            "风险等级",
            "应用高、中风险命令策略。",
            s.bash.policy.risk,
            policy_items_active,
            |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::Risk, v)
        ),
        settings_divider(),
        bash_bool_row(
            "重定向",
            "检查 Shell 输入输出重定向。",
            s.bash.policy.redirects,
            policy_items_active,
            |v| message::SettingsMessage::SecurityBashPolicyCheckToggled(P::Redirects, v)
        ),
    ]
    .spacing(0);
    let validator_group_row = bash_bool_row(
        "Shell 语法检测",
        "关闭后保留各 validator 值，但所有 Shell 语法检测均不生效。",
        s.bash.validators.enabled,
        validator_active,
        |v| message::SettingsMessage::SecurityBashValidatorCheckToggled(V::Enabled, v),
    );
    macro_rules! validator_row {
        ($label:literal, $description:literal, $field:ident, $variant:ident) => {
            bash_bool_row(
                $label,
                $description,
                s.bash.validators.$field,
                validator_items_active,
                |v| message::SettingsMessage::SecurityBashValidatorCheckToggled(V::$variant, v),
            )
        };
    }
    let validator_rows = column![
        validator_group_row,
        settings_divider(),
        validator_row!("空命令", "拒绝空白或空命令。", empty_command, EmptyCommand),
        settings_divider(),
        validator_row!("Heredoc", "检查 heredoc 结构。", heredoc, Heredoc),
        settings_divider(),
        validator_row!("Git commit", "检查 Git 提交命令结构。", git_commit, GitCommit),
        settings_divider(),
        validator_row!("jq", "检查 jq 命令结构。", jq, Jq),
        settings_divider(),
        validator_row!("Shell 元字符", "检查敏感 Shell 元字符。", metacharacters, Metacharacters),
        settings_divider(),
        validator_row!("危险变量", "检查危险变量引用。", dangerous_vars, DangerousVars),
        settings_divider(),
        validator_row!("命令替换", "检查命令替换结构。", substitution, Substitution),
        settings_divider(),
        validator_row!("语法重定向", "检查重定向语法。", redirection, Redirection),
        settings_divider(),
        validator_row!("换行注入", "检查命令换行注入。", newline_injection, NewlineInjection),
        settings_divider(),
        validator_row!("IFS 注入", "检查 IFS 注入。", ifs_injection, IfsInjection),
        settings_divider(),
        validator_row!("进程环境", "检查 /proc 环境访问。", proc_environ, ProcEnviron),
        settings_divider(),
        validator_row!("异常命令", "检查格式异常的命令。", malformed, Malformed),
        settings_divider(),
        validator_row!("混淆参数", "检查混淆命令参数。", obfuscated_flags, ObfuscatedFlags),
        settings_divider(),
        validator_row!("反斜杠", "检查可疑反斜杠。", backslash, Backslash),
        settings_divider(),
        validator_row!("花括号扩展", "检查花括号扩展。", brace_expansion, BraceExpansion),
        settings_divider(),
        validator_row!(
            "Unicode 空白",
            "检查 Unicode 空白字符。",
            unicode_whitespace,
            UnicodeWhitespace
        ),
        settings_divider(),
        validator_row!("哈希注释", "检查哈希注释结构。", hash_comment, HashComment),
        settings_divider(),
        validator_row!(
            "注释引号同步",
            "检查注释与引号状态不同步。",
            comment_quote_desync,
            CommentQuoteDesync
        ),
        settings_divider(),
        validator_row!(
            "引号内换行",
            "拒绝引号内换行；默认关闭以支持多行提交消息。",
            quoted_newline,
            QuotedNewline
        ),
        settings_divider(),
        validator_row!("Zsh 危险结构", "检查 Zsh 危险结构。", zsh_dangerous, ZshDangerous),
        settings_divider(),
        validator_row!("控制字符", "检查命令中的控制字符。", control_chars, ControlChars),
    ]
    .spacing(0);
    let enabled_options = sandbox_enabled_options();
    let enabled_selected =
        enabled_options.iter().find(|option| option.value == s.sandbox_enabled_input).cloned();
    let backend_options = sandbox_backend_options(&s.sandbox_enabled_input);
    let backend_selected =
        backend_options.iter().find(|option| option.value == s.sandbox_backend_input).cloned();
    let help_btn =
        settings_help_button(Message::Settings(message::SettingsMessage::SecurityHelpOpen));
    let sandbox_enabled_row = field_row(
        "沙箱启用",
        "选择自动检测、强制启用或强制禁用沙箱。",
        pick_list(enabled_options, enabled_selected, |value| {
            Message::Settings(message::SettingsMessage::SecuritySandboxEnabledChanged(
                value.value.to_string(),
            ))
        })
        .padding([10, 14])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fixed(280.0)),
    );

    let sandbox_backend_row = field_row(
        "沙箱后端",
        "根据启用策略收窄可选值，避免互相矛盾的配置。",
        pick_list(backend_options, backend_selected, |value| {
            Message::Settings(message::SettingsMessage::SecuritySandboxBackendChanged(
                value.value.to_string(),
            ))
        })
        .padding([10, 14])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fixed(280.0)),
    );

    let sandbox_backend_card =
        settings_section_card("后端说明", sandbox_backend_description(&s.sandbox_backend_input));

    let firejail_args_row = list_editor(
        SecurityListKind::SandboxFirejailArgs,
        "Firejail 参数",
        "仅在使用 Firejail 后端时生效，每次添加一个参数。",
        "例如 --private",
        &s.sandbox_firejail_args,
        s.list_editor.draft(SecurityListKind::SandboxFirejailArgs),
        s.list_editor.pending_remove_index(SecurityListKind::SandboxFirejailArgs),
    );

    let max_memory_row = slider_row(
        "最大内存",
        "限制执行进程可使用的最大内存。",
        slider(32.0..=65_536.0, s.resources_max_memory_mb as f32, |v| {
            Message::Settings(message::SettingsMessage::SecurityResourcesMaxMemoryMbChanged(
                v.round() as u32,
            ))
        })
        .width(Length::Fill),
        format!("{} MB", s.resources_max_memory_mb),
    );

    let max_cpu_row = slider_row(
        "最大 CPU 时长",
        "限制单次执行可消耗的 CPU 时间。",
        slider(1.0..=3600.0, s.resources_max_cpu_time_seconds as f32, |v| {
            Message::Settings(message::SettingsMessage::SecurityResourcesMaxCpuTimeSecondsChanged(
                v.round() as u64,
            ))
        })
        .width(Length::Fill),
        format!("{} s", s.resources_max_cpu_time_seconds),
    );

    let max_subprocesses_row = slider_row(
        "最大子进程数",
        "限制执行过程中允许派生的子进程数量。",
        slider(1.0..=10_000.0, s.resources_max_subprocesses as f32, |v| {
            Message::Settings(message::SettingsMessage::SecurityResourcesMaxSubprocessesChanged(
                v.round() as u32,
            ))
        })
        .width(Length::Fill),
        s.resources_max_subprocesses,
    );

    let memory_monitoring_row = bool_row(
        "内存监控",
        "持续监控执行进程的内存占用。",
        s.resources_memory_monitoring,
        "开启",
        |v| {
            Message::Settings(message::SettingsMessage::SecurityResourcesMemoryMonitoringToggled(v))
        },
    );

    let audit_enabled_row = bool_row(
        "审计日志",
        "记录安全相关动作和事件。",
        s.audit_enabled,
        "开启",
        |v| Message::Settings(message::SettingsMessage::SecurityAuditEnabledToggled(v)),
    );

    let audit_log_path_row = text_row(
        "审计日志路径",
        "指定审计日志文件的写入位置。",
        "audit.log",
        &s.audit_log_path,
        |v| Message::Settings(message::SettingsMessage::SecurityAuditLogPathChanged(v)),
    );

    let audit_size_row = slider_row(
        "审计大小上限",
        "限制单个审计日志文件的最大大小。",
        slider(1.0..=10_000.0, s.audit_max_size_mb as f32, |v| {
            Message::Settings(message::SettingsMessage::SecurityAuditMaxSizeMbChanged(
                v.round() as u32
            ))
        })
        .width(Length::Fill),
        format!("{} MB", s.audit_max_size_mb),
    );

    let audit_sign_row = bool_row(
        "审计签名",
        "为审计事件添加签名，提高完整性。",
        s.audit_sign_events,
        "签名事件",
        |v| Message::Settings(message::SettingsMessage::SecurityAuditSignEventsToggled(v)),
    );

    let otp_enabled_row =
        bool_row("OTP", "启用敏感动作的二次验证。", s.otp_enabled, "开启", |v| {
            Message::Settings(message::SettingsMessage::SecurityOtpEnabledToggled(v))
        });

    let otp_method_row = text_row(
        "OTP 方法",
        "支持 totp、pairing、cli-prompt。",
        "totp | pairing | cli-prompt",
        &s.otp_method_input,
        |v| Message::Settings(message::SettingsMessage::SecurityOtpMethodChanged(v)),
    );

    let otp_ttl_row = slider_row(
        "OTP TTL",
        "单个 OTP 令牌的有效时长。",
        slider(1.0..=600.0, s.otp_token_ttl_secs as f32, |v: f32| {
            Message::Settings(message::SettingsMessage::SecurityOtpTokenTtlSecsChanged(
                v.round() as u64
            ))
        })
        .width(Length::Fill),
        format!("{} s", s.otp_token_ttl_secs),
    );

    let otp_cache_row = slider_row(
        "OTP 缓存时长",
        "验证结果的缓存有效期。",
        slider(1.0..=3600.0, s.otp_cache_valid_secs as f32, |v: f32| {
            Message::Settings(message::SettingsMessage::SecurityOtpCacheValidSecsChanged(
                v.round() as u64
            ))
        })
        .width(Length::Fill),
        format!("{} s", s.otp_cache_valid_secs),
    );

    let otp_actions_row = list_editor(
        SecurityListKind::OtpGatedActions,
        "OTP 动作白名单",
        "需要 OTP 验证的敏感动作，每次添加一个。",
        "例如 shell",
        &s.otp_gated_actions,
        s.list_editor.draft(SecurityListKind::OtpGatedActions),
        s.list_editor.pending_remove_index(SecurityListKind::OtpGatedActions),
    );

    let otp_domains_row = list_editor(
        SecurityListKind::OtpGatedDomains,
        "OTP 域名规则",
        "需要 OTP 验证的网络域名规则，每次添加一个。",
        "例如 admin.example.com",
        &s.otp_gated_domains,
        s.list_editor.draft(SecurityListKind::OtpGatedDomains),
        s.list_editor.pending_remove_index(SecurityListKind::OtpGatedDomains),
    );

    let otp_domain_categories_row = list_editor(
        SecurityListKind::OtpGatedDomainCategories,
        "OTP 域名分类",
        "按域名分类启用 OTP 门控，每次添加一个分类。",
        "例如 payments",
        &s.otp_gated_domain_categories,
        s.list_editor.draft(SecurityListKind::OtpGatedDomainCategories),
        s.list_editor.pending_remove_index(SecurityListKind::OtpGatedDomainCategories),
    );

    let estop_enabled_row = bool_row(
        "紧急停止",
        "启用紧急停止状态机。",
        s.estop_enabled,
        "开启",
        |v| Message::Settings(message::SettingsMessage::SecurityEstopEnabledToggled(v)),
    );

    let estop_state_file_row = text_row(
        "E-Stop 状态文件",
        "指定紧急停止状态文件位置。",
        vw_config_types::paths::ESTOP_STATE_FILE_PATH,
        &s.estop_state_file,
        |v| Message::Settings(message::SettingsMessage::SecurityEstopStateFileChanged(v)),
    );

    let estop_require_otp_row = bool_row(
        "恢复需 OTP",
        "从 E-Stop 恢复时要求额外 OTP 验证。",
        s.estop_require_otp_to_resume,
        "开启",
        |v| Message::Settings(message::SettingsMessage::SecurityEstopRequireOtpToResumeToggled(v)),
    );

    let syscall_enabled_row = bool_row(
        "系统调用异常检测",
        "启用系统调用层面的异常行为检测。",
        s.syscall_anomaly_enabled,
        "开启",
        |v| Message::Settings(message::SettingsMessage::SecuritySyscallAnomalyEnabledToggled(v)),
    );

    let syscall_strict_row = bool_row(
        "严格模式",
        "使用更严格的异常检测规则。",
        s.syscall_anomaly_strict_mode,
        "开启",
        |v| Message::Settings(message::SettingsMessage::SecuritySyscallAnomalyStrictModeToggled(v)),
    );

    let syscall_alert_unknown_row = bool_row(
        "未知系统调用告警",
        "遇到未知系统调用时立即告警。",
        s.syscall_anomaly_alert_on_unknown_syscall,
        "开启",
        |v| {
            Message::Settings(
                message::SettingsMessage::SecuritySyscallAnomalyAlertOnUnknownSyscallToggled(v),
            )
        },
    );

    let denied_row = slider_row(
        "每分钟拒绝阈值",
        "单位时间内拒绝事件的上限。",
        slider(1.0..=10_000.0, s.syscall_anomaly_max_denied_events_per_minute as f32, |v: f32| {
            Message::Settings(
                message::SettingsMessage::SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged(
                    v.round() as u32,
                ),
            )
        })
        .width(Length::Fill),
        s.syscall_anomaly_max_denied_events_per_minute,
    );

    let total_row = slider_row(
        "每分钟总事件阈值",
        "单位时间内允许的系统调用事件总数。",
        slider(1.0..=100_000.0, s.syscall_anomaly_max_total_events_per_minute as f32, |v: f32| {
            Message::Settings(
                message::SettingsMessage::SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged(
                    v.round() as u32,
                ),
            )
        })
        .width(Length::Fill),
        s.syscall_anomaly_max_total_events_per_minute,
    );

    let alerts_row = slider_row(
        "每分钟告警阈值",
        "限制单位时间内的告警触发数量。",
        slider(1.0..=10_000.0, s.syscall_anomaly_max_alerts_per_minute as f32, |v: f32| {
            Message::Settings(
                message::SettingsMessage::SecuritySyscallAnomalyMaxAlertsPerMinuteChanged(
                    v.round() as u32,
                ),
            )
        })
        .width(Length::Fill),
        s.syscall_anomaly_max_alerts_per_minute,
    );

    let cooldown_row = slider_row(
        "告警冷却",
        "两次告警之间的最小间隔。",
        slider(1.0..=3600.0, s.syscall_anomaly_alert_cooldown_secs as f32, |v: f32| {
            Message::Settings(
                message::SettingsMessage::SecuritySyscallAnomalyAlertCooldownSecsChanged(
                    v.round() as u64
                ),
            )
        })
        .width(Length::Fill),
        format!("{} s", s.syscall_anomaly_alert_cooldown_secs),
    );

    let syscall_log_path_row = text_row(
        "系统调用日志路径",
        "指定系统调用异常日志的写入位置。",
        "syscall-anomalies.log",
        &s.syscall_anomaly_log_path,
        |v| Message::Settings(message::SettingsMessage::SecuritySyscallAnomalyLogPathChanged(v)),
    );

    let baseline_syscalls_row = list_editor(
        SecurityListKind::SyscallAnomalyBaselineSyscalls,
        "系统调用基线",
        "允许的系统调用白名单基线，每次添加一个。",
        "例如 openat",
        &s.syscall_anomaly_baseline_syscalls,
        s.list_editor.draft(SecurityListKind::SyscallAnomalyBaselineSyscalls),
        s.list_editor.pending_remove_index(SecurityListKind::SyscallAnomalyBaselineSyscalls),
    );

    let canary_row = bool_row(
        "Canary Token",
        "启用蜜罐令牌与外泄检测。",
        s.canary_tokens,
        "开启",
        |v| Message::Settings(message::SettingsMessage::SecurityCanaryTokensToggled(v)),
    );

    let semantic_guard_row = bool_row(
        "语义注入防护",
        "启用语义层面的注入攻击检测。",
        s.semantic_guard,
        "开启",
        |v| Message::Settings(message::SettingsMessage::SecuritySemanticGuardToggled(v)),
    );

    let semantic_collection_row = text_row(
        "语义集合",
        "指定用于语义分析的数据集合。",
        "semantic_guard",
        &s.semantic_guard_collection,
        |v| Message::Settings(message::SettingsMessage::SecuritySemanticGuardCollectionChanged(v)),
    );

    let semantic_threshold_row = slider_row(
        "语义阈值",
        "语义防护的触发阈值。",
        slider(0.0..=1.0, s.semantic_guard_threshold as f32, |v| {
            Message::Settings(message::SettingsMessage::SecuritySemanticGuardThresholdChanged(
                v as f64,
            ))
        })
        .width(Length::Fill),
        format!("{:.2}", s.semantic_guard_threshold),
    );

    let mut sandbox_rows = column![sandbox_enabled_row, settings_divider(), sandbox_backend_row]
        .spacing(0)
        .width(Length::Fill);
    if s.sandbox_backend_input == "firejail" {
        sandbox_rows = sandbox_rows.push(settings_divider()).push(firejail_args_row);
    }

    let tabs = SECURITY_TABS.iter().fold(
        row![].spacing(8).align_y(Alignment::Center),
        |row, descriptor| {
            row.push(tab_button(
                descriptor.label,
                s.active_tab == descriptor.tab,
                Message::Settings(message::SettingsMessage::SecurityTabSelected(descriptor.tab)),
            ))
        },
    );

    let content: Element<'_, Message> = match s.active_tab {
        SecuritySettingsTab::Bash => column![
            settings_section_card(
                "Bash 安全检查",
                "优先级为总开关 → 分组开关 → 单项开关；关闭父级不会改写已保存的子项。"
            ),
            settings_panel(bash_security_row),
            settings_section_card("策略检查", "命令、路径、风险与重定向策略。"),
            settings_panel(policy_rows),
            settings_section_card("Shell 语法检测", "按 validator 独立控制 Shell 语法检查。"),
            settings_panel(validator_rows),
        ]
        .spacing(16)
        .into(),
        SecuritySettingsTab::SandboxAndResources => column![
            settings_section_card("沙箱", "控制是否启用沙箱以及选择具体后端。"),
            settings_panel(sandbox_rows),
            sandbox_backend_card,
            settings_section_card("资源限制", "约束进程的内存、CPU 与子进程数量。"),
            settings_panel(
                column![
                    max_memory_row,
                    settings_divider(),
                    max_cpu_row,
                    settings_divider(),
                    max_subprocesses_row,
                    settings_divider(),
                    memory_monitoring_row,
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
        SecuritySettingsTab::Audit => column![
            settings_section_card("审计日志", "记录并限制安全事件日志。"),
            settings_panel(
                column![
                    audit_enabled_row,
                    settings_divider(),
                    audit_log_path_row,
                    settings_divider(),
                    audit_size_row,
                    settings_divider(),
                    audit_sign_row,
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
        SecuritySettingsTab::OtpAndEstop => column![
            settings_section_card("OTP", "对敏感动作启用二次验证。"),
            settings_panel(
                column![
                    otp_enabled_row,
                    settings_divider(),
                    otp_method_row,
                    settings_divider(),
                    otp_ttl_row,
                    settings_divider(),
                    otp_cache_row,
                    settings_divider(),
                    otp_actions_row,
                    settings_divider(),
                    otp_domains_row,
                    settings_divider(),
                    otp_domain_categories_row,
                ]
                .spacing(0)
            ),
            settings_section_card("紧急停止", "配置 E-Stop 状态与恢复要求。"),
            settings_panel(
                column![
                    estop_enabled_row,
                    settings_divider(),
                    estop_state_file_row,
                    settings_divider(),
                    estop_require_otp_row,
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
        SecuritySettingsTab::Syscall => column![
            settings_section_card("系统调用异常检测", "监控系统调用行为并限制告警节奏。"),
            settings_panel(
                column![
                    syscall_enabled_row,
                    settings_divider(),
                    syscall_strict_row,
                    settings_divider(),
                    syscall_alert_unknown_row,
                    settings_divider(),
                    denied_row,
                    settings_divider(),
                    total_row,
                    settings_divider(),
                    alerts_row,
                    settings_divider(),
                    cooldown_row,
                    settings_divider(),
                    syscall_log_path_row,
                    settings_divider(),
                    baseline_syscalls_row,
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
        SecuritySettingsTab::Guard => column![
            settings_section_card("防护", "配置 Canary Token 与语义注入防护。"),
            settings_panel(
                column![
                    canary_row,
                    settings_divider(),
                    semantic_guard_row,
                    settings_divider(),
                    semantic_collection_row,
                    settings_divider(),
                    semantic_threshold_row,
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
    };

    let col = column![
        row![
            settings_page_intro("安全配置", "配置沙箱、资源限制、审计与安全门控策略。"),
            container(text(" ")).width(Length::Fill),
            help_btn,
        ]
        .align_y(Alignment::Start),
        tabs,
        content,
    ]
    .spacing(16)
    .width(Length::Fill);

    col.into()
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.security_settings;
    if !s.show_help_modal {
        return dialog;
    }

    let help_text = r#"安全配置说明

一、作用
- security 用于配置沙箱、资源限制、审计、OTP 门控、紧急停止与系统调用异常检测。

二、关键字段
- bash.enabled: 总开关，默认 false；完全访问始终绕过。
- bash.policy.enabled / bash.validators.enabled: 分组开关。
- bash.policy.* / bash.validators.*: 单项开关；有效状态为总开关 && 分组开关 && 单项开关。
- sandbox.enabled: auto/true/false。
- sandbox.backend: auto | landlock | firejail | bubblewrap | docker | none。
- resources.*: 进程资源上限。
- audit.*: 审计日志行为。
- otp.*: 敏感动作二次验证。
- estop.*: 紧急停止状态机。
- syscall_anomaly.*: 系统调用异常检测。
- canary_tokens / semantic_guard*: 注入与外泄防护。

三、示例
{
  "security": {
    "bash": {
      "enabled": true,
      "policy": {"enabled": true, "read_only": true, "redirects": true},
      "validators": {"enabled": true, "heredoc": true, "quoted_newline": false}
    },
    "sandbox": {"enabled": null, "backend": "auto", "firejail_args": []},
    "resources": {
      "max_memory_mb": 512,
      "max_cpu_time_seconds": 60,
      "max_subprocesses": 10,
      "memory_monitoring": true
    },
    "audit": {"enabled": true, "log_path": "audit.log", "max_size_mb": 100, "sign_events": false}
  }
}
"#;

    crate::app::components::system_settings_common::with_settings_help_modal(
        app,
        dialog,
        "Security 配置帮助",
        help_text,
        Message::Settings(message::SettingsMessage::SecurityHelpClose),
    )
}
#[cfg(test)]
#[path = "system_settings_security_tests.rs"]
mod system_settings_security_tests;
